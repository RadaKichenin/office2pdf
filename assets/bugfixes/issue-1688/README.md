# The gap below a DOCX table

A DOCX table carries no vertical spacing of its own: in Word the gap above it
is the preceding paragraph's `w:spacing w:after` and the gap below it is the
following paragraph's `w:spacing w:before`. office2pdf emitted the table as a
bare Typst `#table`, which let Typst's own default paragraph spacing open
1.2em — 13.2pt at the probe's 11pt — below every table whose next paragraph
states no `w:before`. Word's default template states none, so any document
that puts body text under a table gained the gap.

## Why a bare `#table` leaks a gap

Typst resolves the gap between two blocks by *weakness*, not by simple
maximum. In `typst-layout` 0.15.1 — the version this repository pins —
`flow/collect.rs` tags a block's gap in `Collector::block`:

```rust
let fallback = LazyCell::new(|| styles.resolve(ParElem::spacing));
let spacing = |amount| match amount {
    Smart::Auto => Child::Rel((*fallback).into(), 4),
    Smart::Custom(Spacing::Rel(rel)) => Child::Rel(rel.resolve(styles), 3),
    Smart::Custom(Spacing::Fr(fr)) => Child::Fr(fr, 2),
};
```

So an `auto` gap carries the `par.spacing` fallback at weakness 4 and a stated
one weakness 3. `flow/distribute.rs`'s `keep_weak_rel_spacing` then decides
which of two adjacent gaps survives:

```rust
Item::Abs(prev_amount, prev_weakness @ 1..) => {
    if weakness <= prev_weakness
        && (weakness < prev_weakness || amount > prev_amount)
    {
        self.regions.size.y -= amount - prev_amount;
        *item = Item::Abs(amount, weakness);
    }
    return false;
}
```

The arriving gap replaces the standing one only when it is strictly stronger,
or equally weak and larger — the lower level wins outright, and equal levels
keep the larger amount. A stated gap therefore always beats the fallback, and
the fallback survives only when *neither* neighbour states anything, which is
exactly the case this issue reports. It also explains why a stated `w:before`
looked correct before the fix: it won on weakness, not because the table
contributed nothing. The same routine returns `false` when no frame precedes
the spacing at all, which is what drops a wrapped table's own gaps at the
wrapper's edges.

## What Word does

Native Word for Mac 16 exports of seven one-factor probe packages, all Arial
11pt, each a paragraph, then a one-cell borderless table, then a paragraph.
The factor is what the cell paragraph and the paragraph below the table state
for `w:spacing`, and whether the package defines a default table style. Every
number is the cell paragraph's baseline to the following paragraph's baseline,
read off `mutool draw -F trace`. Word quantises to its 0.24pt device grid.

| Probe | Cell `w:after` | Below `w:before` | Table style | Word | before | after |
| --- | --- | --- | --- | --- | --- | --- |
| a | 200 | — | none | 22.56 | 35.85 | 22.65 |
| b | 200 | — | `Normal Table` | 22.56 | 35.85 | 22.65 |
| c | — | — | none | 12.72 | 25.85 | 12.65 |
| d | — | 300 | none | 27.60 | 27.65 | 27.65 |
| e | 200 | 300 | none | 37.44 | 37.65 | 37.65 |
| f | — | 0 | none | 12.72 | 12.65 | 12.65 |
| g | 0 | — | none | 12.72 | 25.85 | 12.65 |

The probes fix these rules:

- **The table itself adds nothing.** Word's gap is one line pitch (12.48pt for
  Arial 11pt) plus the cell paragraph's `w:after` and the paragraph's
  `w:before`, both on the 0.24pt grid: probe a is 12.48 + 10.08, probe d is
  12.48 + 15.12. Nothing is left over for the table.
- **An absent `w:before` is 0, not a default.** Probes c, f and g put the
  paragraph below the table at the same 12.72pt whether it states no `w:before`
  or states `0`, and whether or not the cell paragraph states `w:after="0"`.
  office2pdf separated those cases — 25.85 against 12.65 — because only a
  stated gap reached the emitted Typst.
- **The table style does not change the boundary.** Probe b defines
  `Normal Table` with 0pt top and bottom cell margins and matches probe a
  exactly; the style only moves the cell text inside the row (#1687).

Probes d, e and f already matched before the fix and are unchanged by it,
which is what pins the fix to the absent-`w:before` case rather than to the
boundary in general.

## Fix

`generate_block` in `crates/office2pdf/src/render/typst_gen.rs` wraps a body
`Block::Table` in `#block(width: 100%, above: 0pt, below: 0pt)[…]`. Stating
both gaps puts them at weakness 3, so the neighbour's own `w:spacing` is the
only thing left between the table and the text around it. `write_image_block_open`
already states both of a flow picture's gaps for the same reason; a table
differs only in having no `w:spacing` of its own to state, so both are zero. The wrapper spans
the text column, so a `w:tblPr/w:jc` table still centres in it, and the
table's own `auto` gaps vanish against the wrapper's edges, where Typst trims
weak spacing. A table nested in a cell keeps its previous emission.

## Evidence page

The evidence page is the #1687 handover memo with its closing paragraph's
`w:before="300"` removed and two more closing paragraphs added, so the factor
under test is visible on three lines instead of one. The GT is the native Word
for Mac 16 export.

Against the GT, the before output puts all three closing paragraphs 13.17pt to
13.34pt low; the layout audit reports three large shifts and three fine shifts
and fails. The after output is within 0.18pt everywhere, the audit reports no
finding, the strict render report finds no diff cluster of 20pt² or more, and
the text layer is unchanged. The 0.18pt residual is the GT's own 0.24pt
device grid and is tracked in #1874.

## Regenerating

`build.py` below writes the evidence page and every probe in the table. Run it
with a directory argument; the packages differ only in the arguments to
`build_package`.

```python
import sys
import zipfile

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
CT = "application/vnd.openxmlformats-officedocument.wordprocessingml"

TABLE_NORMAL = (
    '<w:style w:type="table" w:default="1" w:styleId="TableNormal">'
    '<w:name w:val="Normal Table"/>'
    "<w:tblPr><w:tblCellMar>"
    '<w:top w:w="0" w:type="dxa"/><w:left w:w="108" w:type="dxa"/>'
    '<w:bottom w:w="0" w:type="dxa"/><w:right w:w="108" w:type="dxa"/>'
    "</w:tblCellMar></w:tblPr></w:style>"
)


def paragraph(text, before=None, after=None, bold=False):
    spacing = ""
    if before is not None:
        spacing += f' w:before="{before}"'
    if after is not None:
        spacing += f' w:after="{after}"'
    properties = f"<w:spacing{spacing}/>" if spacing else ""
    run_properties = "<w:rPr><w:b/></w:rPr>" if bold else ""
    return (
        f"<w:p><w:pPr>{properties}</w:pPr><w:r>{run_properties}"
        f'<w:t xml:space="preserve">{text}</w:t></w:r></w:p>'
    )


def table(rows, columns):
    # No `w:tblStyle` and no `w:tblCellMar`: the cell margins come from Word's
    # style-less fallback (#1687), so the only boundary under test is the gap
    # below the table.
    grid = "".join(f'<w:gridCol w:w="{column}"/>' for column in columns)
    body = ""
    for row in rows:
        cells = "".join(
            f'<w:tc><w:tcPr><w:tcW w:w="{column}" w:type="dxa"/></w:tcPr>{cell}</w:tc>'
            for column, cell in zip(columns, row)
        )
        body += f"<w:tr>{cells}</w:tr>"
    return (
        f'<w:tbl><w:tblPr><w:tblW w:w="{sum(columns)}" w:type="dxa"/></w:tblPr>'
        f"<w:tblGrid>{grid}</w:tblGrid>{body}</w:tbl>"
    )


def paragraph_style(style_id, name, based_on="Normal", is_default=False):
    default = ' w:default="1"' if is_default else ""
    parent = f'<w:basedOn w:val="{based_on}"/>' if based_on else ""
    return (
        f'<w:style w:type="paragraph"{default} w:styleId="{style_id}">'
        f'<w:name w:val="{name}"/>{parent}</w:style>'
    )


def build_package(path, body, table_style=False):
    styles = (
        f'<w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr>'
        '<w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/>'
        '<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault>'
        '<w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/>'
        "</w:pPr></w:pPrDefault></w:docDefaults>"
        + paragraph_style("Normal", "Normal", based_on=None, is_default=True)
        + paragraph_style("BodyText", "Body Text")
        + paragraph_style("Caption", "caption")
        + (TABLE_NORMAL if table_style else "")
        + "</w:styles>"
    )
    settings = (
        f'<w:settings {W}><w:compat><w:compatSetting w:name="compatibilityMode" '
        'w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>'
    )
    document = (
        f"<w:document {W}><w:body>{body}"
        '<w:sectPr><w:pgSz w:w="12240" w:h="15840"/>'
        '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" '
        'w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>'
    )
    parts = {
        "[Content_Types].xml": (
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            f'<Override PartName="/word/document.xml" ContentType="{CT}.document.main+xml"/>'
            f'<Override PartName="/word/styles.xml" ContentType="{CT}.styles+xml"/>'
            f'<Override PartName="/word/settings.xml" ContentType="{CT}.settings+xml"/></Types>'
        ),
        "_rels/.rels": (
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/>'
            "</Relationships>"
        ),
        "word/document.xml": document,
        "word/_rels/document.xml.rels": (
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{REL}/styles" Target="styles.xml"/>'
            f'<Relationship Id="rId2" Type="{REL}/settings" Target="settings.xml"/></Relationships>'
        ),
        "word/styles.xml": styles,
        "word/settings.xml": settings,
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as package:
        for name, xml in parts.items():
            package.writestr(
                name, '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + xml
            )
    return path


EVIDENCE_COLUMNS = (4680, 4680)
EVIDENCE_ROWS = (
    ("Warehouse lease", "Closes 31 March"),
    ("Night-shift hiring", "Two engineers, April"),
    ("Billing migration", "New cluster, May"),
)
# Neither closing paragraph states `w:before`: Word starts the first one one
# line below the table and every later line follows it.
EVIDENCE_BODY = (
    paragraph("Operations handover", bold=True)
    + paragraph("The table below lists the three items that carry over.", after=200)
    + table(
        [[paragraph(text) for text in row] for row in EVIDENCE_ROWS], EVIDENCE_COLUMNS
    )
    + paragraph("Questions go to the shared tracker.")
    + paragraph("The warehouse item needs an answer before the lease renews.")
    + paragraph("Everything else carries into the next review.")
)

PROBES = {
    "a": (200, None, False),
    "b": (200, None, True),
    "c": (None, None, False),
    "d": (None, 300, False),
    "e": (200, 300, False),
    "f": (None, 0, False),
    "g": (0, None, False),
}

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "."
    print(build_package(f"{out}/issue-1688-evidence.docx", EVIDENCE_BODY))
    for name, (cell_after, below_before, table_style) in PROBES.items():
        body = (
            paragraph("Probe heading for the table boundary.", bold=True)
            + table([[paragraph("Cell paragraph line.", after=cell_after)]], (9360,))
            + paragraph("Paragraph below the table.", before=below_before)
        )
        print(build_package(f"{out}/probe-{name}.docx", body, table_style))
```

Export the ground truth with

```sh
osascript scripts/macos/export_word_pdfs.applescript OUT evidence EVIDENCE.docx
```

staging the inputs and `OUT` inside `~/Library/Containers/com.microsoft.Word/Data/`,
and regenerate the reports with

```sh
python3 scripts/compare_layout.py GT.pdf after.pdf --json --audit --fine-shift 0.5 \
  > assets/bugfixes/issue-1688/layout-audit.json
python3 scripts/compare_render.py GT.pdf after.pdf --page 1 --dpi 150 --fine-shift 0.5 \
  --artifacts-dir target/audit/page-1 --cluster-dispositions DISPOSITIONS.json \
  --cluster-report assets/bugfixes/issue-1688/render-clusters-page-1.json --strict-clusters
```

`compare.jpg` is the side-by-side filed with the issue and predates the fix;
`before.jpg`, `after.jpg` and `gt.jpg` are full page 1 at 150 DPI.

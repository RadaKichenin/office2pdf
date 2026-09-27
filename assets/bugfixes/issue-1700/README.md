# A table row of empty paragraphs collapsed below one line height

Word gives the paragraph mark of an empty `<w:p>` a full line of its own, so a
table row whose cells hold nothing else is as tall as a row of text. office2pdf
emitted nothing at all for such a cell and the row collapsed onto its own rule.

## What Word does

Native Word for Mac 16.113.2 exports of two one-factor probe packages. Only the
blank row's paragraph changes between them; everything else — Arial 11pt stated
in `w:rPrDefault` and in a defined `Normal`, `w:after="0"` with a plain
`w:line="240"` single rule, three 3000-twip cells per row with `w:sz="4"`
(0.48pt) borders, US Letter — is held fixed, so the row height is the only thing
left free.

Each figure is a consecutive rule-top to rule-top distance in points, read off
`mutool draw -F trace`. It is the row's content height plus the allowance the
rule between two rows takes: 0.48pt in Word, which is the rule it paints, and
0.50pt in ours, which is the half point `w:sz="4"` declares. Word's figures
quantise to 0.24pt.

| blank row's mark | row | Word | before | after |
| --- | --- | ---: | ---: | ---: |
| inherited (11pt) | text `a` | 13.200 | 13.149 | 13.149 |
| inherited (11pt) | **blank** | **13.200** | **0.500** | **13.149** |
| inherited (11pt) | text `a` | 12.960 | 13.149 | 13.149 |
| `<w:sz w:val="48"/>` | text `a` | 13.200 | 13.149 | 13.149 |
| `<w:sz w:val="48"/>` | **blank** | **28.080** | **0.500** | **28.098** |
| `<w:sz w:val="48"/>` | text `a` | 13.200 | 13.149 | 13.149 |

Word's blank row is its text row when the mark inherits, and 28.080pt when the
mark states 24pt of its own. Net of the rule that is 27.600pt of content against
24pt of Arial's hhea line — 1.14990em, 27.598pt. So the blank line is the mark's
own, at the mark's own size, and neither a fixed height nor the neighbouring
row's would produce both numbers.

Every after-fix residual is inside the 0.24pt export grid: −0.051pt on the rows
Word prints at 13.200pt, +0.189pt on the one it dithers to 12.960pt, and
+0.018pt on the 24pt blank row. The 0.051pt is two terms pulling against each
other: our content is 0.071pt short, because Word quantises 12.649pt of Arial
line up to 12.72pt, and our rule allowance is 0.02pt long.

## Fix

`empty_cell_paragraph_metric_runs` in
`crates/office2pdf/src/render/typst_gen_tables.rs` sized a blank cell paragraph
from a sibling paragraph in the same cell (issue #625). A cell holding nothing
but empty paragraphs has no sibling, so it fell to `None` and emitted no line —
the zero-height case that function's `TODO` recorded, for want of the mark's own
formatting in the IR.

The DOCX parser now resolves that formatting. `push_paragraph_from_runs` runs
`resolve_run_style` over `w:pPr/w:rPr` for a paragraph with no runs — the same
cascade a text run takes, so the mark's own properties outrank its `w:pStyle`
and both outrank `w:rPrDefault` — and stores the result as
`ParagraphStyle::paragraph_mark_text_style`. `cell_paragraph_mark_metric_runs`
keeps the sibling first and falls back to a stand-in run carrying that
formatting and no text, which the existing `#box(width: 0pt, height: …)` strut
path sizes exactly as it sizes the sibling's line.

A blank cell whose mark resolves nothing — every PowerPoint and Excel cell —
keeps the pre-fix emission.

## Evidence

`gt.jpg`, `before.jpg` and `after.jpg` are full-page 150 DPI renders of
`probe-inherited.docx`: the native Word export, and office2pdf at
`810a15a4` and with this change. `layout-audit.json` and
`render-clusters-page-1.json` compare the same GT and after PDFs.

## Probe packages

Both packages are generated part for part by this script. `probe-inherited.docx`,
the package the evidence images use, has SHA-256
`c27d4d70d1493d4b8cb01fa9473d888323b551cb0585f87684491f49c5335d06` and
`probe-mark-24pt.docx` has
`d1fc15f9d8f6e3da32a4443ba213fdbe0c980592c0f1e2ce84a1a6b92434c367`; a re-run
matches every part, but zip timestamps change those hashes.

```python
"""Generate the issue #1700 probe packages part for part.

`probe-inherited.docx` is the package the evidence images use: a three-row,
three-column table of bordered 3000-twip cells whose middle row holds nothing
but `<w:p/>` in every cell. `probe-mark-24pt.docx` differs in one factor — the
blank row's paragraph marks carry `<w:rPr><w:sz w:val="48"/></w:rPr>` — so a
native export measures whether the blank row follows the mark's own size.
"""

import sys
import zipfile

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'

CONTENT_TYPES = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>'
    '<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>'
    '</Types>'
)

PACKAGE_RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>'
    '</Relationships>'
)

DOCUMENT_RELS = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
    '</Relationships>'
)

# One face and one size for every slot, no paragraph gap, and a plain single
# line rule, so the only thing left free in the table is the row height.
ARIAL = '<w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/>'
STYLES = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    f'<w:styles {W}>'
    '<w:docDefaults>'
    f'<w:rPrDefault><w:rPr>{ARIAL}<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault>'
    '<w:pPrDefault><w:pPr>'
    '<w:spacing w:before="0" w:after="0" w:line="240" w:lineRule="auto"/>'
    '</w:pPr></w:pPrDefault>'
    '</w:docDefaults>'
    '<w:style w:type="paragraph" w:default="1" w:styleId="Normal">'
    '<w:name w:val="Normal"/>'
    '<w:pPr><w:spacing w:before="0" w:after="0" w:line="240" w:lineRule="auto"/></w:pPr>'
    f'<w:rPr>{ARIAL}<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr>'
    '</w:style>'
    '</w:styles>'
)

SIDES = ''.join(
    f'<w:{side} w:val="single" w:sz="4" w:space="0" w:color="000000"/>'
    for side in ('top', 'left', 'bottom', 'right')
)
CELL_PROPERTIES = (
    '<w:tcPr><w:tcW w:w="3000" w:type="dxa"/>'
    f'<w:tcBorders>{SIDES}</w:tcBorders></w:tcPr>'
)

TEXT_PARAGRAPH = f'<w:p><w:r><w:rPr>{ARIAL}<w:sz w:val="22"/></w:rPr><w:t>a</w:t></w:r></w:p>'


def row(paragraph: str, columns: int = 3) -> str:
    cell = f'<w:tc>{CELL_PROPERTIES}{paragraph}</w:tc>'
    return f'<w:tr>{cell * columns}</w:tr>'


def document(blank_paragraph: str) -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f'<w:document {W}><w:body><w:tbl>'
        '<w:tblPr><w:tblW w:w="9000" w:type="dxa"/></w:tblPr>'
        '<w:tblGrid><w:gridCol w:w="3000"/><w:gridCol w:w="3000"/><w:gridCol w:w="3000"/></w:tblGrid>'
        f'{row(TEXT_PARAGRAPH)}{row(blank_paragraph)}{row(TEXT_PARAGRAPH)}'
        '</w:tbl><w:p/>'
        '<w:sectPr>'
        '<w:pgSz w:w="12240" w:h="15840"/>'
        '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" '
        'w:header="720" w:footer="720" w:gutter="0"/>'
        '</w:sectPr>'
        '</w:body></w:document>'
    )


VARIANTS = {
    # The mark states nothing, so Word resolves it through `Normal` and
    # `w:rPrDefault` — Arial 11pt, the same line the text rows take.
    'probe-inherited.docx': '<w:p/>',
    # One factor changed: the mark itself is 24pt.
    'probe-mark-24pt.docx': '<w:p><w:pPr><w:rPr><w:sz w:val="48"/></w:rPr></w:pPr></w:p>',
}


def main(directory: str) -> None:
    for name, blank_paragraph in VARIANTS.items():
        path = f'{directory}/{name}'
        with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as package:
            package.writestr('[Content_Types].xml', CONTENT_TYPES)
            package.writestr('_rels/.rels', PACKAGE_RELS)
            package.writestr('word/_rels/document.xml.rels', DOCUMENT_RELS)
            package.writestr('word/styles.xml', STYLES)
            package.writestr('word/document.xml', document(blank_paragraph))
        print(path)


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else '.')
```

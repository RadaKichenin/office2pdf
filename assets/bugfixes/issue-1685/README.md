# A list whose paragraphs state `w:spacing w:line` gained one extra line per item

Word's default template writes `<w:spacing w:after="160" w:line="259" w:lineRule="auto"/>`
into `w:pPrDefault`, so nearly every authored document states a line multiple on
every paragraph, list items included. office2pdf separated such a list's items by
the paragraph gap **plus one scaled line**, and added the same extra line below the
list. Word adds only the paragraph gap.

## What Word does

Native Word for Mac 16 exports of one-factor probe packages. Only
`w:pPrDefault`'s `w:line` changes between them; everything else — Arial 11pt,
`w:after="160"` (8pt), a one-level bullet list of three `ListParagraph` items
between two `Normal` paragraphs, `<w:contextualSpacing w:val="0"/>` on every item
so that flag plays no part — is held fixed. Each value is a consecutive
baseline-to-baseline distance in points, read off `mutool draw -F stext`; the
gaps run intro → item 1, 1 → 2, 2 → 3, 3 → closing. Word's figures quantise to
0.24pt.

| `w:line` | Word | office2pdf before | office2pdf after |
| --- | --- | --- | --- |
| `240` (1.0) | 20.64 20.64 20.64 20.64 | 20.65 **31.65 31.65 31.65** | 20.65 20.65 20.65 20.65 |
| `259` (1.079) | 21.60 21.60 21.84 21.60 | 21.65 **33.52 33.52 33.52** | 21.65 21.65 21.65 21.65 |
| `360` (1.5) | 26.88 27.12 26.88 27.12 | 26.97 **43.47 43.47 43.47** | 26.97 26.97 26.97 26.97 |
| absent | 20.64 20.64 20.64 20.64 | 20.65 20.65 20.65 20.65 | 20.65 20.65 20.65 20.65 |

Word's gaps are the same inside the list as above it. The excess was 11.00pt at
`240`, 11.87pt at `259` and 16.50pt at `360` — in each case the 11pt font size
times the line multiple, which is one line height. It landed on every gap between
items and on the gap below the list; the gap above the list was already right,
and so was every gap when no `w:line` was stated.

## Fix

`paragraph_uses_full_line_box` in `crates/office2pdf/src/render/typst_gen_lists.rs`
decides whether a list item's paragraph renders under the wrapper block's
full-advance line box. It denied that box to any paragraph stating line spacing,
so `list_boundary_spacing` and `list_edge_spacing` both fell through to their
`line height + gap` branch.

A line multiple is not an exception to that box, it is part of it:
`word_line_leading_pt` multiplies Word's advance by the multiple before handing
the product to the wrapper's fixed text edges, and `powerpoint_paragraph_line_box_em`
does the same with `a:lnSpc`. A paragraph in that set therefore already renders
under a box spanning its own scaled line, and adding the line height again counted
the advance twice. The predicate now accepts a positive proportional multiple
alongside no line spacing at all. An exact line rule stays out: it leaves the Word
path with no wrapper settings, so the caller's `wrapper_spans_full_line` gate
already denies it this branch.

## Probe packages

The four packages above are generated part for part by this script. `probe-259.docx`,
the package the evidence images use, has SHA-256
`81dae3fa94d3ac53f586d02b378bf771aedf6bed1da6eb9578d51ac1ed1369ee`; a re-run
matches every part, but zip timestamps change its hash.

```python
import sys, zipfile

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
CT = "application/vnd.openxmlformats-officedocument.wordprocessingml"


def p(text, style=None, ctx_off=False, num=False, bold=False):
    ppr = f'<w:pStyle w:val="{style}"/>' if style else ""
    if num:
        ppr += '<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>'
    if ctx_off:
        ppr += '<w:contextualSpacing w:val="0"/>'
    rpr = "<w:rPr><w:b/></w:rPr>" if bold else ""
    return (f'<w:p><w:pPr>{ppr}</w:pPr><w:r>{rpr}'
            f'<w:t xml:space="preserve">{text}</w:t></w:r></w:p>')


def paragraph_style(style_id, properties="", based_on="Normal", is_default=False):
    default = ' w:default="1"' if is_default else ""
    parent = f'<w:basedOn w:val="{based_on}"/>' if based_on else ""
    return (f'<w:style w:type="paragraph"{default} w:styleId="{style_id}">'
            f'<w:name w:val="{style_id}"/>{parent}<w:pPr>{properties}</w:pPr></w:style>')


def build(path, line):
    """`line` is the w:line twentieth-of-a-point value, or None for no w:line."""
    if line is None:
        spacing = '<w:spacing w:after="160"/>'
    else:
        spacing = f'<w:spacing w:after="160" w:line="{line}" w:lineRule="auto"/>'

    body = (
        p("Intro paragraph before the list")
        + "".join(
            p(t, style="ListParagraph", num=True, ctx_off=True)
            for t in ["List item 1", "List item 2", "List item 3"]
        )
        + p("Closing paragraph after the list")
    )
    styles = (
        f'<w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr>'
        '<w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/>'
        '<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault>'
        f'<w:pPrDefault><w:pPr>{spacing}</w:pPr></w:pPrDefault></w:docDefaults>'
        + paragraph_style("Normal", based_on=None, is_default=True)
        + paragraph_style("ListParagraph", '<w:ind w:left="720"/>')
        + "</w:styles>"
    )
    numbering = (
        f'<w:numbering {W}><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0">'
        '<w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/>'
        '<w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr>'
        '</w:lvl></w:abstractNum>'
        '<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>'
    )
    settings = (
        f'<w:settings {W}><w:compat><w:compatSetting w:name="compatibilityMode" '
        'w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>'
    )
    document = (
        f'<w:document {W}><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/>'
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
            f'<Override PartName="/word/settings.xml" ContentType="{CT}.settings+xml"/>'
            f'<Override PartName="/word/numbering.xml" ContentType="{CT}.numbering+xml"/></Types>'
        ),
        "_rels/.rels": (
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>'
        ),
        "word/document.xml": document,
        "word/_rels/document.xml.rels": (
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'<Relationship Id="rId1" Type="{REL}/styles" Target="styles.xml"/>'
            f'<Relationship Id="rId2" Type="{REL}/settings" Target="settings.xml"/>'
            f'<Relationship Id="rId3" Type="{REL}/numbering" Target="numbering.xml"/></Relationships>'
        ),
        "word/styles.xml": styles,
        "word/settings.xml": settings,
        "word/numbering.xml": numbering,
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as package:
        for name, xml in parts.items():
            package.writestr(name, '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + xml)


if __name__ == "__main__":
    for tag, line in (("240", 240), ("259", 259), ("absent", None), ("360", 360)):
        out = f"probe-{tag}.docx"
        build(out, line)
        print(out)
```

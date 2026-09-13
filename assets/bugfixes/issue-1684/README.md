# `w:contextualSpacing` ignored between same-style paragraphs

`w:contextualSpacing` is Word's "Don't add space between paragraphs of the same style". The built-in `ListParagraph` style turns it on. The DOCX parser never read it, so every flagged paragraph kept its full `w:spacing` gap.

## What Word does

These are native Word for Mac exports of one-factor probe packages, all in Arial 11pt with single spacing. Each value is the gap between the two probe paragraphs minus the line pitch, read off `mutool draw -F stext`. Word's figures quantise to 0.24pt.

| Probe | Upper paragraph | Lower paragraph | Word | Before | After |
| --- | --- | --- | --- | --- | --- |
| c01 | A, flag, after 10 | A, flag | 0.00 | 10.00 | 0.00 |
| c02 | A, flag, after 10 | A | 0.00 | 10.00 | 0.00 |
| c03 | A, after 10 | A, flag | 10.08 | 10.00 | 10.00 |
| c04 | A | A, flag, before 15 | 0.00 | 15.00 | 0.00 |
| c05 | A, flag | A, before 15 | 15.12 | 15.00 | 15.00 |
| c06 | A, flag, after 10 | B, flag | 10.08 | 10.00 | 10.00 |
| c07 | A, flag, after 10 | A, flag, before 15 | 0.00 | 15.00 | 0.00 |
| c09 | `ListParagraph`, after 10 | `ListParagraph` | 0.00 | 10.00 | 0.00 |
| c10 | `ListParagraph`, `w:val="0"`, after 10 | `ListParagraph`, `w:val="0"` | 10.08 | 10.00 | 10.00 |
| c12 | `ListParagraph`, after 10 | its `w:basedOn` child | 10.08 | 10.00 | 10.00 |
| c13 | no `w:pStyle`, flag, after 10 | `w:pStyle="Normal"`, flag | 0.00 | 10.00 | 0.00 |
| c14 | undefined `w:pStyle`, flag, after 10 | no `w:pStyle`, flag | 0.00 | 10.00 | 0.00 |
| c16 | A, flag, after 10 | A, before 15 | 5.04 | 15.00 | 5.00 |
| c19 | A, flag, after 4 | A, before 15 | 11.04 | 15.00 | 11.00 |
| c20 | A, flag, after 20 | A, before 15 | 0.00 | 20.00 | 0.00 |
| c21 | A, after 5 | A, flag, before 15 | 5.04 | 15.00 | 5.00 |
| c22 | `ListParagraph`, after 10, in a cell | `ListParagraph`, same cell | 0.00 | 10.00 | 0.00 |

Table geometry differs from Word even without the flag. For that reason, each table probe is read as its change from a flag-free control package:

| Probe | Gap | Word | After |
| --- | --- | --- | --- |
| c23 | A, flag, after 10, above a table → its first paragraph (A, flag) | −10.08 | −10.00 |
| c25 | as c23, and the first paragraph also states before 15 | −24.96 | −25.00 |
| c27 | a cell's last paragraph (A, flag, after 10) → the paragraph below the table (A, flag) | 0.00 | 0.00 |
| c29 | table → a flagged default-style paragraph with before 15 | −14.88 | −15.00 |
| c23 | table → a flagged style-A paragraph with before 15 | 0.00 | 0.00 |

The probes fix these rules:

- The flag acts on its own paragraph only. A flagged paragraph drops its `w:after` when the next paragraph has the same effective style, and drops its `w:before` when the previous one does (c02, c03).
- Effective style means the style id. A missing or undefined `w:pStyle` is the default style (c13, c14), and a `w:basedOn` child counts as a different style (c12).
- The flag inherits down the `w:basedOn` chain, and a paragraph's `w:val="0"` overrides it (c09–c11).
- A dropped `w:after` still offsets a `w:before` that stands below it. Word collapses the two gaps to their maximum by subtracting the after from the before, so dropping 10pt above a kept 15pt leaves 5pt (c16, c19, c20).
- A table cell is its own flow (c22). The paragraph above a table and the table's first paragraph are neighbours (c23, c25). The paragraph below a table follows the end-of-row mark, which has the default paragraph style (c23, c29).

The issue's own package (probe c18, which has no styles part) behaves the same way. Word drops the gaps below all four flagged paragraphs, including the fourth one, because the plain paragraph below it shares the default style.

## Fix

`docx_context_contextual_spacing.rs` resolves the flag, each paragraph's effective style and its neighbours from the raw `styles.xml` and `document.xml`. It hands each converted paragraph its drops through the same per-`w:p` cursor pattern as `w:wordWrap`. The drops apply to a text paragraph's resolved `space_before` and `space_after` once Word's fallback `w:after` is in place, and to a picture paragraph's gaps as its style resolves them.

The scan skips `mc:Fallback`, `w:pict` and `w:object`. docx-rs never converts paragraphs under those elements, so counting them would shift every later paragraph's rule. As a further guard, a `w:pStyle` that disagrees with the converted paragraph's stops the context from applying anything more.

## Evidence page

The evidence page has the issue's paragraph shape (four flagged and four plain paragraphs, each stating `w:after="160"`) above a bulleted `ListParagraph` list. The GT is the native Word for Mac export. The before output is from `main` at `8e3e3ffd`, and the after output is from this branch. Everything from the title down to the first list item now matches Word within 0.12pt.

The later list items and the closing paragraph still sit one extra line lower per item gap. That is #1685, which adds a line height to list item gaps whenever the paragraphs state `w:line`, and it reproduces with the flag switched off.

The script below regenerates the package part for part. The package used had SHA-256 `99e2bcf9d9ea1a2b2208c241d12f1fb9bc4d41e5f97f34dbc439d3a0a6a24211`; a re-run matches every part, but zip timestamps change its hash:

```python
import zipfile

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
CT = "application/vnd.openxmlformats-officedocument.wordprocessingml"

def p(text, style=None, ctx=False, after=None, num=False, bold=False):
    ppr = f'<w:pStyle w:val="{style}"/>' if style else ""
    if num:
        ppr += '<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>'
    if after is not None:
        ppr += f'<w:spacing w:after="{after}"/>'
    if ctx:
        ppr += "<w:contextualSpacing/>"
    rpr = "<w:rPr><w:b/></w:rPr>" if bold else ""
    return f'<w:p><w:pPr>{ppr}</w:pPr><w:r>{rpr}<w:t xml:space="preserve">{text}</w:t></w:r></w:p>'

body = (
    p("Quarterly operations review", bold=True)
    + "".join(p(t, ctx=True, after=160) for t in [
        "Budget: approved as submitted.", "Headcount: two support roles approved.",
        "Office lease: renew for one year.", "Vendors: keep the current three."])
    + "".join(p(t, after=160) for t in [
        "Next review: 14 April, same room.", "Minutes: circulated by Operations within a week.",
        "Open questions go to the shared tracker.", "Absent: Finance, who sent written comments."])
    + p("Action items", bold=True)
    + "".join(p(t, style="ListParagraph", num=True) for t in [
        "Close the warehouse lease by March.", "Hire two support engineers for the night shift.",
        "Move the billing service to the new cluster.", "Publish the revised travel policy."])
    + p("Recorded by the operations team.")
)
def paragraph_style(style_id, properties="", based_on="Normal", is_default=False):
    default = ' w:default="1"' if is_default else ""
    parent = f'<w:basedOn w:val="{based_on}"/>' if based_on else ""
    return (f'<w:style w:type="paragraph"{default} w:styleId="{style_id}"><w:name w:val="{style_id}"/>'
            f'{parent}<w:pPr>{properties}</w:pPr></w:style>')

# `A`, `B` and `Child` are unused here; they come from the probe series' shared style sheet.
styles = (
    f'<w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr>'
    '<w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/>'
    '<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault>'
    '<w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="240" w:lineRule="auto"/></w:pPr>'
    '</w:pPrDefault></w:docDefaults>'
    + paragraph_style("Normal", based_on=None, is_default=True)
    + paragraph_style("A") + paragraph_style("B")
    + paragraph_style("ListParagraph", '<w:ind w:left="720"/><w:contextualSpacing/>')
    + paragraph_style("Child", based_on="ListParagraph")
    + "</w:styles>"
)
numbering = (
    f'<w:numbering {W}><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/>'
    '<w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:lvlJc w:val="left"/>'
    '<w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum>'
    '<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>'
)
settings = (
    f'<w:settings {W}><w:compat><w:compatSetting w:name="compatibilityMode" '
    'w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>'
)
document = (
    f'<w:document {W}><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/>'
    '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" '
    'w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>'
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
with zipfile.ZipFile("issue-1684-evidence.docx", "w", zipfile.ZIP_DEFLATED) as package:
    for name, xml in parts.items():
        package.writestr(name, '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + xml)
```

# Paragraph shading lands on the paragraph after a text box

Word writes every text box as `mc:AlternateContent`: the DrawingML box under
`mc:Choice Requires="wps"`, and a VML `w:pict` copy of it under `mc:Fallback`.
docx-rs reads and discards the fallback branch, and a VML text box is rebuilt
from the raw XML without paragraph conversion, so neither branch's paragraphs
reach `convert_paragraph_blocks`.

The `w:shd` scan counted them anyway. Every paragraph after the text box then
read its predecessor's entry, so the fill landed one paragraph low. `w:bidi`
and `w:wordWrap` ride the same per-`w:p` cursor and drifted the same way;
`w:bidi` also counted `</w:p>` rather than `<w:p>`, so an empty `<w:p/>` and a
text box's own paragraph shifted it too.

## Measured

Native Word for Mac 16 export as GT, both read with `mutool draw -F trace`
(fills) and `-F stext` (baselines).

| | Yellow `w:shd` fill (y, pt) | Line it covers |
| --- | --- | --- |
| Word for Mac | 138.96–151.68 | `Shaded paragraph`, baseline 149.28 |
| office2pdf before | 146.60–159.24 | `Plain paragraph after`, baseline 156.91 |
| office2pdf after | 125.95–138.60 | `Shaded paragraph`, baseline 136.26 |

The band's x extent (70.56–541.44) and its height (12.65pt against the GT's
12.72pt) are unchanged by the fix. What moved is which paragraph it sits
behind: the fill now starts 10.32pt above the `Shaded paragraph` baseline,
the same offset Word uses (138.96 against a 149.28 baseline).

## Fix

`docx_context_paragraph_cursor.rs` holds one walk of `document.xml` and one
drift-guarded cursor. It counts a `w:p` exactly when the converter reaches it:
the body's paragraphs, its table cells' and its DrawingML text boxes', but
none under `mc:Fallback`, a VML `w:pict`/`w:object`, or a `w:pPrChange` (whose
`w:pPr` states the properties a revision replaced). `w:shd`, `w:bidi`,
`w:wordWrap` and `w:contextualSpacing` all use it, so a fifth property cannot
count the document a fifth way.

## Evidence page

The package below is the issue's reproduction, in Arial 11pt on US Letter with
1in margins. `gt.jpg` is the native Word for Mac export, `before.jpg` is
`main` at `b16b7252`, and `after.jpg` is this branch.

Everything still separating the two pages is #1690: the inline text box
renders as a plain paragraph with no outline, so its line is 20.65pt rather
than the box's 36pt, `Box text` sits at the left margin instead of inside the
box, and every line below the box — the yellow band included — sits 13.02pt
high. All eight 5% diff clusters and all six layout-audit findings carry that
one cause.

## Package

The script below regenerates the package part for part. The package used had
SHA-256 `114978689737ecbdfb33b535d49f5fc270b4fdd3495bb41995accc6cc767e5d5`;
a re-run matches every part, but zip timestamps change its hash:

```python
import zipfile

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
NS = (
    f'{W} '
    'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
    'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" '
    'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
    'xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" '
    'xmlns:v="urn:schemas-microsoft-com:vml" '
    'xmlns:o="urn:schemas-microsoft-com:office:office" '
    'xmlns:w10="urn:schemas-microsoft-com:office:word" '
    'mc:Ignorable="wps"'
)
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
CT = "application/vnd.openxmlformats-officedocument.wordprocessingml"

BOX_WIDTH_EMU = 1828800   # 2in
BOX_HEIGHT_EMU = 457200   # 0.5in
BOX_PARAGRAPH = (
    '<w:p><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr>'
    '<w:r><w:t xml:space="preserve">Box text</w:t></w:r></w:p>'
)


def paragraph(text, properties=""):
    return (
        f"<w:p><w:pPr>{properties}</w:pPr>"
        f'<w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>'
    )


# The shape Word writes for every text box: the DrawingML box under
# `mc:Choice Requires="wps"`, and a VML copy of the same box under
# `mc:Fallback`. Both hold one paragraph.
text_box_run = (
    "<w:r><mc:AlternateContent>"
    '<mc:Choice Requires="wps"><w:drawing>'
    f'<wp:inline distT="0" distB="0" distL="0" distR="0">'
    f'<wp:extent cx="{BOX_WIDTH_EMU}" cy="{BOX_HEIGHT_EMU}"/>'
    '<wp:effectExtent l="0" t="0" r="0" b="0"/>'
    '<wp:docPr id="1" name="Text Box 1"/><wp:cNvGraphicFramePr/>'
    '<a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">'
    '<wps:wsp><wps:cNvSpPr txBox="1"/><wps:spPr>'
    f'<a:xfrm><a:off x="0" y="0"/><a:ext cx="{BOX_WIDTH_EMU}" cy="{BOX_HEIGHT_EMU}"/></a:xfrm>'
    '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>'
    '<a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>'
    '<a:ln w="9525"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln>'
    "</wps:spPr>"
    f"<wps:txbx><w:txbxContent>{BOX_PARAGRAPH}</w:txbxContent></wps:txbx>"
    '<wps:bodyPr rot="0" vert="horz" wrap="square" lIns="91440" tIns="45720" '
    'rIns="91440" bIns="45720" anchor="t" anchorCtr="0"><a:noAutofit/></wps:bodyPr>'
    "</wps:wsp></a:graphicData></a:graphic></wp:inline></w:drawing></mc:Choice>"
    "<mc:Fallback><w:pict>"
    '<v:shapetype id="_x0000_t202" coordsize="21600,21600" o:spt="202" path="m,l,21600r21600,l21600,xe">'
    '<v:stroke joinstyle="miter"/><v:path gradientshapeok="t" o:connecttype="rect"/></v:shapetype>'
    '<v:shape id="Text_x0020_Box_x0020_1" o:spid="_x0000_s1026" type="#_x0000_t202" '
    'style="width:144pt;height:36pt;visibility:visible;mso-wrap-style:square" '
    'fillcolor="white" strokecolor="black" strokeweight=".75pt">'
    f"<v:textbox><w:txbxContent>{BOX_PARAGRAPH}</w:txbxContent></v:textbox>"
    "</v:shape></w:pict></mc:Fallback>"
    "</mc:AlternateContent></w:r>"
)

body = (
    '<w:p><w:pPr></w:pPr><w:r><w:t xml:space="preserve">Anchor paragraph with a text box'
    f"</w:t></w:r>{text_box_run}</w:p>"
    + paragraph("Plain paragraph")
    + paragraph(
        "Shaded paragraph",
        '<w:shd w:val="clear" w:color="auto" w:fill="FFFF00"/>',
    )
    + paragraph("Plain paragraph after")
)

styles = (
    f"<w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr>"
    '<w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/>'
    '<w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault>'
    '<w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="240" w:lineRule="auto"/></w:pPr>'
    "</w:pPrDefault></w:docDefaults>"
    '<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/>'
    "<w:pPr></w:pPr></w:style></w:styles>"
)
settings = (
    f'<w:settings {W}><w:compat><w:compatSetting w:name="compatibilityMode" '
    'w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>'
)
document = (
    f"<w:document {NS}><w:body>{body}"
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
        f'<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>'
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
with zipfile.ZipFile("issue-1689-evidence.docx", "w", zipfile.ZIP_DEFLATED) as package:
    for name, xml in parts.items():
        package.writestr(name, '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>' + xml)
print("wrote issue-1689-evidence.docx")
```

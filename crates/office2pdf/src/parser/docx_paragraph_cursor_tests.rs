//! Every per-`w:p` cursor counts the same paragraph sequence: the one the
//! converter reaches (issue #1689).
//!
//! Word writes every text box as `mc:AlternateContent` — a DrawingML box
//! under `mc:Choice Requires="wps"` and a VML copy under `mc:Fallback`.
//! docx-rs discards the fallback branch, and a VML `w:pict`/`w:object` text
//! box is rebuilt from the raw XML without paragraph conversion, so the
//! paragraphs inside those subtrees never reach `convert_paragraph_blocks`.
//! A scan that counts them hands every later paragraph its predecessor's
//! entry: the shading of `w:shd` lands one paragraph low, and so do
//! `w:wordWrap` and `w:bidi`.

use super::*;

const DOCUMENT_NAMESPACES: &str = concat!(
    r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" "#,
    r#"xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" "#,
    r#"xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" "#,
    r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" "#,
    r#"xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" "#,
    r#"xmlns:v="urn:schemas-microsoft-com:vml" "#,
    r#"mc:Ignorable="wps""#,
);

const STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
</w:styles>"#;

fn document_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {DOCUMENT_NAMESPACES}><w:body>{body}<w:sectPr/></w:body></w:document>"#
    )
}

fn paragraph(properties: &str, text: &str) -> String {
    format!(
        r#"<w:p><w:pPr>{properties}</w:pPr><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#
    )
}

/// A DrawingML text box with the VML `mc:Fallback` copy Word always writes.
/// `fallback_paragraphs` decides how far a scan that counts the fallback
/// drifts, so a test can distinguish "skips one" from "counts what converts".
fn drawing_text_box_run(fallback_paragraphs: usize) -> String {
    let box_paragraph = paragraph("", "Box text");
    let fallback: String = (0..fallback_paragraphs)
        .map(|index| paragraph("", &format!("Fallback line {index}")))
        .collect();
    format!(
        r#"<w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing>
<wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="1828800" cy="457200"/><wp:docPr id="1" name="Text Box 1"/>
<a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingShape">
<wps:wsp><wps:txbx><w:txbxContent>{box_paragraph}</w:txbxContent></wps:txbx><wps:bodyPr/></wps:wsp>
</a:graphicData></a:graphic></wp:inline></w:drawing></mc:Choice>
<mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent>{fallback}</w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent></w:r>"#
    )
}

/// A VML-only text box, the shape a package written without
/// `mc:AlternateContent` carries. Its paragraph is rebuilt from the raw XML,
/// so it never consumes a cursor slot either.
fn vml_text_box_run() -> String {
    let box_paragraph = paragraph("", "Box text");
    format!(
        r#"<w:r><w:pict><v:shape style="width:144pt;height:36pt"><v:textbox><w:txbxContent>{box_paragraph}</w:txbxContent></v:textbox></v:shape></w:pict></w:r>"#
    )
}

fn parse(document_xml: &str) -> Document {
    let data = build_docx_with_styles_xml(document_xml, STYLES_XML);
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();
    doc
}

/// Every paragraph in reading order as its text plus one of its style fields.
fn paragraph_property<T>(blocks: &[Block], read: &impl Fn(&Paragraph) -> T) -> Vec<(String, T)> {
    let mut values: Vec<(String, T)> = Vec::new();
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                let text: String = paragraph.runs.iter().map(|run| run.text.as_str()).collect();
                values.push((text, read(paragraph)));
            }
            Block::List(list) => {
                for item in &list.items {
                    for paragraph in &item.content {
                        let text: String =
                            paragraph.runs.iter().map(|run| run.text.as_str()).collect();
                        values.push((text, read(paragraph)));
                    }
                }
            }
            Block::Table(table) => {
                for cell in table.rows.iter().flat_map(|row| row.cells.iter()) {
                    values.extend(paragraph_property(&cell.content, read));
                }
            }
            _ => {}
        }
    }
    values
}

fn backgrounds(doc: &Document) -> Vec<(String, Option<Color>)> {
    paragraph_property(all_blocks(doc), &|paragraph: &Paragraph| {
        paragraph.style.background
    })
}

/// The issue's package: a text box, then a plain, a shaded and a plain
/// paragraph. Word paints the yellow band behind `Shaded paragraph`.
#[test]
fn issue_1689_text_box_fallback_does_not_shift_paragraph_shading() {
    let body: String = [
        format!(
            r#"<w:p><w:pPr></w:pPr><w:r><w:t xml:space="preserve">Anchor paragraph with a text box</w:t></w:r>{}</w:p>"#,
            drawing_text_box_run(1)
        ),
        paragraph("", "Plain paragraph"),
        paragraph(
            r#"<w:shd w:val="clear" w:color="auto" w:fill="FFFF00"/>"#,
            "Shaded paragraph",
        ),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let shaded: Vec<String> = backgrounds(&doc)
        .into_iter()
        .filter(|(_, background)| background == &Some(Color::new(0xFF, 0xFF, 0x00)))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(shaded, vec!["Shaded paragraph".to_string()]);
}

/// Triangulation: a fallback holding three paragraphs would shift the scan by
/// three, so the fix cannot be an off-by-one correction.
#[test]
fn a_three_paragraph_fallback_does_not_shift_paragraph_shading() {
    let body: String = [
        format!(r#"<w:p><w:pPr></w:pPr>{}</w:p>"#, drawing_text_box_run(3)),
        paragraph("", "Plain one"),
        paragraph("", "Plain two"),
        paragraph(
            r#"<w:shd w:val="clear" w:color="auto" w:fill="C0E0FF"/>"#,
            "Shaded paragraph",
        ),
        paragraph("", "Plain three"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let shaded: Vec<String> = backgrounds(&doc)
        .into_iter()
        .filter(|(_, background)| background == &Some(Color::new(0xC0, 0xE0, 0xFF)))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(shaded, vec!["Shaded paragraph".to_string()]);
}

/// A VML-only text box drifts the scan the same way, with no
/// `mc:AlternateContent` anywhere in the package.
#[test]
fn a_vml_only_text_box_does_not_shift_paragraph_shading() {
    let body: String = [
        format!(r#"<w:p><w:pPr></w:pPr>{}</w:p>"#, vml_text_box_run()),
        paragraph(
            r#"<w:shd w:val="clear" w:color="auto" w:fill="FFD966"/>"#,
            "Shaded paragraph",
        ),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let shaded: Vec<String> = backgrounds(&doc)
        .into_iter()
        .filter(|(_, background)| background == &Some(Color::new(0xFF, 0xD9, 0x66)))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(shaded, vec!["Shaded paragraph".to_string()]);
}

/// A `w:pPrChange` records the paragraph properties a revision replaced, not
/// the current ones, so its `w:shd` must not reach the paragraph.
#[test]
fn a_pprchange_shading_is_not_the_paragraphs_own() {
    let body: String = [
        paragraph(
            concat!(
                r#"<w:pPrChange w:id="1" w:author="A" w:date="2026-01-01T00:00:00Z">"#,
                r#"<w:pPr><w:shd w:val="clear" w:color="auto" w:fill="FF0000"/></w:pPr></w:pPrChange>"#,
            ),
            "Revised paragraph",
        ),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    assert!(
        backgrounds(&doc)
            .iter()
            .all(|(_, background)| background.is_none()),
        "the replaced properties must not paint anything: {:?}",
        backgrounds(&doc)
    );
}

/// `w:wordWrap` rides the same cursor, so it drifts the same way (issue
/// #730's property, issue #1041's raw scan).
#[test]
fn text_box_fallback_does_not_shift_word_wrap() {
    let body: String = [
        format!(r#"<w:p><w:pPr></w:pPr>{}</w:p>"#, drawing_text_box_run(1)),
        paragraph("", "Plain paragraph"),
        paragraph(r#"<w:wordWrap w:val="0"/>"#, "Character wrapped"),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let word_wraps: Vec<(String, Option<bool>)> =
        paragraph_property(all_blocks(&doc), &|paragraph: &Paragraph| {
            paragraph.style.word_wrap
        })
        .into_iter()
        .filter(|(_, word_wrap)| word_wrap == &Some(false))
        .collect();
    assert_eq!(
        word_wraps,
        vec![("Character wrapped".to_string(), Some(false))]
    );
}

/// `w:bidi` rides it too. Its scan also counted `</w:p>` rather than `<w:p>`,
/// so a text box's own paragraph never got a slot of its own.
#[test]
fn text_box_fallback_does_not_shift_bidi() {
    let body: String = [
        format!(r#"<w:p><w:pPr></w:pPr>{}</w:p>"#, drawing_text_box_run(1)),
        paragraph("", "Plain paragraph"),
        paragraph(r#"<w:bidi/>"#, "Right to left"),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let right_to_left: Vec<String> =
        paragraph_property(all_blocks(&doc), &|paragraph: &Paragraph| {
            paragraph.style.direction
        })
        .into_iter()
        .filter(|(_, direction)| direction == &Some(TextDirection::Rtl))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(right_to_left, vec!["Right to left".to_string()]);
}

/// An empty `<w:p/>` converts to a paragraph of its own, so it consumes a
/// slot. The `w:bidi` scan counted only `</w:p>` and missed it.
#[test]
fn an_empty_paragraph_does_not_shift_bidi() {
    let body: String = [
        "<w:p/>".to_string(),
        paragraph(r#"<w:bidi/>"#, "Right to left"),
        paragraph("", "Plain paragraph after"),
    ]
    .concat();

    let doc = parse(&document_xml(&body));

    let right_to_left: Vec<String> =
        paragraph_property(all_blocks(&doc), &|paragraph: &Paragraph| {
            paragraph.style.direction
        })
        .into_iter()
        .filter(|(_, direction)| direction == &Some(TextDirection::Rtl))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(right_to_left, vec!["Right to left".to_string()]);
}

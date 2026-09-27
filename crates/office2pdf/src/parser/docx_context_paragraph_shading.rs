//! Per-paragraph `w:shd`, the fill Word paints across the paragraph's full
//! width, recovered from the raw document XML.
//!
//! The scan counts the paragraph sequence of [`super::paragraph_cursor`], so a
//! text box's `mc:Fallback` copy cannot shift the fill onto the paragraph below
//! (issue #1689).

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::ir::Color;
use crate::parser::xml_util;

use super::paragraph_cursor::{
    ParagraphCursor, ScannedParagraph, attribute_value, scan_body_paragraphs,
};

fn shading_fill(reader: &Reader<&[u8]>, element: &BytesStart<'_>) -> Option<Color> {
    attribute_value(reader, element, b"fill").and_then(|fill| xml_util::parse_hex_color(&fill))
}

pub(in super::super) struct ParagraphShadingContext {
    backgrounds: ParagraphCursor<Option<Color>>,
}

impl ParagraphShadingContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self {
            backgrounds: ParagraphCursor::new("w:shd", xml.map(Self::scan).unwrap_or_default()),
        }
    }

    /// The next paragraph's shading fill. Must be called exactly once per
    /// converted `w:p`, with that paragraph's `w:pStyle`.
    pub(in super::super) fn next_background(&self, style_id: Option<&str>) -> Option<Color> {
        self.backgrounds
            .next(style_id)
            .and_then(|(_, background)| *background)
    }

    fn scan(xml: &str) -> Vec<ScannedParagraph<Option<Color>>> {
        scan_body_paragraphs(xml, |reader, element, background| {
            if element.local_name().as_ref() == b"shd" {
                *background = shading_fill(reader, element);
            }
        })
    }
}

pub(in super::super) fn scan_style_paragraph_shading(xml: Option<&str>) -> HashMap<String, Color> {
    let Some(xml) = xml else {
        return HashMap::new();
    };
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut backgrounds = HashMap::new();
    let mut paragraph_style_id = None;
    let mut in_paragraph_properties = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match element.local_name().as_ref() {
                b"style" => {
                    let style_type = attribute_value(&reader, &element, b"type");
                    paragraph_style_id = (style_type.as_deref() == Some("paragraph"))
                        .then(|| attribute_value(&reader, &element, b"styleId"))
                        .flatten();
                }
                b"pPr" if paragraph_style_id.is_some() => in_paragraph_properties = true,
                b"shd" if in_paragraph_properties => {
                    if let (Some(style_id), Some(color)) =
                        (paragraph_style_id.as_ref(), shading_fill(&reader, &element))
                    {
                        backgrounds.insert(style_id.clone(), color);
                    }
                }
                _ => {}
            },
            Ok(Event::Empty(element)) if element.local_name().as_ref() == b"shd" => {
                if in_paragraph_properties
                    && let (Some(style_id), Some(color)) =
                        (paragraph_style_id.as_ref(), shading_fill(&reader, &element))
                {
                    backgrounds.insert(style_id.clone(), color);
                }
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                b"style" => {
                    paragraph_style_id = None;
                    in_paragraph_properties = false;
                }
                b"pPr" => in_paragraph_properties = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    backgrounds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_direct_paragraph_shading_in_document_order() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body>
          <w:p><w:pPr><w:shd w:fill="F4F4F4"/></w:pPr></w:p>
          <w:p><w:pPr><w:shd w:fill="auto"/></w:pPr></w:p>
        </w:body></w:document>"#;
        let context = ParagraphShadingContext::from_xml(Some(xml));

        assert_eq!(
            context.next_background(None),
            Some(Color::new(0xF4, 0xF4, 0xF4))
        );
        assert_eq!(context.next_background(None), None);
    }

    /// The fallback copy of a text box holds paragraphs docx-rs discards, so
    /// counting them would paint the fill one paragraph low (issue #1689).
    #[test]
    fn skips_the_fallback_copy_of_a_text_box() {
        let xml = r#"<w:document xmlns:w="urn:w" xmlns:mc="urn:mc" xmlns:v="urn:v"><w:body>
          <w:p><w:r><mc:AlternateContent>
            <mc:Choice Requires="wps"><w:txbxContent><w:p/></w:txbxContent></mc:Choice>
            <mc:Fallback><w:pict><v:textbox><w:txbxContent><w:p/></w:txbxContent></v:textbox></w:pict></mc:Fallback>
          </mc:AlternateContent></w:r></w:p>
          <w:p><w:pPr><w:shd w:fill="FFFF00"/></w:pPr></w:p>
        </w:body></w:document>"#;
        let context = ParagraphShadingContext::from_xml(Some(xml));

        assert_eq!(context.next_background(None), None, "the anchor paragraph");
        assert_eq!(context.next_background(None), None, "the text box's own");
        assert_eq!(
            context.next_background(None),
            Some(Color::new(0xFF, 0xFF, 0x00)),
            "the shaded paragraph itself"
        );
    }

    #[test]
    fn scans_only_paragraph_style_shading() {
        let xml = r#"<w:styles xmlns:w="urn:w">
          <w:style w:type="character" w:styleId="CodeChar"><w:rPr><w:shd w:fill="111111"/></w:rPr></w:style>
          <w:style w:type="paragraph" w:styleId="Code"><w:pPr><w:shd w:fill="E7E7E7"/></w:pPr></w:style>
        </w:styles>"#;
        let backgrounds = scan_style_paragraph_shading(Some(xml));

        assert_eq!(backgrounds.get("Code"), Some(&Color::new(0xE7, 0xE7, 0xE7)));
        assert!(!backgrounds.contains_key("CodeChar"));
    }
}

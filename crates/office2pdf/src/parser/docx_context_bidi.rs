//! Per-paragraph `w:bidi`, the right-to-left paragraph direction, recovered
//! from the raw document XML.
//!
//! The scan counts the paragraph sequence of [`super::paragraph_cursor`], so
//! neither a text box's `mc:Fallback` copy nor an empty `<w:p/>` can shift the
//! direction onto another paragraph (issue #1689).

use super::paragraph_cursor::{ParagraphCursor, ScannedParagraph, scan_body_paragraphs};

pub(in super::super) struct BidiContext {
    directions: ParagraphCursor<bool>,
}

impl BidiContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self {
            directions: ParagraphCursor::new("w:bidi", xml.map(Self::scan).unwrap_or_default()),
        }
    }

    /// Whether the next paragraph is right-to-left. Must be called exactly
    /// once per converted `w:p`, with that paragraph's `w:pStyle`.
    pub(in super::super) fn next_is_bidi(&self, style_id: Option<&str>) -> bool {
        self.directions
            .next(style_id)
            .is_some_and(|(_, is_bidi)| *is_bidi)
    }

    fn scan(xml: &str) -> Vec<ScannedParagraph<bool>> {
        scan_body_paragraphs(xml, |_reader, element, is_bidi| {
            if element.local_name().as_ref() == b"bidi" {
                *is_bidi = true;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_direction_of_each_paragraph_in_order() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body>
          <w:p/>
          <w:p><w:pPr><w:bidi/></w:pPr></w:p>
          <w:p><w:pPr/></w:p>
        </w:body></w:document>"#;
        let context = BidiContext::from_xml(Some(xml));

        assert!(!context.next_is_bidi(None), "the empty paragraph");
        assert!(context.next_is_bidi(None), "the bidi paragraph");
        assert!(!context.next_is_bidi(None), "the plain paragraph");
    }

    /// A text box's own paragraph converts, so it takes a slot; the VML copy
    /// of it under `mc:Fallback` does not (issue #1689).
    #[test]
    fn skips_the_fallback_copy_of_a_text_box() {
        let xml = r#"<w:document xmlns:w="urn:w" xmlns:mc="urn:mc" xmlns:v="urn:v"><w:body>
          <w:p><w:r><mc:AlternateContent>
            <mc:Choice Requires="wps"><w:txbxContent><w:p/></w:txbxContent></mc:Choice>
            <mc:Fallback><w:pict><v:textbox><w:txbxContent><w:p/></w:txbxContent></v:textbox></w:pict></mc:Fallback>
          </mc:AlternateContent></w:r></w:p>
          <w:p><w:pPr><w:bidi/></w:pPr></w:p>
        </w:body></w:document>"#;
        let context = BidiContext::from_xml(Some(xml));

        assert!(!context.next_is_bidi(None), "the anchor paragraph");
        assert!(!context.next_is_bidi(None), "the text box's own");
        assert!(context.next_is_bidi(None), "the bidi paragraph itself");
    }
}

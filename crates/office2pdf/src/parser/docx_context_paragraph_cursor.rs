//! The paragraph sequence every per-`w:p` cursor counts (issue #1689).
//!
//! Several paragraph properties reach the IR through a scan of the raw
//! `document.xml` rather than through docx-rs: `w:shd`, `w:wordWrap`
//! (issue #1041), `w:bidi`, `w:contextualSpacing` (issue #1684), and the
//! paragraph mark's own revision state (issue #1710). Each scan records one
//! entry per paragraph and hands them out through a cursor the converter
//! advances once per `w:p`, so the two sequences have to agree.
//!
//! `document.xml` holds `w:p` elements the converter never reaches. docx-rs
//! reads and discards `mc:Fallback`, which is where Word puts the VML copy of
//! every text box, and a VML `w:pict`/`w:object` text box is rebuilt from the
//! raw XML without paragraph conversion. Counting those paragraphs shifts
//! every later one onto its predecessor's entry, which is how a yellow `w:shd`
//! band landed on the paragraph below the shaded one.
//!
//! One walk and one cursor serve every one of those properties so they cannot
//! count the document differently again. `w:contextualSpacing` also needs the
//! flows a table opens, so it keeps its own walk and shares the skip set and
//! cursor.

use std::cell::Cell;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::parser::xml_util::OOXML_XML_VERSION;

/// One attribute of an element, decoded, by local name.
pub(in super::super) fn attribute_value(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    name: &[u8],
) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.local_name().as_ref() == name)
        .and_then(|attribute| {
            attribute
                .decoded_and_normalized_value(OOXML_XML_VERSION, reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        })
}

/// Subtrees a paragraph scan walks past: those whose `w:p` never reaches
/// paragraph conversion, plus `w:pPrChange` and `w:rPrChange`, whose `w:pPr`
/// and `w:rPr` state the properties a revision replaced rather than the
/// paragraph's current ones.
pub(in super::super) fn is_skipped_subtree(local_name: &[u8]) -> bool {
    matches!(
        local_name,
        b"Fallback" | b"pict" | b"object" | b"pPrChange" | b"rPrChange"
    )
}

/// A `w:p` as a paragraph scan saw it.
pub(in super::super) struct ScannedParagraph<T> {
    /// The paragraph's own `w:pStyle`, unresolved, to check the converter is
    /// on the same paragraph as the scan.
    pub(in super::super) style_id: Option<String>,
    pub(in super::super) value: T,
}

/// One entry per `w:p` of `document.xml` the converter reaches, in the order
/// it reaches them: the body's paragraphs, its table cells' and its DrawingML
/// text boxes', but none under a skipped subtree.
///
/// `read_property` sees every element inside a paragraph's own `w:pPr`,
/// including the run properties of its paragraph mark, and writes what it
/// recognises into that paragraph's value.
pub(in super::super) fn scan_body_paragraphs<T: Default>(
    xml: &str,
    mut read_property: impl FnMut(&Reader<&[u8]>, &BytesStart<'_>, &mut T),
) -> Vec<ScannedParagraph<T>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut paragraphs: Vec<ScannedParagraph<T>> = Vec::new();
    // A text box's paragraphs sit inside the run of an outer paragraph, so
    // the open paragraph is a stack rather than a single index.
    let mut open_paragraphs: Vec<usize> = Vec::new();
    let mut in_body = false;
    let mut in_paragraph_properties = false;
    let mut skipped_depth: usize = 0;

    loop {
        let (element, is_empty) = match reader.read_event() {
            Ok(Event::Start(element)) => (element, false),
            Ok(Event::Empty(element)) => (element, true),
            Ok(Event::End(element)) => {
                let name = element.local_name();
                if skipped_depth > 0 {
                    skipped_depth -= usize::from(is_skipped_subtree(name.as_ref()));
                    continue;
                }
                match name.as_ref() {
                    b"body" => in_body = false,
                    b"p" if in_body => {
                        open_paragraphs.pop();
                        in_paragraph_properties = false;
                    }
                    b"pPr" => in_paragraph_properties = false,
                    _ => {}
                }
                continue;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        let name = element.local_name();
        if skipped_depth > 0 || is_skipped_subtree(name.as_ref()) {
            skipped_depth += usize::from(!is_empty && is_skipped_subtree(name.as_ref()));
            continue;
        }
        match name.as_ref() {
            b"body" if !is_empty => in_body = true,
            b"p" if in_body => {
                paragraphs.push(ScannedParagraph {
                    style_id: None,
                    value: T::default(),
                });
                if !is_empty {
                    open_paragraphs.push(paragraphs.len() - 1);
                }
            }
            b"pPr" if !is_empty && !open_paragraphs.is_empty() => in_paragraph_properties = true,
            b"pStyle" if in_paragraph_properties => {
                if let Some(&index) = open_paragraphs.last() {
                    paragraphs[index].style_id = attribute_value(&reader, &element, b"val");
                }
            }
            _ if in_paragraph_properties => {
                if let Some(&index) = open_paragraphs.last() {
                    read_property(&reader, &element, &mut paragraphs[index].value);
                }
            }
            _ => {}
        }
    }

    paragraphs
}

/// A scanned paragraph sequence, handed out one entry per converted `w:p`.
///
/// A `w:pStyle` that disagrees with the converted paragraph's means the two
/// sequences drifted apart. The cursor then stops handing anything out rather
/// than move a property onto the wrong paragraph, and says so once in the log.
pub(in super::super) struct ParagraphCursor<T> {
    paragraphs: Vec<ScannedParagraph<T>>,
    cursor: Cell<usize>,
    has_drifted: Cell<bool>,
    /// The scanned property, named in the drift warning.
    property: &'static str,
}

impl<T> ParagraphCursor<T> {
    pub(in super::super) fn new(
        property: &'static str,
        paragraphs: Vec<ScannedParagraph<T>>,
    ) -> Self {
        Self {
            paragraphs,
            cursor: Cell::new(0),
            has_drifted: Cell::new(false),
            property,
        }
    }

    /// The next paragraph's entry with its index. Must be called exactly once
    /// per converted `w:p`, with that paragraph's `w:pStyle`.
    pub(in super::super) fn next(&self, style_id: Option<&str>) -> Option<(usize, &T)> {
        let index = self.cursor.get();
        self.cursor.set(index + 1);
        if self.has_drifted.get() {
            return None;
        }
        match self.paragraphs.get(index) {
            Some(paragraph) if paragraph.style_id.as_deref() == style_id => {
                Some((index, &paragraph.value))
            }
            Some(paragraph) => {
                self.has_drifted.set(true);
                tracing::warn!(
                    property = self.property,
                    paragraph_index = index,
                    scanned_style = paragraph.style_id.as_deref().unwrap_or(""),
                    converted_style = style_id.unwrap_or(""),
                    "paragraph scan drifted from the converted paragraphs; \
                     leaving the remaining paragraphs as stated"
                );
                None
            }
            None => None,
        }
    }

    /// An entry a previous [`Self::next`] already handed out.
    pub(in super::super) fn get(&self, index: usize) -> &T {
        &self.paragraphs[index].value
    }

    /// Every scanned entry, for tests that assert on a whole document at once
    /// rather than driving the cursor.
    #[cfg(test)]
    pub(in super::super) fn values(&self) -> impl Iterator<Item = &T> {
        self.paragraphs.iter().map(|paragraph| &paragraph.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `w:shd`-style property is read the same way, so the walk is
    /// exercised here with a marker attribute.
    fn scan_markers(xml: &str) -> Vec<(Option<String>, Option<String>)> {
        scan_body_paragraphs(xml, |reader, element, marker: &mut Option<String>| {
            if element.local_name().as_ref() == b"mark" {
                *marker = attribute_value(reader, element, b"val");
            }
        })
        .into_iter()
        .map(|paragraph| (paragraph.style_id, paragraph.value))
        .collect()
    }

    #[test]
    fn counts_every_paragraph_the_converter_reaches() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body>
          <w:p><w:pPr><w:pStyle w:val="A"/><w:mark w:val="first"/></w:pPr></w:p>
          <w:p/>
          <w:tbl><w:tr><w:tc><w:p><w:pPr><w:mark w:val="cell"/></w:pPr></w:p></w:tc></w:tr></w:tbl>
          <w:p><w:pPr><w:mark w:val="last"/></w:pPr></w:p>
        </w:body></w:document>"#;

        assert_eq!(
            scan_markers(xml),
            vec![
                (Some("A".to_string()), Some("first".to_string())),
                (None, None),
                (None, Some("cell".to_string())),
                (None, Some("last".to_string())),
            ]
        );
    }

    #[test]
    fn skips_the_paragraphs_conversion_never_reaches() {
        let xml = r#"<w:document xmlns:w="urn:w" xmlns:mc="urn:mc" xmlns:v="urn:v"><w:body>
          <w:p><w:r><mc:AlternateContent>
            <mc:Choice Requires="wps"><w:txbxContent><w:p><w:pPr><w:mark w:val="box"/></w:pPr></w:p></w:txbxContent></mc:Choice>
            <mc:Fallback><w:pict><v:textbox><w:txbxContent>
              <w:p><w:pPr><w:mark w:val="fallback-one"/></w:pPr></w:p>
              <w:p><w:pPr><w:mark w:val="fallback-two"/></w:pPr></w:p>
            </w:txbxContent></v:textbox></w:pict></mc:Fallback>
          </mc:AlternateContent></w:r></w:p>
          <w:p><w:pPr><w:mark w:val="after"/></w:pPr></w:p>
        </w:body></w:document>"#;

        assert_eq!(
            scan_markers(xml),
            vec![
                (None, None),
                (None, Some("box".to_string())),
                (None, Some("after".to_string())),
            ]
        );
    }

    #[test]
    fn skips_the_properties_a_revision_replaced() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body>
          <w:p><w:pPr><w:pPrChange><w:pPr><w:pStyle w:val="Old"/><w:mark w:val="old"/></w:pPr></w:pPrChange></w:pPr></w:p>
        </w:body></w:document>"#;

        assert_eq!(scan_markers(xml), vec![(None, None)]);
    }

    #[test]
    fn stops_handing_out_entries_once_the_styles_disagree() {
        let cursor: ParagraphCursor<u8> = ParagraphCursor::new(
            "w:mark",
            vec![
                ScannedParagraph {
                    style_id: Some("A".to_string()),
                    value: 1,
                },
                ScannedParagraph {
                    style_id: Some("B".to_string()),
                    value: 2,
                },
                ScannedParagraph {
                    style_id: Some("C".to_string()),
                    value: 3,
                },
            ],
        );

        assert_eq!(cursor.next(Some("A")).map(|(_, value)| *value), Some(1));
        assert_eq!(
            cursor.next(Some("Z")).map(|(_, value)| *value),
            None,
            "a disagreeing style stops the cursor"
        );
        assert_eq!(
            cursor.next(Some("C")).map(|(_, value)| *value),
            None,
            "and it stays stopped, because the drift is cumulative"
        );
    }

    #[test]
    fn runs_out_quietly_past_the_last_scanned_paragraph() {
        let cursor: ParagraphCursor<u8> = ParagraphCursor::new(
            "w:mark",
            vec![ScannedParagraph {
                style_id: None,
                value: 7,
            }],
        );

        assert_eq!(
            cursor.next(None).map(|(index, value)| (index, *value)),
            Some((0, 7))
        );
        assert_eq!(cursor.next(None).map(|(_, value)| *value), None);
    }
}

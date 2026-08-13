//! Per-paragraph `w:wordWrap` recovered from the raw document XML.
//!
//! The published `docx-rs` does not parse `w:pPr/w:wordWrap`; only the
//! workspace's patched fork does. `cargo publish` verifies the packaged crate
//! against the published dependency graph, where `[patch.crates-io]` does not
//! apply, so reading the fork's field made the crate unpublishable — the
//! v0.6.6 release's crates.io job failed on exactly that (issue #1041).
//! Scanning the raw XML keeps the behaviour of issue #730 without the fork.
//!
//! The scan mirrors [`super::docx_contexts`]'s paragraph-shading context: one
//! entry per `w:p` in document order (nested table paragraphs included, via
//! the same stack), consumed once per converted paragraph.

use std::cell::Cell;
use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// `w:wordWrap`'s `w:val`, with the ST_OnOff spellings Word writes. An absent
/// attribute means "on", the element's own default.
fn word_wrap_value(reader: &Reader<&[u8]>, element: &BytesStart<'_>) -> Option<bool> {
    let value = element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.local_name().as_ref() == b"val")
        .and_then(|attribute| {
            attribute
                .decode_and_unescape_value(reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        });
    match value.as_deref() {
        Some("0") | Some("false") | Some("off") => Some(false),
        _ => Some(true),
    }
}

pub(in super::super) struct WordWrapContext {
    values: Vec<Option<bool>>,
    cursor: Cell<usize>,
}

impl WordWrapContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self {
            values: xml.map(Self::scan).unwrap_or_default(),
            cursor: Cell::new(0),
        }
    }

    /// The next paragraph's `w:wordWrap`, `None` when it states none. Must be
    /// called exactly once per converted `w:p`, like the shading context.
    pub(in super::super) fn next_word_wrap(&self) -> Option<bool> {
        let index = self.cursor.get();
        self.cursor.set(index + 1);
        self.values.get(index).copied().flatten()
    }

    fn scan(xml: &str) -> Vec<Option<bool>> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut values: Vec<Option<bool>> = Vec::new();
        let mut paragraph_stack: Vec<usize> = Vec::new();
        let mut in_body = false;
        let mut in_paragraph_properties = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(element)) => match element.local_name().as_ref() {
                    b"body" => in_body = true,
                    b"p" if in_body => {
                        values.push(None);
                        paragraph_stack.push(values.len() - 1);
                    }
                    b"pPr" if !paragraph_stack.is_empty() => in_paragraph_properties = true,
                    b"wordWrap" if in_paragraph_properties => {
                        if let Some(index) = paragraph_stack.last().copied() {
                            values[index] = word_wrap_value(&reader, &element);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Empty(element)) => match element.local_name().as_ref() {
                    b"p" if in_body => values.push(None),
                    b"wordWrap" if in_paragraph_properties => {
                        if let Some(index) = paragraph_stack.last().copied() {
                            values[index] = word_wrap_value(&reader, &element);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(element)) => match element.local_name().as_ref() {
                    b"body" => in_body = false,
                    b"p" if in_body => {
                        paragraph_stack.pop();
                        in_paragraph_properties = false;
                    }
                    b"pPr" => in_paragraph_properties = false,
                    _ => {}
                },
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }

        values
    }
}

/// Each paragraph style's `w:wordWrap` from `styles.xml`, by style id, for the
/// style chain of issue #730.
pub(in super::super) fn scan_style_word_wrap(xml: Option<&str>) -> HashMap<String, bool> {
    let Some(xml) = xml else {
        return HashMap::new();
    };
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut map: HashMap<String, bool> = HashMap::new();
    let mut current_style: Option<String> = None;
    let mut in_paragraph_properties = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                match element.local_name().as_ref() {
                    b"style" => {
                        // Paragraph styles only, like the shading scanner: a
                        // stray `w:wordWrap` on a character style must not
                        // leak into the paragraph-style chain.
                        let mut style_id: Option<String> = None;
                        let mut style_type: Option<String> = None;
                        for attribute in element.attributes().flatten() {
                            let value = attribute
                                .decode_and_unescape_value(reader.decoder())
                                .ok()
                                .map(|value| value.into_owned());
                            match attribute.key.local_name().as_ref() {
                                b"styleId" => style_id = value,
                                b"type" => style_type = value,
                                _ => {}
                            }
                        }
                        current_style =
                            style_id.filter(|_| style_type.as_deref() == Some("paragraph"));
                    }
                    b"pPr" if current_style.is_some() => in_paragraph_properties = true,
                    b"wordWrap" if in_paragraph_properties => {
                        if let (Some(style_id), Some(value)) =
                            (current_style.clone(), word_wrap_value(&reader, &element))
                        {
                            map.insert(style_id, value);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                b"style" => current_style = None,
                b"pPr" => in_paragraph_properties = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    map
}

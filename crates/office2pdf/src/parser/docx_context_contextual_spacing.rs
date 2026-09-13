//! Per-paragraph `w:contextualSpacing`, resolved from the raw document and
//! style XML (issue #1684).
//!
//! The flag is Word's "Don't add space between paragraphs of the same style",
//! on by default in the built-in `ListParagraph` style. Native Word exports of
//! one-factor probes fixed how it acts:
//!
//! - It acts on its own paragraph only. A flagged paragraph drops its
//!   `w:after` when the next paragraph has the same style, and its `w:before`
//!   when the previous one does; the neighbour's own flag does not matter.
//! - "Same style" compares effective style ids. A missing or undefined
//!   `w:pStyle` means the default paragraph style, and a `w:basedOn` child is
//!   a different style from its parent.
//! - The flag inherits down the `w:basedOn` chain from `w:pPrDefault`, and a
//!   paragraph's own `w:val="0"` overrides its style's.
//! - A dropped `w:after` still offsets a `w:before` that survives below it:
//!   dropping 10pt above a kept 15pt leaves 5pt, and dropping 20pt leaves
//!   none. Word collapses the two gaps to their maximum by subtracting the
//!   `w:after` from the `w:before`, and the subtraction outlives the drop.
//! - A table cell is a flow of its own, like the body. The paragraph directly
//!   above a table and the table's first paragraph are neighbours, but their
//!   gaps add up across the boundary instead of collapsing, so no offset
//!   applies there. The paragraph directly below a table follows the table's
//!   end-of-row mark, which has the default paragraph style.
//!
//! The published `docx-rs` parses none of this, and reading a fork field would
//! make the crate unpublishable (issue #1041), so the flag is scanned from the
//! XML like `w:wordWrap`, and adjacency is read off the same element tree.
//!
//! Paragraphs are counted in document order exactly as the converter reaches
//! them: every `w:p` of the body, its table cells and its DrawingML text
//! boxes, but none under `mc:Fallback` (docx-rs discards that branch) or VML
//! `w:pict`/`w:object` (their text boxes are rebuilt from the raw XML without
//! paragraph conversion). A `w:pStyle` that disagrees with the converted
//! paragraph's means the two sequences drifted apart, and the context then
//! stops applying anything rather than move a gap onto the wrong paragraph.

use crate::parser::xml_util::OOXML_XML_VERSION;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

fn attribute_value(
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

/// `w:contextualSpacing`'s ST_OnOff value; an absent `w:val` means on.
fn contextual_spacing_value(reader: &Reader<&[u8]>, element: &BytesStart<'_>) -> bool {
    !matches!(
        attribute_value(reader, element, b"val").as_deref(),
        Some("0" | "false" | "off")
    )
}

/// Subtrees whose paragraphs never reach paragraph conversion (see the module
/// comment), or whose properties are not the element's current ones.
fn is_skipped_subtree(local_name: &[u8]) -> bool {
    matches!(local_name, b"Fallback" | b"pict" | b"object" | b"pPrChange")
}

/// One paragraph style's own link in the flag's `w:basedOn` chain.
#[derive(Default)]
struct ParagraphStyleDefinition {
    based_on: Option<String>,
    contextual_spacing: Option<bool>,
}

/// What `styles.xml` contributes: each paragraph's flag, and which style a
/// paragraph effectively has.
#[derive(Default)]
struct StyleSheet {
    styles: HashMap<String, ParagraphStyleDefinition>,
    default_style_id: Option<String>,
    /// `w:docDefaults/w:pPrDefault`'s flag, where every chain ends.
    document_default: Option<bool>,
}

impl StyleSheet {
    fn scan(xml: &str) -> Self {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut sheet = StyleSheet::default();
        let mut current_style: Option<(String, ParagraphStyleDefinition)> = None;
        let mut in_paragraph_defaults = false;
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
                        b"style" => {
                            if let Some((style_id, definition)) = current_style.take() {
                                sheet.styles.insert(style_id, definition);
                            }
                        }
                        b"pPrDefault" => in_paragraph_defaults = false,
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
                b"style" => {
                    // Paragraph styles only: `w:pStyle` cannot name any other
                    // type, and a character style's flag would be noise.
                    let is_paragraph_style =
                        attribute_value(&reader, &element, b"type").as_deref() == Some("paragraph");
                    let style_id = attribute_value(&reader, &element, b"styleId")
                        .filter(|_| is_paragraph_style);
                    let is_default = matches!(
                        attribute_value(&reader, &element, b"default").as_deref(),
                        Some("1" | "true")
                    );
                    if let Some(style_id) = style_id {
                        // The first default wins, as it does for the style map.
                        if is_default && sheet.default_style_id.is_none() {
                            sheet.default_style_id = Some(style_id.clone());
                        }
                        if is_empty {
                            sheet
                                .styles
                                .insert(style_id, ParagraphStyleDefinition::default());
                        } else {
                            current_style = Some((style_id, ParagraphStyleDefinition::default()));
                        }
                    }
                }
                b"basedOn" if !in_paragraph_properties => {
                    if let Some((_, definition)) = current_style.as_mut() {
                        definition.based_on = attribute_value(&reader, &element, b"val");
                    }
                }
                b"pPrDefault" if !is_empty => in_paragraph_defaults = true,
                b"pPr" if !is_empty => in_paragraph_properties = true,
                b"contextualSpacing" if in_paragraph_properties => {
                    let value = contextual_spacing_value(&reader, &element);
                    if let Some((_, definition)) = current_style.as_mut() {
                        definition.contextual_spacing = Some(value);
                    } else if in_paragraph_defaults {
                        sheet.document_default = Some(value);
                    }
                }
                _ => {}
            }
        }

        sheet
    }

    /// The style a paragraph takes its formatting from: its `w:pStyle` when
    /// that names a defined paragraph style, else the default style (`None`
    /// when the document defines none, which every bare paragraph shares).
    fn effective_style_id<'a>(&'a self, style_id: Option<&'a str>) -> Option<&'a str> {
        match style_id {
            Some(style_id) if self.styles.contains_key(style_id) => Some(style_id),
            _ => self.default_style_id.as_deref(),
        }
    }

    /// The flag a style's `w:basedOn` chain states, nearest link first. A
    /// cycle drops the parents, as it does for the style map (issue #1453).
    fn style_contextual_spacing(&self, style_id: &str) -> Option<bool> {
        let mut chain: Vec<&ParagraphStyleDefinition> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut current: Option<&str> = Some(style_id);
        while let Some(id) = current {
            let Some(definition) = self.styles.get(id) else {
                break;
            };
            if !seen.insert(id) {
                chain.truncate(1);
                break;
            }
            chain.push(definition);
            current = definition.based_on.as_deref();
        }
        chain
            .iter()
            .find_map(|definition| definition.contextual_spacing)
    }

    fn paragraph_contextual_spacing(&self, paragraph: &ScannedParagraph) -> bool {
        paragraph
            .contextual_spacing
            .or_else(|| {
                self.effective_style_id(paragraph.style_id.as_deref())
                    .and_then(|style_id| self.style_contextual_spacing(style_id))
            })
            .or(self.document_default)
            .unwrap_or(false)
    }
}

/// Where a flow stands, as seen by the next paragraph to arrive in it.
#[derive(Clone, Copy)]
enum FlowCursor {
    Start,
    Paragraph(usize),
    TableEnd,
}

/// What a paragraph's `w:before` meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Above {
    /// The paragraph directly above in the same flow.
    Paragraph(usize),
    /// The paragraph directly above the table this paragraph opens. Word
    /// drops both gaps across the boundary (probe c25), but adds them rather
    /// than collapsing them when they stand (c26).
    ParagraphAboveTable(usize),
    /// A table's end-of-row mark, which has the default paragraph style: a
    /// flagged default-style paragraph below a table drops its `w:before`
    /// (probes c29/c30), one of another style keeps it (c23).
    TableEnd,
}

/// A `w:p` as the document scan saw it.
#[derive(Default)]
struct ScannedParagraph {
    /// The paragraph's own `w:pStyle`, unresolved.
    style_id: Option<String>,
    /// The paragraph's own `w:contextualSpacing`, if it states one.
    contextual_spacing: Option<bool>,
    above: Option<Above>,
    /// The paragraph its `w:after` meets: the one directly below in its flow,
    /// or the first paragraph of a table directly below. The last paragraph
    /// of a cell meets none (probes c27/c28).
    next: Option<usize>,
}

fn scan_paragraphs(xml: &str) -> Vec<ScannedParagraph> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut paragraphs: Vec<ScannedParagraph> = Vec::new();
    // One cursor per open flow: the body, a table cell, a text box.
    let mut flows: Vec<FlowCursor> = Vec::new();
    let mut open_paragraphs: Vec<usize> = Vec::new();
    // The paragraph above a table, until the table's first paragraph arrives.
    let mut paragraph_above_table: Option<usize> = None;
    let mut in_body = false;
    let mut in_paragraph_properties = false;
    let mut skipped_depth: usize = 0;

    let open_paragraph = |paragraphs: &mut Vec<ScannedParagraph>,
                          flows: &[FlowCursor],
                          paragraph_above_table: &mut Option<usize>|
     -> usize {
        let index = paragraphs.len();
        let above = match (paragraph_above_table.take(), flows.last()) {
            (Some(above), _) => {
                paragraphs[above].next = Some(index);
                Some(Above::ParagraphAboveTable(above))
            }
            (None, Some(&FlowCursor::Paragraph(above))) => {
                paragraphs[above].next = Some(index);
                Some(Above::Paragraph(above))
            }
            (None, Some(FlowCursor::TableEnd)) => Some(Above::TableEnd),
            (None, Some(FlowCursor::Start) | None) => None,
        };
        paragraphs.push(ScannedParagraph {
            above,
            ..ScannedParagraph::default()
        });
        index
    };

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
                    b"body" => {
                        in_body = false;
                        flows.pop();
                    }
                    b"tc" | b"txbxContent" if in_body => {
                        flows.pop();
                    }
                    b"tbl" if in_body => {
                        paragraph_above_table = None;
                        if let Some(flow) = flows.last_mut() {
                            *flow = FlowCursor::TableEnd;
                        }
                    }
                    b"p" if in_body => {
                        if let Some(index) = open_paragraphs.pop()
                            && let Some(flow) = flows.last_mut()
                        {
                            *flow = FlowCursor::Paragraph(index);
                        }
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
            b"body" if !is_empty => {
                in_body = true;
                flows.push(FlowCursor::Start);
            }
            b"tc" | b"txbxContent" if in_body && !is_empty => flows.push(FlowCursor::Start),
            b"tbl" if in_body => {
                // A nested table opening a cell leaves the outer table's
                // pending paragraph waiting: the first paragraph in document
                // order is the one Word meets.
                if let Some(&FlowCursor::Paragraph(above)) = flows.last() {
                    paragraph_above_table = Some(above);
                }
            }
            b"p" if in_body => {
                let index = open_paragraph(&mut paragraphs, &flows, &mut paragraph_above_table);
                if is_empty {
                    if let Some(flow) = flows.last_mut() {
                        *flow = FlowCursor::Paragraph(index);
                    }
                } else {
                    open_paragraphs.push(index);
                }
            }
            b"pPr" if !is_empty && !open_paragraphs.is_empty() => {
                in_paragraph_properties = true;
            }
            b"pStyle" if in_paragraph_properties => {
                if let Some(&index) = open_paragraphs.last() {
                    paragraphs[index].style_id = attribute_value(&reader, &element, b"val");
                }
            }
            b"contextualSpacing" if in_paragraph_properties => {
                if let Some(&index) = open_paragraphs.last() {
                    paragraphs[index].contextual_spacing =
                        Some(contextual_spacing_value(&reader, &element));
                }
            }
            _ => {}
        }
    }

    paragraphs
}

/// What the flag does to one paragraph.
#[derive(Debug, Default, PartialEq, Eq)]
struct ParagraphContextualRule {
    /// The paragraph's own `w:pStyle`, to check the converter is on the same
    /// paragraph as the scan.
    style_id: Option<String>,
    drops_before: bool,
    drops_after: bool,
    /// The paragraph above, when it dropped its `w:after` toward this one.
    after_dropped_above: Option<usize>,
}

fn resolve_rules(
    paragraphs: Vec<ScannedParagraph>,
    sheet: &StyleSheet,
) -> Vec<ParagraphContextualRule> {
    let style_ids: Vec<Option<&str>> = paragraphs
        .iter()
        .map(|paragraph| sheet.effective_style_id(paragraph.style_id.as_deref()))
        .collect();
    let flags: Vec<bool> = paragraphs
        .iter()
        .map(|paragraph| sheet.paragraph_contextual_spacing(paragraph))
        .collect();
    let row_end_style_id: Option<&str> = sheet.effective_style_id(None);

    let drops_after: Vec<bool> = paragraphs
        .iter()
        .enumerate()
        .map(|(index, paragraph)| {
            flags[index]
                && paragraph
                    .next
                    .is_some_and(|next| style_ids[next] == style_ids[index])
        })
        .collect();
    paragraphs
        .iter()
        .enumerate()
        .map(|(index, paragraph)| {
            let has_same_style_above = match paragraph.above {
                Some(Above::Paragraph(above) | Above::ParagraphAboveTable(above)) => {
                    style_ids[above] == style_ids[index]
                }
                Some(Above::TableEnd) => style_ids[index] == row_end_style_id,
                None => false,
            };
            ParagraphContextualRule {
                style_id: paragraph.style_id.clone(),
                drops_before: flags[index] && has_same_style_above,
                drops_after: drops_after[index],
                // `drops_after[above]` already means the paragraph above
                // shares this one's style, because this is its `next`.
                after_dropped_above: match paragraph.above {
                    Some(Above::Paragraph(above)) if drops_after[above] => Some(above),
                    _ => None,
                },
            }
        })
        .collect()
}

/// `w:contextualSpacing` for every paragraph of `document.xml`, consumed once
/// per converted `w:p` like the other paragraph cursors.
///
/// TODO(header/footer parts): those parts convert through their own paragraph
/// path with no cursor, so their flagged paragraphs still keep every gap.
pub(in super::super) struct ContextualSpacingContext {
    rules: Vec<ParagraphContextualRule>,
    cursor: Cell<usize>,
    has_drifted: Cell<bool>,
    /// Each dropped `w:after` as it stood before the drop, for the offset it
    /// still applies to the `w:before` below it.
    dropped_space_after: RefCell<HashMap<usize, f64>>,
}

impl ContextualSpacingContext {
    pub(in super::super) fn from_xml(document_xml: Option<&str>, styles_xml: Option<&str>) -> Self {
        let rules = match document_xml {
            Some(document_xml) => {
                let sheet = styles_xml.map(StyleSheet::scan).unwrap_or_default();
                resolve_rules(scan_paragraphs(document_xml), &sheet)
            }
            None => Vec::new(),
        };
        Self {
            rules,
            cursor: Cell::new(0),
            has_drifted: Cell::new(false),
            dropped_space_after: RefCell::new(HashMap::new()),
        }
    }

    /// The next paragraph's share of the context. Must be called exactly once
    /// per converted `w:p`, with that paragraph's `w:pStyle`.
    pub(in super::super) fn next_paragraph(
        &self,
        style_id: Option<&str>,
    ) -> ParagraphContextualSpacing<'_> {
        let index = self.cursor.get();
        self.cursor.set(index + 1);
        let index = match self.rules.get(index) {
            _ if self.has_drifted.get() => None,
            Some(rule) if rule.style_id.as_deref() == style_id => Some(index),
            Some(rule) => {
                self.has_drifted.set(true);
                tracing::warn!(
                    paragraph_index = index,
                    scanned_style = rule.style_id.as_deref().unwrap_or(""),
                    converted_style = style_id.unwrap_or(""),
                    "w:contextualSpacing scan drifted from the converted paragraphs; \
                     leaving the remaining paragraph gaps as stated"
                );
                None
            }
            None => None,
        };
        ParagraphContextualSpacing {
            context: self,
            index,
        }
    }
}

/// One converted paragraph's handle on [`ContextualSpacingContext`].
#[derive(Clone, Copy)]
pub(in super::super) struct ParagraphContextualSpacing<'a> {
    context: &'a ContextualSpacingContext,
    index: Option<usize>,
}

impl ParagraphContextualSpacing<'_> {
    /// Drop the resolved gaps the flag suppresses, and offset a surviving
    /// `w:before` by the dropped `w:after` above it.
    pub(in super::super) fn apply(
        self,
        space_before: &mut Option<f64>,
        space_after: &mut Option<f64>,
    ) {
        let Some(index) = self.index else {
            return;
        };
        let rule: &ParagraphContextualRule = &self.context.rules[index];
        if rule.drops_after
            && let Some(after) = space_after.replace(0.0)
        {
            self.context
                .dropped_space_after
                .borrow_mut()
                .insert(index, after);
        }
        if rule.drops_before {
            if space_before.is_some() {
                *space_before = Some(0.0);
            }
        } else if let Some(above) = rule.after_dropped_above
            && let Some(before) = *space_before
            && let Some(dropped) = self
                .context
                .dropped_space_after
                .borrow()
                .get(&above)
                .copied()
        {
            *space_before = Some((before - dropped).max(0.0));
        }
    }
}

#[cfg(test)]
#[path = "docx_context_contextual_spacing_tests.rs"]
mod tests;

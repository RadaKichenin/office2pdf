use std::cell::Cell;

use crate::ir::{BorderSide, Color, Insets};

use super::docx_context_shape::{ShapeBuilder, ShapeScanState};

#[derive(Debug, Clone, Default)]
pub(in super::super) struct DrawingTextBoxInfo {
    pub(in super::super) width_pt: Option<f64>,
    pub(in super::super) height_pt: Option<f64>,
    /// Outline from the shape's `a:ln`, `None` when it paints none.
    pub(in super::super) stroke: Option<BorderSide>,
    /// Background from the shape's `a:solidFill`.
    pub(in super::super) fill: Option<Color>,
    /// Where the box's text starts inside it, from `wps:bodyPr`.
    pub(in super::super) padding: Insets,
}

pub(in super::super) struct DrawingTextBoxContext {
    text_boxes: Vec<DrawingTextBoxInfo>,
    cursor: Cell<usize>,
}

impl DrawingTextBoxContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self {
            text_boxes: xml.map(scan_drawing_text_boxes).unwrap_or_default(),
            cursor: Cell::new(0),
        }
    }

    pub(in super::super) fn consume_next(&self) -> DrawingTextBoxInfo {
        let index = self.cursor.get();
        self.cursor.set(index + 1);
        self.text_boxes.get(index).cloned().unwrap_or_default()
    }
}

/// Scan `word/document.xml` for every `wps:wsp` text box drawing, in document
/// order: its on-page extent, and the frame it paints around its text.
///
/// The element dispatch is [`ShapeScanState`]'s, shared with the geometry-only
/// shape scan, so `wp:extent`, `a:ln`, `a:solidFill` and `wps:bodyPr` are read
/// one way for both (issue #1690).
fn scan_drawing_text_boxes(xml: &str) -> Vec<DrawingTextBoxInfo> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut result: Vec<DrawingTextBoxInfo> = Vec::new();
    let mut in_body: bool = false;
    let mut drawing_depth: usize = 0;
    let mut state: ShapeScanState = ShapeScanState::default();
    let mut builder: Option<ShapeBuilder> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(quick_xml::events::Event::Start(ref element)) => {
                match element.local_name().as_ref() {
                    b"body" => in_body = true,
                    b"drawing" if in_body => {
                        if drawing_depth == 0 {
                            builder = Some(ShapeBuilder::default());
                            state.reset();
                        }
                        drawing_depth += 1;
                    }
                    // Only the box's own `wps:wsp` describes the box. A nested
                    // `w:drawing` inside `w:txbxContent` — a picture in one of
                    // the box's paragraphs — carries its own `wp:extent`,
                    // `a:ln` and `a:solidFill`, and would otherwise overwrite
                    // the box's size and frame with the picture's.
                    _ if drawing_depth == 1 => state.start(builder.as_mut(), element),
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Empty(ref element)) => {
                if drawing_depth == 1 {
                    state.empty(builder.as_mut(), element);
                }
            }
            Ok(quick_xml::events::Event::End(ref element)) => match element.local_name().as_ref() {
                b"body" => in_body = false,
                b"drawing" if drawing_depth > 0 => {
                    drawing_depth -= 1;
                    if drawing_depth == 0
                        && let Some(builder) = builder.take()
                        && builder.is_text_box()
                    {
                        result.push(text_box_info(&builder));
                    }
                }
                other if drawing_depth == 1 => state.end(other),
                _ => {}
            },
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    result
}

fn text_box_info(builder: &ShapeBuilder) -> DrawingTextBoxInfo {
    let (width_pt, height_pt) = builder.box_size_pt();
    let (stroke, fill) = builder.text_box_frame();
    DrawingTextBoxInfo {
        width_pt,
        height_pt,
        stroke,
        fill,
        padding: builder.text_box_insets(),
    }
}

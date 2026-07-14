//! Raw-XML side-channel for floating drawing *shapes* (`wps:wsp`).
//!
//! docx-rs (the upstream DOCX parser) only models `<w:drawing>` data as either a
//! picture (`Pic`) or a text box (`TextBox`). A DrawingML word-processing shape
//! that carries geometry but no text box — e.g. a `<a:prstGeom prst="rect">`
//! rectangle or a `prst="line"` connector/arrow authored by LibreOffice — parses
//! into a `Drawing` with `data == None`, so its geometry, fill and stroke are
//! lost entirely (issue #176).
//!
//! This module scans the raw `word/document.xml` for such shapes in document
//! order and exposes them through a cursor that the main walk advances once per
//! geometry-only drawing it encounters, mirroring [`DrawingTextBoxContext`] and
//! [`VmlTextBoxContext`].
//!
//! [`DrawingTextBoxContext`]: super::docx_context_drawing::DrawingTextBoxContext
//! [`VmlTextBoxContext`]: super::docx_context_vml::VmlTextBoxContext

use std::cell::Cell;
use std::collections::HashMap;

use docx_rs::FromXML;
use quick_xml::events::{BytesStart, Event};

use crate::ir::{
    ArrowHead, BorderLineStyle, BorderSide, Color, FloatingShape, Insets, Shape, ShapeKind,
    TextBoxVerticalAlign, WrapMode,
};
use crate::parser::units::emu_to_pt;
use crate::parser::xml_util::parse_hex_color;

#[derive(Debug, Clone)]
pub(in super::super) struct WpgShapeInfo {
    pub(in super::super) shape: Option<FloatingShape>,
    pub(in super::super) content: Vec<docx_rs::DocumentChild>,
    pub(in super::super) width: f64,
    pub(in super::super) height: f64,
    pub(in super::super) padding: Insets,
    pub(in super::super) vertical_align: TextBoxVerticalAlign,
    pub(in super::super) text_color: Option<Color>,
    pub(in super::super) offset_x: f64,
    pub(in super::super) offset_y: f64,
    pub(in super::super) wrap_mode: WrapMode,
}

#[derive(Debug, Clone, Default)]
pub(in super::super) struct WpgDrawingInfo {
    pub(in super::super) children: Vec<WpgShapeInfo>,
}

/// Default stroke width (pt) when a shape's `<a:ln w="0">` requests the
/// renderer's hairline default. Word/LibreOffice treat `w="0"` as "thin but
/// visible"; 0 pt would make the outline disappear.
const DEFAULT_STROKE_WIDTH_PT: f64 = 0.75;

/// EMU per point (914400 EMU/inch ÷ 72 pt/inch).
const EMU_PER_POINT: f64 = 12700.0;

/// Floating geometry-only shapes scanned from `word/document.xml`, consumed in
/// document order alongside the docx-rs element walk.
#[derive(Debug, Clone)]
pub(in super::super) struct DrawingShapeContext {
    shapes: Vec<FloatingShape>,
    cursor: Cell<usize>,
    wpg_drawings: Vec<Option<WpgDrawingInfo>>,
    wpg_cursor: Cell<usize>,
    canvas_image_offsets: Vec<Option<(f64, f64)>>,
    canvas_cursor: Cell<usize>,
}

impl DrawingShapeContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self::from_xml_with_theme(xml, None)
    }

    pub(in super::super) fn from_xml_with_theme(
        xml: Option<&str>,
        theme_xml: Option<&str>,
    ) -> Self {
        Self {
            shapes: xml.map(scan_drawing_shapes).unwrap_or_default(),
            cursor: Cell::new(0),
            wpg_drawings: xml
                .map(|xml| scan_wpg_drawings(xml, theme_xml))
                .unwrap_or_default(),
            wpg_cursor: Cell::new(0),
            canvas_image_offsets: xml.map(scan_canvas_image_offsets).unwrap_or_default(),
            canvas_cursor: Cell::new(0),
        }
    }

    /// Return the next scanned shape, advancing the cursor. Returns `None` once
    /// the scanned shapes are exhausted so a mismatched walk degrades to "no
    /// shape" rather than panicking.
    pub(in super::super) fn consume_next(&self) -> Option<FloatingShape> {
        let index: usize = self.cursor.get();
        self.cursor.set(index + 1);
        self.shapes.get(index).cloned()
    }

    /// Advance once for every docx-rs `Drawing`, returning WPG children only
    /// when the matching raw drawing is a WordprocessingGroup.
    pub(in super::super) fn consume_wpg_drawing(&self) -> Option<WpgDrawingInfo> {
        let index: usize = self.wpg_cursor.get();
        self.wpg_cursor.set(index + 1);
        self.wpg_drawings.get(index).cloned().flatten()
    }

    /// Return the selected picture's offset inside a WordprocessingCanvas.
    /// The cursor advances for every docx-rs drawing so AlternateContent
    /// fallbacks remain metadata-only and are never rendered a second time.
    pub(in super::super) fn consume_canvas_image_offset(&self) -> Option<(f64, f64)> {
        let index: usize = self.canvas_cursor.get();
        self.canvas_cursor.set(index + 1);
        self.canvas_image_offsets.get(index).copied().flatten()
    }
}

/// Which `<wp:positionH>` / `<wp:positionV>` axis the current `<wp:posOffset>`
/// text belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PositionAxis {
    None,
    Horizontal,
    Vertical,
}

/// Mutable accumulator for a single `<w:drawing>` while scanning.
#[derive(Default)]
struct ShapeBuilder {
    has_wsp: bool,
    has_wpg: bool,
    has_text_box: bool,
    preset: Option<String>,
    box_width_pt: Option<f64>,
    box_height_pt: Option<f64>,
    offset_x_pt: f64,
    offset_y_pt: f64,
    flip_h: bool,
    flip_v: bool,
    fill_color: Option<Color>,
    fill_none: bool,
    line_color: Option<Color>,
    line_width_pt: Option<f64>,
    line_none: bool,
    has_line: bool,
    head_arrow: bool,
    tail_arrow: bool,
}

impl ShapeBuilder {
    /// Build a [`FloatingShape`] from the accumulated geometry, or `None` when
    /// this drawing is not a geometry-only shape (it is a picture or a text box,
    /// both handled by docx-rs).
    fn finish(self) -> Option<FloatingShape> {
        self.finish_with_text_box(false)
    }

    fn finish_wpg(self) -> Option<FloatingShape> {
        self.finish_with_text_box(true)
    }

    fn finish_with_text_box(self, allow_text_box: bool) -> Option<FloatingShape> {
        if !self.has_wsp || self.has_wpg || (self.has_text_box && !allow_text_box) {
            return None;
        }

        let width: f64 = self.box_width_pt.unwrap_or(0.0);
        let height: f64 = self.box_height_pt.unwrap_or(0.0);
        let kind: ShapeKind = self.resolve_kind(width, height);

        let fill: Option<Color> = if self.fill_none {
            None
        } else {
            self.fill_color
        };
        let stroke: Option<BorderSide> = self.resolve_stroke();

        // A shape with neither fill, stroke nor a line geometry would render as
        // nothing — skip it so we stay in sync with the renderer.
        let is_line: bool = matches!(kind, ShapeKind::Line { .. });
        if fill.is_none() && stroke.is_none() && !is_line {
            return None;
        }

        Some(FloatingShape {
            shape: Shape {
                kind,
                fill,
                gradient_fill: None,
                stroke,
                rotation_deg: None,
                opacity: None,
                shadow: None,
            },
            width,
            height,
            offset_x: self.offset_x_pt,
            offset_y: self.offset_y_pt,
            wrap_mode: WrapMode::None,
        })
    }

    fn resolve_kind(&self, width: f64, height: f64) -> ShapeKind {
        match self.preset.as_deref() {
            Some("line") | Some("straightConnector1") => {
                // Endpoints run corner-to-corner of the bounding box; flips swap
                // the diagonal direction (no-op for axis-aligned lines).
                let (x1, x2) = if self.flip_h {
                    (width, 0.0)
                } else {
                    (0.0, width)
                };
                let (y1, y2) = if self.flip_v {
                    (height, 0.0)
                } else {
                    (0.0, height)
                };
                ShapeKind::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    head_end: arrow(self.head_arrow),
                    tail_end: arrow(self.tail_arrow),
                }
            }
            Some("ellipse") | Some("oval") => ShapeKind::Ellipse,
            Some("roundRect") => ShapeKind::RoundedRectangle {
                radius_fraction: 0.1,
            },
            Some("triangle") => ShapeKind::Polygon {
                vertices: vec![(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)],
            },
            Some("diamond") => ShapeKind::Polygon {
                vertices: vec![(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)],
            },
            // "rect" and any unsupported preset fall back to a rectangle so the
            // shape's area, fill and outline are still conveyed.
            _ => ShapeKind::Rectangle,
        }
    }

    fn resolve_stroke(&self) -> Option<BorderSide> {
        if self.line_none || !self.has_line {
            return None;
        }
        let width: f64 = match self.line_width_pt {
            Some(width) if width > 0.0 => width,
            _ => DEFAULT_STROKE_WIDTH_PT,
        };
        Some(BorderSide {
            width,
            color: self.line_color.unwrap_or(Color { r: 0, g: 0, b: 0 }),
            style: BorderLineStyle::Solid,
        })
    }
}

fn arrow(present: bool) -> ArrowHead {
    if present {
        ArrowHead::Triangle
    } else {
        ArrowHead::None
    }
}

fn attribute_value(element: &BytesStart<'_>, name: &[u8]) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (attribute.key.local_name().as_ref() == name)
            .then(|| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
    })
}

fn emu_attr_to_pt(element: &BytesStart<'_>, name: &[u8]) -> Option<f64> {
    attribute_value(element, name)
        .and_then(|value| value.parse::<i64>().ok())
        .map(emu_to_pt)
}

fn bool_attr(element: &BytesStart<'_>, name: &[u8]) -> bool {
    matches!(
        attribute_value(element, name).as_deref(),
        Some("1") | Some("true")
    )
}

/// Scan `word/document.xml`, returning one [`FloatingShape`] per geometry-only
/// `wps:wsp` drawing, in document order.
fn scan_drawing_shapes(xml: &str) -> Vec<FloatingShape> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut result: Vec<FloatingShape> = Vec::new();

    let mut drawing_depth: usize = 0;
    let mut sppr_depth: usize = 0;
    let mut line_depth: usize = 0;
    let mut axis: PositionAxis = PositionAxis::None;
    let mut in_position_offset: bool = false;
    let mut builder: Option<ShapeBuilder> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) => match element.local_name().as_ref() {
                b"drawing" => {
                    drawing_depth += 1;
                    if drawing_depth == 1 {
                        builder = Some(ShapeBuilder::default());
                        sppr_depth = 0;
                        line_depth = 0;
                        axis = PositionAxis::None;
                        in_position_offset = false;
                    }
                }
                b"positionH" => axis = PositionAxis::Horizontal,
                b"positionV" => axis = PositionAxis::Vertical,
                b"posOffset" => in_position_offset = true,
                b"spPr" if builder.is_some() => sppr_depth += 1,
                b"ln" if builder.is_some() => {
                    line_depth += 1;
                    if let Some(builder) = builder.as_mut() {
                        builder.has_line = true;
                        builder.line_width_pt =
                            emu_attr_to_pt(element, b"w").or(builder.line_width_pt);
                    }
                }
                other => handle_geometry_element(
                    builder.as_mut(),
                    other,
                    element,
                    sppr_depth,
                    line_depth,
                ),
            },
            Ok(Event::Empty(ref element)) => {
                handle_geometry_element(
                    builder.as_mut(),
                    element.local_name().as_ref(),
                    element,
                    sppr_depth,
                    line_depth,
                );
            }
            Ok(Event::Text(ref text)) => {
                if in_position_offset
                    && let Some(builder) = builder.as_mut()
                    && let Ok(raw) = text.xml_content()
                    && let Ok(emu) = raw.trim().parse::<i64>()
                {
                    let pt: f64 = (emu as f64) / EMU_PER_POINT;
                    match axis {
                        PositionAxis::Horizontal => builder.offset_x_pt = pt,
                        PositionAxis::Vertical => builder.offset_y_pt = pt,
                        PositionAxis::None => {}
                    }
                }
            }
            Ok(Event::End(ref element)) => match element.local_name().as_ref() {
                b"posOffset" => in_position_offset = false,
                b"positionH" | b"positionV" => axis = PositionAxis::None,
                b"spPr" if sppr_depth > 0 => sppr_depth -= 1,
                b"ln" if line_depth > 0 => line_depth -= 1,
                b"drawing" if drawing_depth > 0 => {
                    drawing_depth -= 1;
                    if drawing_depth == 0
                        && let Some(shape) = builder.take().and_then(ShapeBuilder::finish)
                    {
                        result.push(shape);
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    result
}

/// Apply a geometry/fill/stroke element (`wsp`, `txbx`, `extent`, `prstGeom`,
/// `xfrm`, `srgbClr`, `noFill`, `tailEnd`, `headEnd`) to the current builder.
fn handle_geometry_element(
    builder: Option<&mut ShapeBuilder>,
    local_name: &[u8],
    element: &BytesStart<'_>,
    sppr_depth: usize,
    line_depth: usize,
) {
    let Some(builder) = builder else {
        return;
    };

    match local_name {
        b"wsp" => builder.has_wsp = true,
        b"wgp" => builder.has_wpg = true,
        b"txbx" => builder.has_text_box = true,
        // The anchor extent gives the on-page bounding box.
        b"extent" => {
            if let Some(width) = emu_attr_to_pt(element, b"cx") {
                builder.box_width_pt = Some(width);
            }
            if let Some(height) = emu_attr_to_pt(element, b"cy") {
                builder.box_height_pt = Some(height);
            }
        }
        b"prstGeom" => {
            if let Some(preset) = attribute_value(element, b"prst") {
                builder.preset = Some(preset);
            }
        }
        b"xfrm" => {
            builder.flip_h = bool_attr(element, b"flipH");
            builder.flip_v = bool_attr(element, b"flipV");
        }
        b"srgbClr" if sppr_depth > 0 => {
            if let Some(color) = attribute_value(element, b"val").and_then(|v| parse_hex_color(&v))
            {
                if line_depth > 0 {
                    builder.line_color = builder.line_color.or(Some(color));
                } else {
                    builder.fill_color = builder.fill_color.or(Some(color));
                }
            }
        }
        b"noFill" if sppr_depth > 0 => {
            if line_depth > 0 {
                builder.line_none = true;
            } else {
                builder.fill_none = true;
            }
        }
        b"tailEnd" if line_depth > 0 => builder.tail_arrow = arrow_type_present(element),
        b"headEnd" if line_depth > 0 => builder.head_arrow = arrow_type_present(element),
        _ => {}
    }
}

fn arrow_type_present(element: &BytesStart<'_>) -> bool {
    !matches!(attribute_value(element, b"type").as_deref(), Some("none"))
}

#[derive(Default)]
struct CanvasDrawingBuilder {
    record_index: usize,
    is_canvas: bool,
    picture_depth: usize,
    shape_properties_depth: usize,
    transform_depth: usize,
    offset: Option<(f64, f64)>,
}

fn scan_canvas_image_offsets(xml: &str) -> Vec<Option<(f64, f64)>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut records: Vec<Option<(f64, f64)>> = Vec::new();
    let mut drawings: Vec<CanvasDrawingBuilder> = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) if element.local_name().as_ref() == b"drawing" => {
                let record_index: usize = records.len();
                records.push(None);
                drawings.push(CanvasDrawingBuilder {
                    record_index,
                    ..CanvasDrawingBuilder::default()
                });
            }
            Ok(Event::Start(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_canvas_start(drawing, element);
                }
            }
            Ok(Event::Empty(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_canvas_element(drawing, element);
                }
            }
            Ok(Event::End(ref element)) if element.local_name().as_ref() == b"drawing" => {
                if let Some(drawing) = drawings.pop() {
                    records[drawing.record_index] = drawing.offset;
                }
            }
            Ok(Event::End(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_canvas_end(drawing, element.local_name().as_ref());
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    records
}

fn handle_canvas_start(drawing: &mut CanvasDrawingBuilder, element: &BytesStart<'_>) {
    match element.local_name().as_ref() {
        b"graphicData"
            if attribute_value(element, b"uri")
                .is_some_and(|uri| uri.ends_with("/wordprocessingCanvas")) =>
        {
            drawing.is_canvas = true;
        }
        b"pic" if drawing.is_canvas => drawing.picture_depth += 1,
        b"spPr" if drawing.picture_depth > 0 => drawing.shape_properties_depth += 1,
        b"xfrm" if drawing.shape_properties_depth > 0 => drawing.transform_depth += 1,
        _ => handle_canvas_element(drawing, element),
    }
}

fn handle_canvas_element(drawing: &mut CanvasDrawingBuilder, element: &BytesStart<'_>) {
    if drawing.transform_depth == 0 || element.local_name().as_ref() != b"off" {
        return;
    }
    let Some(x) = numeric_attr(element, b"x") else {
        return;
    };
    let Some(y) = numeric_attr(element, b"y") else {
        return;
    };
    drawing.offset = Some((x / EMU_PER_POINT, y / EMU_PER_POINT));
}

fn handle_canvas_end(drawing: &mut CanvasDrawingBuilder, local_name: &[u8]) {
    match local_name {
        b"xfrm" if drawing.transform_depth > 0 => drawing.transform_depth -= 1,
        b"spPr" if drawing.shape_properties_depth > 0 => drawing.shape_properties_depth -= 1,
        b"pic" if drawing.picture_depth > 0 => drawing.picture_depth -= 1,
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
struct AffineTransform {
    scale_x: f64,
    scale_y: f64,
    translate_x: f64,
    translate_y: f64,
}

impl Default for AffineTransform {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }
}

impl AffineTransform {
    fn compose(self, child: Self) -> Self {
        Self {
            scale_x: self.scale_x * child.scale_x,
            scale_y: self.scale_y * child.scale_y,
            translate_x: self.translate_x + child.translate_x * self.scale_x,
            translate_y: self.translate_y + child.translate_y * self.scale_y,
        }
    }

    fn point(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.translate_x + x * self.scale_x,
            self.translate_y + y * self.scale_y,
        )
    }
}

#[derive(Default)]
struct GroupTransformBuilder {
    offset_x: f64,
    offset_y: f64,
    extent_x: f64,
    extent_y: f64,
    child_offset_x: f64,
    child_offset_y: f64,
    child_extent_x: f64,
    child_extent_y: f64,
}

impl GroupTransformBuilder {
    fn finish(&self) -> AffineTransform {
        let scale_x: f64 = if self.child_extent_x.abs() > f64::EPSILON {
            self.extent_x / self.child_extent_x
        } else {
            1.0
        };
        let scale_y: f64 = if self.child_extent_y.abs() > f64::EPSILON {
            self.extent_y / self.child_extent_y
        } else {
            1.0
        };
        AffineTransform {
            scale_x,
            scale_y,
            translate_x: self.offset_x - self.child_offset_x * scale_x,
            translate_y: self.offset_y - self.child_offset_y * scale_y,
        }
    }
}

#[derive(Default)]
struct WpgChildBuilder {
    shape: ShapeBuilder,
    parent_transform: AffineTransform,
    offset_x: f64,
    offset_y: f64,
    extent_x: f64,
    extent_y: f64,
    content: Vec<docx_rs::DocumentChild>,
    padding: Insets,
    vertical_align: TextBoxVerticalAlign,
    text_color: Option<Color>,
    shape_properties_depth: usize,
    shape_transform_depth: usize,
    line_depth: usize,
    fill_reference_depth: usize,
    line_reference_depth: usize,
    font_reference_depth: usize,
}

struct WpgDrawingBuilder {
    record_index: usize,
    is_wpg: bool,
    anchor_offset_x: f64,
    anchor_offset_y: f64,
    position_axis: PositionAxis,
    in_position_offset: bool,
    wrap_mode: WrapMode,
    group_transforms: Vec<AffineTransform>,
    group_transform_builder: Option<GroupTransformBuilder>,
    group_properties_depth: usize,
    group_transform_depth: usize,
    child: Option<WpgChildBuilder>,
    children: Vec<WpgShapeInfo>,
}

impl WpgDrawingBuilder {
    fn new(record_index: usize) -> Self {
        Self {
            record_index,
            is_wpg: false,
            anchor_offset_x: 0.0,
            anchor_offset_y: 0.0,
            position_axis: PositionAxis::None,
            in_position_offset: false,
            wrap_mode: WrapMode::None,
            group_transforms: Vec::new(),
            group_transform_builder: None,
            group_properties_depth: 0,
            group_transform_depth: 0,
            child: None,
            children: Vec::new(),
        }
    }

    fn finish_child(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let (local_x, local_y) = child.parent_transform.point(child.offset_x, child.offset_y);
        let width_emu: f64 = child.extent_x * child.parent_transform.scale_x.abs();
        let height_emu: f64 = child.extent_y * child.parent_transform.scale_y.abs();
        let width: f64 = width_emu / EMU_PER_POINT;
        let height: f64 = height_emu / EMU_PER_POINT;
        let offset_x: f64 = self.anchor_offset_x + local_x / EMU_PER_POINT;
        let offset_y: f64 = self.anchor_offset_y + local_y / EMU_PER_POINT;

        child.shape.box_width_pt = Some(width);
        child.shape.box_height_pt = Some(height);
        child.shape.offset_x_pt = offset_x;
        child.shape.offset_y_pt = offset_y;
        let shape: Option<FloatingShape> = child.shape.finish_wpg();
        let padding: Insets = shape
            .as_ref()
            .map(|shape| shape_text_padding(&shape.shape.kind, width, height, child.padding))
            .unwrap_or(child.padding);

        if shape.is_some() || !child.content.is_empty() {
            self.children.push(WpgShapeInfo {
                shape,
                content: child.content,
                width,
                height,
                padding,
                vertical_align: child.vertical_align,
                text_color: child.text_color,
                offset_x,
                offset_y,
                wrap_mode: self.wrap_mode,
            });
        }
    }
}

fn shape_text_padding(kind: &ShapeKind, width: f64, height: f64, body: Insets) -> Insets {
    let (horizontal_fraction, top_fraction, bottom_fraction): (f64, f64, f64) = match kind {
        ShapeKind::Ellipse => {
            let circle_inset: f64 = (1.0 - std::f64::consts::FRAC_1_SQRT_2) / 2.0;
            (circle_inset, circle_inset, circle_inset)
        }
        ShapeKind::Polygon { vertices }
            if vertices.as_slice() == [(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)] =>
        {
            (0.25, 0.5, 0.0)
        }
        ShapeKind::Polygon { vertices }
            if vertices.as_slice() == [(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)] =>
        {
            (0.25, 0.25, 0.25)
        }
        _ => (0.0, 0.0, 0.0),
    };

    Insets {
        left: body.left + width * horizontal_fraction,
        right: body.right + width * horizontal_fraction,
        top: body.top + height * top_fraction,
        bottom: body.bottom + height * bottom_fraction,
    }
}

fn numeric_attr(element: &BytesStart<'_>, name: &[u8]) -> Option<f64> {
    attribute_value(element, name).and_then(|value| value.parse::<f64>().ok())
}

fn scan_wpg_drawings(xml: &str, theme_xml: Option<&str>) -> Vec<Option<WpgDrawingInfo>> {
    let text_box_contents: Vec<Vec<docx_rs::DocumentChild>> = scan_wpg_text_box_contents(xml);
    let mut text_box_cursor: usize = 0;
    let theme_colors: HashMap<String, Color> = parse_theme_colors(theme_xml.unwrap_or_default());
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut records: Vec<Option<WpgDrawingInfo>> = Vec::new();
    let mut drawings: Vec<WpgDrawingBuilder> = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) if element.local_name().as_ref() == b"drawing" => {
                let record_index: usize = records.len();
                records.push(None);
                drawings.push(WpgDrawingBuilder::new(record_index));
            }
            Ok(Event::Start(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_wpg_start(
                        drawing,
                        element,
                        &theme_colors,
                        &text_box_contents,
                        &mut text_box_cursor,
                    );
                }
            }
            Ok(Event::Empty(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_wpg_empty(drawing, element, &theme_colors);
                }
            }
            Ok(Event::Text(ref text)) => {
                if let Some(drawing) = drawings.last_mut()
                    && drawing.in_position_offset
                    && let Ok(raw) = text.xml_content()
                    && let Ok(emu) = raw.trim().parse::<f64>()
                {
                    match drawing.position_axis {
                        PositionAxis::Horizontal => drawing.anchor_offset_x = emu / EMU_PER_POINT,
                        PositionAxis::Vertical => drawing.anchor_offset_y = emu / EMU_PER_POINT,
                        PositionAxis::None => {}
                    }
                }
            }
            Ok(Event::End(ref element)) if element.local_name().as_ref() == b"drawing" => {
                if let Some(mut drawing) = drawings.pop() {
                    drawing.finish_child();
                    if drawing.is_wpg {
                        records[drawing.record_index] = Some(WpgDrawingInfo {
                            children: drawing.children,
                        });
                    }
                }
            }
            Ok(Event::End(ref element)) => {
                if let Some(drawing) = drawings.last_mut() {
                    handle_wpg_end(drawing, element.local_name().as_ref());
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    records
}

fn handle_wpg_start(
    drawing: &mut WpgDrawingBuilder,
    element: &BytesStart<'_>,
    theme_colors: &HashMap<String, Color>,
    text_box_contents: &[Vec<docx_rs::DocumentChild>],
    text_box_cursor: &mut usize,
) {
    match element.local_name().as_ref() {
        b"positionH" => drawing.position_axis = PositionAxis::Horizontal,
        b"positionV" => drawing.position_axis = PositionAxis::Vertical,
        b"posOffset" => drawing.in_position_offset = true,
        b"wrapSquare" => drawing.wrap_mode = WrapMode::Square,
        b"wrapTight" => drawing.wrap_mode = WrapMode::Tight,
        b"wrapTopAndBottom" => drawing.wrap_mode = WrapMode::TopAndBottom,
        b"wgp" => {
            drawing.is_wpg = true;
            drawing.group_transforms.push(AffineTransform::default());
        }
        b"grpSp" if drawing.is_wpg => {
            let inherited: AffineTransform =
                drawing.group_transforms.last().copied().unwrap_or_default();
            drawing.group_transforms.push(inherited);
        }
        b"grpSpPr" if drawing.is_wpg => {
            drawing.group_properties_depth += 1;
            drawing.group_transform_builder = Some(GroupTransformBuilder::default());
        }
        b"wsp" if drawing.is_wpg => {
            drawing.finish_child();
            let mut child = WpgChildBuilder {
                parent_transform: drawing.group_transforms.last().copied().unwrap_or_default(),
                ..WpgChildBuilder::default()
            };
            child.shape.has_wsp = true;
            drawing.child = Some(child);
        }
        b"spPr" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.shape_properties_depth += 1;
            }
        }
        b"xfrm" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut()
                && child.shape_properties_depth > 0
            {
                child.shape_transform_depth += 1;
                child.shape.flip_h = bool_attr(element, b"flipH");
                child.shape.flip_v = bool_attr(element, b"flipV");
            }
        }
        b"xfrm" if drawing.group_properties_depth > 0 => {
            drawing.group_transform_depth += 1;
        }
        b"ln" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.line_depth += 1;
                child.shape.has_line = true;
                child.shape.line_width_pt =
                    emu_attr_to_pt(element, b"w").or(child.shape.line_width_pt);
            }
        }
        b"fillRef" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.fill_reference_depth += 1;
                if numeric_attr(element, b"idx").unwrap_or_default() > 0.0 {
                    child.shape.fill_color = Some(Color::new(68, 114, 196));
                }
            }
        }
        b"lnRef" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.line_reference_depth += 1;
                if numeric_attr(element, b"idx").unwrap_or_default() > 0.0 {
                    child.shape.has_line = true;
                }
            }
        }
        b"fontRef" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.font_reference_depth += 1;
            }
        }
        b"txbx" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.shape.has_text_box = true;
                child.content = text_box_contents
                    .get(*text_box_cursor)
                    .cloned()
                    .unwrap_or_default();
                *text_box_cursor += 1;
            }
        }
        b"bodyPr" if drawing.child.is_some() => {
            if let Some(child) = drawing.child.as_mut() {
                child.vertical_align = match attribute_value(element, b"anchor").as_deref() {
                    Some("ctr") => TextBoxVerticalAlign::Center,
                    Some("b") => TextBoxVerticalAlign::Bottom,
                    _ => TextBoxVerticalAlign::Top,
                };
                child.padding = Insets {
                    left: emu_attr_to_pt(element, b"lIns").unwrap_or_default(),
                    top: emu_attr_to_pt(element, b"tIns").unwrap_or_default(),
                    right: emu_attr_to_pt(element, b"rIns").unwrap_or_default(),
                    bottom: emu_attr_to_pt(element, b"bIns").unwrap_or_default(),
                };
            }
        }
        _ => handle_wpg_geometry_element(drawing, element, theme_colors),
    }
}

fn handle_wpg_empty(
    drawing: &mut WpgDrawingBuilder,
    element: &BytesStart<'_>,
    theme_colors: &HashMap<String, Color>,
) {
    handle_wpg_geometry_element(drawing, element, theme_colors);
}

fn handle_wpg_geometry_element(
    drawing: &mut WpgDrawingBuilder,
    element: &BytesStart<'_>,
    theme_colors: &HashMap<String, Color>,
) {
    let element_name = element.local_name();
    let local_name: &[u8] = element_name.as_ref();
    if drawing.group_transform_depth > 0 {
        if let Some(group) = drawing.group_transform_builder.as_mut() {
            match local_name {
                b"off" => {
                    group.offset_x = numeric_attr(element, b"x").unwrap_or_default();
                    group.offset_y = numeric_attr(element, b"y").unwrap_or_default();
                }
                b"ext" => {
                    group.extent_x = numeric_attr(element, b"cx").unwrap_or_default();
                    group.extent_y = numeric_attr(element, b"cy").unwrap_or_default();
                }
                b"chOff" => {
                    group.child_offset_x = numeric_attr(element, b"x").unwrap_or_default();
                    group.child_offset_y = numeric_attr(element, b"y").unwrap_or_default();
                }
                b"chExt" => {
                    group.child_extent_x = numeric_attr(element, b"cx").unwrap_or_default();
                    group.child_extent_y = numeric_attr(element, b"cy").unwrap_or_default();
                }
                _ => {}
            }
        }
        return;
    }

    let Some(child) = drawing.child.as_mut() else {
        return;
    };
    if child.shape_transform_depth > 0 {
        match local_name {
            b"off" => {
                child.offset_x = numeric_attr(element, b"x").unwrap_or_default();
                child.offset_y = numeric_attr(element, b"y").unwrap_or_default();
            }
            b"ext" => {
                child.extent_x = numeric_attr(element, b"cx").unwrap_or_default();
                child.extent_y = numeric_attr(element, b"cy").unwrap_or_default();
            }
            _ => {}
        }
    }

    if local_name == b"schemeClr"
        && let Some(name) = attribute_value(element, b"val")
        && let Some(color) = theme_colors.get(&name).copied()
    {
        if child.font_reference_depth > 0 {
            child.text_color = Some(color);
        } else if child.line_reference_depth > 0 || child.line_depth > 0 {
            child.shape.line_color = Some(color);
            child.shape.has_line = true;
        } else if child.fill_reference_depth > 0 || child.shape_properties_depth > 0 {
            child.shape.fill_color = Some(color);
        }
    }

    handle_geometry_element(
        Some(&mut child.shape),
        local_name,
        element,
        child.shape_properties_depth,
        child.line_depth,
    );
}

fn handle_wpg_end(drawing: &mut WpgDrawingBuilder, local_name: &[u8]) {
    match local_name {
        b"posOffset" => drawing.in_position_offset = false,
        b"positionH" | b"positionV" => drawing.position_axis = PositionAxis::None,
        b"xfrm"
            if drawing
                .child
                .as_ref()
                .is_some_and(|child| child.shape_transform_depth > 0) =>
        {
            if let Some(child) = drawing.child.as_mut() {
                child.shape_transform_depth -= 1;
            }
        }
        b"xfrm" if drawing.group_transform_depth > 0 => {
            drawing.group_transform_depth -= 1;
            if drawing.group_transform_depth == 0
                && let Some(group) = drawing.group_transform_builder.take()
            {
                let parent: AffineTransform = if drawing.group_transforms.len() > 1 {
                    drawing.group_transforms[drawing.group_transforms.len() - 2]
                } else {
                    AffineTransform::default()
                };
                if let Some(current) = drawing.group_transforms.last_mut() {
                    *current = parent.compose(group.finish());
                }
            }
        }
        b"spPr" => {
            if let Some(child) = drawing.child.as_mut()
                && child.shape_properties_depth > 0
            {
                child.shape_properties_depth -= 1;
            }
        }
        b"ln" => {
            if let Some(child) = drawing.child.as_mut()
                && child.line_depth > 0
            {
                child.line_depth -= 1;
            }
        }
        b"fillRef" => {
            if let Some(child) = drawing.child.as_mut()
                && child.fill_reference_depth > 0
            {
                child.fill_reference_depth -= 1;
            }
        }
        b"lnRef" => {
            if let Some(child) = drawing.child.as_mut()
                && child.line_reference_depth > 0
            {
                child.line_reference_depth -= 1;
            }
        }
        b"fontRef" => {
            if let Some(child) = drawing.child.as_mut()
                && child.font_reference_depth > 0
            {
                child.font_reference_depth -= 1;
            }
        }
        b"wsp" => drawing.finish_child(),
        b"grpSpPr" if drawing.group_properties_depth > 0 => {
            drawing.group_properties_depth -= 1;
        }
        b"grpSp" | b"wgp" if drawing.is_wpg => {
            drawing.group_transforms.pop();
        }
        _ => {}
    }
}

fn scan_wpg_text_box_contents(xml: &str) -> Vec<Vec<docx_rs::DocumentChild>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut result: Vec<Vec<docx_rs::DocumentChild>> = Vec::new();
    let mut wpg_depth: usize = 0;
    let mut capture: Option<(quick_xml::Writer<Vec<u8>>, usize)> = None;

    while let Ok(event) = reader.read_event_into(&mut buffer) {
        if let Some((writer, depth)) = capture.as_mut() {
            match &event {
                Event::Start(element) => {
                    *depth += usize::from(element.local_name().as_ref() == b"txbxContent");
                    let _ = writer.write_event(event.into_owned());
                }
                Event::End(element)
                    if element.local_name().as_ref() == b"txbxContent" && *depth == 1 =>
                {
                    let (writer, _) = capture.take().expect("capture should exist");
                    result.push(parse_wpg_text_box_document(writer.into_inner()));
                }
                Event::End(element) => {
                    if element.local_name().as_ref() == b"txbxContent" {
                        *depth -= 1;
                    }
                    let _ = writer.write_event(event.into_owned());
                }
                Event::Eof => break,
                _ => {
                    let _ = writer.write_event(event.into_owned());
                }
            }
            buffer.clear();
            continue;
        }

        match &event {
            Event::Start(element) if element.local_name().as_ref() == b"wgp" => wpg_depth += 1,
            Event::End(element) if element.local_name().as_ref() == b"wgp" && wpg_depth > 0 => {
                wpg_depth -= 1;
            }
            Event::Start(element)
                if element.local_name().as_ref() == b"txbxContent" && wpg_depth > 0 =>
            {
                capture = Some((quick_xml::Writer::new(Vec::new()), 1));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    result
}

fn parse_wpg_text_box_document(inner_xml: Vec<u8>) -> Vec<docx_rs::DocumentChild> {
    let Ok(inner) = String::from_utf8(inner_xml) else {
        return Vec::new();
    };
    let xml = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><w:body>{inner}</w:body></w:document>"#
    );
    docx_rs::Document::from_xml(xml.as_bytes())
        .map(|document| {
            document
                .children
                .into_iter()
                .filter(|child| {
                    matches!(
                        child,
                        docx_rs::DocumentChild::Paragraph(_) | docx_rs::DocumentChild::Table(_)
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_theme_colors(xml: &str) -> HashMap<String, Color> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buffer: Vec<u8> = Vec::new();
    let mut result: HashMap<String, Color> = HashMap::new();
    let mut current_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) => {
                let name = element.local_name();
                if matches!(
                    name.as_ref(),
                    b"dk1"
                        | b"lt1"
                        | b"dk2"
                        | b"lt2"
                        | b"accent1"
                        | b"accent2"
                        | b"accent3"
                        | b"accent4"
                        | b"accent5"
                        | b"accent6"
                        | b"hlink"
                        | b"folHlink"
                ) {
                    current_name = Some(String::from_utf8_lossy(name.as_ref()).into_owned());
                }
                if matches!(name.as_ref(), b"srgbClr" | b"sysClr")
                    && let Some(key) = current_name.as_ref()
                    && let Some(value) = attribute_value(element, b"val")
                        .filter(|_| name.as_ref() == b"srgbClr")
                        .or_else(|| attribute_value(element, b"lastClr"))
                    && let Some(color) = parse_hex_color(&value)
                {
                    result.insert(key.clone(), color);
                }
            }
            Ok(Event::Empty(ref element)) => {
                let name = element.local_name();
                if matches!(name.as_ref(), b"srgbClr" | b"sysClr")
                    && let Some(key) = current_name.as_ref()
                    && let Some(value) = attribute_value(element, b"val")
                        .filter(|_| name.as_ref() == b"srgbClr")
                        .or_else(|| attribute_value(element, b"lastClr"))
                    && let Some(color) = parse_hex_color(&value)
                {
                    result.insert(key.clone(), color);
                }
            }
            Ok(Event::End(ref element)) => {
                if current_name.as_deref()
                    == std::str::from_utf8(element.local_name().as_ref()).ok()
                {
                    current_name = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    result
}

#[cfg(test)]
#[path = "docx_context_shape_tests.rs"]
mod tests;

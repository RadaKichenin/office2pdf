//! `a:custGeom` path translation.
//!
//! A shape that declares custom geometry carries an `<a:pathLst>` of drawing
//! commands in its own coordinate space (`<a:path w= h=>`, with either axis
//! optional). Discarding it and
//! substituting a rectangle turned every decorative curve into a block and
//! every circular frame into a square (issue #855).
//!
//! The commands are flattened to subpaths in the shape's normalized 0..1 box,
//! which [`crate::ir::ShapeKind::Path`] renders as one path under the even-odd
//! fill rule. Curves are sampled rather than preserved: at the sizes these
//! decorations print, the sampling error is far below the rectangle it
//! replaces.
//!
//! A coordinate is not always a number. A geometry round-tripped through
//! LibreOffice states each one as a guide name — `<a:pt x="f38" y="f37"/>` —
//! and puts the arithmetic in the `<a:gdLst>` beside it, so the whole path
//! read as empty and the rectangle fallback stood in for a shape the deck had
//! fully described (issue #1205). Both forms resolve through
//! [`GuideList`], and `<a:arcTo>` resolves its radii and angles the same way.
//! The geometry's `<a:rect>` uses those same guides to define the box where
//! shape text belongs; callers that lay out text retain that rectangle while
//! picture masks and Word drawing shapes can keep requesting subpaths alone.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::ir::Subpath;
use crate::parser::pptx::geometry_guides::{GuideList, ShapeExtent, to_radians};
use crate::parser::xml_util::{get_attr_i64, get_attr_str};

/// Points sampled per cubic or quadratic segment. Sixteen keeps a full circle
/// — four segments, so 64 points — within about 0.2% of its radius, well under
/// a printed point at slide sizes.
const CURVE_SAMPLES: usize = 16;

/// The most points one `<a:arcTo>` may contribute. `swAng` is unbounded, and
/// a spiral of many turns must not sample itself into a megabyte of vertices.
const MAX_ARC_SAMPLES: usize = 512;

/// A custom geometry's text rectangle, normalized to its shape box.
///
/// These are edge coordinates rather than insets: `(0, 0, 1, 1)` is the
/// full box. Keeping them normalized makes the parsed geometry independent
/// of whether its caller measured the shape in EMU or points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GeometryTextRect {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
}

/// The parts of `<a:custGeom>` used by a shape with text.
pub(crate) struct ParsedCustomGeometry {
    pub(crate) subpaths: Vec<Subpath>,
    pub(crate) text_rect: Option<GeometryTextRect>,
}

/// Flatten the `<a:pathLst>` of a `<a:custGeom>` into vertices normalized to
/// the shape's bounding box.
///
/// `extent` is the shape's own box. It is what the `w`, `h` and `ss` a guide
/// formula names evaluate to, and it is the coordinate space of an
/// `<a:path>` that declares none of its own — which is the form every
/// guide-driven geometry takes. Its units only have to match the geometry's
/// own, since the result is normalized against the same box.
///
/// The reader is positioned just after the `<a:custGeom>` start tag and is
/// consumed through its end tag either way, so a geometry this cannot express
/// still leaves the caller's parse in step.
///
/// Returns an empty vector when nothing usable was found — no path, a
/// degenerate coordinate space, or a subpath with too few points to draw.
/// The caller keeps its rectangle fallback for those.
///
/// **Every** subpath is returned, each as its own polygon. A geometry's
/// subpaths come from separate `<a:path>` elements and from a `moveTo`
/// part-way through one, and the deck on #866 draws its wave line-art as
/// dozens of thin ribbons inside a single path. Concatenating them welded the
/// end of one ribbon to the start of the next and painted the wedge between;
/// keeping only the largest threw the rest of the art away. An inner boundary
/// carves a hole rather than painting solid because the caller hands every
/// subpath to one [`crate::ir::ShapeKind::Path`], which fills even-odd
/// (issue #870).
pub(crate) fn parse_custom_geometry(
    reader: &mut Reader<&[u8]>,
    extent: ShapeExtent,
) -> Vec<Subpath> {
    parse_custom_geometry_with_text_rect(reader, extent).subpaths
}

/// Parse both the drawable paths and the explicit geometry text rectangle.
pub(crate) fn parse_custom_geometry_with_text_rect(
    reader: &mut Reader<&[u8]>,
    extent: ShapeExtent,
) -> ParsedCustomGeometry {
    let mut depth: usize = 1;
    let mut builder = SubpathBuilder::new(extent);
    let mut guides = GuideList::new(extent);
    let mut text_rect: Option<GeometryTextRect> = None;
    // `a:pt` children accumulate here; a curve command reads its control
    // points from the list once the command closes.
    let mut pending_points: Vec<(f64, f64)> = Vec::new();
    let mut command: Option<Command> = None;

    loop {
        let event = reader.read_event();
        match event {
            Ok(Event::Start(ref element)) => {
                depth += 1;
                match element.local_name().as_ref() {
                    b"path" => builder.start_path(element),
                    b"arcTo" => apply_arc(element, &guides, builder.vertices()),
                    b"rect" => {
                        if let Some(rect) = resolve_text_rect(element, &guides, extent) {
                            text_rect = Some(rect);
                        }
                    }
                    other => {
                        if let Some(kind) = Command::from_tag(other) {
                            command = Some(kind);
                            pending_points.clear();
                        }
                    }
                }
            }
            Ok(Event::Empty(ref element)) => match element.local_name().as_ref() {
                // `<a:gd>` appears in both `<a:avLst>` and `<a:gdLst>`, in the
                // order a later formula may name an earlier one, and both
                // precede the `<a:pathLst>` that reads them.
                b"gd" => define_guide(element, &mut guides),
                b"pt" => {
                    if let Some(point) = resolve_point(element, &guides) {
                        pending_points.push(point);
                    }
                }
                // An arc carries its whole definition in its attributes and
                // starts wherever the pen already is, so it applies the
                // moment it is read rather than waiting for child points.
                b"arcTo" => apply_arc(element, &guides, builder.vertices()),
                b"rect" => {
                    if let Some(rect) = resolve_text_rect(element, &guides, extent) {
                        text_rect = Some(rect);
                    }
                }
                b"close" => builder.close(),
                b"path" => builder.end_path(),
                _ => {}
            },
            Ok(Event::End(ref element)) => {
                depth -= 1;
                match element.local_name().as_ref() {
                    b"close" => builder.close(),
                    b"path" => builder.end_path(),
                    other => {
                        if let Some(kind) = command.take()
                            && Command::from_tag(other) == Some(kind)
                        {
                            // A `moveTo` after any point starts a new subpath.
                            // One `<a:path>` may hold several — `moveTo lnTo
                            // lnTo close moveTo lnTo …` is one shape in the
                            // deck on #866 — and concatenating them joined the
                            // end of one outline to the start of the next,
                            // painting the wedge between (issue #866).
                            if kind == Command::Move {
                                builder.start_subpath();
                            }
                            apply_command(kind, &pending_points, builder.vertices());
                            pending_points.clear();
                        }
                    }
                }
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    ParsedCustomGeometry {
        subpaths: builder.finish(),
        text_rect,
    }
}

/// Bind one `<a:gd name= fmla=>`.
fn define_guide(element: &BytesStart, guides: &mut GuideList) {
    if let (Some(name), Some(formula)) = (
        get_attr_str(element, b"name"),
        get_attr_str(element, b"fmla"),
    ) {
        guides.define(&name, &formula);
    }
}

/// Read one `<a:pt x= y=>`, resolving each coordinate through the guides.
///
/// A point whose coordinates do not resolve is dropped rather than defaulted:
/// substituting zero would stake a vertex at the shape's top-left corner and
/// drag the outline through it.
fn resolve_point(element: &BytesStart, guides: &GuideList) -> Option<(f64, f64)> {
    let x: f64 = guides.resolve(&get_attr_str(element, b"x")?)?;
    let y: f64 = guides.resolve(&get_attr_str(element, b"y")?)?;
    Some((x, y))
}

/// Resolve `<a:rect l= t= r= b=>` through the geometry guide list.
///
/// Invalid or inverted rectangles retain the caller's full-box fallback.
/// Valid coordinates are clipped to the shape because an out-of-range guide
/// in an untrusted document must not move text outside its fixed page.
fn resolve_text_rect(
    element: &BytesStart,
    guides: &GuideList,
    extent: ShapeExtent,
) -> Option<GeometryTextRect> {
    if !extent.is_usable() {
        return None;
    }
    let resolve = |key: &[u8]| -> Option<f64> { guides.resolve(&get_attr_str(element, key)?) };
    let left = resolve(b"l")?;
    let top = resolve(b"t")?;
    let right = resolve(b"r")?;
    let bottom = resolve(b"b")?;
    if right < left || bottom < top {
        return None;
    }

    Some(GeometryTextRect {
        left: (left / extent.width).clamp(0.0, 1.0),
        top: (top / extent.height).clamp(0.0, 1.0),
        right: (right / extent.width).clamp(0.0, 1.0),
        bottom: (bottom / extent.height).clamp(0.0, 1.0),
    })
}

/// Append the elliptical arc of one `<a:arcTo wR= hR= stAng= swAng=>`.
///
/// The arc starts at the pen's current point, so the ellipse is the one whose
/// point at `stAng` is already there: its centre is that point stepped back
/// along the radii. From there the arc sweeps `swAng`. Both angles are in
/// 60000ths of a degree, and both turn clockwise on the page, which the
/// y-down coordinate space gives without a sign flip.
///
/// Skipping the segment squared off every corner it was drawing: a freeform
/// rounded rectangle came out a plain rectangle, outline, fill and shadow
/// silhouette alike (issue #1205).
fn apply_arc(element: &BytesStart, guides: &GuideList, current: &mut Vec<(f64, f64)>) {
    let attribute = |key: &[u8]| -> Option<f64> { guides.resolve(&get_attr_str(element, key)?) };
    let (Some(radius_x), Some(radius_y), Some(start_angle_units), Some(swing_angle_units)) = (
        attribute(b"wR"),
        attribute(b"hR"),
        attribute(b"stAng"),
        attribute(b"swAng"),
    ) else {
        return;
    };

    append_sampled_arc(
        current,
        radius_x,
        radius_y,
        start_angle_units,
        swing_angle_units,
    );
}

/// Append a DrawingML elliptical arc to a path as sampled line vertices.
///
/// Preset geometries and custom geometries use the same angle units and arc
/// semantics. Sharing this sampler keeps their rounded outlines equally
/// smooth and avoids two subtly different interpretations of the current pen
/// position.
pub(crate) fn append_sampled_arc(
    current: &mut Vec<(f64, f64)>,
    radius_x: f64,
    radius_y: f64,
    start_angle_units: f64,
    swing_angle_units: f64,
) {
    let Some(start) = current.last().copied() else {
        return;
    };

    let start_angle: f64 = to_radians(start_angle_units);
    let swing: f64 = to_radians(swing_angle_units);
    if !(start_angle.is_finite()
        && swing.is_finite()
        && radius_x.is_finite()
        && radius_y.is_finite())
    {
        return;
    }
    let centre: (f64, f64) = (
        start.0 - radius_x * start_angle.cos(),
        start.1 - radius_y * start_angle.sin(),
    );

    let samples: usize = arc_sample_count(swing);
    for step in 1..=samples {
        let angle: f64 = start_angle + swing * (step as f64 / samples as f64);
        current.push((
            centre.0 + radius_x * angle.cos(),
            centre.1 + radius_y * angle.sin(),
        ));
    }
}

/// Sample an arc at the rate a Bezier quarter-circle is sampled at, so a
/// corner arc and a corner curve print the same smoothness.
fn arc_sample_count(swing: f64) -> usize {
    let quarter_turns: f64 = swing.abs() / std::f64::consts::FRAC_PI_2;
    let samples: f64 = (quarter_turns * CURVE_SAMPLES as f64).ceil();
    (samples as usize).clamp(1, MAX_ARC_SAMPLES)
}

/// Accumulates subpaths, and knows the coordinate space each one normalizes
/// against.
struct SubpathBuilder {
    extent: ShapeExtent,
    paths: Vec<Subpath>,
    vertices: Vec<(f64, f64)>,
    /// `a:close` on the subpath being collected. It must travel with the
    /// vertices rather than be inferred from them: an elbow connector's last
    /// point is nowhere near its first, and so is a spiral's (issue #1205).
    closed: bool,
    /// The `<a:path w= h=>` coordinate space. An undeclared or zero axis uses
    /// the corresponding axis of the shape's own extent.
    space: ShapeExtent,
}

impl SubpathBuilder {
    fn new(extent: ShapeExtent) -> Self {
        Self {
            extent,
            paths: Vec::new(),
            vertices: Vec::new(),
            closed: false,
            space: extent,
        }
    }

    fn vertices(&mut self) -> &mut Vec<(f64, f64)> {
        &mut self.vertices
    }

    /// `<a:path>`: bank whatever the previous one left and read the new
    /// coordinate space.
    ///
    /// `w` and `h` are independent optional axes. A missing or zero axis uses
    /// the corresponding shape extent — the form a guide-driven geometry
    /// uses, since its guides are already in the shape's units (issue #1418).
    fn start_path(&mut self, element: &BytesStart) {
        self.start_subpath();
        let width: f64 = get_attr_i64(element, b"w")
            .map(|value| value as f64)
            .filter(|value| *value > 0.0)
            .unwrap_or(self.extent.width);
        let height: f64 = get_attr_i64(element, b"h")
            .map(|value| value as f64)
            .filter(|value| *value > 0.0)
            .unwrap_or(self.extent.height);
        self.space = ShapeExtent::new(width, height);
    }

    fn end_path(&mut self) {
        self.start_subpath();
        self.space = self.extent;
    }

    /// Bank the subpath in hand, if any, and begin the next.
    fn start_subpath(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        let vertices: Vec<(f64, f64)> = std::mem::take(&mut self.vertices);
        let closed: bool = std::mem::take(&mut self.closed);
        if !self.space.is_usable() {
            return;
        }
        self.paths.push(Subpath {
            vertices: vertices
                .into_iter()
                .map(|(x, y)| (x / self.space.width, y / self.space.height))
                .collect(),
            closed,
        });
    }

    /// `a:close` returns to the subpath's first point. The renderer closes the
    /// outline itself, so the duplicate vertex is dropped.
    fn close(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        self.closed = true;
        if let (Some(first), Some(last)) = (
            self.vertices.first().copied(),
            self.vertices.last().copied(),
        ) && (first.0 - last.0).abs() < f64::EPSILON
            && (first.1 - last.1).abs() < f64::EPSILON
        {
            self.vertices.pop();
        }
    }

    fn finish(mut self) -> Vec<Subpath> {
        self.start_subpath();
        self.paths.retain(Subpath::encloses_or_draws);
        self.paths
    }
}

/// The drawing commands a path can carry. `close` is handled separately: it
/// takes no points and may arrive as an empty element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Move,
    Line,
    CubicBezier,
    QuadraticBezier,
}

impl Command {
    fn from_tag(tag: &[u8]) -> Option<Self> {
        match tag {
            b"moveTo" => Some(Self::Move),
            b"lnTo" => Some(Self::Line),
            b"cubicBezTo" => Some(Self::CubicBezier),
            b"quadBezTo" => Some(Self::QuadraticBezier),
            _ => None,
        }
    }
}

fn apply_command(command: Command, points: &[(f64, f64)], current: &mut Vec<(f64, f64)>) {
    match command {
        Command::Move | Command::Line => {
            if let Some(point) = points.first() {
                current.push(*point);
            }
        }
        Command::CubicBezier => {
            let (Some(start), [control_one, control_two, end]) = (current.last().copied(), points)
            else {
                return;
            };
            sample_cubic(start, *control_one, *control_two, *end, current);
        }
        Command::QuadraticBezier => {
            let (Some(start), [control, end]) = (current.last().copied(), points) else {
                return;
            };
            // A quadratic is the cubic whose controls sit two thirds of the way
            // from each endpoint to the quadratic's own control point.
            let lift = |from: (f64, f64)| {
                (
                    from.0 + 2.0 / 3.0 * (control.0 - from.0),
                    from.1 + 2.0 / 3.0 * (control.1 - from.1),
                )
            };
            sample_cubic(start, lift(start), lift(*end), *end, current);
        }
    }
}

fn sample_cubic(
    start: (f64, f64),
    control_one: (f64, f64),
    control_two: (f64, f64),
    end: (f64, f64),
    out: &mut Vec<(f64, f64)>,
) {
    for step in 1..=CURVE_SAMPLES {
        let t: f64 = step as f64 / CURVE_SAMPLES as f64;
        let inverse: f64 = 1.0 - t;
        let axis = |a: f64, b: f64, c: f64, d: f64| -> f64 {
            inverse * inverse * inverse * a
                + 3.0 * inverse * inverse * t * b
                + 3.0 * inverse * t * t * c
                + t * t * t * d
        };
        out.push((
            axis(start.0, control_one.0, control_two.0, end.0),
            axis(start.1, control_one.1, control_two.1, end.1),
        ));
    }
}

#[cfg(test)]
#[path = "pptx_custom_geometry_tests.rs"]
mod tests;

//! `a:custGeom` path translation.
//!
//! A shape that declares custom geometry carries an `<a:pathLst>` of drawing
//! commands in its own coordinate space (`<a:path w= h=>`). Discarding it and
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
//! [`GuideList`].

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::ir::Subpath;
use crate::parser::pptx::geometry_guides::{GuideList, ShapeExtent};
use crate::parser::xml_util::{get_attr_i64, get_attr_str};

/// Points sampled per cubic or quadratic segment. Sixteen keeps a full circle
/// — four segments, so 64 points — within about 0.2% of its radius, well under
/// a printed point at slide sizes.
const CURVE_SAMPLES: usize = 16;

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
    let mut depth: usize = 1;
    let mut builder = SubpathBuilder::new(extent);
    let mut guides = GuideList::new(extent);
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

    builder.finish()
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
    /// The `<a:path w= h=>` coordinate space, or the shape's own extent when
    /// the path declares none.
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
    /// `w`/`h` default to 0, which DrawingML reads as "the shape's own
    /// space" — the form a guide-driven geometry uses, since its guides are
    /// already in the shape's units.
    fn start_path(&mut self, element: &BytesStart) {
        self.start_subpath();
        let width: f64 = get_attr_i64(element, b"w").unwrap_or(0) as f64;
        let height: f64 = get_attr_i64(element, b"h").unwrap_or(0) as f64;
        self.space = if width > 0.0 && height > 0.0 {
            ShapeExtent::new(width, height)
        } else {
            self.extent
        };
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

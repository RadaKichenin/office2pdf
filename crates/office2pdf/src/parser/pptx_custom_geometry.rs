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

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::parser::xml_util::get_attr_i64;

/// Points sampled per cubic or quadratic segment. Sixteen keeps a full circle
/// — four segments, so 64 points — within about 0.2% of its radius, well under
/// a printed point at slide sizes.
const CURVE_SAMPLES: usize = 16;

/// The smallest number of distinct vertices that can enclose an area.
const MIN_POLYGON_VERTICES: usize = 3;

/// Flatten the `<a:pathLst>` of a `<a:custGeom>` into vertices normalized to
/// the shape's bounding box.
///
/// The reader is positioned just after the `<a:custGeom>` start tag and is
/// consumed through its end tag either way, so a geometry this cannot express
/// still leaves the caller's parse in step.
///
/// Returns an empty vector when nothing usable was found — no path, a
/// degenerate coordinate space, or fewer than three distinct points in any
/// subpath. The caller keeps its rectangle fallback for those.
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
pub(super) fn parse_custom_geometry(reader: &mut Reader<&[u8]>) -> Vec<Vec<(f64, f64)>> {
    let mut depth: usize = 1;
    let mut paths: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let mut path_width: f64 = 0.0;
    let mut path_height: f64 = 0.0;
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
                    b"path" => {
                        flush_path(&mut paths, &mut current, path_width, path_height);
                        path_width = get_attr_i64(element, b"w").unwrap_or(0) as f64;
                        path_height = get_attr_i64(element, b"h").unwrap_or(0) as f64;
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
                b"pt" => {
                    if let (Some(x), Some(y)) =
                        (get_attr_i64(element, b"x"), get_attr_i64(element, b"y"))
                    {
                        pending_points.push((x as f64, y as f64));
                    }
                }
                b"close" => close_path(&mut current),
                b"path" => {
                    flush_path(&mut paths, &mut current, path_width, path_height);
                    path_width = 0.0;
                    path_height = 0.0;
                }
                _ => {}
            },
            Ok(Event::End(ref element)) => {
                depth -= 1;
                match element.local_name().as_ref() {
                    b"close" => close_path(&mut current),
                    b"path" => {
                        flush_path(&mut paths, &mut current, path_width, path_height);
                        path_width = 0.0;
                        path_height = 0.0;
                    }
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
                            if kind == Command::Move && !current.is_empty() {
                                flush_path(&mut paths, &mut current, path_width, path_height);
                            }
                            apply_command(kind, &pending_points, &mut current);
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
    flush_path(&mut paths, &mut current, path_width, path_height);

    paths.retain(|path| path.len() >= MIN_POLYGON_VERTICES);
    paths
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

/// `a:close` returns to the subpath's first point. The polygon renderer closes
/// the outline itself, so the duplicate vertex is dropped.
fn close_path(current: &mut Vec<(f64, f64)>) {
    if let (Some(first), Some(last)) = (current.first().copied(), current.last().copied())
        && (first.0 - last.0).abs() < f64::EPSILON
        && (first.1 - last.1).abs() < f64::EPSILON
    {
        current.pop();
    }
}

/// Normalize the collected points against the path's declared coordinate space
/// and bank them, leaving `current` empty for the next path.
fn flush_path(
    paths: &mut Vec<Vec<(f64, f64)>>,
    current: &mut Vec<(f64, f64)>,
    width: f64,
    height: f64,
) {
    if current.is_empty() {
        return;
    }
    let points = std::mem::take(current);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    paths.push(
        points
            .into_iter()
            .map(|(x, y)| (x / width, y / height))
            .collect(),
    );
}

#[cfg(test)]
#[path = "pptx_custom_geometry_tests.rs"]
mod tests;

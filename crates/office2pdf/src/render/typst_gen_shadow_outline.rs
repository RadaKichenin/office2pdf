//! The crisp `a:outerShdw` outline around a polygon (issue #1206).
//!
//! A ring is the silhouette pushed out by a fixed distance — a Minkowski
//! dilation — not a copy scaled onto an expanded bounding box. The two agree
//! only on a rectangle: a scale moves each vertex in proportion to its
//! distance from the centre, so a vertex near the centroid barely moves while
//! a far one overshoots, and a spike grows along its own axis instead of
//! gaining a uniform rim.
//!
//! Each corner follows the source outline's join so crisp shadows preserve the
//! silhouette PowerPoint casts (issues #1090, #1206).

use std::f64::consts::PI;

/// A point in the shape's own frame, in points.
pub(super) type Point = (f64, f64);

/// Below this the two offset edge lines are treated as one straight run: the
/// corner turns through less than a thousandth of a radian, well under a
/// tenth of a point of travel on any shape a slide can hold.
const STRAIGHT_TURN_EPSILON: f64 = 1e-3;

/// Below this an arc is written out as its own centre — a degenerate circle
/// is a point, and Typst rejects a zero-radius `curve.cubic` control frame.
const DEGENERATE_RADIUS_PT: f64 = 1e-6;

/// How far a ring's boundary reaches past a corner, along the direction that
/// bisects the two edges' outward normals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum CornerReach {
    /// The offset edges simply meet, which is what `a:miter` draws.
    Mitre,
    /// The equidistant contour: `a:round`'s arc, of the offset's own radius.
    /// DrawingML's default join (#1090), and what a crisp shadow casts.
    Round,
}

/// The arc that turns one corner of an offset outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CornerArc {
    pub centre: Point,
    pub radius: f64,
    /// Where the arc starts, as an angle around `centre`.
    pub start_angle: f64,
    /// How far it turns, signed: positive counter-clockwise in the coordinate
    /// frame the vertices are given in.
    pub sweep: f64,
}

/// One corner of an offset outline: where the boundary arrives from the
/// previous edge, and where it leaves along the next.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OffsetCorner {
    pub entry: Point,
    pub exit: Point,
    /// `None` when the two coincide and the corner is a single point.
    pub arc: Option<CornerArc>,
}

/// Offset a closed ring of vertices outward by `distance`, turning each
/// corner as `reach` prescribes.
///
/// A negative `distance` erodes instead, which is how a hole contracts when
/// the surrounding silhouette expands. The result is in the same frame and order as
/// the input, one corner per surviving vertex; consecutive duplicates are
/// dropped because a zero-length edge has no normal to offset along.
pub(super) fn offset_ring(
    vertices: &[Point],
    distance: f64,
    reach: CornerReach,
) -> Vec<OffsetCorner> {
    let ring: Vec<Point> = distinct_vertices(vertices);
    let count: usize = ring.len();
    if count < 3 {
        return Vec::new();
    }
    // A ring listed the other way round is the same shape, so the offset has
    // to go the same way: the winding decides which side of an edge is out.
    let orientation: f64 = if signed_area(&ring) > 0.0 { 1.0 } else { -1.0 };
    let normals: Vec<Point> = (0..count)
        .map(|index| {
            let (ax, ay): Point = ring[index];
            let (bx, by): Point = ring[(index + 1) % count];
            let length: f64 = (bx - ax).hypot(by - ay);
            (
                orientation * (by - ay) / length,
                -orientation * (bx - ax) / length,
            )
        })
        .collect();
    let lengths: Vec<f64> = (0..count)
        .map(|index| {
            let (ax, ay): Point = ring[index];
            let (bx, by): Point = ring[(index + 1) % count];
            (bx - ax).hypot(by - ay)
        })
        .collect();

    (0..count)
        .map(|index| {
            offset_corner(
                ring[index],
                normals[(index + count - 1) % count],
                normals[index],
                (lengths[(index + count - 1) % count], lengths[index]),
                orientation,
                distance,
                reach,
            )
        })
        .collect()
}

/// Drop consecutive duplicates, including across the wrap-around.
fn distinct_vertices(vertices: &[Point]) -> Vec<Point> {
    let mut ring: Vec<Point> = Vec::with_capacity(vertices.len());
    for &(x, y) in vertices {
        if ring
            .last()
            .is_none_or(|&(px, py)| (x - px).hypot(y - py) > DEGENERATE_RADIUS_PT)
        {
            ring.push((x, y));
        }
    }
    while ring.len() > 1 {
        let first: Point = ring[0];
        let last: Point = ring[ring.len() - 1];
        if (first.0 - last.0).hypot(first.1 - last.1) > DEGENERATE_RADIUS_PT {
            break;
        }
        ring.pop();
    }
    ring
}

fn signed_area(ring: &[Point]) -> f64 {
    (0..ring.len())
        .map(|index| {
            let (ax, ay): Point = ring[index];
            let (bx, by): Point = ring[(index + 1) % ring.len()];
            ax * by - bx * ay
        })
        .sum()
}

/// Turn one corner.
///
/// The two offset edge lines both sit at `n_i . (p - vertex) == distance`, so
/// a single number — how far the boundary reaches along the mitre direction,
/// in that same measure — pins the arc: the circle tangent to both lines that
/// reaches it. `Mitre` asks for exactly `distance` (the lines' own crossing),
/// and `Round` for the equidistant contour.
fn offset_corner(
    vertex: Point,
    previous_normal: Point,
    next_normal: Point,
    edge_lengths: (f64, f64),
    orientation: f64,
    distance: f64,
    reach: CornerReach,
) -> OffsetCorner {
    let dot: f64 =
        (previous_normal.0 * next_normal.0 + previous_normal.1 * next_normal.1).clamp(-1.0, 1.0);
    let cross: f64 = previous_normal.0 * next_normal.1 - previous_normal.1 * next_normal.0;
    // Positive where the outline turns around the outside of the shape.
    let turn: f64 = orientation * cross;
    let straight: Point = (
        vertex.0 + distance * next_normal.0,
        vertex.1 + distance * next_normal.1,
    );
    if turn.abs() < STRAIGHT_TURN_EPSILON && dot > 0.0 {
        return OffsetCorner {
            entry: straight,
            exit: straight,
            arc: None,
        };
    }
    if 1.0 + dot < STRAIGHT_TURN_EPSILON {
        // The edges double back on themselves: there is no mitre to take, so
        // the offset caps the spike with a half circle of its own radius.
        return spike_cap(vertex, previous_normal, next_normal, orientation, distance);
    }

    // `mitre` carries the crossing of the two offset lines at unit distance;
    // `n_i . mitre == 1`, so a point `vertex + k * mitre` sits at `k` in the
    // same measure the offset lines are quoted in.
    let mitre: Point = (
        (previous_normal.0 + next_normal.0) / (1.0 + dot),
        (previous_normal.1 + next_normal.1) / (1.0 + dot),
    );
    let cosine: f64 = 1.0 / mitre.0.hypot(mitre.1);
    // Which way the arc's farthest point lies from its own centre along the
    // mitre direction. The arc has to leave each offset edge along the edge's
    // own direction — anything else would double back and fold the outline —
    // and that tangent fixes the sense: a convex turn sweeps around the
    // outside of its centre, a concave one around the inside.
    let bulge: f64 = if turn < 0.0 { -1.0 } else { 1.0 };
    let target: f64 = corner_reach(reach, distance, dot, cosine, turn);

    let (mut centre_offset, mut radius): (f64, f64) = if target <= distance {
        let radius: f64 = (distance - target) / (1.0 - bulge * cosine);
        (distance - radius, radius)
    } else {
        let radius: f64 = (target - distance) / (1.0 + bulge * cosine);
        (distance + radius, radius)
    };
    // An arc leaves its edges at `centre_offset * (mitre . edge)` from the
    // vertex; letting that run past the edge's midpoint would have the two
    // corners of a short edge swap places and fold the ring inside out.
    let travel_limit: f64 = corner_travel_limit(mitre, edge_lengths, previous_normal, next_normal);
    if centre_offset.abs() > travel_limit {
        centre_offset = centre_offset.signum() * travel_limit;
        radius = (distance - centre_offset).abs();
    }

    let centre: Point = (
        vertex.0 + centre_offset * mitre.0,
        vertex.1 + centre_offset * mitre.1,
    );
    if radius < DEGENERATE_RADIUS_PT {
        return OffsetCorner {
            entry: centre,
            exit: centre,
            arc: None,
        };
    }
    let span: f64 = distance - centre_offset;
    let entry: Point = (
        centre.0 + span * previous_normal.0,
        centre.1 + span * previous_normal.1,
    );
    let exit: Point = (
        centre.0 + span * next_normal.0,
        centre.1 + span * next_normal.1,
    );
    OffsetCorner {
        entry,
        exit,
        arc: Some(CornerArc {
            centre,
            radius,
            start_angle: (entry.1 - centre.1).atan2(entry.0 - centre.0),
            sweep: corner_sweep(centre, entry, exit, previous_normal, orientation),
        }),
    }
}

/// How far the boundary reaches along the mitre direction, in the same
/// measure the offset lines are quoted in.
fn corner_reach(reach: CornerReach, distance: f64, _dot: f64, cosine: f64, turn: f64) -> f64 {
    match reach {
        CornerReach::Mitre => distance,
        CornerReach::Round => {
            if turn * distance > 0.0 {
                distance * cosine
            } else {
                distance
            }
        }
    }
}

/// The angle the arc turns through, taking the direction from the tangent it
/// has to leave the incoming edge along.
fn corner_sweep(
    centre: Point,
    entry: Point,
    exit: Point,
    previous_normal: Point,
    orientation: f64,
) -> f64 {
    let start: f64 = (entry.1 - centre.1).atan2(entry.0 - centre.0);
    let end: f64 = (exit.1 - centre.1).atan2(exit.0 - centre.0);
    // The incoming edge runs with the shape's own winding, which the outward
    // normal is a quarter turn from; rotating the normal back gives it.
    let travel: Point = (
        -orientation * previous_normal.1,
        orientation * previous_normal.0,
    );
    let radial: Point = (entry.0 - centre.0, entry.1 - centre.1);
    // Counter-clockwise motion at `radial` heads along (-y, x).
    let counter_clockwise: bool = (-radial.1 * travel.0 + radial.0 * travel.1) > 0.0;
    let mut sweep: f64 = end - start;
    if counter_clockwise {
        while sweep <= 0.0 {
            sweep += 2.0 * PI;
        }
    } else {
        while sweep >= 0.0 {
            sweep -= 2.0 * PI;
        }
    }
    sweep
}

/// How far a corner's arc may pull its tangent points along the two edges it
/// joins before it would reach past their midpoints.
fn corner_travel_limit(
    mitre: Point,
    edge_lengths: (f64, f64),
    previous_normal: Point,
    next_normal: Point,
) -> f64 {
    // The tangent points move along their own edges only through the mitre
    // term: the normal term is perpendicular to the edge and contributes
    // nothing.
    [
        (edge_lengths.0, previous_normal),
        (edge_lengths.1, next_normal),
    ]
    .into_iter()
    .map(|(length, normal)| {
        // The edge direction is the normal turned a quarter circle.
        let travel: f64 = (mitre.0 * -normal.1 + mitre.1 * normal.0).abs();
        if travel <= DEGENERATE_RADIUS_PT {
            f64::MAX
        } else {
            0.5 * length / travel
        }
    })
    .fold(f64::MAX, f64::min)
}

/// A vertex whose two edges double back: the offset caps it with a half
/// circle, which is what both a round join and a dilation leave there.
fn spike_cap(
    vertex: Point,
    previous_normal: Point,
    next_normal: Point,
    orientation: f64,
    distance: f64,
) -> OffsetCorner {
    let radius: f64 = distance.abs();
    let entry: Point = (
        vertex.0 + distance * previous_normal.0,
        vertex.1 + distance * previous_normal.1,
    );
    let exit: Point = (
        vertex.0 + distance * next_normal.0,
        vertex.1 + distance * next_normal.1,
    );
    if radius < DEGENERATE_RADIUS_PT {
        return OffsetCorner {
            entry: vertex,
            exit: vertex,
            arc: None,
        };
    }
    OffsetCorner {
        entry,
        exit,
        arc: Some(CornerArc {
            centre: vertex,
            radius,
            start_angle: (entry.1 - vertex.1).atan2(entry.0 - vertex.0),
            sweep: corner_sweep(vertex, entry, exit, previous_normal, orientation),
        }),
    }
}

/// Approximate `arc` by cubic Bézier segments, each turning at most a quarter
/// circle so the standard `4/3 tan(delta/4)` handle stays accurate to about a
/// ten-thousandth of the radius.
///
/// Each entry is `(control_start, control_end, end)`, ready for Typst's
/// `curve.cubic`.
pub(super) fn arc_beziers(arc: &CornerArc) -> Vec<(Point, Point, Point)> {
    let steps: usize = ((arc.sweep.abs() / (PI / 2.0)).ceil() as usize).max(1);
    let step: f64 = arc.sweep / steps as f64;
    let handle: f64 = 4.0 / 3.0 * (step / 4.0).tan();
    (0..steps)
        .map(|index| {
            let start: f64 = arc.start_angle + step * index as f64;
            let end: f64 = start + step;
            let at = |angle: f64| -> Point {
                (
                    arc.centre.0 + arc.radius * angle.cos(),
                    arc.centre.1 + arc.radius * angle.sin(),
                )
            };
            let start_point: Point = at(start);
            let end_point: Point = at(end);
            (
                (
                    start_point.0 - handle * arc.radius * start.sin(),
                    start_point.1 + handle * arc.radius * start.cos(),
                ),
                (
                    end_point.0 + handle * arc.radius * end.sin(),
                    end_point.1 - handle * arc.radius * end.cos(),
                ),
                end_point,
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "typst_gen_shadow_outline_tests.rs"]
mod tests;

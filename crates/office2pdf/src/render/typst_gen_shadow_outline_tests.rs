//! Tests for the offset outline a crisp polygon shadow follows (issue #1206).

use std::f64::consts::PI;

use super::{CornerReach, OffsetCorner, offset_ring};

/// A tall isoceles triangle in its own frame, apex up, listed clockwise on a
/// y-down page. Its apex sits far from the bounding box's own corner, which
/// is what separates an offset from a scale.
fn tall_triangle() -> Vec<(f64, f64)> {
    vec![(60.0, 0.0), (120.0, 240.0), (0.0, 240.0)]
}

/// Perpendicular distance from `point` to the infinite line through `a`, `b`,
/// positive on the side the polygon's outward normal points to.
fn signed_edge_distance(a: (f64, f64), b: (f64, f64), point: (f64, f64)) -> f64 {
    let (edge_x, edge_y): (f64, f64) = (b.0 - a.0, b.1 - a.1);
    let length: f64 = edge_x.hypot(edge_y);
    // Clockwise on a y-down page puts the interior to the left of each edge,
    // so (dy, -dx) points out of the shape.
    ((point.0 - a.0) * edge_y - (point.1 - a.1) * edge_x) / length
}

fn corner_points(corners: &[OffsetCorner]) -> Vec<(f64, f64)> {
    corners
        .iter()
        .flat_map(|corner| [corner.entry, corner.exit])
        .collect()
}

/// The defect itself: a ring is the outline pushed out by a fixed distance,
/// so every edge moves by that distance whatever its angle. Scaling the
/// vertices onto an expanded bounding box moves a slanted edge by less and
/// the apex by more, which is what #1206 reports.
#[test]
fn every_edge_of_an_offset_ring_moves_by_the_same_distance() {
    let vertices: Vec<(f64, f64)> = tall_triangle();
    let offset: f64 = 8.0;
    let corners: Vec<OffsetCorner> = offset_ring(&vertices, offset, CornerReach::Mitre);
    assert_eq!(corners.len(), vertices.len());

    for (index, corner) in corners.iter().enumerate() {
        let previous: (f64, f64) = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current: (f64, f64) = vertices[index];
        let next: (f64, f64) = vertices[(index + 1) % vertices.len()];
        let entry: f64 = signed_edge_distance(previous, current, corner.entry);
        let exit: f64 = signed_edge_distance(current, next, corner.exit);
        assert!(
            (entry - offset).abs() < 1e-9 && (exit - offset).abs() < 1e-9,
            "corner {index} sits {entry:.4}/{exit:.4}pt off its edges, not {offset}pt",
        );
    }
}

/// A scale would put the apex `offset` above the shape; the offset puts it
/// `offset / sin(half-angle)` above, because the two slanted edges have to
/// clear the apex on both sides.
#[test]
fn a_mitred_ring_lifts_a_sharp_apex_past_the_bounding_box() {
    let vertices: Vec<(f64, f64)> = tall_triangle();
    let offset: f64 = 8.0;
    let corners: Vec<OffsetCorner> = offset_ring(&vertices, offset, CornerReach::Mitre);

    let half_angle: f64 = (60.0_f64).atan2(240.0);
    let expected_y: f64 = -offset / half_angle.sin();
    assert!(
        (corners[0].entry.1 - expected_y).abs() < 1e-6,
        "apex at {:.4}pt, expected {expected_y:.4}pt (a scale would say {:.4}pt)",
        corners[0].entry.1,
        -offset,
    );
}

/// `a:round` is DrawingML's default join (#1090): the offset edges are joined
/// by an arc of the offset's own radius, centred on the fill vertex.
#[test]
fn a_round_ring_turns_a_convex_corner_on_an_arc_of_the_offset() {
    let square: Vec<(f64, f64)> = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)];
    let offset: f64 = 6.0;
    let corners: Vec<OffsetCorner> = offset_ring(&square, offset, CornerReach::Round);

    for (index, corner) in corners.iter().enumerate() {
        let arc = corner
            .arc
            .as_ref()
            .unwrap_or_else(|| panic!("corner {index} has no arc"));
        assert!((arc.radius - offset).abs() < 1e-9, "radius {}", arc.radius);
        assert!(
            (arc.centre.0 - square[index].0).abs() < 1e-9
                && (arc.centre.1 - square[index].1).abs() < 1e-9,
            "arc {index} centred at {:?}, not on the vertex {:?}",
            arc.centre,
            square[index],
        );
        assert!(
            (arc.sweep.abs() - PI / 2.0).abs() < 1e-9,
            "a right angle turns a quarter circle, not {:.4} rad",
            arc.sweep,
        );
    }
}

/// A crisp shadow has no ramp to follow, so its single ring is the silhouette
/// itself: with no outline to outset it, that is the fill path unchanged.
#[test]
fn a_zero_offset_ring_is_the_outline_itself() {
    let vertices: Vec<(f64, f64)> = tall_triangle();
    let corners: Vec<OffsetCorner> = offset_ring(&vertices, 0.0, CornerReach::Round);
    for (corner, vertex) in corners.iter().zip(vertices.iter()) {
        assert!(corner.arc.is_none(), "a zero offset turns no arc");
        assert!(
            (corner.entry.0 - vertex.0).abs() < 1e-9 && (corner.entry.1 - vertex.1).abs() < 1e-9,
            "{:?} should be the vertex {vertex:?}",
            corner.entry,
        );
    }
}

/// Listing the same ring the other way round describes the same shape, so it
/// has to offset the same way — outward, not inward.
#[test]
fn ring_orientation_does_not_flip_the_offset_direction() {
    let vertices: Vec<(f64, f64)> = tall_triangle();
    let reversed: Vec<(f64, f64)> = vertices.iter().rev().copied().collect();
    let forward: Vec<(f64, f64)> = corner_points(&offset_ring(&vertices, 7.0, CornerReach::Mitre));
    let backward: Vec<(f64, f64)> = corner_points(&offset_ring(&reversed, 7.0, CornerReach::Mitre));

    let bounds = |points: &[(f64, f64)]| -> (f64, f64, f64, f64) {
        points.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(x0, y0, x1, y1), (x, y)| (x0.min(*x), y0.min(*y), x1.max(*x), y1.max(*y)),
        )
    };
    let (fx0, fy0, fx1, fy1) = bounds(&forward);
    let (bx0, by0, bx1, by1) = bounds(&backward);
    assert!(
        (fx0 - bx0).abs() < 1e-6
            && (fy0 - by0).abs() < 1e-6
            && (fx1 - bx1).abs() < 1e-6
            && (fy1 - by1).abs() < 1e-6,
        "forward {:?} vs reversed {:?}",
        (fx0, fy0, fx1, fy1),
        (bx0, by0, bx1, by1),
    );
    assert!(fy0 < 0.0, "the ring grows past the shape, not into it");
}

//! Tests for the offset outline a polygon shadow ring follows (issue #1206).

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

/// The blurred contour turns *inside* the equidistant one at a convex corner,
/// because an isotropic Gaussian's coverage there is the product of the two
/// edges' own tails rather than a single edge's (#1204). The polygon ring has
/// to follow the same contour the rectangle's does.
#[test]
fn a_blurred_ring_turns_a_convex_corner_inside_the_dilated_one() {
    let square: Vec<(f64, f64)> = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 80.0), (0.0, 80.0)];
    let sigma: f64 = 5.0;
    let expansion: f64 = 5.0;
    let dilated: Vec<OffsetCorner> = offset_ring(&square, expansion, CornerReach::Round);
    let blurred: Vec<OffsetCorner> = offset_ring(
        &square,
        expansion,
        CornerReach::Blurred { sigma, expansion },
    );

    // The top-left corner's outward diagonal: how far each ring reaches.
    let reach = |corners: &[OffsetCorner]| -> f64 {
        let arc = corners[0].arc.as_ref().expect("a corner arc");
        let unit: f64 = std::f64::consts::FRAC_1_SQRT_2;
        // The farthest point of the arc along (-1,-1)/sqrt(2).
        -(arc.centre.0 + arc.centre.1) * unit + arc.radius
    };
    let dilated_reach: f64 = reach(&dilated);
    let blurred_reach: f64 = reach(&blurred);
    assert!(
        blurred_reach < dilated_reach - 0.5,
        "blurred reach {blurred_reach:.3}pt should sit well inside the dilated {dilated_reach:.3}pt",
    );
    assert!(
        blurred_reach > 0.0,
        "the contour still clears the corner: {blurred_reach:.3}pt",
    );
}

/// At a concave turn the dilation runs the two offset edges together at a
/// mitre point, but the blur reaches *past* it: the notch has material on
/// both sides feeding coverage into it, so the shadow penetrates deeper than
/// the equidistant contour does.
#[test]
fn a_blurred_ring_reaches_past_the_mitre_of_a_concave_turn() {
    // An arrow-like notch: vertices listed clockwise on a y-down page, with
    // vertex 2 concave.
    let notched: Vec<(f64, f64)> = vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (60.0, 50.0),
        (100.0, 100.0),
        (0.0, 100.0),
    ];
    let sigma: f64 = 6.0;
    let expansion: f64 = 4.0;
    let mitred: Vec<OffsetCorner> = offset_ring(&notched, expansion, CornerReach::Round);
    let blurred: Vec<OffsetCorner> = offset_ring(
        &notched,
        expansion,
        CornerReach::Blurred { sigma, expansion },
    );

    // The notch points along +x; deeper means a larger x.
    let mitre_x: f64 = mitred[2].entry.0;
    assert!(
        mitred[2].arc.is_none(),
        "the dilation mitres a concave turn",
    );
    let blurred_x: f64 = deepest_x(&blurred[2]);
    assert!(
        blurred_x > mitre_x + 0.1,
        "blurred notch reaches {blurred_x:.3}pt, mitre {mitre_x:.3}pt",
    );
    // The arc has to stay one continuous run between the two offset edges:
    // its own endpoints bound it, so no point of it doubles back past them.
    let arc = blurred[2].arc.as_ref().expect("the notch's arc");
    assert!(
        arc.sweep.abs() < PI,
        "a right-angle notch turns less than a half circle, not {:.3} rad",
        arc.sweep,
    );
}

/// The farthest point an offset corner reaches along +x, walked over the arc
/// itself rather than read off its bounding circle.
fn deepest_x(corner: &OffsetCorner) -> f64 {
    let Some(arc) = corner.arc.as_ref() else {
        return corner.entry.0.max(corner.exit.0);
    };
    (0..=256)
        .map(|step| {
            let angle: f64 = arc.start_angle + arc.sweep * f64::from(step) / 256.0;
            arc.centre.0 + arc.radius * angle.cos()
        })
        .fold(f64::MIN, f64::max)
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

/// The generalisation has to agree with the right-angle model the rectangle
/// branch already carries (#1204), or a square-cornered polygon and a `#rect`
/// would cast two different shadows.
#[test]
fn the_wedge_contour_matches_the_rectangle_corner_model() {
    let square: Vec<(f64, f64)> = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    for sigma in [2.0_f64, 5.0, 11.0] {
        for expansion in [-2.0_f64, 0.0, 3.0, 9.0] {
            let corners: Vec<OffsetCorner> = offset_ring(
                &square,
                expansion,
                CornerReach::Blurred { sigma, expansion },
            );
            let expected: f64 =
                crate::render::typst_gen::shapes::shadow_ring_corner_radius(0.0, expansion, sigma);
            let arc_radius: f64 = corners[0].arc.as_ref().map_or(0.0, |arc| arc.radius);
            assert!(
                (arc_radius - expected).abs() < 1e-6,
                "sigma {sigma}, expansion {expansion}: polygon arc {arc_radius:.6} \
                 against the rectangle's {expected:.6}",
            );
        }
    }
}

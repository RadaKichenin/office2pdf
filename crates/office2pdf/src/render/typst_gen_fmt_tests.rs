use super::*;
use crate::ir::{BorderLineStyle, BorderSide, Color};

#[test]
fn test_rgb_literal() {
    assert_eq!(rgb(&Color::new(66, 133, 244)), "rgb(66, 133, 244)");
    assert_eq!(rgb(&Color::new(0, 0, 0)), "rgb(0, 0, 0)");
}

#[test]
fn test_rgb_with_alpha_literal() {
    assert_eq!(
        rgb_with_alpha(&Color::new(255, 192, 0), 128),
        "rgb(255, 192, 0, 128)"
    );
}

#[test]
fn test_stroke_value_solid() {
    let side = BorderSide {
        width: 1.5,
        color: Color::new(10, 20, 30),
        style: BorderLineStyle::Solid,
    };
    assert_eq!(stroke_value(&side, false), "1.5pt + rgb(10, 20, 30)");
    assert_eq!(
        stroke_value(&side, true),
        "1.5pt + rgb(10, 20, 30)",
        "double_is_plain must not affect Solid"
    );
}

#[test]
fn test_stroke_value_integral_width_has_no_decimal_point() {
    let side = BorderSide {
        width: 2.0,
        color: Color::new(0, 0, 0),
        style: BorderLineStyle::Solid,
    };
    assert_eq!(stroke_value(&side, false), "2pt + rgb(0, 0, 0)");
}

#[test]
fn test_stroke_value_dashed() {
    let side = BorderSide {
        width: 0.75,
        color: Color::new(200, 0, 0),
        style: BorderLineStyle::Dashed,
    };
    assert_eq!(
        stroke_value(&side, false),
        "(paint: rgb(200, 0, 0), thickness: 0.75pt, dash: \"dashed\")"
    );
}

#[test]
fn test_stroke_value_dotted() {
    let side = BorderSide {
        width: 1.0,
        color: Color::new(0, 0, 0),
        style: BorderLineStyle::Dotted,
    };
    assert_eq!(
        stroke_value(&side, true),
        "(paint: rgb(0, 0, 0), thickness: 1pt, dash: \"dotted\")",
        "double_is_plain must not affect genuinely dashed styles"
    );
}

// Tables historically render Double borders as a plain stroke while shapes
// send them through the dash dict (mapping to dash: "solid"). stroke_value
// must preserve both behaviors until the divergence is fixed with visual
// verification.
#[test]
fn test_stroke_value_double_preserves_caller_divergence() {
    let side = BorderSide {
        width: 1.0,
        color: Color::new(5, 6, 7),
        style: BorderLineStyle::Double,
    };
    assert_eq!(
        stroke_value(&side, true),
        "1pt + rgb(5, 6, 7)",
        "table borders treat Double as plain"
    );
    assert_eq!(
        stroke_value(&side, false),
        "(paint: rgb(5, 6, 7), thickness: 1pt, dash: \"solid\")",
        "shape strokes send Double through the dash dict"
    );
}

#[test]
fn test_format_f64_integral_drops_fraction() {
    assert_eq!(format_f64(12.0), "12");
    assert_eq!(format_f64(0.0), "0");
    assert_eq!(format_f64(-3.0), "-3");
}

#[test]
fn test_format_f64_fractional_keeps_value() {
    assert_eq!(format_f64(1.5), "1.5");
    assert_eq!(format_f64(0.75), "0.75");
}

/// ECMA-376 defines the `a:prstDash` presets as multiples of the line width,
/// so the dash array scales with the stroke (issue #678).
#[test]
fn drawingml_dash_array_scales_with_line_width() {
    // 0.5pt is the audited fixture's `<a:ln w="6350">`.
    assert_eq!(
        drawingml_dash_array_pt(BorderLineStyle::Dashed, 0.5),
        Some(vec![2.0, 1.5]),
        "dash is 4w on, 3w off"
    );
    assert_eq!(
        drawingml_dash_array_pt(BorderLineStyle::DashDot, 0.5),
        Some(vec![2.0, 1.5, 0.5, 1.5]),
        "dashDot is 4w, 3w, 1w, 3w"
    );
}

/// Triangulation: a different width scales the whole array, so the lengths
/// cannot be a table of constants.
#[test]
fn drawingml_dash_array_doubles_with_a_doubled_width() {
    assert_eq!(
        drawingml_dash_array_pt(BorderLineStyle::Dashed, 1.0),
        Some(vec![4.0, 3.0])
    );
}

/// Styles with no dash rhythm keep the plain or named form.
#[test]
fn drawingml_dash_array_is_absent_for_undashed_styles() {
    assert_eq!(drawingml_dash_array_pt(BorderLineStyle::Solid, 0.5), None);
    assert_eq!(drawingml_dash_array_pt(BorderLineStyle::None, 0.5), None);
    // A zero width would make every length zero.
    assert_eq!(drawingml_dash_array_pt(BorderLineStyle::Dashed, 0.0), None);
}

/// `Dotted` and `DashDotDot` keep the named patterns: their ratios are not
/// measured, and each variant buckets presets whose real ratios differ, so a
/// single array would be wrong for some of them.
#[test]
fn drawingml_dash_array_is_absent_for_unmeasured_buckets() {
    assert_eq!(drawingml_dash_array_pt(BorderLineStyle::Dotted, 0.5), None);
    assert_eq!(
        drawingml_dash_array_pt(BorderLineStyle::DashDotDot, 0.5),
        None
    );
}

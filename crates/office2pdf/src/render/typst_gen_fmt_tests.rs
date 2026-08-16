use super::*;
use crate::ir::{BorderLineStyle, BorderSide, Color, LineJoin};

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
        join: LineJoin::Round,
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
        join: LineJoin::Round,
    };
    assert_eq!(stroke_value(&side, false), "2pt + rgb(0, 0, 0)");
}

#[test]
fn test_stroke_value_dashed() {
    let side = BorderSide {
        width: 0.75,
        color: Color::new(200, 0, 0),
        style: BorderLineStyle::Dashed,
        join: LineJoin::Round,
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
        join: LineJoin::Round,
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
        join: LineJoin::Round,
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

/// Every `a:prstDash` preset carries its own rhythm (issue #758).
///
/// The multiples are the ones measured off a rendered PDF's `d` operator; see
/// `drawingml_dash_array_pt`. At w=1pt the array *is* the multiples, so a
/// wrong ratio shows up here as a wrong number rather than a scaled one.
#[test]
fn drawingml_dash_array_differs_between_presets() {
    let at_1pt = |style| drawingml_dash_array_pt(style, 1.0);
    assert_eq!(at_1pt(BorderLineStyle::Dotted), Some(vec![1.0, 3.0]));
    assert_eq!(at_1pt(BorderLineStyle::SystemDot), Some(vec![1.0, 1.0]));
    assert_eq!(at_1pt(BorderLineStyle::Dashed), Some(vec![4.0, 3.0]));
    assert_eq!(at_1pt(BorderLineStyle::SystemDash), Some(vec![3.0, 1.0]));
    assert_eq!(at_1pt(BorderLineStyle::LargeDash), Some(vec![8.0, 3.0]));
    assert_eq!(
        at_1pt(BorderLineStyle::DashDot),
        Some(vec![4.0, 3.0, 1.0, 3.0])
    );
    assert_eq!(
        at_1pt(BorderLineStyle::SystemDashDot),
        Some(vec![3.0, 1.0, 1.0, 1.0])
    );
    assert_eq!(
        at_1pt(BorderLineStyle::LargeDashDot),
        Some(vec![8.0, 3.0, 1.0, 3.0])
    );
    assert_eq!(
        at_1pt(BorderLineStyle::LargeDashDotDot),
        Some(vec![8.0, 3.0, 1.0, 3.0, 1.0, 3.0])
    );
    assert_eq!(
        at_1pt(BorderLineStyle::SystemDashDotDot),
        Some(vec![3.0, 1.0, 1.0, 1.0, 1.0, 1.0])
    );
}

/// The three families are no longer interchangeable within themselves: the
/// dash-family presets differ, and so do the dash-dot ones. A regression that
/// re-merged any pair would pass the per-preset table above only if it also
/// changed the expected numbers, but it would fail here immediately.
#[test]
fn drawingml_dash_array_keeps_same_family_presets_distinct() {
    let arrays = [
        BorderLineStyle::Dashed,
        BorderLineStyle::SystemDash,
        BorderLineStyle::LargeDash,
        BorderLineStyle::DashDot,
        BorderLineStyle::SystemDashDot,
        BorderLineStyle::LargeDashDot,
        BorderLineStyle::Dotted,
        BorderLineStyle::SystemDot,
    ]
    .map(|style| drawingml_dash_array_pt(style, 1.0));
    for (i, left) in arrays.iter().enumerate() {
        for right in arrays.iter().skip(i + 1) {
            assert_ne!(left, right, "two presets share one dash array");
        }
    }
}

/// `lgDashDot` is a long dash followed by a dot, not a dot pattern. It used to
/// map to `Dotted` and render as evenly spaced dots (issue #758).
#[test]
fn large_dash_dot_is_not_a_dot_pattern() {
    let dotted = drawingml_dash_array_pt(BorderLineStyle::Dotted, 1.0).unwrap();
    let large_dash_dot = drawingml_dash_array_pt(BorderLineStyle::LargeDashDot, 1.0).unwrap();
    assert_eq!(large_dash_dot[0], 8.0 * dotted[0], "its dash is 8w, not 1w");
    assert_eq!(large_dash_dot.len(), 4, "a dash and a dot, not one dot");
}

/// A DrawingML `a:ln` that names no join rounds its corners, while Typst's own
/// stroke default is miter, so the join has to be written out (issue #1090).
///
/// Ground truth is a native macOS PowerPoint export of `customGeo.pptx` page
/// 46, whose title banner takes a theme `a:ln` with no join child: its
/// `mutool draw -F trace` reports `linejoin="1"` — PDF's round join — against
/// the `linejoin="0"` our miter default produced.
#[test]
fn drawingml_stroke_value_rounds_a_corner_by_default() {
    let side = BorderSide {
        width: 3.0,
        color: Color::new(255, 255, 255),
        style: BorderLineStyle::Solid,
        join: LineJoin::Round,
    };
    assert_eq!(
        drawingml_stroke_value(&side),
        "(paint: rgb(255, 255, 255), thickness: 3pt, join: \"round\")"
    );
}

/// Triangulation: `a:bevel` and `a:miter` are their own joins, so the value
/// cannot be a constant `"round"`.
#[test]
fn drawingml_stroke_value_carries_each_stated_join() {
    let with_join = |join| {
        drawingml_stroke_value(&BorderSide {
            width: 1.0,
            color: Color::new(0, 0, 0),
            style: BorderLineStyle::Solid,
            join,
        })
    };
    assert_eq!(
        with_join(LineJoin::Bevel),
        "(paint: rgb(0, 0, 0), thickness: 1pt, join: \"bevel\")"
    );
    assert_eq!(
        with_join(LineJoin::Miter),
        "(paint: rgb(0, 0, 0), thickness: 1pt, join: \"miter\")"
    );
}

/// A dashed outline turns its corners the same way a solid one does, so the
/// join survives alongside the width-proportional dash array of issue #678.
#[test]
fn drawingml_stroke_value_keeps_the_join_on_a_dashed_outline() {
    let side = BorderSide {
        width: 0.5,
        color: Color::new(200, 0, 0),
        style: BorderLineStyle::Dashed,
        join: LineJoin::Miter,
    };
    assert_eq!(
        drawingml_stroke_value(&side),
        "(paint: rgb(200, 0, 0), thickness: 0.5pt, dash: (2pt, 1.5pt), join: \"miter\")"
    );
}

/// Word table borders and Excel cell borders have no DrawingML join, and
/// `stroke_value` is theirs: it must keep emitting the plain shorthand rather
/// than gain a key from a field it does not model.
#[test]
fn stroke_value_never_writes_a_join() {
    let side = BorderSide {
        width: 1.0,
        color: Color::new(0, 0, 0),
        style: BorderLineStyle::Solid,
        join: LineJoin::Miter,
    };
    assert_eq!(stroke_value(&side, true), "1pt + rgb(0, 0, 0)");
}

/// `DashDotDot` is Word's and Excel's alone once every preset has its own
/// variant, and those two formats keep the named patterns.
#[test]
fn drawingml_dash_array_is_absent_for_the_word_only_style() {
    assert_eq!(
        drawingml_dash_array_pt(BorderLineStyle::DashDotDot, 0.5),
        None
    );
}

//! Canonical Typst value formatters shared by the codegen modules.
//!
//! New color literals and stroke values in generated Typst source should be
//! built with these helpers so the output stays uniform and golden tests
//! don't drift on formatting details.

use crate::ir::{BorderLineStyle, BorderSide, Color};

/// Format a Typst `rgb(r, g, b)` color literal.
pub(super) fn rgb(color: &Color) -> String {
    format!("rgb({}, {}, {})", color.r, color.g, color.b)
}

/// Format a Typst `rgb(r, g, b, a)` color literal with an alpha channel.
pub(super) fn rgb_with_alpha(color: &Color, alpha: u8) -> String {
    format!("rgb({}, {}, {}, {})", color.r, color.g, color.b, alpha)
}

/// Format a stroke value: `Wpt + rgb(...)` for plain styles, a
/// `(paint: ..., thickness: ..., dash: "...")` dict for patterned ones.
///
/// `double_is_plain` preserves an existing divergence: table borders render
/// `Double` as a plain stroke, while shape strokes send it through the dash
/// dict (where it maps to `dash: "solid"`). Unifying that is a visible-output
/// change and belongs in its own visually-verified fix.
pub(super) fn stroke_value(side: &BorderSide, double_is_plain: bool) -> String {
    let is_plain = match side.style {
        BorderLineStyle::Solid | BorderLineStyle::None => true,
        BorderLineStyle::Double => double_is_plain,
        _ => false,
    };
    if is_plain {
        format!("{}pt + {}", format_f64(side.width), rgb(&side.color))
    } else {
        format!(
            "(paint: {}, thickness: {}pt, dash: \"{}\")",
            rgb(&side.color),
            format_f64(side.width),
            super::border_line_style_to_typst(side.style),
        )
    }
}

/// The dash array for a DrawingML stroke, in points, scaled by the line width.
///
/// ECMA-376 states the `a:prstDash` presets as multiples of the line width
/// `w`, not as absolute lengths, so a thin line takes a proportionally short
/// dash. Typst's named patterns carry fixed point lengths instead, which made
/// a 0.5pt stroke's `dash` period 6.0pt against GT's 3.5pt (issue #678).
///
/// Only the two ratios measured against GT are applied:
///
/// | preset | multiples | measured at w=0.5pt |
/// | --- | --- | --- |
/// | `dash` | 4w, 3w | 2.0, 1.5 |
/// | `dashDot` | 4w, 3w, 1w, 3w | 2.0, 1.5, 0.5, 1.5 |
///
/// `Dotted` and `DashDotDot` return `None` and keep the named patterns. Their
/// ratios are not verified here, and `BorderLineStyle` cannot express them
/// faithfully in any case: `parser/pptx_shapes.rs` buckets presets with
/// different ratios into one variant — `dot`, `sysDot` and `lgDashDot` all
/// become `Dotted`, though `lgDashDot` is not a dot pattern at all. Giving
/// that bucket a single array would be wrong for two of its three presets.
/// Tracked in issue #758; this function deliberately leaves it alone.
///
/// The same bucketing limits the two arms that *are* applied: `Dashed` also
/// holds `lgDash` (8w/3w) and `sysDash` (3w/1w), and `DashDot` also holds
/// `sysDashDot` (3w/1w/1w/1w), so each array is right only for the preset it
/// was measured on. That is strictly better than the fixed absolute lengths
/// it replaces, and #758 is what makes it exact.
///
/// Scoped to DrawingML strokes: Word table borders and Excel cell borders
/// share `BorderLineStyle` but not this rule, so they keep the named patterns.
pub(super) fn drawingml_dash_array_pt(style: BorderLineStyle, width_pt: f64) -> Option<Vec<f64>> {
    let w = if width_pt > 0.0 {
        width_pt
    } else {
        return None;
    };
    let multiples: &[f64] = match style {
        BorderLineStyle::Dashed => &[4.0, 3.0],
        BorderLineStyle::DashDot => &[4.0, 3.0, 1.0, 3.0],
        BorderLineStyle::Dotted
        | BorderLineStyle::DashDotDot
        | BorderLineStyle::Solid
        | BorderLineStyle::Double
        | BorderLineStyle::None => return None,
    };
    Some(multiples.iter().map(|m| m * w).collect())
}

/// A DrawingML stroke value, using width-proportional dashes where the style
/// has a dash rhythm (issue #678).
pub(super) fn drawingml_stroke_value(side: &BorderSide) -> String {
    match drawingml_dash_array_pt(side.style, side.width) {
        Some(array) => {
            let lengths: Vec<String> = array
                .iter()
                .map(|len| format!("{}pt", format_f64(*len)))
                .collect();
            format!(
                "(paint: {}, thickness: {}pt, dash: ({}))",
                rgb(&side.color),
                format_f64(side.width),
                lengths.join(", "),
            )
        }
        None => stroke_value(side, false),
    }
}

/// Format a float without a trailing `.0` on integral values.
pub(super) fn format_f64(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
#[path = "typst_gen_fmt_tests.rs"]
mod tests;

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
/// The `a:prstDash` presets are rhythms in multiples of the line width `w`,
/// not absolute lengths, so a thin line takes a proportionally short dash.
/// Typst's named patterns carry fixed point lengths instead, which made a
/// 0.5pt stroke's `dash` period 6.0pt against GT's 3.5pt (issue #678).
///
/// Every multiple below was read off a rendered PDF's `d` operator rather than
/// quoted from a specification: a deck with one 2pt line per preset, rendered
/// by LibreOffice 24.2.7.2, emits `2.01256 w` and arrays whose entries divide
/// by that width to exact integers.
///
/// | preset | measured array ÷ w |
/// | --- | --- |
/// | `dot` | 1, 3 |
/// | `sysDot` | 1, 1 |
/// | `dash` | 4, 3 |
/// | `sysDash` | 3, 1 |
/// | `lgDash` | 8, 3 |
/// | `dashDot` | 4, 3, 1, 3 |
/// | `sysDashDot` | 3, 1, 1, 1 |
/// | `lgDashDot` | 8, 3, 1, 3 |
/// | `lgDashDotDot` | 8, 3, 1, 3, 1, 3 |
/// | `sysDashDotDot` | 3, 1, 1, 1, 1, 1 |
///
/// LibreOffice is a second implementation, not PowerPoint; what it pins down
/// is the ratio each preset name denotes, which is the part `BorderLineStyle`
/// has to carry. The `dash` and `dashDot` rows agree with the earlier direct
/// GT measurement in #678.
///
/// Scoped to DrawingML strokes: Word table borders and Excel cell borders
/// share `BorderLineStyle` but not this rule, so they keep the named patterns.
/// `DashDotDot` is theirs alone — no preset maps to it — and has no array.
pub(super) fn drawingml_dash_array_pt(style: BorderLineStyle, width_pt: f64) -> Option<Vec<f64>> {
    let w = if width_pt > 0.0 {
        width_pt
    } else {
        return None;
    };
    let multiples: &[f64] = match style {
        BorderLineStyle::Dotted => &[1.0, 3.0],
        BorderLineStyle::SystemDot => &[1.0, 1.0],
        BorderLineStyle::Dashed => &[4.0, 3.0],
        BorderLineStyle::SystemDash => &[3.0, 1.0],
        BorderLineStyle::LargeDash => &[8.0, 3.0],
        BorderLineStyle::DashDot => &[4.0, 3.0, 1.0, 3.0],
        BorderLineStyle::SystemDashDot => &[3.0, 1.0, 1.0, 1.0],
        BorderLineStyle::LargeDashDot => &[8.0, 3.0, 1.0, 3.0],
        BorderLineStyle::LargeDashDotDot => &[8.0, 3.0, 1.0, 3.0, 1.0, 3.0],
        BorderLineStyle::SystemDashDotDot => &[3.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        BorderLineStyle::DashDotDot
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

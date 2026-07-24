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

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::*;
use crate::ir::Color;

fn scheme_with(colors: &[(&str, Color)], aliases: &[(&str, &str)]) -> (ColorsMap, AliasMap) {
    let colors: ColorsMap = colors
        .iter()
        .map(|(name, color)| ((*name).to_string(), *color))
        .collect();
    let aliases: AliasMap = aliases
        .iter()
        .map(|(from, to)| ((*from).to_string(), (*to).to_string()))
        .collect();
    (colors, aliases)
}

type ColorsMap = HashMap<String, Color>;
type AliasMap = HashMap<String, String>;

#[test]
fn resolve_scheme_color_direct_lookup() {
    let (colors, aliases) = scheme_with(&[("accent1", Color::new(68, 114, 196))], &[]);
    let scheme = SchemeColors {
        colors: &colors,
        aliases: &aliases,
    };
    assert_eq!(
        resolve_scheme_color(&scheme, "accent1"),
        Some(Color::new(68, 114, 196))
    );
    assert_eq!(resolve_scheme_color(&scheme, "accent2"), None);
}

#[test]
fn resolve_scheme_color_follows_alias() {
    // pptx clrMap maps bg1 → lt1 for typical slides.
    let (colors, aliases) = scheme_with(&[("lt1", Color::new(255, 255, 255))], &[("bg1", "lt1")]);
    let scheme = SchemeColors {
        colors: &colors,
        aliases: &aliases,
    };
    assert_eq!(
        resolve_scheme_color(&scheme, "bg1"),
        Some(Color::new(255, 255, 255))
    );
}

#[test]
fn resolve_scheme_color_falls_back_to_unaliased_name() {
    // Alias points at a missing entry; the raw name still resolves.
    let (colors, aliases) = scheme_with(&[("bg1", Color::new(1, 2, 3))], &[("bg1", "lt9")]);
    let scheme = SchemeColors {
        colors: &colors,
        aliases: &aliases,
    };
    assert_eq!(
        resolve_scheme_color(&scheme, "bg1"),
        Some(Color::new(1, 2, 3))
    );
}

#[test]
fn tint_blends_toward_white() {
    // OOXML tint 0.4: channel = 255 - (255 - c) * 0.4
    let out = apply_color_transforms(Color::new(0, 100, 255), &[ColorTransform::Tint(0.4)]);
    assert_eq!(out, Color::new(153, 193, 255));
}

#[test]
fn shade_scales_toward_black() {
    let out = apply_color_transforms(Color::new(200, 100, 50), &[ColorTransform::Shade(0.5)]);
    assert_eq!(out, Color::new(100, 50, 25));
}

#[test]
fn lum_mod_and_off_adjust_lightness_in_hsl() {
    // lumMod 0.5 halves lightness; pure red keeps its hue.
    let out = apply_color_transforms(Color::new(255, 0, 0), &[ColorTransform::LumMod(0.5)]);
    assert_eq!(out, Color::new(128, 0, 0));

    // lumOff +0.2 raises lightness toward a lighter red.
    let out = apply_color_transforms(Color::new(255, 0, 0), &[ColorTransform::LumOff(0.2)]);
    assert_eq!(out, Color::new(255, 102, 102));
}

#[test]
fn no_transforms_is_identity() {
    let color = Color::new(12, 34, 56);
    assert_eq!(apply_color_transforms(color, &[]), color);
}

fn parse_first_color(xml: &str, colors: &ColorsMap, aliases: &AliasMap) -> ParsedColor {
    let mut reader = Reader::from_str(xml);
    let scheme = SchemeColors { colors, aliases };
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                return parse_color_from_start(&mut reader, e, &scheme);
            }
            Ok(Event::Empty(ref e)) => {
                return parse_color_from_empty(e, &scheme);
            }
            Ok(Event::Eof) => panic!("no color element in fixture"),
            _ => {}
        }
    }
}

#[test]
fn parse_srgb_color_with_lum_transforms() {
    // Realistic DrawingML: accent fill darkened via lumMod as PowerPoint emits.
    let (colors, aliases) = scheme_with(&[], &[]);
    let parsed = parse_first_color(
        r#"<a:srgbClr val="FF0000"><a:lumMod val="50000"/></a:srgbClr>"#,
        &colors,
        &aliases,
    );
    assert_eq!(parsed.color, Some(Color::new(128, 0, 0)));
    assert_eq!(parsed.alpha, None);
}

#[test]
fn parse_scheme_color_with_alpha() {
    let (colors, aliases) = scheme_with(&[("accent1", Color::new(68, 114, 196))], &[]);
    let parsed = parse_first_color(
        r#"<a:schemeClr val="accent1"><a:alpha val="50000"/></a:schemeClr>"#,
        &colors,
        &aliases,
    );
    assert_eq!(parsed.color, Some(Color::new(68, 114, 196)));
    assert_eq!(parsed.alpha, Some(0.5));
}

#[test]
fn parse_sys_color_uses_last_clr() {
    let (colors, aliases) = scheme_with(&[], &[]);
    let parsed = parse_first_color(
        r#"<a:sysClr val="windowText" lastClr="000000"/>"#,
        &colors,
        &aliases,
    );
    assert_eq!(parsed.color, Some(Color::new(0, 0, 0)));
}

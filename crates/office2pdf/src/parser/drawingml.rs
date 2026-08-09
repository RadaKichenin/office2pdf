//! Shared DrawingML color primitives: scheme-color resolution and OOXML
//! color transforms (tint/shade/lumMod/lumOff/alpha).
//!
//! DrawingML color markup (`<a:srgbClr>`, `<a:schemeClr>`, `<a:sysClr>` with
//! nested transform children) is identical across pptx, docx, and xlsx parts.
//! This module holds the single implementation; format parsers supply their
//! own theme palette and alias map through [`SchemeColors`].

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::ir::{Color, DeclaredFontClass};
use crate::parser::xml_util::{get_attr_i64, get_attr_str, parse_hex_color};

/// A format-agnostic view of a theme's color scheme.
///
/// `aliases` carries format-specific scheme aliases, such as the pptx
/// `<p:clrMap>` or xlsx's implicit `bg`/`tx` mappings. Pass an empty map when
/// the format has no alias layer.
pub(crate) struct SchemeColors<'a> {
    pub(crate) colors: &'a HashMap<String, Color>,
    pub(crate) aliases: &'a HashMap<String, String>,
}

/// Resolve a scheme color name (e.g. `accent1`, `bg1`) against the theme.
///
/// The alias map is consulted first; if the aliased entry is missing, the raw
/// name is tried so a partially populated theme still resolves.
pub(crate) fn resolve_scheme_color(scheme: &SchemeColors<'_>, scheme_name: &str) -> Option<Color> {
    let mapped_name = scheme
        .aliases
        .get(scheme_name)
        .map(String::as_str)
        .unwrap_or(scheme_name);

    scheme
        .colors
        .get(mapped_name)
        .copied()
        .or_else(|| scheme.colors.get(scheme_name).copied())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ColorTransform {
    Tint(f64),
    Shade(f64),
    LumMod(f64),
    LumOff(f64),
}

/// A parsed DrawingML color: the resolved RGB value plus an optional alpha.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ParsedColor {
    pub(crate) color: Option<Color>,
    pub(crate) alpha: Option<f64>,
}

fn parse_base_color(element: &BytesStart<'_>, scheme: &SchemeColors<'_>) -> Option<Color> {
    match element.local_name().as_ref() {
        b"srgbClr" => get_attr_str(element, b"val").and_then(|hex| parse_hex_color(&hex)),
        b"schemeClr" => {
            get_attr_str(element, b"val").and_then(|name| resolve_scheme_color(scheme, &name))
        }
        b"sysClr" => get_attr_str(element, b"lastClr").and_then(|hex| parse_hex_color(&hex)),
        _ => None,
    }
}

pub(crate) fn parse_color_transform(element: &BytesStart<'_>) -> Option<ColorTransform> {
    let val = get_attr_i64(element, b"val")? as f64 / 100_000.0;
    match element.local_name().as_ref() {
        b"tint" => Some(ColorTransform::Tint(val)),
        b"shade" => Some(ColorTransform::Shade(val)),
        b"lumMod" => Some(ColorTransform::LumMod(val)),
        b"lumOff" => Some(ColorTransform::LumOff(val)),
        _ => None,
    }
}

/// Decode one 0-255 sRGB channel to linear light.
///
/// `a:shade` scales light, not the encoded byte. Halving the byte of `#4472C4`
/// gives `#223962`, where PowerPoint renders `#2F528F` — the value this curve
/// reproduces on all three channels (issue #667).
fn srgb_channel_to_linear(channel: f64) -> f64 {
    let normalized: f64 = channel / 255.0;
    if normalized <= 0.040_45 {
        normalized / 12.92
    } else {
        ((normalized + 0.055) / 1.055).powf(2.4)
    }
}

/// Re-encode linear light as a 0-255 sRGB channel, inverting
/// [`srgb_channel_to_linear`].
fn linear_to_srgb_channel(linear: f64) -> f64 {
    let encoded: f64 = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    encoded * 255.0
}

pub(crate) fn apply_color_transforms(color: Color, transforms: &[ColorTransform]) -> Color {
    // Tint and shade run before the luminance transforms, but not in the same
    // space: tint blends toward white across the sRGB bytes, while shade scales
    // in linear light (decode, multiply, re-encode) because that is what
    // reproduces PowerPoint's output (issue #667). Tint is left on the bytes
    // for want of a ground truth saying otherwise.
    let mut r: f64 = color.r as f64;
    let mut g: f64 = color.g as f64;
    let mut b: f64 = color.b as f64;

    for transform in transforms {
        match transform {
            ColorTransform::Tint(t) => {
                r = 255.0 - (255.0 - r) * t;
                g = 255.0 - (255.0 - g) * t;
                b = 255.0 - (255.0 - b) * t;
            }
            ColorTransform::Shade(s) => {
                r = linear_to_srgb_channel(srgb_channel_to_linear(r) * s);
                g = linear_to_srgb_channel(srgb_channel_to_linear(g) * s);
                b = linear_to_srgb_channel(srgb_channel_to_linear(b) * s);
            }
            _ => {}
        }
    }

    let tinted = Color::new(
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    );

    // Then apply luminance transforms in HSL space.
    let has_lum_transforms: bool = transforms
        .iter()
        .any(|t| matches!(t, ColorTransform::LumMod(_) | ColorTransform::LumOff(_)));

    if !has_lum_transforms {
        return tinted;
    }

    let (mut hue, mut saturation, mut lightness) = rgb_to_hsl(tinted);

    for transform in transforms {
        match transform {
            ColorTransform::LumMod(value) => {
                lightness = (lightness * value).clamp(0.0, 1.0);
            }
            ColorTransform::LumOff(value) => {
                lightness = (lightness + value).clamp(0.0, 1.0);
            }
            _ => {}
        }
    }

    saturation = saturation.clamp(0.0, 1.0);
    hue = hue.rem_euclid(360.0);
    hsl_to_rgb(hue, saturation, lightness)
}

/// Parse a self-closing color element (no transform children possible).
pub(crate) fn parse_color_from_empty(
    element: &BytesStart<'_>,
    scheme: &SchemeColors<'_>,
) -> ParsedColor {
    ParsedColor {
        color: parse_base_color(element, scheme),
        alpha: None,
    }
}

/// Parse a color element with children, consuming events through its end tag
/// and applying any nested transforms.
pub(crate) fn parse_color_from_start(
    reader: &mut Reader<&[u8]>,
    element: &BytesStart<'_>,
    scheme: &SchemeColors<'_>,
) -> ParsedColor {
    let base_color = parse_base_color(element, scheme);
    let mut transforms: Vec<ColorTransform> = Vec::new();
    let mut alpha: Option<f64> = None;
    let mut depth: usize = 1;

    while depth > 0 {
        match reader.read_event() {
            Ok(Event::Start(ref child)) => {
                depth += 1;
                if let Some(transform) = parse_color_transform(child) {
                    transforms.push(transform);
                } else if child.local_name().as_ref() == b"alpha" {
                    alpha = get_attr_i64(child, b"val").map(|v| v as f64 / 100_000.0);
                }
            }
            Ok(Event::Empty(ref child)) => {
                if let Some(transform) = parse_color_transform(child) {
                    transforms.push(transform);
                } else if child.local_name().as_ref() == b"alpha" {
                    alpha = get_attr_i64(child, b"val").map(|v| v as f64 / 100_000.0);
                }
            }
            Ok(Event::End(_)) => {
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let color = base_color.map(|base| apply_color_transforms(base, &transforms));

    ParsedColor { color, alpha }
}

fn rgb_to_hsl(color: Color) -> (f64, f64, f64) {
    let red = color.r as f64 / 255.0;
    let green = color.g as f64 / 255.0;
    let blue = color.b as f64 / 255.0;

    let max = red.max(green.max(blue));
    let min = red.min(green.min(blue));
    let delta = max - min;
    let lightness = (max + min) / 2.0;

    if delta == 0.0 {
        return (0.0, 0.0, lightness);
    }

    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue_sector = if max == red {
        ((green - blue) / delta).rem_euclid(6.0)
    } else if max == green {
        ((blue - red) / delta) + 2.0
    } else {
        ((red - green) / delta) + 4.0
    };

    (60.0 * hue_sector, saturation, lightness)
}

fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> Color {
    if saturation == 0.0 {
        let channel = (lightness * 255.0).round() as u8;
        return Color::new(channel, channel, channel);
    }

    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_prime = hue / 60.0;
    let secondary = chroma * (1.0 - ((hue_prime.rem_euclid(2.0)) - 1.0).abs());
    let match_lightness = lightness - chroma / 2.0;

    let (red, green, blue) = match hue_prime {
        h if (0.0..1.0).contains(&h) => (chroma, secondary, 0.0),
        h if (1.0..2.0).contains(&h) => (secondary, chroma, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, chroma, secondary),
        h if (3.0..4.0).contains(&h) => (0.0, secondary, chroma),
        h if (4.0..5.0).contains(&h) => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };

    let to_u8 = |value: f64| ((value + match_lightness).clamp(0.0, 1.0) * 255.0).round() as u8;

    Color::new(to_u8(red), to_u8(green), to_u8(blue))
}

/// Scheme slot names defined by `<a:clrScheme>`.
const CLR_SCHEME_SLOTS: &[&str] = &[
    "dk1", "dk2", "lt1", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6",
    "hlink", "folHlink",
];

/// `accent1`..`accent6` of a parsed theme palette, in order.
///
/// A chart series that states no fill of its own takes its colour from this
/// list. The result is empty unless all six are present, so a partial theme
/// leaves the renderer on its built-in palette rather than cycling through a
/// short list and repeating colours the file never named (issue #670).
pub(crate) fn theme_accent_palette(colors: &HashMap<String, Color>) -> Vec<Color> {
    let accents: Vec<Color> = (1..=6)
        .filter_map(|index| colors.get(&format!("accent{index}")).copied())
        .collect();
    if accents.len() == 6 {
        accents
    } else {
        Vec::new()
    }
}

/// Parse just the `<a:clrScheme>` palette out of a theme part
/// (`theme1.xml`) into a scheme-name → color map.
///
/// `srgbClr` uses `val`; `sysClr` uses the application-resolved `lastClr`.
/// The pptx parser keeps its own combined single-pass reader because it also
/// collects fonts and fill styles from the same document.
pub(crate) fn parse_theme_color_scheme(xml: &str) -> HashMap<String, Color> {
    let mut colors: HashMap<String, Color> = HashMap::new();
    let mut reader = Reader::from_str(xml);
    let mut current_slot: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                let name = std::str::from_utf8(local.as_ref()).unwrap_or("");
                if CLR_SCHEME_SLOTS.contains(&name) {
                    current_slot = Some(name.to_string());
                }
            }
            Ok(Event::Empty(ref e)) => {
                if let Some(ref slot) = current_slot {
                    let local = e.local_name();
                    let color = match local.as_ref() {
                        b"srgbClr" => get_attr_str(e, b"val").and_then(|hex| parse_hex_color(&hex)),
                        b"sysClr" => {
                            get_attr_str(e, b"lastClr").and_then(|hex| parse_hex_color(&hex))
                        }
                        _ => None,
                    };
                    if let Some(color) = color {
                        colors.insert(slot.clone(), color);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                let name = std::str::from_utf8(local.as_ref()).unwrap_or("");
                if current_slot.as_deref() == Some(name) {
                    current_slot = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    colors
}

/// The Latin typefaces a theme part's `<a:fontScheme>` declares. A slot the
/// theme leaves empty stays `None`, because DrawingML spells "inherit" as an
/// empty `typeface` and an empty family would select nothing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ThemeFontScheme {
    pub(crate) major_latin: Option<String>,
    pub(crate) minor_latin: Option<String>,
}

impl ThemeFontScheme {
    /// Resolve a DrawingML `typeface` attribute. The placeholders `+mj-lt`
    /// and `+mn-lt` name this scheme's major and minor Latin fonts; any
    /// other value is a literal family name.
    pub(crate) fn resolve_typeface(&self, typeface: &str) -> Option<String> {
        match typeface {
            "" => None,
            "+mj-lt" => self.major_latin.clone(),
            "+mn-lt" => self.minor_latin.clone(),
            literal => Some(literal.to_string()),
        }
    }

    /// The face a chart sets its strings in, given whatever its
    /// `c:chartSpace/c:txPr` declared.
    ///
    /// Chart text is body text, so a chart that names nothing lands where an
    /// explicit `+mn-lt` would: the theme's minor font. Without this the text
    /// fell through to the engine's own default, which is a serif face that
    /// appears nowhere else in the document (issue #668).
    pub(crate) fn resolve_chart_text_typeface(&self, declared: Option<&str>) -> Option<String> {
        match declared {
            Some(typeface) => self.resolve_typeface(typeface),
            None => self.minor_latin.clone(),
        }
    }
}

/// Parse just the `<a:fontScheme>` Latin typefaces out of a theme part
/// (`theme1.xml`).
///
/// The pptx parser keeps its own combined single-pass reader because it
/// also collects colors and fill styles from the same document.
pub(crate) fn parse_theme_font_scheme(xml: &str) -> ThemeFontScheme {
    let mut fonts = ThemeFontScheme::default();
    let mut reader = Reader::from_str(xml);
    let mut in_major: bool = false;
    let mut in_minor: bool = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"majorFont" => in_major = true,
                b"minorFont" => in_minor = true,
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"majorFont" => in_major = false,
                b"minorFont" => in_minor = false,
                _ => {}
            },
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"latin" => {
                let typeface: Option<String> =
                    get_attr_str(e, b"typeface").filter(|typeface| !typeface.is_empty());
                // Only a slot's first `<a:latin>` is its Latin face; the
                // script-specific `<a:font>` entries that follow are not.
                if in_major {
                    fonts.major_latin = fonts.major_latin.take().or(typeface);
                } else if in_minor {
                    fonts.minor_latin = fonts.minor_latin.take().or(typeface);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    fonts
}

#[cfg(test)]
#[path = "drawingml_tests.rs"]
mod tests;

/// Classify a face from `@pitchFamily` and `@panose` (issue #891).
///
/// `pitchFamily`'s high nibble is the family — 1 roman, 2 swiss, 3 modern —
/// and is checked first because it states the class outright. PANOSE digit 2
/// is the serif style, where 11..13 are the sans variants and 15 is rounded
/// sans (Calibri's `020F…`); 2..10 are the serif ones. Digit 1 must be 02,
/// Latin Text, for digit 2 to carry that meaning at all.
pub(crate) fn declared_font_class(
    panose: Option<&str>,
    pitch_family: Option<&str>,
) -> Option<DeclaredFontClass> {
    if let Some(value) = pitch_family.and_then(|raw| raw.trim().parse::<u8>().ok()) {
        match value >> 4 {
            1 => return Some(DeclaredFontClass::Serif),
            2 => return Some(DeclaredFontClass::SansSerif),
            3 => return Some(DeclaredFontClass::Monospace),
            _ => {}
        }
    }
    let panose = panose?.trim();
    if panose.len() < 4 || u8::from_str_radix(&panose[0..2], 16).ok()? != 2 {
        return None;
    }
    match u8::from_str_radix(&panose[2..4], 16).ok()? {
        11..=13 | 15 => Some(DeclaredFontClass::SansSerif),
        2..=10 => Some(DeclaredFontClass::Serif),
        _ => None,
    }
}

/// Collect every `<a:latin>` in a part that states its family class.
///
/// One sweep per part beats threading the attributes through each of the ten
/// places a run's typeface is read, and the answer is the same: a family's
/// class does not vary by where it is named.
pub(crate) fn scan_declared_font_classes(xml: &str, out: &mut HashMap<String, DeclaredFontClass>) {
    let mut reader: Reader<&[u8]> = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.local_name().as_ref() == b"latin" =>
            {
                let Some(typeface) = get_attr_str(e, b"typeface") else {
                    continue;
                };
                // `+mn-lt`/`+mj-lt` name the theme's slot, not a face.
                if typeface.starts_with('+') || typeface.trim().is_empty() {
                    continue;
                }
                if let Some(class) = declared_font_class(
                    get_attr_str(e, b"panose").as_deref(),
                    get_attr_str(e, b"pitchFamily").as_deref(),
                ) {
                    out.entry(typeface.trim().to_ascii_lowercase())
                        .or_insert(class);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

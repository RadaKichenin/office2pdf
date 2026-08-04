use std::collections::HashMap;

use super::*;
use crate::ir::Color;
use crate::parser::drawingml::ThemeFontScheme;

fn accent_theme() -> HashMap<String, Color> {
    // Office default theme slots a workbook actually ships in theme1.xml.
    HashMap::from([
        ("dk1".to_string(), Color::new(0, 0, 0)),
        ("lt1".to_string(), Color::new(255, 255, 255)),
        ("dk2".to_string(), Color::new(68, 84, 106)),
        ("lt2".to_string(), Color::new(231, 230, 230)),
        ("accent1".to_string(), Color::new(68, 114, 196)),
    ])
}

fn drawing_with_fill(color_markup: &str) -> String {
    format!(
        r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:sp>
      <xdr:spPr>
        <a:solidFill>{color_markup}</a:solidFill>
      </xdr:spPr>
      <xdr:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>hello</a:t></a:r></a:p>
      </xdr:txBody>
    </xdr:sp>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#
    )
}

fn fill_of(color_markup: &str, theme: &HashMap<String, Color>) -> Option<Color> {
    let boxes = parse_drawing_text_boxes(
        &drawing_with_fill(color_markup),
        theme,
        &ThemeFontScheme::default(),
    );
    assert_eq!(boxes.len(), 1, "fixture should yield one text box");
    boxes[0].fill
}

#[test]
fn scheme_accent_fill_resolves_against_workbook_theme() {
    let fill = fill_of(r#"<a:schemeClr val="accent1"/>"#, &accent_theme());
    assert_eq!(fill, Some(Color::new(68, 114, 196)));
}

#[test]
fn scheme_fill_applies_lum_transforms() {
    // "accent1, lighter 60%" as Excel emits it: lumMod 40% + lumOff 60%.
    let fill = fill_of(
        r#"<a:schemeClr val="accent1"><a:lumMod val="40000"/><a:lumOff val="60000"/></a:schemeClr>"#,
        &accent_theme(),
    );
    // Matches the pptx transform math (tint/shade in RGB, lum in HSL).
    let expected = crate::parser::drawingml::apply_color_transforms(
        Color::new(68, 114, 196),
        &[
            crate::parser::drawingml::ColorTransform::LumMod(0.4),
            crate::parser::drawingml::ColorTransform::LumOff(0.6),
        ],
    );
    assert_eq!(fill, Some(expected));
}

#[test]
fn srgb_fill_with_shade_still_darkens() {
    // The old hand-rolled Empty(<a:shade>) path must keep working through the
    // shared parser.
    let fill = fill_of(
        r#"<a:srgbClr val="C86432"><a:shade val="50000"/></a:srgbClr>"#,
        &accent_theme(),
    );
    // 50% of #C86432's light, re-encoded (issue #667). Halving the sRGB bytes
    // would give (100, 50, 25).
    assert_eq!(fill, Some(Color::new(146, 71, 34)));
}

#[test]
fn background_scheme_names_use_spreadsheet_aliases() {
    // xlsx has no clrMap part; bg1/tx1 must map onto lt1/dk1.
    let theme = accent_theme();
    assert_eq!(
        fill_of(r#"<a:schemeClr val="bg1"/>"#, &theme),
        Some(Color::new(255, 255, 255))
    );
    assert_eq!(
        fill_of(r#"<a:schemeClr val="tx1"/>"#, &theme),
        Some(Color::new(0, 0, 0))
    );
    assert_eq!(
        fill_of(r#"<a:schemeClr val="bg2"/>"#, &theme),
        Some(Color::new(231, 230, 230))
    );
}

#[test]
fn scheme_fill_falls_back_to_light_dark_without_theme() {
    // Workbooks without a theme part keep the historical fallback.
    let empty = HashMap::new();
    assert_eq!(
        fill_of(r#"<a:schemeClr val="bg1"/>"#, &empty),
        Some(Color::new(255, 255, 255))
    );
    assert_eq!(
        fill_of(r#"<a:schemeClr val="tx1"/>"#, &empty),
        Some(Color::new(0, 0, 0))
    );
    assert_eq!(fill_of(r#"<a:schemeClr val="accent1"/>"#, &empty), None);
}

#[test]
fn theme_color_scheme_parses_from_theme_xml() {
    let theme_xml = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;
    let colors = crate::parser::drawingml::parse_theme_color_scheme(theme_xml);
    assert_eq!(colors.get("dk1"), Some(&Color::new(0, 0, 0)));
    assert_eq!(colors.get("lt1"), Some(&Color::new(255, 255, 255)));
    assert_eq!(colors.get("dk2"), Some(&Color::new(0x44, 0x54, 0x6A)));
    assert_eq!(colors.get("accent1"), Some(&Color::new(0x44, 0x72, 0xC4)));
    assert_eq!(colors.get("hlink"), Some(&Color::new(0x05, 0x63, 0xC1)));
    assert_eq!(colors.get("accent2"), None);
}

fn office_theme_fonts() -> ThemeFontScheme {
    // The font scheme an Excel-saved workbook ships in theme1.xml.
    ThemeFontScheme {
        major_latin: Some("Calibri Light".to_string()),
        minor_latin: Some("Calibri".to_string()),
    }
}

fn drawing_with_run_properties(run_properties: &str) -> String {
    format!(
        r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <xdr:twoCellAnchor>
    <xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>
    <xdr:to><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>
    <xdr:sp>
      <xdr:txBody>
        <a:bodyPr/>
        <a:p><a:r>{run_properties}<a:t>accent1</a:t></a:r></a:p>
      </xdr:txBody>
    </xdr:sp>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#
    )
}

fn font_of(run_properties: &str, fonts: &ThemeFontScheme) -> Option<String> {
    let boxes = parse_drawing_text_boxes(
        &drawing_with_run_properties(run_properties),
        &HashMap::new(),
        fonts,
    );
    assert_eq!(boxes.len(), 1, "fixture should yield one text box");
    boxes[0].paragraphs[0].runs[0].style.font_family.clone()
}

#[test]
fn drawing_run_without_a_typeface_inherits_the_theme_minor_font() {
    // Excel writes shape labels as `<a:rPr lang="en-US" sz="1100"/>` with no
    // typeface at all; DrawingML resolves that to the theme's minor Latin
    // font, not to the renderer's serif default (issue #461).
    assert_eq!(
        font_of(r#"<a:rPr lang="en-US" sz="1100"/>"#, &office_theme_fonts()),
        Some("Calibri".to_string())
    );
}

#[test]
fn drawing_run_without_run_properties_inherits_the_theme_minor_font() {
    assert_eq!(
        font_of("", &office_theme_fonts()),
        Some("Calibri".to_string())
    );
}

#[test]
fn drawing_run_uses_its_explicit_latin_typeface() {
    assert_eq!(
        font_of(
            r#"<a:rPr lang="en-US" sz="1100"><a:latin typeface="Georgia"/></a:rPr>"#,
            &office_theme_fonts()
        ),
        Some("Georgia".to_string())
    );
}

#[test]
fn drawing_run_resolves_theme_typeface_placeholders() {
    // `+mj-lt` and `+mn-lt` name the theme's major and minor Latin fonts.
    assert_eq!(
        font_of(
            r#"<a:rPr lang="en-US"><a:latin typeface="+mj-lt"/></a:rPr>"#,
            &office_theme_fonts()
        ),
        Some("Calibri Light".to_string())
    );
    assert_eq!(
        font_of(
            r#"<a:rPr lang="en-US"><a:latin typeface="+mn-lt"/></a:rPr>"#,
            &office_theme_fonts()
        ),
        Some("Calibri".to_string())
    );
}

#[test]
fn drawing_run_font_stays_unset_without_a_theme_font_scheme() {
    // A workbook with no readable theme part leaves font selection to the
    // renderer's existing fallback rather than inventing a family.
    assert_eq!(
        font_of(r#"<a:rPr lang="en-US"/>"#, &ThemeFontScheme::default()),
        None
    );
}

#[test]
fn theme_font_scheme_parses_from_theme_xml() {
    let theme_xml = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;
    let fonts = crate::parser::drawingml::parse_theme_font_scheme(theme_xml);
    assert_eq!(fonts.major_latin.as_deref(), Some("Calibri Light"));
    assert_eq!(fonts.minor_latin.as_deref(), Some("Calibri"));
}

#[test]
fn theme_font_scheme_ignores_empty_typefaces() {
    // Themes spell "inherit" as an empty typeface; it must not become a font.
    let theme_xml = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface=""/></a:majorFont>
      <a:minorFont><a:latin typeface=""/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;
    let fonts = crate::parser::drawingml::parse_theme_font_scheme(theme_xml);
    assert_eq!(fonts, ThemeFontScheme::default());
}

/// `(font_size, bold, italic)` of the single run the fixture produces.
fn run_emphasis(run_properties: &str) -> (Option<f64>, Option<bool>, Option<bool>) {
    let boxes = parse_drawing_text_boxes(
        &drawing_with_run_properties(run_properties),
        &HashMap::new(),
        &ThemeFontScheme::default(),
    );
    assert_eq!(boxes.len(), 1, "fixture should yield one text box");
    let style = &boxes[0].paragraphs[0].runs[0].style;
    (style.font_size, style.bold, style.italic)
}

#[test]
fn self_closing_run_properties_still_carry_size_and_emphasis() {
    // Excel writes shape-label run properties as a self-closing element,
    // which quick-xml reports as `Event::Empty`; reading them only on
    // `Event::Start` dropped every attribute (issue #466).
    assert_eq!(
        run_emphasis(r#"<a:rPr lang="en-US" sz="1400" b="1" i="1"/>"#),
        (Some(14.0), Some(true), Some(true))
    );
}

#[test]
fn run_properties_with_children_keep_working() {
    // Triangulation: the same attributes on an element that has children
    // must resolve identically, so the fix cannot special-case one spelling.
    assert_eq!(
        run_emphasis(
            r#"<a:rPr lang="en-US" sz="1400" b="1" i="1"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:rPr>"#
        ),
        (Some(14.0), Some(true), Some(true))
    );
}

#[test]
fn run_properties_omit_emphasis_when_disabled() {
    // `b="0"`/`i="0"` mean "not bold"/"not italic"; they must not set the flag.
    assert_eq!(
        run_emphasis(r#"<a:rPr lang="en-US" sz="900" b="0" i="0"/>"#),
        (Some(9.0), None, None)
    );
}

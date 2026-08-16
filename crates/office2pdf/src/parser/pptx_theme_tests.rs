use super::*;

// ── Theme test helpers ────────────────────────────────────────────

pub(super) fn make_theme_xml(
    colors: &[(&str, &str)],
    major_font: &str,
    minor_font: &str,
) -> String {
    let mut color_xml = String::new();
    for (name, hex) in colors {
        if *name == "dk1" || *name == "lt1" {
            color_xml.push_str(&format!(
                r#"<a:{name}><a:sysClr val="windowText" lastClr="{hex}"/></a:{name}>"#
            ));
        } else {
            color_xml.push_str(&format!(r#"<a:{name}><a:srgbClr val="{hex}"/></a:{name}>"#));
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="Test">{color_xml}</a:clrScheme><a:fontScheme name="Test"><a:majorFont><a:latin typeface="{major_font}"/></a:majorFont><a:minorFont><a:latin typeface="{minor_font}"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#
    )
}

pub(super) fn standard_theme_colors() -> Vec<(&'static str, &'static str)> {
    vec![
        ("dk1", "000000"),
        ("dk2", "1F4D78"),
        ("lt1", "FFFFFF"),
        ("lt2", "E7E6E6"),
        ("accent1", "4472C4"),
        ("accent2", "ED7D31"),
        ("accent3", "A5A5A5"),
        ("accent4", "FFC000"),
        ("accent5", "5B9BD5"),
        ("accent6", "70AD47"),
        ("hlink", "0563C1"),
        ("folHlink", "954F72"),
    ]
}

pub(super) fn build_test_pptx_with_theme(
    slide_cx_emu: i64,
    slide_cy_emu: i64,
    slide_xmls: &[String],
    theme_xml: &str,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = FileOptions::default();

    let mut ct = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    ct.push_str(r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#);
    ct.push_str(r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#);
    ct.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    for i in 0..slide_xmls.len() {
        ct.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
            i + 1
        ));
    }
    ct.push_str("</Types>");
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(ct.as_bytes()).unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
    )
    .unwrap();

    let mut pres = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldSz cx="{}" cy="{}"/><p:sldIdLst>"#,
        slide_cx_emu, slide_cy_emu
    );
    for i in 0..slide_xmls.len() {
        pres.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            256 + i,
            2 + i
        ));
    }
    pres.push_str("</p:sldIdLst></p:presentation>");
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(pres.as_bytes()).unwrap();

    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    pres_rels.push_str(
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#,
    );
    for i in 0..slide_xmls.len() {
        pres_rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            2 + i,
            1 + i
        ));
    }
    pres_rels.push_str("</Relationships>");
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(pres_rels.as_bytes()).unwrap();

    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(theme_xml.as_bytes()).unwrap();

    for (i, slide_xml) in slide_xmls.iter().enumerate() {
        zip.start_file(format!("ppt/slides/slide{}.xml", i + 1), opts)
            .unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();
    }

    zip.finish().unwrap().into_inner()
}

pub(super) fn build_test_pptx_with_layout_master(
    slide_cx_emu: i64,
    slide_cy_emu: i64,
    slide_xml: &str,
    layout_xml: &str,
    master_xml: &str,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = FileOptions::default();

    let ct = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/></Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(ct.as_bytes()).unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
    )
    .unwrap();

    let pres = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldSz cx="{slide_cx_emu}" cy="{slide_cy_emu}"/><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst></p:presentation>"#,
    );
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(pres.as_bytes()).unwrap();

    let pres_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/></Relationships>"#;
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(pres_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#;
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slideLayouts/slideLayout1.xml", opts)
        .unwrap();
    zip.write_all(layout_xml.as_bytes()).unwrap();

    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", opts)
        .unwrap();
    zip.write_all(layout_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slideMasters/slideMaster1.xml", opts)
        .unwrap();
    zip.write_all(master_xml.as_bytes()).unwrap();

    zip.finish().unwrap().into_inner()
}

pub(super) fn build_test_pptx_with_theme_layout_master(
    slide_cx_emu: i64,
    slide_cy_emu: i64,
    slide_xml: &str,
    layout_xml: &str,
    master_xml: &str,
    theme_xml: &str,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = FileOptions::default();

    let ct = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/></Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(ct.as_bytes()).unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
    )
    .unwrap();

    let pres = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldSz cx="{slide_cx_emu}" cy="{slide_cy_emu}"/><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst></p:presentation>"#,
    );
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(pres.as_bytes()).unwrap();

    let pres_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#;
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(pres_rels.as_bytes()).unwrap();

    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(theme_xml.as_bytes()).unwrap();

    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#;
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slideLayouts/slideLayout1.xml", opts)
        .unwrap();
    zip.write_all(layout_xml.as_bytes()).unwrap();

    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", opts)
        .unwrap();
    zip.write_all(layout_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slideMasters/slideMaster1.xml", opts)
        .unwrap();
    zip.write_all(master_xml.as_bytes()).unwrap();

    zip.finish().unwrap().into_inner()
}

pub(super) fn build_test_pptx_with_layout_master_multi_slide(
    slide_cx_emu: i64,
    slide_cy_emu: i64,
    slide_xmls: &[String],
    layout_xml: &str,
    master_xml: &str,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = FileOptions::default();

    let mut ct = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    ct.push_str(r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#);
    ct.push_str(r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#);
    ct.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    for i in 0..slide_xmls.len() {
        ct.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
            i + 1
        ));
    }
    ct.push_str(r#"<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>"#);
    ct.push_str(r#"<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>"#);
    ct.push_str("</Types>");
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(ct.as_bytes()).unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#,
    )
    .unwrap();

    let mut pres = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldSz cx="{slide_cx_emu}" cy="{slide_cy_emu}"/><p:sldIdLst>"#,
    );
    for i in 0..slide_xmls.len() {
        pres.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            256 + i,
            2 + i
        ));
    }
    pres.push_str("</p:sldIdLst></p:presentation>");
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(pres.as_bytes()).unwrap();

    let mut pres_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    pres_rels.push_str(
        r#"<Relationship Id="rId100" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#,
    );
    for i in 0..slide_xmls.len() {
        pres_rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
            2 + i,
            1 + i
        ));
    }
    pres_rels.push_str("</Relationships>");
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(pres_rels.as_bytes()).unwrap();

    for (i, slide_xml) in slide_xmls.iter().enumerate() {
        let slide_num = i + 1;
        zip.start_file(format!("ppt/slides/slide{slide_num}.xml"), opts)
            .unwrap();
        zip.write_all(slide_xml.as_bytes()).unwrap();

        let slide_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#;
        zip.start_file(format!("ppt/slides/_rels/slide{slide_num}.xml.rels"), opts)
            .unwrap();
        zip.write_all(slide_rels.as_bytes()).unwrap();
    }

    zip.start_file("ppt/slideLayouts/slideLayout1.xml", opts)
        .unwrap();
    zip.write_all(layout_xml.as_bytes()).unwrap();

    let layout_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", opts)
        .unwrap();
    zip.write_all(layout_rels.as_bytes()).unwrap();

    zip.start_file("ppt/slideMasters/slideMaster1.xml", opts)
        .unwrap();
    zip.write_all(master_xml.as_bytes()).unwrap();

    zip.finish().unwrap().into_inner()
}

// ── Theme unit tests ──────────────────────────────────────────────

#[test]
fn test_parse_theme_xml_colors() {
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let theme = parse_theme_xml(&theme_xml);

    assert_eq!(theme.colors.len(), 12);
    assert_eq!(theme.colors["dk1"], Color::new(0, 0, 0));
    assert_eq!(theme.colors["lt1"], Color::new(255, 255, 255));
    assert_eq!(theme.colors["accent1"], Color::new(0x44, 0x72, 0xC4));
    assert_eq!(theme.colors["accent2"], Color::new(0xED, 0x7D, 0x31));
    assert_eq!(theme.colors["hlink"], Color::new(0x05, 0x63, 0xC1));
    assert_eq!(theme.colors["folHlink"], Color::new(0x95, 0x4F, 0x72));
}

#[test]
fn test_parse_theme_xml_fonts() {
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let theme = parse_theme_xml(&theme_xml);

    assert_eq!(theme.major_font, Some("Calibri Light".to_string()));
    assert_eq!(theme.minor_font, Some("Calibri".to_string()));
}

#[test]
fn test_parse_theme_xml_sys_clr() {
    let theme_xml = make_theme_xml(&[("dk1", "111111"), ("lt1", "EEEEEE")], "Arial", "Arial");
    let theme = parse_theme_xml(&theme_xml);

    assert_eq!(theme.colors["dk1"], Color::new(0x11, 0x11, 0x11));
    assert_eq!(theme.colors["lt1"], Color::new(0xEE, 0xEE, 0xEE));
}

#[test]
fn test_parse_theme_xml_empty() {
    let theme = parse_theme_xml("");
    assert!(theme.colors.is_empty());
    assert!(theme.major_font.is_none());
    assert!(theme.minor_font.is_none());
}

#[test]
fn test_resolve_theme_font_major() {
    let theme = ThemeData {
        major_font: Some("Calibri Light".to_string()),
        minor_font: Some("Calibri".to_string()),
        ..ThemeData::default()
    };
    assert_eq!(resolve_theme_font("+mj-lt", &theme), "Calibri Light");
}

#[test]
fn test_resolve_theme_font_minor() {
    let theme = ThemeData {
        major_font: Some("Calibri Light".to_string()),
        minor_font: Some("Calibri".to_string()),
        ..ThemeData::default()
    };
    assert_eq!(resolve_theme_font("+mn-lt", &theme), "Calibri");
}

#[test]
fn test_resolve_theme_font_explicit() {
    let theme = ThemeData::default();
    assert_eq!(resolve_theme_font("Arial", &theme), "Arial");
}

// ── Theme integration tests (full PPTX parsing) ───────────────────

#[test]
fn test_scheme_color_in_shape_fill() {
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    assert_eq!(page.elements.len(), 1);
    let shape = get_shape(&page.elements[0]);
    assert_eq!(shape.fill, Some(Color::new(0x44, 0x72, 0xC4)));
}

#[test]
fn test_scheme_color_in_line_stroke() {
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:schemeClr val="dk1"/></a:solidFill></a:ln></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    let stroke = shape.stroke.as_ref().expect("Expected stroke");
    assert_eq!(stroke.color, Color::new(0, 0, 0));
}

#[test]
fn test_scheme_color_in_text_run() {
    let runs_xml = r#"<a:r><a:rPr><a:solidFill><a:schemeClr val="accent2"/></a:solidFill></a:rPr><a:t>Themed text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 2_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].text, "Themed text");
    assert_eq!(para.runs[0].style.color, Some(Color::new(0xED, 0x7D, 0x31)));
}

#[test]
fn test_hyperlink_run_inherits_theme_color_underline_and_keeps_boundary() {
    let runs_xml = concat!(
        r#"<a:r><a:rPr lang="en-US"/><a:t>Plain text, then </a:t></a:r>"#,
        r#"<a:r><a:rPr lang="en-US"><a:hlinkClick r:id="rId2"/></a:rPr><a:t>Click here</a:t></a:r>"#,
        r#"<a:r><a:rPr lang="en-US"/><a:t> after</a:t></a:r>"#,
    );
    let shape = make_formatted_text_box(0, 0, 4_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let theme_xml = make_theme_xml(&[("hlink", "778BA2")], "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs.len(), 3);
    assert_eq!(para.runs[0].text, "Plain text, then ");
    assert_eq!(para.runs[1].text, "Click here");
    assert_eq!(para.runs[1].style.color, Some(Color::new(0x77, 0x8B, 0xA2)));
    assert_eq!(para.runs[1].style.underline, Some(true));
    assert_eq!(para.runs[2].text, " after");
}

#[test]
fn test_hyperlink_run_keeps_explicit_color_and_underline_none() {
    let runs_xml = concat!(
        r#"<a:r><a:rPr lang="en-US" u="none">"#,
        r#"<a:solidFill><a:srgbClr val="CC3300"/></a:solidFill>"#,
        r#"<a:hlinkClick r:id="rId2"/></a:rPr><a:t>Styled link</a:t></a:r>"#,
    );
    let shape = make_formatted_text_box(0, 0, 2_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let theme_xml = make_theme_xml(&[("hlink", "778BA2")], "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].style.color, Some(Color::new(0xCC, 0x33, 0x00)));
    assert_eq!(para.runs[0].style.underline, Some(false));
}

#[test]
fn test_theme_major_font_in_text() {
    let runs_xml = r#"<a:r><a:rPr><a:latin typeface="+mj-lt"/></a:rPr><a:t>Heading</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 2_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].text, "Heading");
    assert_eq!(
        para.runs[0].style.font_family,
        Some("Calibri Light".to_string())
    );
}

#[test]
fn test_theme_minor_font_in_text() {
    let runs_xml = r#"<a:r><a:rPr><a:latin typeface="+mn-lt"/></a:rPr><a:t>Body text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 2_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].text, "Body text");
    assert_eq!(para.runs[0].style.font_family, Some("Calibri".to_string()));
}

#[test]
fn test_pptx_with_theme_colors_and_fonts_combined() {
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="2000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent5"/></a:solidFill></p:spPr></p:sp>"#;
    let runs_xml = r#"<a:r><a:rPr b="1" sz="2400"><a:solidFill><a:schemeClr val="dk2"/></a:solidFill><a:latin typeface="+mj-lt"/></a:rPr><a:t>Theme styled</a:t></a:r>"#;
    let text_box = make_formatted_text_box(3_000_000, 0, 4_000_000, 1_000_000, runs_xml);
    let slide = make_slide_xml(&[shape_xml.to_string(), text_box]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    assert_eq!(page.elements.len(), 2);

    let shape = get_shape(&page.elements[0]);
    assert_eq!(shape.fill, Some(Color::new(0x5B, 0x9B, 0xD5)));

    let blocks = text_box_blocks(&page.elements[1]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    let run = &para.runs[0];
    assert_eq!(run.text, "Theme styled");
    assert_eq!(run.style.color, Some(Color::new(0x1F, 0x4D, 0x78)));
    assert_eq!(run.style.font_family, Some("Calibri Light".to_string()));
    assert_eq!(run.style.bold, Some(true));
    assert_eq!(run.style.font_size, Some(24.0));
}

#[test]
fn test_no_theme_scheme_color_ignored() {
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    assert!(shape.fill.is_none());
}

#[test]
fn test_scheme_color_tint_blends_toward_white() {
    // accent3=#A5A5A5 with tint 50% → each channel: 255 - (255-165)*0.5 = 210 = 0xD2
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent3"><a:tint val="50000"/></a:schemeClr></a:solidFill></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    assert_eq!(shape.fill, Some(Color::new(0xD2, 0xD2, 0xD2)));
}

#[test]
fn test_scheme_color_shade_blends_toward_black() {
    // accent1=#4472C4 with shade 50% → #2F528F (issue #667: the scale is in
    // linear light, so it is not each byte * 0.5)
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr></a:solidFill></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let theme_xml = make_theme_xml(&standard_theme_colors(), "Calibri Light", "Calibri");
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    // accent1 = (0x44, 0x72, 0xC4) = (68, 114, 196)
    // shade 50% in linear light: (47, 82, 143) = (0x2F, 0x52, 0x8F)
    assert_eq!(shape.fill, Some(Color::new(0x2F, 0x52, 0x8F)));
}

#[test]
fn test_scheme_color_lum_mod_applies_to_shape_fill() {
    let shape_xml = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent1"><a:lumMod val="50000"/></a:schemeClr></a:solidFill></p:spPr></p:sp>"#;
    let slide = make_slide_xml(&[shape_xml.to_string()]);
    let theme_xml = make_theme_xml(
        &[("dk1", "000000"), ("lt1", "FFFFFF"), ("accent1", "808080")],
        "Calibri Light",
        "Calibri",
    );
    let data = build_test_pptx_with_theme(SLIDE_CX, SLIDE_CY, &[slide], &theme_xml);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    assert_eq!(shape.fill, Some(Color::new(0x40, 0x40, 0x40)));
}

#[test]
fn test_layout_shape_uses_master_color_map_with_luminance_offset() {
    let slide_xml = make_empty_slide_xml();
    let layout_shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="tx1"><a:lumOff val="50000"/></a:schemeClr></a:solidFill><a:ln w="6350"><a:noFill/></a:ln></p:spPr></p:sp>"#;
    let layout_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{layout_shape}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#
    );
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt1" tx2="dk1" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#;
    let theme_xml = make_theme_xml(
        &[
            ("dk1", "000000"),
            ("dk2", "222222"),
            ("lt1", "FFFFFF"),
            ("lt2", "EEEEEE"),
            ("accent1", "4472C4"),
            ("accent2", "ED7D31"),
            ("accent3", "A5A5A5"),
            ("accent4", "FFC000"),
            ("accent5", "5B9BD5"),
            ("accent6", "70AD47"),
            ("hlink", "0563C1"),
            ("folHlink", "954F72"),
        ],
        "Calibri Light",
        "Calibri",
    );
    let data = build_test_pptx_with_theme_layout_master(
        SLIDE_CX,
        SLIDE_CY,
        &slide_xml,
        &layout_xml,
        master_xml,
        &theme_xml,
    );

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let shape = get_shape(&page.elements[0]);
    assert_eq!(shape.fill, Some(Color::new(0x80, 0x80, 0x80)));
}

#[test]
fn test_parse_theme_line_style_widths() {
    // Theme lnStyleLst widths back <a:lnRef idx="N"> outline resolution
    // (issue #318): idx=1/2/3 map to the 1st/2nd/3rd entry's EMU width.
    let theme_xml = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="X"><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme>
    <a:fontScheme name="X"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="X">
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;
    let theme = parse_theme_xml(theme_xml);
    let widths: Vec<i64> = theme
        .line_styles
        .iter()
        .map(|style| style.width_emu)
        .collect();
    assert_eq!(widths, vec![6350, 12700, 19050]);
}

/// Each `lnStyleLst` entry keeps its own corner join, and an entry naming none
/// stays unstated so the DrawingML round default decides (issue #1090).
///
/// PowerPoint's stock themes write `<a:miter lim="800000"/>` on every entry,
/// so a deck built on one must keep mitering while `customGeo.pptx`'s
/// join-less theme rounds.
#[test]
fn test_parse_theme_line_style_joins() {
    let theme_xml = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="X"><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme>
    <a:fontScheme name="X"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="X">
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:miter lim="800000"/></a:ln>
        <a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:bevel/></a:ln>
        <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>
      </a:lnStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;
    let theme = parse_theme_xml(theme_xml);
    let joins: Vec<Option<LineJoin>> = theme.line_styles.iter().map(|style| style.join).collect();
    assert_eq!(
        joins,
        vec![Some(LineJoin::Miter), Some(LineJoin::Bevel), None]
    );
}

/// A `<a:ln>` nested inside a `lnStyleLst` entry — the arrowhead and
/// compound-line sub-elements can carry one — must not be mistaken for a
/// fourth entry, which would shift every `<a:lnRef idx>` past its style.
#[test]
fn test_theme_line_styles_ignore_nested_lines() {
    let theme_xml = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="X"><a:dk1><a:srgbClr val="000000"/></a:dk1></a:clrScheme>
    <a:fontScheme name="X"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="X">
      <a:lnStyleLst>
        <a:ln w="6350"><a:miter lim="800000"/><a:extLst><a:ext><a:ln w="99999"><a:bevel/></a:ln></a:ext></a:extLst></a:ln>
        <a:ln w="12700"><a:round/></a:ln>
      </a:lnStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;
    let theme = parse_theme_xml(theme_xml);
    assert_eq!(
        theme
            .line_styles
            .iter()
            .map(|style| (style.width_emu, style.join))
            .collect::<Vec<_>>(),
        vec![
            (6350, Some(LineJoin::Miter)),
            (12700, Some(LineJoin::Round))
        ]
    );
}

// ── `<a:effectRef>` resolution (issue #740) ────────────────────────────

/// Theme carrying three distinct effect styles. The first two are the shadows
/// PowerPoint's stock themes ship; the third states its color as `phClr` so
/// the placeholder path is exercised too.
fn theme_xml_with_effect_styles() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="T"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="T"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme><a:fmtScheme name="T"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="38000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="76200" dist="38100" dir="10800000" rotWithShape="0"><a:srgbClr val="FF0000"><a:alpha val="60000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="12700" dist="12700" dir="2700000" rotWithShape="0"><a:schemeClr val="phClr"/></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#
}

/// `idx="1"` picks the first effect style. Values are the ones stock themes
/// carry: a 40000 EMU blur, a 20000 EMU drop straight down, 38% black.
#[test]
fn effect_ref_resolves_the_first_theme_effect_style() {
    let theme = parse_theme_xml(theme_xml_with_effect_styles());
    let shadow = resolve_effect_ref(1, None, &theme, &default_color_map())
        .expect("idx 1 resolves to the first effect style");

    assert!((shadow.blur_radius - emu_to_pt(40000)).abs() < 1e-9);
    assert!((shadow.distance - emu_to_pt(20000)).abs() < 1e-9);
    assert!(
        (shadow.direction - 90.0).abs() < 1e-9,
        "5400000 is straight down"
    );
    assert_eq!(shadow.color, Color::new(0, 0, 0));
    assert!((shadow.opacity - 0.38).abs() < 1e-6);
}

/// Triangulation: a different index resolves to that entry's own values, so
/// the resolution cannot be a fixed shadow.
#[test]
fn effect_ref_resolves_each_index_to_its_own_entry() {
    let theme = parse_theme_xml(theme_xml_with_effect_styles());
    let shadow = resolve_effect_ref(2, None, &theme, &default_color_map())
        .expect("idx 2 resolves to the second effect style");

    assert!((shadow.blur_radius - emu_to_pt(76200)).abs() < 1e-9);
    assert!((shadow.distance - emu_to_pt(38100)).abs() < 1e-9);
    assert!((shadow.direction - 180.0).abs() < 1e-9);
    assert_eq!(shadow.color, Color::new(0xFF, 0, 0));
    assert!((shadow.opacity - 0.60).abs() < 1e-6);
}

/// The reference's own color binds `phClr` inside the entry, matching how
/// `<p:bgRef>` binds it for a fill entry.
#[test]
fn effect_ref_binds_its_color_to_the_placeholder() {
    let theme = parse_theme_xml(theme_xml_with_effect_styles());
    let reference_color = Color::new(0x44, 0x72, 0xC4);
    let shadow = resolve_effect_ref(3, Some(reference_color), &theme, &default_color_map())
        .expect("idx 3 resolves to the third effect style");

    assert_eq!(shadow.color, reference_color);
}

/// `idx="0"` means no effect, and an index past the list resolves to nothing
/// rather than to the last entry.
#[test]
fn effect_ref_index_zero_and_out_of_range_resolve_to_nothing() {
    let theme = parse_theme_xml(theme_xml_with_effect_styles());
    assert!(resolve_effect_ref(0, None, &theme, &default_color_map()).is_none());
    assert!(resolve_effect_ref(4, None, &theme, &default_color_map()).is_none());
}

/// A shape whose only effect is the `<p:style>` reference gets the theme's
/// shadow — the end-to-end path issue #740 reported as broken.
fn slide_with_styled_shape(sp_pr_effect: &str, style_body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Panel"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom>{sp_pr_effect}</p:spPr><p:style>{style_body}</p:style><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
    )
}

fn first_shape_shadow(slide: &str) -> Option<Shadow> {
    let data = build_test_pptx_with_theme_layout_master(
        9144000,
        6858000,
        slide,
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sldLayout>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:sldMaster>"#,
        theme_xml_with_effect_styles(),
    );
    let (doc, _warnings) = PptxParser.parse(&data, &ConvertOptions::default()).unwrap();
    doc.pages
        .iter()
        .flat_map(|page| match page {
            Page::Fixed(fixed) => fixed.elements.iter().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .find_map(|element| match element.kind {
            FixedElementKind::Shape(ref shape) => shape.shadow.clone(),
            _ => None,
        })
}

#[test]
fn styled_shape_inherits_the_theme_shadow_through_effect_ref() {
    let slide = slide_with_styled_shape(
        "",
        r#"<a:lnRef idx="2"><a:schemeClr val="accent1"/></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="1"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="lt1"/></a:fontRef>"#,
    );

    let shadow = first_shape_shadow(&slide).expect("the theme effect reaches the shape");
    assert!((shadow.blur_radius - emu_to_pt(40000)).abs() < 1e-9);
    assert!((shadow.distance - emu_to_pt(20000)).abs() < 1e-9);
    assert!((shadow.opacity - 0.38).abs() < 1e-6);
}

/// An `<a:effectLst/>` of the shape's own means "no effect" and must win over
/// the reference — otherwise every shape that switches its theme shadow off
/// would get it back.
#[test]
fn an_empty_effect_list_on_the_shape_suppresses_the_reference() {
    let slide = slide_with_styled_shape(
        "<a:effectLst/>",
        r#"<a:effectRef idx="1"><a:schemeClr val="accent1"/></a:effectRef>"#,
    );

    assert!(first_shape_shadow(&slide).is_none());
}

/// The shape's own shadow wins over the reference when both are present.
#[test]
fn a_direct_shadow_wins_over_the_effect_ref() {
    let slide = slide_with_styled_shape(
        r#"<a:effectLst><a:outerShdw blurRad="25400" dist="25400" dir="0"><a:srgbClr val="00FF00"/></a:outerShdw></a:effectLst>"#,
        r#"<a:effectRef idx="1"><a:schemeClr val="accent1"/></a:effectRef>"#,
    );

    let shadow = first_shape_shadow(&slide).expect("the direct shadow survives");
    assert_eq!(shadow.color, Color::new(0, 0xFF, 0));
}

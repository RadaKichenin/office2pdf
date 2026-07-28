use super::*;

#[test]
fn test_heading1_style_applies_defaults() {
    let h1_style = docx_rs::Style::new("Heading1", docx_rs::StyleType::Paragraph)
        .name("Heading 1")
        .outline_lvl(0);

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Title"))
                .style("Heading1"),
        ],
        vec![h1_style],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert_eq!(run.style.font_size, Some(24.0));
    assert_eq!(run.style.bold, Some(true));
}

#[test]
fn test_heading2_style_applies_defaults() {
    let h2_style = docx_rs::Style::new("Heading2", docx_rs::StyleType::Paragraph)
        .name("Heading 2")
        .outline_lvl(1);

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Subtitle"))
                .style("Heading2"),
        ],
        vec![h2_style],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert_eq!(run.style.font_size, Some(20.0));
    assert_eq!(run.style.bold, Some(true));
}

#[test]
fn test_heading3_through_6_defaults() {
    let expected: Vec<(usize, &str, f64)> = vec![
        (2, "Heading3", 16.0),
        (3, "Heading4", 14.0),
        (4, "Heading5", 12.0),
        (5, "Heading6", 11.0),
    ];

    for (outline_lvl, style_id, expected_size) in expected {
        let style = docx_rs::Style::new(style_id, docx_rs::StyleType::Paragraph)
            .name(format!("Heading {}", outline_lvl + 1))
            .outline_lvl(outline_lvl);

        let data = build_docx_bytes_with_styles(
            vec![
                docx_rs::Paragraph::new()
                    .add_run(docx_rs::Run::new().add_text("Heading text"))
                    .style(style_id),
            ],
            vec![style],
        );

        let parser = DocxParser;
        let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
        let run = first_run(&doc);

        assert_eq!(
            run.style.font_size,
            Some(expected_size),
            "Heading {} should have size {expected_size}pt",
            outline_lvl + 1
        );
        assert_eq!(
            run.style.bold,
            Some(true),
            "Heading {} should be bold",
            outline_lvl + 1
        );
    }
}

#[test]
fn test_style_with_explicit_formatting() {
    let custom = docx_rs::Style::new("CustomStyle", docx_rs::StyleType::Paragraph)
        .name("Custom Style")
        .size(36)
        .bold();

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Custom styled"))
                .style("CustomStyle"),
        ],
        vec![custom],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert_eq!(run.style.font_size, Some(18.0));
    assert_eq!(run.style.bold, Some(true));
}

#[test]
fn test_explicit_run_formatting_overrides_style() {
    let h1_style = docx_rs::Style::new("Heading1", docx_rs::StyleType::Paragraph)
        .name("Heading 1")
        .outline_lvl(0);

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Small heading").size(20))
                .style("Heading1"),
        ],
        vec![h1_style],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert_eq!(run.style.font_size, Some(10.0));
    assert_eq!(run.style.bold, Some(true));
}

#[test]
fn test_style_alignment_applied_to_paragraph() {
    let centered = docx_rs::Style::new("CenteredStyle", docx_rs::StyleType::Paragraph)
        .name("Centered")
        .align(docx_rs::AlignmentType::Center);

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Centered paragraph"))
                .style("CenteredStyle"),
        ],
        vec![centered],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let para = first_paragraph(&doc);

    assert_eq!(para.style.alignment, Some(Alignment::Center));
}

#[test]
fn test_normal_style_no_heading_defaults() {
    let normal = docx_rs::Style::new("Normal", docx_rs::StyleType::Paragraph).name("Normal");

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Normal text"))
                .style("Normal"),
        ],
        vec![normal],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert!(run.style.font_size.is_none());
    assert!(run.style.bold.is_none());
}

#[test]
fn test_heading_with_mixed_paragraphs() {
    let h1 = docx_rs::Style::new("Heading1", docx_rs::StyleType::Paragraph)
        .name("Heading 1")
        .outline_lvl(0);
    let h2 = docx_rs::Style::new("Heading2", docx_rs::StyleType::Paragraph)
        .name("Heading 2")
        .outline_lvl(1);

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Title"))
                .style("Heading1"),
            docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text("Body text")),
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Subtitle"))
                .style("Heading2"),
        ],
        vec![h1, h2],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let blocks = all_blocks(&doc);

    if let Block::Paragraph(p) = &blocks[0] {
        assert_eq!(p.runs[0].style.font_size, Some(24.0));
        assert_eq!(p.runs[0].style.bold, Some(true));
    } else {
        panic!("Expected Paragraph");
    }

    if let Block::Paragraph(p) = &blocks[1] {
        assert!(p.runs[0].style.font_size.is_none());
        assert!(p.runs[0].style.bold.is_none());
    } else {
        panic!("Expected Paragraph");
    }

    if let Block::Paragraph(p) = &blocks[2] {
        assert_eq!(p.runs[0].style.font_size, Some(20.0));
        assert_eq!(p.runs[0].style.bold, Some(true));
    } else {
        panic!("Expected Paragraph");
    }
}

#[test]
fn test_style_with_color_and_font() {
    let custom = docx_rs::Style::new("Fancy", docx_rs::StyleType::Paragraph)
        .name("Fancy Style")
        .color("FF0000")
        .fonts(docx_rs::RunFonts::new().ascii("Georgia"));

    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Fancy text"))
                .style("Fancy"),
        ],
        vec![custom],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let run = first_run(&doc);

    assert_eq!(run.style.color, Some(Color::new(255, 0, 0)));
    assert_eq!(run.style.font_family, Some("Georgia".to_string()));
}

#[test]
fn test_runs_inherit_document_default_font() {
    let styles = docx_rs::Styles::new()
        .default_fonts(docx_rs::RunFonts::new().ascii("Raleway"))
        .default_size(18);

    let link = docx_rs::Hyperlink::new("https://example.com", docx_rs::HyperlinkType::External)
        .add_run(
            docx_rs::Run::new()
                .color("1155cc")
                .underline("single")
                .add_text("Linked text"),
        );
    let paragraph = docx_rs::Paragraph::new()
        .add_run(docx_rs::Run::new().add_text("Plain text "))
        .add_hyperlink(link);
    let data = build_docx_bytes_with_stylesheet(vec![paragraph], styles);

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let para = first_paragraph(&doc);

    assert_eq!(para.runs.len(), 2);
    assert_eq!(para.runs[0].style.font_family.as_deref(), Some("Raleway"));
    assert_eq!(para.runs[0].style.font_size, Some(9.0));
    assert_eq!(para.runs[1].href.as_deref(), Some("https://example.com"));
    assert_eq!(para.runs[1].style.font_family.as_deref(), Some("Raleway"));
    assert_eq!(para.runs[1].style.font_size, Some(9.0));
    assert_eq!(para.runs[1].style.color, Some(Color::new(17, 85, 204)));
    assert_eq!(para.runs[1].style.underline, Some(true));
}

#[test]
fn test_direct_jc_center_applied_to_paragraph() {
    // Direct <w:jc w:val="center"/> in the paragraph's own pPr (no style).
    let data = build_docx_bytes_with_styles(
        vec![
            docx_rs::Paragraph::new()
                .add_run(docx_rs::Run::new().add_text("Centered directly"))
                .align(docx_rs::AlignmentType::Center),
        ],
        vec![],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let para = first_paragraph(&doc);

    assert_eq!(para.style.alignment, Some(Alignment::Center));
}

#[test]
fn test_default_paragraph_style_applies_without_pstyle() {
    // w:default="1" marks the style that paragraphs without an explicit
    // pStyle inherit (issue #288): its spacing must survive the cascade.
    let mut normal = docx_rs::Style::new("Normal", docx_rs::StyleType::Paragraph)
        .name("Normal")
        .size(24)
        .line_spacing(
            docx_rs::LineSpacing::new()
                .after(160)
                .line(360)
                .line_rule(docx_rs::LineSpacingType::Auto),
        );
    normal.default = true;

    let data = build_docx_bytes_with_styles(
        vec![docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text("본문 문단"))],
        vec![normal],
    );

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let Page::Flow(flow) = &doc.pages[0] else {
        panic!("expected flow page");
    };
    let Block::Paragraph(paragraph) = flow
        .content
        .iter()
        .find(|b| matches!(b, Block::Paragraph(_)))
        .expect("paragraph")
    else {
        unreachable!()
    };
    assert_eq!(
        paragraph.style.space_after,
        Some(8.0),
        "default style spacing (160 twips = 8pt) must apply to pStyle-less paragraphs"
    );
    assert_eq!(paragraph.runs[0].style.font_size, Some(12.0));
}

#[test]
fn test_scan_default_paragraph_style_id_from_raw_styles_xml() {
    let xml = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:style w:type="character" w:default="1" w:styleId="DefaultCharacter"/>
      <w:style w:type="paragraph" w:default="1" w:styleId="BodyDefault"/>
    </w:styles>"#;

    assert_eq!(
        styles::scan_default_paragraph_style_id(xml).as_deref(),
        Some("BodyDefault")
    );
}

#[test]
fn test_doc_default_theme_font_resolves_via_theme() {
    // docDefaults referencing asciiTheme="minorHAnsi" must resolve to the
    // theme's minor latin typeface instead of falling back to the renderer
    // default (issue #287). docx-rs's builder can't author theme slots, so
    // exercise the resolver directly.
    let theme_xml = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements><a:fontScheme name="Office">
    <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
    <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
  </a:fontScheme></a:themeElements>
</a:theme>"#;
    let theme = parse_theme_fonts(theme_xml);
    assert_eq!(theme.minor_latin.as_deref(), Some("Calibri"));
    assert_eq!(theme.major_latin.as_deref(), Some("Calibri Light"));

    let run_property = serde_json::json!({ "fonts": { "asciiTheme": "minorHAnsi" } });
    assert_eq!(
        resolve_theme_font_family(&run_property, &theme).as_deref(),
        Some("Calibri")
    );
    let heading_property = serde_json::json!({ "fonts": { "asciiTheme": "majorHAnsi" } });
    assert_eq!(
        resolve_theme_font_family(&heading_property, &theme).as_deref(),
        Some("Calibri Light")
    );
    let no_theme = serde_json::json!({ "fonts": { "ascii": "Arial" } });
    assert_eq!(resolve_theme_font_family(&no_theme, &theme), None);
}

#[test]
fn test_paragraph_shading_extracted_as_background() {
    // Word paints w:pPr/w:shd behind the whole paragraph (code blocks in
    // the CLI-manual fixture); the fill must reach the IR (issue #351).
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:shd w:val="clear" w:fill="F4F4F4"/></w:pPr>
      <w:r><w:t>$ cargo install office2pdf-cli</w:t></w:r>
    </w:p>
    <w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr>
  </w:body>
</w:document>"#;
    let data = build_docx_with_columns(document_xml);

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let para = first_paragraph(&doc);

    assert_eq!(para.style.background, Some(Color::new(0xF4, 0xF4, 0xF4)));
}

#[test]
fn test_paragraph_bottom_border_extracted() {
    // w:pBdr bottom rules (resume header underline, letterhead frames) must
    // reach the IR with Word's eighth-point width unit (issue #368).
    let mut ruled = docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text("JAMIE PARKER"));
    ruled.property = ruled.property.set_borders(
        docx_rs::ParagraphBorders::with_empty().set(
            docx_rs::ParagraphBorder::new(docx_rs::ParagraphBorderPosition::Bottom)
                .val(docx_rs::BorderType::Single)
                .size(6)
                .color("1E2761"),
        ),
    );
    let data = build_docx_bytes(vec![ruled]);

    let parser = DocxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let para = first_paragraph(&doc);

    let border = para.style.border.as_ref().expect("border must be parsed");
    let bottom = border.bottom.as_ref().expect("bottom side present");
    assert_eq!(bottom.width, 0.75, "w:sz is eighths of a point");
    assert_eq!(bottom.color, Color::new(0x1E, 0x27, 0x61));
    assert_eq!(bottom.style, BorderLineStyle::Solid);
    assert!(border.top.is_none());
}

/// `w:docDefaults` carrying the justification, line spacing, and space-after a
/// generated document states once for its whole body.
const DOC_DEFAULT_STYLES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:pPrDefault>
      <w:pPr>
        <w:spacing w:after="100" w:line="278" w:lineRule="auto"/>
        <w:jc w:val="both"/>
      </w:pPr>
    </w:pPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="Heading 1"/>
    <w:pPr><w:spacing w:after="150"/><w:jc w:val="left"/><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
</w:styles>"#;

fn doc_default_paragraph(document_xml: &str) -> Paragraph {
    let data = build_docx_with_styles_xml(document_xml, DOC_DEFAULT_STYLES_XML);
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();
    let Page::Flow(flow) = &doc.pages[0] else {
        panic!("expected flow page");
    };
    let Block::Paragraph(paragraph) = flow
        .content
        .iter()
        .find(|block| matches!(block, Block::Paragraph(_)))
        .expect("paragraph")
    else {
        unreachable!()
    };
    paragraph.clone()
}

fn proportional_line_spacing(style: &ParagraphStyle) -> Option<f64> {
    match style.line_spacing {
        Some(LineSpacing::Proportional(factor)) => Some(factor),
        _ => None,
    }
}

#[test]
fn test_doc_default_paragraph_properties_reach_a_paragraph_without_a_style() {
    // w:docDefaults/w:pPrDefault sits below every named style and below the
    // w:default="1" style. Reading only w:rPrDefault left a document that
    // states its body layout there ragged, single-spaced, and gapless, which
    // repaginated the technical brief from 39 pages to 31 (issue #574).
    let paragraph = doc_default_paragraph(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>본문 문단</w:t></w:r></w:p></w:body>
</w:document>"#,
    );

    assert_eq!(
        paragraph.style.alignment,
        Some(Alignment::Justify),
        "w:pPrDefault w:jc=both must reach a paragraph with no pStyle"
    );
    assert_eq!(
        paragraph.style.space_after,
        Some(5.0),
        "w:pPrDefault w:after=100 twips must reach a paragraph with no pStyle"
    );
    assert_eq!(
        proportional_line_spacing(&paragraph.style),
        Some(278.0 / 240.0),
        "w:pPrDefault w:line=278 auto must reach a paragraph with no pStyle"
    );
}

#[test]
fn test_named_style_overrides_doc_defaults_but_inherits_what_it_does_not_state() {
    // A named style states only what it changes. Heading1 here sets its own
    // alignment and space-after, so those win, but it states no line spacing
    // and must still inherit the document default's (issue #574).
    let paragraph = doc_default_paragraph(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>1. 개요</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
    );

    assert_eq!(
        paragraph.style.alignment,
        Some(Alignment::Left),
        "the style's own w:jc must win over the document default"
    );
    assert_eq!(
        paragraph.style.space_after,
        Some(7.5),
        "the style's own 150 twips must win over the document default's 100"
    );
    assert_eq!(
        proportional_line_spacing(&paragraph.style),
        Some(278.0 / 240.0),
        "the line spacing the style does not state must come from w:pPrDefault"
    );
}

#[test]
fn test_direct_paragraph_formatting_overrides_doc_defaults() {
    // Direct w:pPr sits above both the style and the document default.
    let paragraph = doc_default_paragraph(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:spacing w:after="240" w:line="480" w:lineRule="auto"/><w:jc w:val="center"/></w:pPr>
      <w:r><w:t>가운데 문단</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#,
    );

    assert_eq!(paragraph.style.alignment, Some(Alignment::Center));
    assert_eq!(paragraph.style.space_after, Some(12.0));
    assert_eq!(proportional_line_spacing(&paragraph.style), Some(2.0));
}

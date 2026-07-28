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

#[test]
fn test_tracked_insertion_renders_and_tracked_deletion_does_not() {
    // Word's final view — "No Markup", and what accepting every revision
    // produces — keeps w:ins content and drops w:del content. Both were
    // falling through the paragraph child match, so an accepted insertion
    // vanished from the output along with the deletion (issue #583).
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t xml:space="preserve">v0.6.4 기준 </w:t></w:r>
      <w:del w:id="901" w:author="문서 검토자" w:date="2026-07-26T10:00:00Z">
        <w:r><w:delText xml:space="preserve">기능 목록 재확인</w:delText></w:r>
      </w:del>
      <w:ins w:id="902" w:author="문서 검토자" w:date="2026-07-26T10:05:00Z">
        <w:r><w:t xml:space="preserve">기능·품질 지표 실측값 반영</w:t></w:r>
      </w:ins>
    </w:p>
  </w:body>
</w:document>"#;

    let data = build_docx_with_columns(document_xml);
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

    let text: String = paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();

    assert!(
        text.contains("기능·품질 지표 실측값 반영"),
        "the tracked insertion is ordinary final-document text: {text:?}"
    );
    assert!(
        !text.contains("기능 목록 재확인"),
        "the tracked deletion is not in the final document: {text:?}"
    );
    assert!(
        text.starts_with("v0.6.4 기준"),
        "the untracked run keeps its place: {text:?}"
    );
}

#[test]
fn test_insertion_that_was_later_deleted_is_dropped() {
    // A w:del nested inside a w:ins is text that was inserted and then
    // deleted again, so the final document does not contain it (issue #583).
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t xml:space="preserve">남는 문장</w:t></w:r>
      <w:ins w:id="903" w:author="검토자" w:date="2026-07-26T10:05:00Z">
        <w:del w:id="904" w:author="검토자" w:date="2026-07-26T10:06:00Z">
          <w:r><w:delText xml:space="preserve">되돌린 문장</w:delText></w:r>
        </w:del>
      </w:ins>
    </w:p>
  </w:body>
</w:document>"#;

    let data = build_docx_with_columns(document_xml);
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

    let text: String = paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();

    assert_eq!(text, "남는 문장");
}

#[test]
fn test_tracked_changes_resolve_the_same_way_in_a_footer() {
    // A header or footer resolves to the final view like the body does; the
    // two loops share one flattening so they cannot drift (issue #583).
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/docx/office2pdf_technical_brief_ko.docx"
    ))
    .expect("fixture");
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();

    // The brief's footers carry a PAGE and a NUMPAGES field, which travel the
    // same match as the tracked-change variants.
    let footer_fields: usize = doc
        .pages
        .iter()
        .filter_map(|page| match page {
            Page::Flow(flow) => flow.footer.as_ref(),
            _ => None,
        })
        .flat_map(|footer| footer.paragraphs.iter())
        .flat_map(|paragraph| paragraph.elements.iter())
        .filter(|element| {
            matches!(
                element,
                crate::ir::HFInline::PageNumber(_) | crate::ir::HFInline::TotalPages(_)
            )
        })
        .count();

    assert!(
        footer_fields >= 12,
        "every section's footer keeps its PAGE and NUMPAGES fields, got {footer_fields}"
    );
}

#[test]
fn test_header_and_footer_runs_inherit_the_document_default_run_style() {
    // Header and footer parts are read before the stylesheet is, so their runs
    // were left with only what they state themselves and fell through to the
    // renderer's own family and size. Word resolves them through the same run
    // cascade as the body (issue #578).
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/docx/office2pdf_technical_brief_ko.docx"
    ))
    .expect("fixture");
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();

    let Page::Flow(flow) = doc
        .pages
        .iter()
        .find(|page| matches!(page, Page::Flow(flow) if flow.header.is_some()))
        .expect("a section with a header")
    else {
        unreachable!()
    };
    let header = flow.header.as_ref().expect("header");

    // The brief's `w:rPrDefault` names Calibri at 10pt. The header's first run
    // states a colour, a weight, and 8pt, but no family.
    let styled_run = header
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.elements.iter())
        .find_map(|element| match element {
            crate::ir::HFInline::Run(run) if !run.text.trim().is_empty() => Some(&run.style),
            _ => None,
        })
        .expect("a header text run");

    assert_eq!(
        styled_run.font_family.as_deref(),
        Some("Calibri"),
        "the family the run does not state comes from w:rPrDefault"
    );
    assert_eq!(
        styled_run.font_size,
        Some(8.0),
        "the size the run does state still wins"
    );

    let footer = flow.footer.as_ref().expect("footer");
    let page_number_style = footer
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.elements.iter())
        .find_map(|element| match element {
            crate::ir::HFInline::PageNumber(style) => Some(style),
            _ => None,
        })
        .expect("a PAGE field");

    assert_eq!(
        page_number_style.font_family.as_deref(),
        Some("Calibri"),
        "a PAGE field resolves through the same cascade as the literals beside it"
    );
}

#[test]
fn test_footnote_text_resolves_its_paragraph_style_and_run_properties() {
    // A note is read before the stylesheet is, so its runs used to arrive as
    // one unstyled string and rendered at the engine's own footnote size and
    // face. Word resolves them through the same cascade as the body: the
    // note's w:pStyle supplies what the runs leave unstated (issue #580).
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/docx/office2pdf_technical_brief_ko.docx"
    ))
    .expect("fixture");
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();

    let note_runs: Vec<&Run> = doc
        .pages
        .iter()
        .filter_map(|page| match page {
            Page::Flow(flow) => Some(flow),
            _ => None,
        })
        .flat_map(|flow| flow.content.iter())
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .flat_map(|paragraph| paragraph.runs.iter())
        .filter_map(|run| run.footnote.as_ref())
        .flatten()
        .collect();

    assert!(!note_runs.is_empty(), "the brief carries footnotes");

    // `FootnoteBody` sets `w:sz="16"` and `w:color="404040"`, and inherits the
    // document default family.
    for note_run in &note_runs {
        assert_eq!(
            note_run.style.font_size,
            Some(8.0),
            "the note's style sets 8pt: {:?}",
            note_run.text
        );
        assert_eq!(
            note_run
                .style
                .color
                .map(|color| (color.r, color.g, color.b)),
            Some((0x40, 0x40, 0x40)),
            "the note's style sets #404040: {:?}",
            note_run.text
        );
        assert_eq!(
            note_run.style.font_family.as_deref(),
            Some("Calibri"),
            "the family the note does not state comes from the document default"
        );
    }
}

#[test]
fn test_footnote_run_properties_override_the_notes_style() {
    // A run inside a note states its own properties over the note's style.
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:rPr><w:rStyle w:val="FootnoteReference"/></w:rPr>
    <w:footnoteReference w:id="2"/></w:r></w:p></w:body>
</w:document>"#;
    let footnotes_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="2">
    <w:p>
      <w:pPr><w:pStyle w:val="FootnoteBody"/></w:pPr>
      <w:r><w:t xml:space="preserve">보통 </w:t></w:r>
      <w:r><w:rPr><w:b/><w:sz w:val="24"/></w:rPr><w:t xml:space="preserve">강조</w:t></w:r>
    </w:p>
  </w:footnote>
</w:footnotes>"#;
    let styles_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="FootnoteBody">
    <w:name w:val="Footnote Body"/>
    <w:rPr><w:sz w:val="16"/></w:rPr>
  </w:style>
</w:styles>"#;

    let data = build_docx_with_notes_xml(document_xml, styles_xml, footnotes_xml);
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();
    let Page::Flow(flow) = &doc.pages[0] else {
        panic!("expected flow page");
    };
    let note: &Vec<Run> = flow
        .content
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .flat_map(|paragraph| paragraph.runs.iter())
        .find_map(|run| run.footnote.as_ref())
        .expect("a footnote");

    assert_eq!(note.len(), 2, "both runs survive: {note:?}");
    assert_eq!(note[0].text, "보통 ");
    assert_eq!(note[0].style.font_size, Some(8.0), "from the note's style");
    assert_eq!(note[1].text, "강조");
    assert_eq!(
        note[1].style.font_size,
        Some(12.0),
        "the run's own size wins"
    );
    assert_eq!(note[1].style.bold, Some(true), "the run's own weight wins");
}

#[test]
fn test_section_page_numbering_restarts_and_picks_its_numerals() {
    // A front matter that restarts at `i` states both the restart and the
    // format in w:pgNumType. office2pdf read neither and warned that it was
    // falling back to one global decimal counter (issue #582).
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/docx/office2pdf_technical_brief_ko.docx"
    ))
    .expect("fixture");
    let (doc, warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();

    let numbering: Vec<Option<crate::ir::PageNumbering>> = doc
        .pages
        .iter()
        .filter_map(|page| match page {
            Page::Flow(flow) => Some(flow.page_numbering),
            _ => None,
        })
        .collect();

    // The cover states nothing, the front matter restarts at i, the body
    // restarts at 1 in decimal.
    assert_eq!(
        numbering[1],
        Some(crate::ir::PageNumbering {
            start: Some(1),
            format: crate::ir::PageNumberFormat::LowerRoman
        }),
        "the front matter restarts at i"
    );
    assert_eq!(
        numbering[2],
        Some(crate::ir::PageNumbering {
            start: Some(1),
            format: crate::ir::PageNumberFormat::Decimal
        }),
        "the body restarts at 1 in decimal"
    );

    assert!(
        !warnings.iter().any(|warning| matches!(
            warning,
            crate::error::ConvertWarning::FallbackUsed { from, .. }
                if from == "section page number restart"
        )),
        "the restart is no longer a fallback: {warnings:?}"
    );
}

#[test]
fn test_seq_fields_number_captions_in_document_order() {
    // Word keeps a caption's number in a SEQ field, not in its text. Dropping
    // the field left every caption in the brief without one (issue #577).
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/docx/office2pdf_technical_brief_ko.docx"
    ))
    .expect("fixture");
    let (doc, _warnings) = DocxParser.parse(&data, &ConvertOptions::default()).unwrap();

    let caption_text = |lead: &str| -> Vec<String> {
        doc.pages
            .iter()
            .filter_map(|page| match page {
                Page::Flow(flow) => Some(flow),
                _ => None,
            })
            .flat_map(|flow| flow.content.iter())
            .filter_map(|block| match block {
                Block::Paragraph(paragraph) => Some(paragraph),
                _ => None,
            })
            .map(|paragraph| {
                paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>()
            })
            // A caption leads with the label and the field's number; the
            // contents-page headings share the label but not a number.
            .filter(|text| {
                text.strip_prefix(lead)
                    .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            })
            .collect()
    };

    let tables = caption_text("표 ");
    let figures = caption_text("그림 ");
    assert_eq!(tables.len(), 33, "every table caption: {tables:?}");
    assert_eq!(figures.len(), 5, "every figure caption: {figures:?}");

    // Each identifier counts from one, independently, in document order.
    for (index, caption) in tables.iter().enumerate() {
        assert!(
            caption.starts_with(&format!("표 {}", index + 1)),
            "table caption {} reads {caption:?}",
            index + 1
        );
    }
    for (index, caption) in figures.iter().enumerate() {
        assert!(
            caption.starts_with(&format!("그림 {}", index + 1)),
            "figure caption {} reads {caption:?}",
            index + 1
        );
    }
}

use super::*;

#[test]
fn test_text_box_extraction() {
    let shape = make_text_box(0, 0, 1_000_000, 500_000, "Hello World");
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    assert_eq!(page.elements.len(), 1, "Expected 1 element");

    let blocks = text_box_blocks(&page.elements[0]);
    assert!(!blocks.is_empty(), "Expected at least one block");

    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs.len(), 1);
    assert_eq!(para.runs[0].text, "Hello World");
}

#[test]
fn test_text_box_position_and_size() {
    let x = 1_000_000i64;
    let y = 500_000i64;
    let cx = 5_000_000i64;
    let cy = 2_000_000i64;
    let shape = make_text_box(x, y, cx, cy, "Positioned");
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let elem = &page.elements[0];

    let expected_x = emu_to_pt(x);
    let expected_y = emu_to_pt(y);
    let expected_w = emu_to_pt(cx);
    let expected_h = emu_to_pt(cy);

    assert!(
        (elem.x - expected_x).abs() < 0.1,
        "Expected x ~{expected_x}, got {}",
        elem.x
    );
    assert!(
        (elem.y - expected_y).abs() < 0.1,
        "Expected y ~{expected_y}, got {}",
        elem.y
    );
    assert!(
        (elem.width - expected_w).abs() < 0.1,
        "Expected width ~{expected_w}, got {}",
        elem.width
    );
    assert!(
        (elem.height - expected_h).abs() < 0.1,
        "Expected height ~{expected_h}, got {}",
        elem.height
    );
}

#[test]
fn test_text_box_bold_formatting() {
    let runs_xml = r#"<a:r><a:rPr b="1"/><a:t>Bold text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 1_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].text, "Bold text");
    assert_eq!(para.runs[0].style.bold, Some(true));
}

#[test]
fn test_text_box_italic_formatting() {
    let runs_xml = r#"<a:r><a:rPr i="1"/><a:t>Italic text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 1_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].text, "Italic text");
    assert_eq!(para.runs[0].style.italic, Some(true));
}

#[test]
fn test_text_box_font_size() {
    let runs_xml = r#"<a:r><a:rPr sz="2400"/><a:t>Large text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 1_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs[0].style.font_size, Some(24.0));
}

/// Parse a single formatted run and return its resolved style.
fn run_style_for(runs_xml: &str) -> TextStyle {
    let shape = make_formatted_text_box(0, 0, 6_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    match &blocks[0] {
        Block::Paragraph(p) => p.runs[0].style.clone(),
        _ => panic!("Expected Paragraph"),
    }
}

#[test]
fn test_text_box_positive_character_tracking() {
    // DrawingML `spc` is hundredths of a point, like `sz`. The eyebrow run on
    // the introduction deck's title slide declares spc="220" — 2.2pt per gap.
    let runs_xml = r#"<a:r><a:rPr sz="1150" b="1" spc="220"/><a:t>OPEN SOURCE · RUST</a:t></a:r>"#;

    assert_eq!(run_style_for(runs_xml).letter_spacing, Some(2.2));
}

#[test]
fn test_text_box_negative_character_tracking() {
    // The same deck's hero wordmark tightens with spc="-100".
    let runs_xml = r#"<a:r><a:rPr sz="6800" b="1" spc="-100"/><a:t>office2pdf</a:t></a:r>"#;

    assert_eq!(run_style_for(runs_xml).letter_spacing, Some(-1.0));
}

#[test]
fn test_text_box_without_spc_has_no_tracking() {
    let runs_xml = r#"<a:r><a:rPr sz="1800"/><a:t>Untracked</a:t></a:r>"#;

    assert_eq!(run_style_for(runs_xml).letter_spacing, None);
}

#[test]
fn test_text_box_combined_formatting() {
    let runs_xml = r#"<a:r><a:rPr b="1" i="1" u="sng" strike="sngStrike" sz="1800"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="Arial"/></a:rPr><a:t>Styled text</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 1_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    let run = &para.runs[0];
    assert_eq!(run.text, "Styled text");
    assert_eq!(run.style.bold, Some(true));
    assert_eq!(run.style.italic, Some(true));
    assert_eq!(run.style.underline, Some(true));
    assert_eq!(run.style.strikethrough, Some(true));
    assert_eq!(run.style.font_size, Some(18.0));
    assert_eq!(run.style.color, Some(Color::new(255, 0, 0)));
    assert_eq!(run.style.font_family, Some("Arial".to_string()));
}

#[test]
fn test_multiple_text_boxes() {
    let shape1 = make_text_box(100_000, 100_000, 2_000_000, 500_000, "Box 1");
    let shape2 = make_text_box(100_000, 700_000, 2_000_000, 500_000, "Box 2");
    let slide = make_slide_xml(&[shape1, shape2]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    assert_eq!(page.elements.len(), 2, "Expected 2 text boxes");

    let get_text = |elem: &FixedElement| -> String {
        let blocks = text_box_blocks(elem);
        blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph(p) => {
                    Some(p.runs.iter().map(|r| r.text.as_str()).collect::<String>())
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(get_text(&page.elements[0]), "Box 1");
    assert_eq!(get_text(&page.elements[1]), "Box 2");
}

#[test]
fn test_multiple_slides() {
    let slide1 = make_slide_xml(&[make_text_box(0, 0, 1_000_000, 500_000, "Slide 1")]);
    let slide2 = make_slide_xml(&[make_text_box(0, 0, 1_000_000, 500_000, "Slide 2")]);
    let slide3 = make_slide_xml(&[make_text_box(0, 0, 1_000_000, 500_000, "Slide 3")]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide1, slide2, slide3]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    assert_eq!(doc.pages.len(), 3, "Expected 3 pages");
    for page in &doc.pages {
        assert!(matches!(page, Page::Fixed(_)));
    }
}

#[test]
fn test_text_box_multiple_paragraphs() {
    let paras_xml = r#"<a:p><a:r><a:rPr/><a:t>Paragraph 1</a:t></a:r></a:p><a:p><a:r><a:rPr/><a:t>Paragraph 2</a:t></a:r></a:p>"#;
    let shape = make_multi_para_text_box(0, 0, 3_000_000, 2_000_000, paras_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let paras: Vec<&Paragraph> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .collect();
    assert!(paras.len() >= 2, "Expected at least 2 paragraphs");
    assert_eq!(paras[0].runs[0].text, "Paragraph 1");
    assert_eq!(paras[1].runs[0].text, "Paragraph 2");
}

#[test]
fn test_text_box_multiple_runs() {
    let runs_xml =
        r#"<a:r><a:rPr b="1"/><a:t>Bold </a:t></a:r><a:r><a:rPr i="1"/><a:t>Italic</a:t></a:r>"#;
    let shape = make_formatted_text_box(0, 0, 2_000_000, 500_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.runs.len(), 2);
    assert_eq!(para.runs[0].text, "Bold ");
    assert_eq!(para.runs[0].style.bold, Some(true));
    assert_eq!(para.runs[1].text, "Italic");
    assert_eq!(para.runs[1].style.italic, Some(true));
}

#[test]
fn test_paragraph_alignment_center() {
    let paras_xml = r#"<a:p><a:pPr algn="ctr"/><a:r><a:rPr/><a:t>Centered</a:t></a:r></a:p>"#;
    let shape = make_multi_para_text_box(0, 0, 2_000_000, 500_000, paras_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.style.alignment, Some(Alignment::Center));
}

#[test]
fn test_body_pr_vert_sets_text_rotation() {
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="V"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="2743200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr vert="vert"/><a:p><a:r><a:rPr lang="en-US"/><a:t>Vertical it should be!</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let slide = make_slide_xml(&[shape.to_string()]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    let text_box = text_box_data(&page.elements[0]);
    assert_eq!(text_box.text_rotation_deg, Some(270.0));
}

#[test]
fn test_body_pr_vert270_sets_reverse_rotation() {
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="V"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="2743200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr vert="vert270"/><a:p><a:r><a:rPr lang="en-US"/><a:t>Up</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let slide = make_slide_xml(&[shape.to_string()]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    let text_box = text_box_data(&page.elements[0]);
    assert_eq!(text_box.text_rotation_deg, Some(90.0));
}

#[test]
fn test_vert_text_in_preset_shape_centers_column() {
    // Preset geometries confine text to an inset text rect we don't model;
    // vert text anchored at the box edge would land on the shape's sloped
    // boundary (issue #286) — the overlay centers it instead.
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="P"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1417740" cy="1317072"/></a:xfrm><a:prstGeom prst="pentagon"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr vert="vert" anchor="t"/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US"/><a:t>text</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let slide = make_slide_xml(&[shape.to_string()]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    // Shape background + transparent text overlay.
    let text_box = page
        .elements
        .iter()
        .find_map(|elem| match &elem.kind {
            FixedElementKind::TextBox(tb) => Some(tb),
            _ => None,
        })
        .expect("text overlay present");
    assert_eq!(text_box.text_rotation_deg, Some(270.0));
    assert_eq!(
        text_box.vertical_align,
        TextBoxVerticalAlign::Center,
        "vert text inside a preset shape must center its column"
    );
}

#[test]
fn test_vert_text_in_plain_rect_keeps_anchor() {
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="V"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="2743200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr vert="vert" anchor="t"/><a:p><a:r><a:rPr lang="en-US"/><a:t>Up</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let slide = make_slide_xml(&[shape.to_string()]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);

    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    let text_box = text_box_data(&page.elements[0]);
    assert_eq!(
        text_box.vertical_align,
        TextBoxVerticalAlign::Top,
        "plain rectangles keep their bodyPr anchor"
    );
}

#[test]
fn test_text_box_paragraph_before_after_spacing() {
    // a:spcBef / a:spcAft carry PowerPoint's inter-paragraph gaps in
    // hundredths of a point; dropping them packed bullet lists into a dense
    // block (issue #359).
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="TextBox"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="4000000" cy="2000000"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:p><a:pPr><a:spcBef><a:spcPts val="400"/></a:spcBef><a:spcAft><a:spcPts val="600"/></a:spcAft></a:pPr><a:r><a:rPr lang="en-US"/><a:t>spaced bullet</a:t></a:r></a:p></p:txBody></p:sp>"#.to_string();
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();

    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    let para = match &blocks[0] {
        Block::Paragraph(p) => p,
        _ => panic!("Expected Paragraph"),
    };
    assert_eq!(para.style.space_before, Some(4.0));
    assert_eq!(para.style.space_after, Some(6.0));
}

// ── Hangul kinsoku break markers (issue #438) ────────────────────────
//
// PowerPoint keeps a Hangul syllable on the line when the following
// terminal punctuation overflows: it hangs the mark past the margin
// (Windows) or breaks before it (macOS). UAX #14 LB13 instead glues the
// mark to the syllable, wrapping both. The parser marks the position
// with an IR-only sentinel that the renderer turns into an empty
// `#box[]` (a UAX #14 Contingent Break), restoring the opportunity
// without touching the PDF text layer.

fn parse_single_paragraph(runs_xml: &str) -> Paragraph {
    let shape = make_formatted_text_box(0, 0, 4_000_000, 1_000_000, runs_xml);
    let slide = make_slide_xml(&[shape]);
    let data = build_test_pptx(SLIDE_CX, SLIDE_CY, &[slide]);
    let parser = PptxParser;
    let (doc, _warnings) = parser.parse(&data, &ConvertOptions::default()).unwrap();
    let page = first_fixed_page(&doc);
    let blocks = text_box_blocks(&page.elements[0]);
    match &blocks[0] {
        Block::Paragraph(p) => p.clone(),
        _ => panic!("Expected Paragraph"),
    }
}

#[test]
fn test_hangul_terminal_punct_gets_break_marker() {
    let para = parse_single_paragraph(r#"<a:r><a:rPr lang="ko-KR"/><a:t>라는 뜻인가?</a:t></a:r>"#);
    assert_eq!(para.runs[0].text, "라는 뜻인가\u{200B}?");
}

#[test]
fn test_hangul_terminal_punct_marker_spans_run_boundary() {
    // The syllable and the mark can arrive in differently-styled runs.
    let para = parse_single_paragraph(
        r#"<a:r><a:rPr lang="ko-KR" b="1"/><a:t>뜻인가</a:t></a:r><a:r><a:rPr lang="ko-KR"/><a:t>?</a:t></a:r>"#,
    );
    assert_eq!(para.runs[0].text, "뜻인가");
    assert_eq!(para.runs[1].text, "\u{200B}?");
}

#[test]
fn test_hangul_break_marker_covers_fullwidth_and_ellipsis() {
    let para = parse_single_paragraph(
        r#"<a:r><a:rPr lang="ko-KR"/><a:t>질문？최고！끝。쉼、다음…</a:t></a:r>"#,
    );
    assert_eq!(
        para.runs[0].text,
        "질문\u{200B}？최고\u{200B}！끝\u{200B}。쉼\u{200B}、다음\u{200B}…"
    );
}

#[test]
fn test_no_break_marker_for_percent_or_latin_context() {
    // '%' glues to the syllable in PowerPoint (measured in #438), and a
    // Latin letter before the mark is outside the rule entirely.
    let para = parse_single_paragraph(
        r#"<a:r><a:rPr lang="ko-KR"/><a:t>확률이 3%라는 것. OK?</a:t></a:r>"#,
    );
    assert_eq!(para.runs[0].text, "확률이 3%라는 것\u{200B}. OK?");
}

#[test]
fn test_no_break_marker_between_hangul_syllables() {
    let para = parse_single_paragraph(r#"<a:r><a:rPr lang="ko-KR"/><a:t>가나다라마</a:t></a:r>"#);
    assert_eq!(para.runs[0].text, "가나다라마");
}

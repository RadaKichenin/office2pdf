use super::*;

// ── Unicode NFC normalization tests ──────────────────────────────

#[test]
fn test_escape_typst_normalizes_korean_nfd_to_nfc() {
    let nfd_korean = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}";
    let nfc_korean = "한글";
    let result = escape_typst(nfd_korean);
    assert_eq!(
        result, nfc_korean,
        "NFD Korean jamo should be normalized to composed hangul"
    );
}

#[test]
fn test_escape_typst_normalizes_combining_diacritics() {
    let nfd_cafe = "cafe\u{0301}";
    let nfc_cafe = "caf\u{00E9}";
    let result = escape_typst(nfd_cafe);
    assert_eq!(
        result, nfc_cafe,
        "Combining diacritics should be normalized to NFC"
    );
}

#[test]
fn test_escape_typst_nfc_with_special_chars() {
    let nfd_input = "cafe\u{0301} \\$5";
    let result = escape_typst(nfd_input);
    assert!(
        result.contains("caf\u{00E9}"),
        "Should contain NFC-normalized é: {result}"
    );
    assert!(
        result.contains("\\$"),
        "Should still escape $ sign: {result}"
    );
}

#[test]
fn test_generate_typst_nfc_korean_in_paragraph() {
    let nfd_korean = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}";
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph(nfd_korean)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("한글"),
        "Generated Typst should contain NFC-composed Korean: {result}"
    );
    assert!(
        !result.contains('\u{1112}'),
        "Generated Typst should not contain decomposed jamo: {result}"
    );
}

#[test]
fn test_generate_typst_nfc_diacritics_in_paragraph() {
    let nfd_resume = "re\u{0301}sume\u{0301}";
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph(nfd_resume)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("r\u{00E9}sum\u{00E9}"),
        "Generated Typst should contain NFC-composed résumé: {result}"
    );
}

#[test]
fn test_escape_typst_already_nfc_unchanged() {
    let nfc_text = "Hello 한글 café";
    let result = escape_typst(nfc_text);
    assert_eq!(result, nfc_text, "Already-NFC text should be unchanged");
}

// --- US-103: Multi-column section layout codegen tests ---

#[test]
fn test_generate_flow_page_with_equal_columns() {
    let doc = make_doc(vec![Page::Flow(FlowPage {
        size: PageSize::default(),
        margins: Margins::default(),
        content: vec![make_paragraph("Column text")],
        header: None,
        footer: None,
        columns: Some(ColumnLayout {
            num_columns: 2,
            spacing: 36.0,
            column_widths: None,
        }),
        line_grid_pitch: None,
        line_grid_snaps_lines: false,
        page_numbering: None,
    })]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#columns(2, gutter: 36pt)"),
        "Should contain columns() call. Got: {result}"
    );
    assert!(
        result.contains("Column text"),
        "Should contain the text content. Got: {result}"
    );
}

#[test]
fn test_generate_flow_page_with_three_columns() {
    let doc = make_doc(vec![Page::Flow(FlowPage {
        size: PageSize::default(),
        margins: Margins::default(),
        content: vec![make_paragraph("Three col text")],
        header: None,
        footer: None,
        columns: Some(ColumnLayout {
            num_columns: 3,
            spacing: 18.0,
            column_widths: None,
        }),
        line_grid_pitch: None,
        line_grid_snaps_lines: false,
        page_numbering: None,
    })]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#columns(3, gutter: 18pt)"),
        "Should contain columns(3, ...). Got: {result}"
    );
}

#[test]
fn test_generate_flow_page_with_unequal_columns() {
    let doc = make_doc(vec![Page::Flow(FlowPage {
        size: PageSize::default(),
        margins: Margins::default(),
        content: vec![make_paragraph("Unequal col text")],
        header: None,
        footer: None,
        columns: Some(ColumnLayout {
            num_columns: 2,
            spacing: 36.0,
            column_widths: Some(vec![300.0, 150.0]),
        }),
        line_grid_pitch: None,
        line_grid_snaps_lines: false,
        page_numbering: None,
    })]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#grid(columns: (300pt, 150pt)"),
        "Unequal columns should use grid(). Got: {result}"
    );
}

#[test]
fn test_generate_column_break() {
    let doc = make_doc(vec![Page::Flow(FlowPage {
        size: PageSize::default(),
        margins: Margins::default(),
        content: vec![
            make_paragraph("Before break"),
            Block::ColumnBreak,
            make_paragraph("After break"),
        ],
        header: None,
        footer: None,
        columns: Some(ColumnLayout {
            num_columns: 2,
            spacing: 36.0,
            column_widths: None,
        }),
        line_grid_pitch: None,
        line_grid_snaps_lines: false,
        page_numbering: None,
    })]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#colbreak()"),
        "Should contain colbreak(). Got: {result}"
    );
}

#[test]
fn test_generate_no_columns_no_wrapper() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph("Normal text")])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        !result.contains("#columns("),
        "Should not contain columns(). Got: {result}"
    );
    assert!(
        !result.contains("#grid(columns:"),
        "Should not contain grid(columns:). Got: {result}"
    );
}

// ── BiDi / RTL codegen tests ──────────────────────────────────────

#[test]
fn test_generate_rtl_paragraph() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            direction: Some(TextDirection::Rtl),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "مرحبا بالعالم".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#set text(dir: rtl)"),
        "RTL paragraph should emit #set text(dir: rtl). Got: {result}"
    );
}

#[test]
fn test_generate_ltr_paragraph_no_direction() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph("Hello World")])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        !result.contains("dir: rtl"),
        "LTR paragraph should not emit dir: rtl. Got: {result}"
    );
}

#[test]
fn test_generate_mixed_rtl_ltr_paragraphs() {
    let doc = make_doc(vec![make_flow_page(vec![
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                direction: Some(TextDirection::Rtl),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "مرحبا 123".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        }),
        make_paragraph("English text"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#set text(dir: rtl)"),
        "Should contain RTL direction for Arabic paragraph. Got: {result}"
    );
    assert!(result.contains("مرحبا 123"), "Arabic text should appear");
    assert!(
        result.contains("English text"),
        "English text should appear"
    );
}

// --- US-204: Codegen/render robustness tests ---

#[test]
fn test_codegen_robustness_zero_pages() {
    let doc = make_doc(vec![]);
    let output = generate_typst(&doc).unwrap();
    assert!(output.images.is_empty());
}

#[test]
fn test_codegen_robustness_flow_page_empty_content() {
    let doc = make_doc(vec![make_flow_page(vec![])]);
    let output = generate_typst(&doc).unwrap();
    assert!(!output.source.is_empty());
}

#[test]
fn test_generate_fixed_page_empty_elements() {
    let doc = make_doc(vec![Page::Fixed(FixedPage {
        size: PageSize::default(),
        elements: vec![],
        background_color: None,
        background_gradient: None,
    })]);
    let output = generate_typst(&doc).unwrap();
    assert!(!output.source.is_empty());
}

#[test]
fn test_generate_table_page_empty_rows() {
    let doc = make_doc(vec![Page::Sheet(SheetPage {
        name: String::new(),
        size: PageSize::default(),
        margins: Margins::default(),
        table: Table {
            rows: vec![],
            column_widths: vec![],
            ..Table::default()
        },
        header: None,
        footer: None,
        charts: vec![],
        images: Vec::new(),
        text_boxes: Vec::new(),
    })]);
    let output = generate_typst(&doc).unwrap();
    assert!(!output.source.is_empty());
}

#[test]
fn test_generate_paragraph_all_alignment_variants() {
    for alignment in [
        Some(Alignment::Left),
        Some(Alignment::Center),
        Some(Alignment::Right),
        Some(Alignment::Justify),
        None,
    ] {
        let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                alignment,
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: format!("Alignment: {alignment:?}"),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })])]);
        let output = generate_typst(&doc);
        assert!(
            output.is_ok(),
            "Codegen should not fail for alignment {alignment:?}"
        );
    }
}

#[test]
fn test_generate_shape_shadow_all_kinds() {
    let shadow = Shadow {
        blur_radius: 4.0,
        color: Color { r: 0, g: 0, b: 0 },
        opacity: 0.5,
        direction: 45.0,
        distance: 3.0,
    };

    let shape_kinds = vec![
        ShapeKind::Rectangle,
        ShapeKind::Ellipse,
        ShapeKind::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 0.0,
            head_end: ArrowHead::None,
            tail_end: ArrowHead::None,
        },
        ShapeKind::RoundedRectangle {
            radius_fraction: 0.1,
        },
        ShapeKind::Polygon {
            vertices: vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)],
        },
    ];

    for kind in shape_kinds {
        let doc = make_doc(vec![Page::Fixed(FixedPage {
            size: PageSize {
                width: 960.0,
                height: 540.0,
            },
            elements: vec![FixedElement {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 100.0,
                kind: FixedElementKind::Shape(Shape {
                    kind: kind.clone(),
                    fill: Some(Color { r: 255, g: 0, b: 0 }),
                    gradient_fill: None,
                    stroke: None,
                    opacity: None,
                    shadow: Some(shadow.clone()),
                    rotation_deg: None,
                }),
            }],
            background_color: None,
            background_gradient: None,
        })]);
        let output = generate_typst(&doc);
        assert!(
            output.is_ok(),
            "Codegen should not panic for shape kind {kind:?} with shadow"
        );
    }
}

#[test]
fn test_column_break_with_empty_content() {
    let segments = split_at_column_breaks(&[]);
    assert_eq!(segments.len(), 1);
    assert!(segments[0].is_empty());
}

#[test]
fn test_column_break_only_breaks() {
    let blocks = vec![Block::ColumnBreak, Block::ColumnBreak];
    let segments = split_at_column_breaks(&blocks);
    assert_eq!(segments.len(), 3);
    assert!(segments.iter().all(|segment| segment.is_empty()));
}

// --- US-315: text escaping for Typst-significant characters ---

#[test]
fn test_escape_typst_backslash() {
    assert_eq!(escape_typst("path\\to\\file"), "path\\\\to\\\\file");
}

#[test]
fn test_escape_typst_hash() {
    assert_eq!(escape_typst("#hashtag"), "\\#hashtag");
}

#[test]
fn test_escape_typst_dollar() {
    assert_eq!(escape_typst("$100"), "\\$100");
}

#[test]
fn test_escape_typst_brackets() {
    assert_eq!(escape_typst("[content]"), "\\[content\\]");
}

#[test]
fn test_escape_typst_braces() {
    assert_eq!(escape_typst("{code}"), "\\{code\\}");
}

#[test]
fn test_escape_typst_all_special_chars() {
    let input = r"#*_`<>@\~/$[]{}";
    let result = escape_typst(input);
    assert_eq!(result, "\\#\\*\\_\\`\\<\\>\\@\\\\\\~\\/\\$\\[\\]\\{\\}");
}

#[test]
fn test_escape_typst_in_paragraph_output() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph(
        "Price: $100 path\\to",
    )])]);
    let output = generate_typst(&doc).unwrap().source;
    assert!(
        output.contains("\\$100"),
        "Dollar sign should be escaped in output: {output}"
    );
    assert!(
        output.contains("path\\\\to"),
        "Backslash should be escaped in output: {output}"
    );
}

// --- US-316: single-stop gradient fallback ---

#[test]
fn test_gradient_single_stop_fallback_to_solid() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: None,
        background_gradient: Some(GradientFill {
            stops: vec![GradientStop {
                position: 0.5,
                color: Color::new(255, 128, 0),
            }],
            angle: 0.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        !output.source.contains("gradient.linear"),
        "Single-stop gradient should fall back to solid fill: {}",
        output.source,
    );
    assert!(
        output.source.contains("rgb(255, 128, 0)"),
        "Single-stop gradient should use the stop color as solid fill: {}",
        output.source,
    );
}

#[test]
fn test_gradient_two_stops_still_works() {
    let page = Page::Fixed(FixedPage {
        size: PageSize {
            width: 720.0,
            height: 540.0,
        },
        elements: vec![],
        background_color: None,
        background_gradient: Some(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: Color::new(255, 0, 0),
                },
                GradientStop {
                    position: 1.0,
                    color: Color::new(0, 0, 255),
                },
            ],
            angle: 90.0,
        }),
    });
    let doc = make_doc(vec![page]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("gradient.linear"),
        "Two-stop gradient should still produce gradient.linear: {}",
        output.source,
    );
}

// --- US-382/383: unstyled run after styled run must not create `](` pattern ---

#[test]
fn test_unstyled_run_with_parens_after_styled_run() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![
            Run {
                text: "bold text".to_string(),
                style: TextStyle {
                    bold: Some(true),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            },
            Run {
                text: "(parenthetical note)".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            },
        ],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        !result.contains("](\\(") || !result.contains("]("),
        "Unstyled text with parens after styled run must be wrapped safely. Got: {result}"
    );
    assert!(
        result.contains("#[") || result.contains("\\("),
        "Unstyled text should be wrapped in #[...] to prevent syntax issues. Got: {result}"
    );
}

#[test]
fn test_escape_typst_escapes_leading_numeric_enum_marker() {
    // "2026. 07. 17." at line start would otherwise be re-typeset as a
    // Typst numbered list, dropping the zero padding ("2026. 7. 17.").
    let result = escape_typst("2026. 07. 17.");
    assert!(
        result.starts_with("2026\\."),
        "leading digits-period must be escaped: {result}"
    );
}

#[test]
fn test_escape_typst_keeps_mid_text_numbers_untouched() {
    let result = escape_typst("가격은 2026. 07 기준");
    assert!(result.contains("07"), "digits must survive: {result}");
}

#[test]
fn test_escape_typst_numeric_without_following_space_untouched() {
    // "3.14" is not an enum marker.
    assert_eq!(escape_typst("3.14"), "3.14");
}

// ── Preserved-space tests (issue #352) ───────────────────────────

#[test]
fn test_escape_typst_preserves_consecutive_spaces() {
    // Word keeps literal space runs (xml:space="preserve") that documents
    // use for manual alignment; Typst markup collapses them to one space.
    let result = escape_typst("Invoice #: INV-0342    Date: July 10");
    assert!(
        result.contains("#\"    \";"),
        "runs of spaces must survive markup collapsing: {result}"
    );
}

#[test]
fn test_escape_typst_preserves_leading_space_runs() {
    // Leading indentation ("      2. 계정 현행화 양식 1부.", code lines)
    // is stripped by markup whitespace handling.
    let result = escape_typst("      2. indented");
    assert!(
        result.starts_with("#\"      \";"),
        "leading space runs must survive: {result}"
    );
    assert!(
        result.ends_with("2. indented"),
        "text must follow: {result}"
    );
}

#[test]
fn test_escape_typst_preserves_spaces_after_hard_linebreak() {
    // Code blocks carry hard breaks followed by indentation.
    let result = escape_typst("match x {\n  b\"w:p\" => 1,\n}");
    assert!(
        result.contains("#linebreak()#\"  \";"),
        "post-break indentation must survive: {result}"
    );
}

#[test]
fn test_escape_typst_single_interior_space_untouched() {
    assert_eq!(escape_typst("a b"), "a b");
}

// ── Smart-typography escape tests (issue #353) ───────────────────

#[test]
fn test_escape_typst_keeps_straight_double_quotes() {
    // Typst smart quotes turned literal "quoted" into curly “quoted”.
    let result = escape_typst("run \"quoted\" text");
    assert!(
        result.contains("\\\"quoted\\\""),
        "straight double quotes must be escaped so smartquote cannot rewrite them: {result}"
    );
}

#[test]
fn test_escape_typst_keeps_straight_single_quotes() {
    let result = escape_typst("it's 'fine'");
    assert!(
        result.contains("it\\'s \\'fine\\'"),
        "straight apostrophes must be escaped: {result}"
    );
}

#[test]
fn test_escape_typst_keeps_double_hyphens() {
    // `--` ligates to an en dash, corrupting CLI flags like --font-path.
    let result = escape_typst("office2pdf --font-path dir --version");
    assert!(
        result.contains("\\-\\-font\\-path") || result.contains("\\-\\-font-path"),
        "double hyphens must not ligate to an en dash: {result}"
    );
    assert!(
        !result.contains("--"),
        "no raw double hyphen may remain: {result}"
    );
}

#[test]
fn test_escape_typst_keeps_hyphen_before_digits() {
    // A hyphen before digits becomes a Unicode minus (−18%) in markup.
    let result = escape_typst("blended CAC, -18%");
    assert!(
        result.contains("\\-18"),
        "hyphen before digits must stay a hyphen-minus: {result}"
    );
}

/// Word's East Asian/Latin auto space becomes a quarter of the *run's* size.
///
/// Sized in points rather than `em` because the spacing is emitted between the
/// run's `#text(size:)` calls: an `em` there resolves against the paragraph's
/// default size, which put 2.75pt at every boundary of a 10.5pt run and made a
/// line wide enough to re-wrap (issue #521).
#[test]
fn the_auto_space_marker_becomes_a_quarter_of_the_runs_size() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "2026\u{E001}년".to_string(),
            style: TextStyle {
                font_family: Some("Malgun Gothic".to_string()),
                font_size: Some(10.5),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#h(2.625pt)"),
        "0.25 x 10.5pt should reach the output as points: {result}"
    );
    assert!(
        !result.contains('\u{E001}'),
        "the marker must never be emitted literally: {result}"
    );
}

#[test]
fn the_auto_space_scales_with_the_run_not_the_document() {
    // Triangulation: a different run size must produce a different gap, so a
    // single measured constant cannot pass.
    for (size, expected) in [(9.5, "#h(2.375pt)"), (16.0, "#h(4pt)")] {
        let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "3\u{E001}자".to_string(),
                style: TextStyle {
                    font_family: Some("Malgun Gothic".to_string()),
                    font_size: Some(size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })])]);
        let result = generate_typst(&doc).unwrap().source;
        assert!(
            result.contains(expected),
            "a {size}pt run should emit {expected}: {result}"
        );
    }
}

// ── Hangul eojeol line breaking (issue #626) ─────────────────────────

/// The Korean sentence the issue measured, cut to the part that fits one
/// assertion. Word breaks it only at the spaces.
const EOJEOL_SENTENCE: &str = "본 계약은 갑과 을이";

/// A Korean paragraph, optionally justified, in the given font.
fn korean_paragraph(text: &str, alignment: Option<Alignment>, family: Option<&str>) -> Block {
    Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            alignment,
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: text.to_string(),
            style: TextStyle {
                font_family: family.map(str::to_string),
                east_asian_font_family: family.map(str::to_string),
                font_size: family.map(|_| 10.5),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })
}

#[test]
fn a_docx_paragraph_keeps_each_hangul_eojeol_whole() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        EOJEOL_SENTENCE,
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    // A one-syllable eojeol needs no frame — nothing can break inside it.
    assert!(
        result.contains("본 #box[계약은] #box[갑과] #box[을이]"),
        "each multi-syllable eojeol should be an unbreakable inline box: {result}"
    );
}

#[test]
fn a_justified_docx_paragraph_keeps_syllable_breaking() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        EOJEOL_SENTENCE,
        Some(Alignment::Justify),
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(EOJEOL_SENTENCE),
        "a justified line still breaks between syllables, as Word does: {result}"
    );
    assert!(
        !result.contains("#box["),
        "no eojeol frame may be emitted on a justified line: {result}"
    );
}

#[test]
fn a_slide_paragraph_keeps_syllable_breaking() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![make_text_box(10.0, 10.0, 300.0, 100.0, EOJEOL_SENTENCE)],
    )]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(EOJEOL_SENTENCE),
        "PowerPoint breaks Korean mid-word, and our slide output matches it: {result}"
    );
    assert!(
        !result.contains("#box["),
        "no eojeol frame may reach a slide: {result}"
    );
}

#[test]
fn a_sheet_cell_keeps_syllable_breaking() {
    let doc = make_doc(vec![make_sheet_page(
        "Sheet1",
        595.0,
        842.0,
        Margins::default(),
        make_simple_table(vec![vec![EOJEOL_SENTENCE]]),
    )]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(EOJEOL_SENTENCE),
        "a spreadsheet cell keeps Excel's own breaking: {result}"
    );
    assert!(
        !result.contains("#box["),
        "no eojeol frame may reach a sheet cell: {result}"
    );
}

#[test]
fn a_docx_table_cell_keeps_each_hangul_eojeol_whole() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell(EOJEOL_SENTENCE)],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("본 #box[계약은] #box[갑과] #box[을이]"),
        "Word breaks a table cell's Korean at eojeol too: {result}"
    );
}

#[test]
fn latin_text_is_untouched() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph(
        "The parties agree to cooperate",
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("The parties agree to cooperate"),
        "Latin already breaks at spaces and needs no frame: {result}"
    );
    assert!(!result.contains("#box["), "no frame for Latin: {result}");
}

#[test]
fn only_the_tokens_carrying_hangul_are_framed() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "2026년 VAT 별도 API 연동",
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#box[2026년] VAT #box[별도] API #box[연동]"),
        "a Latin/digit token keeps its own break opportunities: {result}"
    );
}

/// The auto space of issue #521 marks a boundary *inside* one eojeol, so it
/// must stay inside the frame — Typst maps every `#h()` to a space in the
/// paragraph text, which would otherwise reopen a mid-word break opportunity.
#[test]
fn the_east_asian_auto_space_stays_inside_the_eojeol() {
    // The `#box(…)` form this looks for is the frame that restores a *fixed*
    // line box, which the paragraph only declares when its Korean face
    // resolves. A machine without one — every CI runner here — emits the bare
    // `#box[` instead, so the premise cannot hold.
    if crate::render::pdf::font_line_metrics_em("Malgun Gothic").is_none() {
        return; // no Korean face available (e.g. a runner with no CJK fonts)
    }
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "2026\u{E001}년 계약",
        None,
        Some("Malgun Gothic"),
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    let framed: &str = result
        .split_once("#box(")
        .expect("an eojeol frame should be emitted")
        .1;
    assert!(
        framed.contains("#h(2.625pt)"),
        "the auto space belongs inside the frame: {result}"
    );
}

/// A frame seats its baseline on its own bottom edge, so under the fixed text
/// edges Word's line model needs (issues #354, #508) the framed text would sink
/// by the descent. The frame restores both edges and shifts its baseline back.
#[test]
fn a_framed_eojeol_keeps_the_paragraphs_baseline() {
    // The correction exists only under a fixed line box, and the paragraph
    // derives that box from its Korean face's own metrics. Without one — every
    // CI runner here — there is no box to restore and nothing to assert.
    if crate::render::pdf::font_line_metrics_em("Malgun Gothic").is_none() {
        return; // no Korean face available (e.g. a runner with no CJK fonts)
    }
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        EOJEOL_SENTENCE,
        None,
        Some("Malgun Gothic"),
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    let (top_em, bottom_em) =
        emitted_line_box_em(&result).expect("a Korean paragraph declares a fixed line box");
    let top_pt: f64 = top_em * 10.5;
    let bottom_pt: f64 = bottom_em * 10.5;
    let expected: String = format!(
        "#box(baseline: {}pt)[#text(top-edge: {}pt, bottom-edge: -{}pt)[",
        format_f64(bottom_pt),
        format_f64(top_pt),
        format_f64(bottom_pt)
    );
    assert!(
        result.contains(&expected),
        "the frame should restore the line box and shift the baseline back by the descent\n\
         expected: {expected}\nin: {result}"
    );
    assert_eq!(
        result.matches(&expected).count(),
        3,
        "every multi-syllable eojeol should carry the correction: {result}"
    );
}

/// Triangulation: the shift is the paragraph's own descent, not a constant.
#[test]
fn the_frames_baseline_shift_scales_with_the_font_size() {
    // Same premise as the test above: the shift is the fixed line box's own
    // descent, and that box needs the Korean face's measured metrics.
    if crate::render::pdf::font_line_metrics_em("Malgun Gothic").is_none() {
        return; // no Korean face available (e.g. a runner with no CJK fonts)
    }
    let mut shifts: Vec<String> = Vec::new();
    for size in [10.5_f64, 20.0_f64] {
        let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: EOJEOL_SENTENCE.to_string(),
                style: TextStyle {
                    font_family: Some("Malgun Gothic".to_string()),
                    east_asian_font_family: Some("Malgun Gothic".to_string()),
                    font_size: Some(size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })])]);
        let result = generate_typst(&doc).unwrap().source;
        let (_top_em, bottom_em) = emitted_line_box_em(&result).expect("fixed line box");
        let expected: String = format!("#box(baseline: {}pt)", format_f64(bottom_em * size));
        assert!(
            result.contains(&expected),
            "a {size}pt paragraph should shift by {expected}: {result}"
        );
        shifts.push(expected);
    }
    assert_ne!(
        shifts[0], shifts[1],
        "the shift must not be a single measured constant"
    );
}

/// Letter spacing crosses a frame boundary by a rule that is not one step per
/// item — measured on typst 0.14, framing a 13pt tracked heading's words made
/// it narrower and a 9pt one's wider — so a tracked run keeps today's
/// emission rather than a guessed correction.
#[test]
fn a_letter_spaced_run_is_not_framed() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "활용 설치부터".to_string(),
            style: TextStyle {
                letter_spacing: Some(0.5),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#text(tracking: 0.5pt)[활용 설치부터]"),
        "a tracked run stays one text item: {result}"
    );
    assert!(
        !result.contains("#box["),
        "no frame for tracked text: {result}"
    );
}

#[test]
fn an_unspaced_eojeol_is_still_framed() {
    // Triangulation: the exclusion must key on the spacing, not on the words.
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "활용 설치부터",
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#box[활용] #box[설치부터]"),
        "an unspaced paragraph is framed as usual: {result}"
    );
}

/// Cutting a run at an eojeol boundary can leave a date at the start of the
/// next escaping unit, where Typst reads `2026. ` as an enumeration marker and
/// puts the date on a line of its own — which is what happened to the official
/// letter's `시행일자: 2026. 7. 17.`.
#[test]
fn a_date_after_an_eojeol_is_not_retypeset_as_a_list_item() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "시행일자: 2026. 7. 17.",
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#box[시행일자:] 2026\\. 7. 17."),
        "the date's first dot must be escaped once the frame precedes it: {result}"
    );
}

/// The same hazard without any frame: whichever way a run is cut, the one
/// leading space this function emits literally must not hide the marker.
#[test]
fn a_leading_space_does_not_hide_an_enum_marker() {
    assert_eq!(escape_typst(" 2026. 7. 17."), " 2026\\. 7. 17.");
    assert_eq!(escape_typst("2026. 7. 17."), "2026\\. 7. 17.");
    assert_eq!(
        escape_typst(" 2026 7 17"),
        " 2026 7 17",
        "a bare number is not a marker and must not gain an escape"
    );
    // An indentation run leaves as a code-mode string, which cannot open an
    // enumeration, so it must not gain an escape it does not need.
    assert!(
        escape_typst("      2. indented").ends_with("2. indented"),
        "an indented number keeps its plain dot"
    );
}

/// A token no line could hold would take a frame of its own and start a new
/// line, costing a line Word does not spend. Such a token is not an eojeol.
#[test]
fn a_pathologically_long_token_is_not_framed() {
    let long_token: String = "가".repeat(40);
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        &format!("계약 {long_token} 종료"),
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!("#box[계약] {long_token} #box[종료]")),
        "an over-long token keeps today's syllable breaking: {result}"
    );
}

/// A table cell whose single paragraph names a family and a size, so the
/// eojeol width guard has metrics to measure against (issue #626).
fn make_text_cell_styled(text: &str, family: &str, font_size: f64) -> TableCell {
    TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some(family.to_string()),
                    east_asian_font_family: Some(family.to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    }
}

/// A slide text box whose content is a one-item bullet list.
fn make_text_box_with_list(x: f64, y: f64, w: f64, h: f64, text: &str) -> FixedElement {
    let mut element: FixedElement = make_text_box(x, y, w, h, text);
    if let FixedElementKind::TextBox(ref mut data) = element.kind {
        data.content = vec![Block::List(List {
            kind: ListKind::Unordered,
            items: vec![ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: text.to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                }],
                level: 0,
                start_at: None,
            }],
            level_styles: BTreeMap::new(),
        })];
    }
    element
}

// Typst line-leading markup at an eojeol boundary (issue #626)

/// Cutting a run at an eojeol boundary makes the inter-word text its own
/// escaping unit, so a bare ` + ` reaches `escape_typst` at the start of a
/// content block — where Typst reads it as a numbered-list marker, deletes the
/// `+` from the page and puts a `1.` in the text layer instead.
#[test]
fn a_plus_between_two_eojeol_is_not_retypeset_as_a_list_item() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "런타임 초기화 + 프로필 생성",
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#box[초기화] \\+ #box[프로필]"),
        "the `+` must be escaped once a frame precedes it: {result}"
    );
}

/// The same hazard for `=`, which Typst reads as a heading marker and which
/// was not in the escape set at all.
#[test]
fn an_equals_between_two_eojeol_is_not_retypeset_as_a_heading() {
    let doc = make_doc(vec![make_flow_page(vec![korean_paragraph(
        "부하 시험 = 결과 보고",
        None,
        None,
    )])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#box[시험] \\= #box[결과]"),
        "the `=` must be escaped once a frame precedes it: {result}"
    );
}

/// The whole set of Typst markup that is only meaningful at a line start,
/// exercised directly: through one leading space and at index 0, and only when
/// the marker is really one.
#[test]
fn every_line_leading_marker_is_neutralised_through_one_space() {
    // Bullet, numbered list, heading — the three that need the positional rule.
    assert_eq!(escape_typst(" + "), " \\+ ");
    assert_eq!(escape_typst("+ x"), "\\+ x");
    assert_eq!(escape_typst(" = "), " \\= ");
    assert_eq!(escape_typst("= x"), "\\= x");
    assert_eq!(
        escape_typst(" == "),
        " \\== ",
        "escaping the first equals is enough to break a level-2 heading"
    );
    assert_eq!(escape_typst(" - "), " \\- ");
    assert_eq!(escape_typst(" 2. x"), " 2\\. x");
    // A term list opens with `/`, which is escaped wherever it appears.
    assert_eq!(escape_typst(" / term: x"), " \\/ term: x");

    // Not markers: no trailing whitespace, or a leading run of two spaces that
    // leaves as a code-mode string.
    assert_eq!(escape_typst(" =x"), " =x");
    assert_eq!(escape_typst("+"), "+");
    assert_eq!(escape_typst("a = b"), "a = b");
    assert!(
        escape_typst("  = x").ends_with("= x"),
        "an indented equals keeps its plain form: {}",
        escape_typst("  = x")
    );
}

/// A DOCX list item is a Word paragraph like any other, so its Korean breaks
/// at eojeol too. It used to bypass the whole path by calling `generate_run`
/// directly.
#[test]
fn a_docx_list_item_keeps_each_hangul_eojeol_whole() {
    let doc = make_doc(vec![make_flow_page(vec![Block::List(List {
        kind: ListKind::Unordered,
        items: vec![ListItem {
            content: vec![Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: EOJEOL_SENTENCE.to_string(),
                    style: TextStyle::default(),
                    href: None,
                    footnote: None,
                }],
            }],
            level: 0,
            start_at: None,
        }],
        level_styles: BTreeMap::new(),
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("본 #box[계약은] #box[갑과] #box[을이]"),
        "a list item's Korean should be framed like any other paragraph: {result}"
    );
}

/// A slide's list keeps PowerPoint's own mid-word breaking.
#[test]
fn a_slide_list_item_keeps_syllable_breaking() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![make_text_box_with_list(
            10.0,
            10.0,
            300.0,
            100.0,
            EOJEOL_SENTENCE,
        )],
    )]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(EOJEOL_SENTENCE),
        "PowerPoint breaks a bullet's Korean mid-word: {result}"
    );
    assert!(
        !result.contains("#box["),
        "no eojeol frame may reach a slide list: {result}"
    );
}

/// A token wider than the column cannot break inside its frame, so the frame
/// would take a line of its own and still overflow it. Word breaks such a
/// token at character level, and so must we.
#[test]
fn a_token_wider_than_its_column_is_not_framed() {
    // 20 syllables at 10.5pt Malgun Gothic is 210pt — far wider than the
    // 150pt column, and short enough that a character ceiling would let it
    // through.
    let long_token: String = "가나다라마바사아자차카타파하가나다라마바".to_string();
    // The premise is that the token *measures* over the column, so the guard
    // needs the same advance the generator does. Without a Korean face — every
    // CI runner here — nothing measures and the generator falls back to its
    // character ceiling, which this deliberately 20-character token clears.
    if crate::render::pdf::text_advance_em("Malgun Gothic", false, &long_token).is_none() {
        return; // no Korean face available (e.g. a runner with no CJK fonts)
    }
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell_styled(
                &format!("주 {long_token}"),
                "Malgun Gothic",
                10.5,
            )],
            height: None,
        }],
        column_widths: vec![150.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        !result.contains(&format!("[{long_token}]]")),
        "an over-wide token must keep the engine's syllable breaking: {result}"
    );
}

/// Triangulation for the guard above: the same token in a column wide enough
/// to hold it *is* framed, so the rule keys on width and not on the token.
///
/// Deliberately unguarded, unlike its partner: without a Korean face the token
/// is framed by the character ceiling instead of by the width rule, so the
/// assertion still holds and keeps guarding "a frame is emitted at all" on a
/// runner with no CJK fonts. Only the *width* half of the triangulation needs
/// the face, and that half lives in the test above.
#[test]
fn the_same_token_is_framed_when_the_column_can_hold_it() {
    let long_token: String = "가나다라마바사아자차카타파하가나다라마바".to_string();
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell_styled(
                &format!("주 {long_token}"),
                "Malgun Gothic",
                10.5,
            )],
            height: None,
        }],
        column_widths: vec![400.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!("[{long_token}]]")),
        "a token the column can hold keeps its frame: {result}"
    );
}

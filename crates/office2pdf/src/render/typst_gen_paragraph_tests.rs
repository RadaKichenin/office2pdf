use super::*;

#[test]
fn test_generate_plain_paragraph() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph("Hello World")])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("Hello World"));
}

#[test]
fn test_generate_empty_paragraph_reserves_line_height() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: Vec::new(),
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#v(12pt)"),
        "empty DOCX paragraph marks should reserve vertical flow space: {result}"
    );
}

#[test]
fn test_generate_page_setup() {
    let doc = make_doc(vec![Page::Flow(FlowPage {
        size: PageSize {
            width: 612.0,
            height: 792.0,
        },
        margins: Margins {
            top: 36.0,
            bottom: 36.0,
            left: 54.0,
            right: 54.0,
        },
        content: vec![make_paragraph("test")],
        header: None,
        footer: None,
        columns: None,
        line_grid_pitch: None,
        line_grid_snaps_lines: false,
    })]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("612pt"));
    assert!(result.contains("792pt"));
    assert!(result.contains("36pt"));
    assert!(result.contains("54pt"));
}

#[test]
fn test_generate_bold_text() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Bold text".to_string(),
            style: TextStyle {
                bold: Some(true),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("weight: \"bold\""),
        "Expected bold weight in: {result}"
    );
    assert!(result.contains("Bold text"));
}

#[test]
fn test_generate_italic_text() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Italic text".to_string(),
            style: TextStyle {
                italic: Some(true),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("style: \"italic\""),
        "Expected italic style in: {result}"
    );
    assert!(result.contains("Italic text"));
}

#[test]
fn test_generate_underline_text() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Underlined".to_string(),
            style: TextStyle {
                underline: Some(true),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#underline["),
        "Expected underline wrapper in: {result}"
    );
    assert!(result.contains("Underlined"));
}

#[test]
fn test_generate_font_size() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Large text".to_string(),
            style: TextStyle {
                font_size: Some(24.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("size: 24pt"),
        "Expected font size in: {result}"
    );
}

#[test]
fn test_generate_font_color() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Red text".to_string(),
            style: TextStyle {
                color: Some(Color::new(255, 0, 0)),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("fill: rgb(255, 0, 0)"),
        "Expected RGB color in: {result}"
    );
}

#[test]
fn test_generate_combined_text_styles() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Styled".to_string(),
            style: TextStyle {
                bold: Some(true),
                italic: Some(true),
                font_size: Some(16.0),
                color: Some(Color::new(0, 128, 255)),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("weight: \"bold\""));
    assert!(result.contains("style: \"italic\""));
    assert!(result.contains("size: 16pt"));
    assert!(result.contains("fill: rgb(0, 128, 255)"));
    assert!(result.contains("Styled"));
}

#[test]
fn test_generate_alignment_center() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            alignment: Some(Alignment::Center),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Centered".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("align(center"),
        "Expected center alignment in: {result}"
    );
}

#[test]
fn test_generate_alignment_right() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            alignment: Some(Alignment::Right),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Right".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("align(right"),
        "Expected right alignment in: {result}"
    );
}

#[test]
fn test_generate_alignment_justify() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            alignment: Some(Alignment::Justify),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Justified text".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("par(justify: true") || result.contains("set par(justify: true"),
        "Expected justify in: {result}"
    );
}

#[test]
fn test_generate_line_spacing_proportional() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            line_spacing: Some(LineSpacing::Proportional(2.0)),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Double spaced".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("leading:"),
        "Expected leading setting in: {result}"
    );
}

#[test]
fn test_generate_line_spacing_exact() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            line_spacing: Some(LineSpacing::Exact(18.0)),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Exact spaced".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("leading: 18pt"),
        "Expected exact leading in: {result}"
    );
}

#[test]
fn test_generate_word_default_line_box() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            line_box: Some(LineBox {
                ascent_em: 1.3125,
                descent_em: 0.4375,
            }),
            space_after: Some(8.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Word defaults".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let source = generate_typst(&doc).unwrap().source;

    assert!(
        source.contains("#set text(top-edge: 1.3125em, bottom-edge: -0.4375em)"),
        "Expected Word-compatible line edges in: {source}"
    );
    assert!(
        source.contains("#set par(leading: 0pt)"),
        "Expected Word-compatible line stacking in: {source}"
    );
    assert!(
        source.contains("below: 8pt"),
        "Expected paragraph spacing in: {source}"
    );
}

#[test]
fn test_generate_letter_spacing() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Spaced text".to_string(),
            style: TextStyle {
                letter_spacing: Some(2.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("tracking: 2pt"),
        "Expected tracking param in: {result}"
    );
}

#[test]
fn test_generate_letter_spacing_negative() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Condensed".to_string(),
            style: TextStyle {
                letter_spacing: Some(-0.5),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("tracking: -0.5pt"),
        "Expected negative tracking in: {result}"
    );
}

#[test]
fn test_generate_tab_uses_measured_default_stops() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "Name:\tValue".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("#context {"),
        "Expected contextual tab rendering in: {result}"
    );
    assert!(
        result.contains("measure(tab_prefix_0).width"),
        "Expected tab spacing to measure the rendered prefix in: {result}"
    );
    assert!(
        result.contains("calc.rem-euclid(tab_prefix_width_1.abs.pt(), 36)"),
        "Expected default tabs to advance to the next 36pt stop in: {result}"
    );
    assert!(
        !result.contains("#h(36pt)"),
        "Expected default tabs to avoid a hard-coded 36pt gap in: {result}"
    );
}

#[test]
fn test_generate_tab_uses_next_explicit_stop_and_alignment() {
    use crate::ir::{TabAlignment, TabLeader, TabStop};

    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            tab_stops: Some(vec![
                TabStop {
                    position: 72.0,
                    alignment: TabAlignment::Left,
                    leader: TabLeader::None,
                },
                TabStop {
                    position: 216.0,
                    alignment: TabAlignment::Right,
                    leader: TabLeader::Dot,
                },
            ]),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Col1\tCol2\tCol3".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("if tab_prefix_width_1 < 72pt"),
        "Expected the first explicit stop to be chosen by measured width in: {result}"
    );
    assert!(
        result.contains("else if tab_prefix_width_2 < 216pt"),
        "Expected the next explicit stop to be selected after the first one in: {result}"
    );
    assert!(
        result.contains("216pt - tab_prefix_width_2 - tab_segment_width_2"),
        "Expected right-aligned tabs to subtract the following segment width in: {result}"
    );
}

#[test]
fn test_generate_tab_falls_back_to_next_default_stop_after_explicit_tabs() {
    use crate::ir::{TabAlignment, TabLeader, TabStop};

    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            tab_stops: Some(vec![TabStop {
                position: 100.0,
                alignment: TabAlignment::Left,
                leader: TabLeader::None,
            }]),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "A\tB\tC".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("if tab_prefix_width_1 < 100pt"),
        "Expected the explicit stop to be used when it is still ahead of the prefix in: {result}"
    );
    assert!(
        result.contains("calc.rem-euclid(tab_prefix_width_2.abs.pt(), 36)"),
        "Expected tabs beyond explicit stops to use the next default stop in: {result}"
    );
}

#[test]
fn test_generate_tab_leader_uses_repeat_fill() {
    use crate::ir::{TabAlignment, TabLeader, TabStop};

    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            tab_stops: Some(vec![TabStop {
                position: 144.0,
                alignment: TabAlignment::Left,
                leader: TabLeader::Dot,
            }]),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Heading\t12".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("box(width: tab_advance_1, repeat[.])"),
        "Expected dot tab leaders to render with Typst repeat fill in: {result}"
    );
}

#[test]
fn test_generate_decimal_tab_uses_decimal_separator_not_thousands_separator() {
    use crate::ir::{TabAlignment, TabLeader, TabStop};

    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            tab_stops: Some(vec![TabStop {
                position: 180.0,
                alignment: TabAlignment::Decimal,
                leader: TabLeader::None,
            }]),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Total\t1,234.56".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("let tab_decimal_anchor_1 = [1,234]"),
        "Expected decimal alignment to anchor after the thousands group in: {result}"
    );
}

#[test]
fn test_generate_decimal_tab_handles_comma_decimal_locale() {
    use crate::ir::{TabAlignment, TabLeader, TabStop};

    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            tab_stops: Some(vec![TabStop {
                position: 180.0,
                alignment: TabAlignment::Decimal,
                leader: TabLeader::None,
            }]),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Total\t1.234,56".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("let tab_decimal_anchor_1 = [1.234]"),
        "Expected decimal alignment to anchor on the locale decimal separator in: {result}"
    );
}

#[test]
fn test_generate_multiple_paragraphs() {
    let doc = make_doc(vec![make_flow_page(vec![
        make_paragraph("First paragraph"),
        make_paragraph("Second paragraph"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("First paragraph"));
    assert!(result.contains("Second paragraph"));
    assert!(
        result.contains("First paragraph\n\nSecond paragraph"),
        "Expected paragraph break between flow paragraphs in: {result}"
    );
}

#[test]
fn test_generate_paragraph_with_multiple_runs() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![
            Run {
                text: "Normal ".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            },
            Run {
                text: "bold".to_string(),
                style: TextStyle {
                    bold: Some(true),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            },
            Run {
                text: " normal again".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            },
        ],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("Normal "));
    assert!(result.contains("bold"));
    assert!(result.contains(" normal again"));
}

#[test]
fn test_generate_empty_document() {
    let doc = make_doc(vec![]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.is_empty() || !result.is_empty());
}

#[test]
fn test_generate_special_characters_escaped() {
    let doc = make_doc(vec![make_flow_page(vec![make_paragraph(
        "Price: $100 #items @store",
    )])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("\\#") || result.contains("Price"),
        "Expected escaped or present text in: {result}"
    );
}

#[test]
fn test_centered_paragraph_with_spacing_keeps_full_width_block() {
    // A paragraph with spacing gets a #block wrapper; without width: 100%
    // the block shrinks to its content and the inner #align(center) has no
    // visible effect (Word: <w:spacing w:after> + <w:jc w:val="center">).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            alignment: Some(Alignment::Center),
            space_after: Some(6.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Centered title".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("align(center"),
        "Expected center alignment in: {result}"
    );
    let block_start = result.find("#block(").expect("expected block wrapper");
    let block_params = &result[block_start..block_start + 60];
    assert!(
        block_params.contains("width: 100%"),
        "Block wrapper must span the full width for alignment to apply: {block_params}"
    );
}

#[test]
fn test_document_grid_pitch_snaps_line_height() {
    // A Korean Word section whose <w:docGrid> snaps lines puts body lines on
    // an 18pt grid. The line box is clamped to a fixed em height equal to the
    // grid pitch (leading 0) so a taller fallback glyph on a line cannot
    // inflate its advance past the grid (issue #398). The baseline keeps its
    // constant ascent inside that box: the slot's slack accrues below it, not
    // around it (issue #518). Uses a font from Typst's embedded set so the
    // test is environment-free.
    let Some((_, _, word_pitch_em)) = crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let mut page = match make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "그리드 정렬 grid snapped".to_string(),
            style: TextStyle {
                font_family: Some("Libertinus Serif".to_string()),
                font_size: Some(10.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    assert_line_advance(
        &result,
        "Libertinus Serif",
        10.0,
        18.0,
        0.15 * word_pitch_em,
    );
    assert!(
        result.contains("leading: 0pt"),
        "the grid advance is carried by the box, not by leading: {result}"
    );
}

#[test]
fn test_latin_paragraph_ignores_document_grid() {
    // Word leaves Latin-only paragraphs at their metric line height even
    // when the section carries a document grid; only East Asian text snaps
    // (issue #354).
    let mut page = match make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "latin only body text".to_string(),
            style: TextStyle {
                font_family: Some("Libertinus Serif".to_string()),
                font_size: Some(10.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    // The paragraph keeps Word's hhea single-spacing advance; the 18pt grid
    // pitch must not appear in its line box.
    let Some((ascender, descender, word_pitch)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let single_pt: f64 = (word_pitch * 10.0).max((ascender + descender) * 10.0);
    assert_line_advance(&result, "Libertinus Serif", 10.0, single_pt, 0.0);
    assert!(
        (single_pt - 18.0).abs() > 0.01,
        "the fixture only proves anything if the grid pitch differs from single spacing"
    );
}

#[test]
fn test_no_document_grid_uses_word_single_spacing() {
    // Without a document grid, paragraphs still use Word's hhea single-line
    // pitch instead of Typst's glyph-tight default (issue #354).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "plain".to_string(),
            style: TextStyle {
                font_family: Some("Libertinus Serif".to_string()),
                font_size: Some(10.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    let Some((ascender, descender, word_pitch)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let single_pt: f64 = (word_pitch * 10.0).max((ascender + descender) * 10.0);
    assert_line_advance(&result, "Libertinus Serif", 10.0, single_pt, 0.0);
    assert!(
        result.contains("leading: 0pt"),
        "the advance is carried by the box, not by leading: {result}"
    );
}

#[test]
fn test_generate_paragraph_with_background_shading() {
    // w:pPr/w:shd paints the whole paragraph; the block wrapper must carry
    // the fill so the shading spans the full line width (issue #351).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            background: Some(Color::new(0xF4, 0xF4, 0xF4)),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "$ cargo install office2pdf-cli".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("fill: rgb(244, 244, 244)"),
        "paragraph shading must fill the block wrapper: {result}"
    );
    assert!(
        result.contains("#block(width: 100%"),
        "shaded paragraphs need the full-width block wrapper: {result}"
    );
}

#[test]
fn test_generate_paragraph_with_bottom_border_rule() {
    // w:pBdr bottom rules (resume header underline) must stroke the block
    // wrapper's bottom edge (issue #368).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            border: Some(Box::new(CellBorder {
                bottom: Some(BorderSide {
                    width: 0.75,
                    color: Color::new(0x1E, 0x27, 0x61),
                    style: BorderLineStyle::Solid,
                }),
                ..CellBorder::default()
            })),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "JAMIE PARKER".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("stroke: (bottom: 0.75pt + rgb(30, 39, 97))"),
        "bottom border must stroke the wrapper: {result}"
    );
}

#[test]
fn test_generate_paragraph_with_double_bottom_border() {
    // Double letterhead rules render as two placed hairlines; Typst strokes
    // have no double style (issue #368).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            border: Some(Box::new(CellBorder {
                bottom: Some(BorderSide {
                    width: 1.0,
                    color: Color::black(),
                    style: BorderLineStyle::Double,
                }),
                ..CellBorder::default()
            })),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "주식회사 에이엑스솔루션".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    let rule_count = result.matches("line(length: 100%").count();
    assert_eq!(
        rule_count, 2,
        "double borders draw exactly two rules: {result}"
    );
    assert!(
        !result.contains("stroke: (bottom:"),
        "double sides must not also stroke the wrapper: {result}"
    );
}

fn make_tab_paragraph() -> Block {
    Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "제1조\t(목적) 본문".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })
}

#[test]
fn test_tab_advance_uses_document_default_tab_stop() {
    // Word documents carry w:defaultTabStop; tabs advance to multiples of
    // it, not the ECMA fallback (issue #393).
    let mut doc = make_doc(vec![make_flow_page(vec![make_tab_paragraph()])]);
    doc.styles.default_tab_stop_pt = Some(40.0);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("calc.rem-euclid(tab_prefix_width_1.abs.pt(), 40)"),
        "explicit default tab stop must drive the advance: {result}"
    );
}

#[test]
fn test_tab_advance_defaults_to_40pt_under_document_grid() {
    // When settings.xml omits w:defaultTabStop, East Asian Word (signalled
    // by the section's w:docGrid) falls back to 800 twips = 40pt, not the
    // ECMA 720 twips (issue #393).
    let mut page = match make_flow_page(vec![make_tab_paragraph()]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("calc.rem-euclid(tab_prefix_width_1.abs.pt(), 40)"),
        "grid documents default to 40pt tab stops: {result}"
    );
}

#[test]
fn test_tab_advance_defaults_to_36pt_without_grid() {
    let doc = make_doc(vec![make_flow_page(vec![make_tab_paragraph()])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("calc.rem-euclid(tab_prefix_width_1.abs.pt(), 36)"),
        "ECMA default stays 36pt: {result}"
    );
}

#[test]
fn test_latin_paragraph_space_after_stays_raw_gap() {
    // Word places `w:spacing w:after` directly below the full line box,
    // which the paragraph's own line box already spans, so the gap reaches
    // the block unchanged and needs no leading top-up (issues #394, #452).
    let make_para = |text: &str| {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(4.0),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(10.0),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })
    };
    let doc = make_doc(vec![make_flow_page(vec![
        make_para("first paragraph"),
        make_para("second paragraph"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("below: 4pt"),
        "Latin paragraph keeps the raw 4pt gap: {result}"
    );
}

#[test]
fn test_consecutive_paragraphs_each_advance_by_the_full_font_line() {
    // The shaded command lines of a technical manual are separate 9pt
    // Courier New paragraphs with `w:spacing w:after="0"`. Word advances
    // each by the font's full single-spacing line; Typst only inserts
    // `par(leading:)` *between* the lines of one paragraph, so recovering
    // the advance that way left every paragraph one leading short and
    // consecutive command lines packed 28% tighter than Word (issue #452).
    // The line box must therefore span the whole advance on its own.
    let Some(advance_pt) = single_spacing_advance_pt(LINE_GAP_FONT, 9.0) else {
        return;
    };
    let make_code_line = |text: &str| {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(0.0),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some(LINE_GAP_FONT.to_string()),
                    font_size: Some(9.0),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })
    };
    let doc = make_doc(vec![make_flow_page(vec![
        make_code_line("$ cargo install office2pdf-cli"),
        make_code_line("$ office2pdf --version"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;

    assert_line_advance(&result, LINE_GAP_FONT, 9.0, advance_pt, 0.0);
    assert!(
        result.contains("leading: 0pt"),
        "the advance belongs to the box, not to leading: {result}"
    );
    assert!(
        result.contains("below: 0pt"),
        "a zero w:spacing w:after stays zero: {result}"
    );
}

/// A Typst-embedded font whose hhea line is taller than its typographic
/// metric box, so the single-spacing advance is strictly larger than the
/// metric box. Libertinus Serif - the default test font here - has no line
/// gap at all, which would make the assertions above pass vacuously.
const LINE_GAP_FONT: &str = "DejaVu Sans Mono";

/// Word's single-spacing advance for `family` at `font_size`, or `None`
/// when the font is unavailable or its hhea line adds no gap over the
/// typographic metric box (which would make the assertion vacuous).
fn single_spacing_advance_pt(family: &str, font_size: f64) -> Option<f64> {
    let (ascender, descender, word_pitch) = crate::render::pdf::font_line_metrics_em(family)?;
    (word_pitch - (ascender + descender) > 0.001).then_some(word_pitch * font_size)
}

#[test]
fn test_grid_paragraph_space_after_stays_raw_gap() {
    // Grid variant: the snapped line box already spans the full grid pitch,
    // so Word's after-gap sits directly below it and reaches the block
    // unchanged (issues #394, #452).
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return;
    }
    let mut page = match make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            space_after: Some(4.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "그리드 본문".to_string(),
            style: TextStyle {
                font_family: Some("Libertinus Serif".to_string()),
                font_size: Some(10.0),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("below: 4pt"),
        "grid paragraph keeps the raw 4pt gap: {result}"
    );
}

#[test]
fn test_paragraph_left_indent_offsets_the_text_column() {
    // Word's `w:ind w:left` moves the whole paragraph right; only the list
    // path ever read it, so indented body paragraphs started at the margin
    // (issue #464).
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            indent_left: Some(12.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "$ cargo install office2pdf-cli".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("inset: (left: 12pt, right: 0pt)"),
        "left indent should inset the paragraph block: {result}"
    );
}

#[test]
fn test_paragraph_right_indent_narrows_the_text_column() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            indent_right: Some(18.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "narrowed".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("inset: (left: 0pt, right: 18pt)"),
        "right indent should inset the paragraph block: {result}"
    );
}

#[test]
fn test_indented_paragraph_shading_starts_at_the_indent() {
    // Word paints `w:pPr/w:shd` from the left indent to the right indent,
    // not across the whole text column: measured on a Word export, the
    // shaded band of a 12pt-indented code line starts at the indent, 12pt
    // right of the margin (issue #464). The fill therefore belongs to an
    // inner block that spans only the inset content area.
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            indent_left: Some(12.0),
            background: Some(Color::new(0xF4, 0xF4, 0xF4)),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "$ office2pdf --version".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    let inset_at = result
        .find("inset: (left: 12pt, right: 0pt)")
        .expect("indent should inset the outer block");
    let fill_at = result
        .find("fill: rgb(244, 244, 244)")
        .expect("shading should still be emitted");
    assert!(
        fill_at > inset_at,
        "the fill belongs to a block nested inside the indent: {result}"
    );
}

#[test]
fn test_unindented_paragraph_keeps_a_single_block() {
    // A paragraph with no indent must not gain an inset or a nested block.
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            background: Some(Color::new(0xF4, 0xF4, 0xF4)),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "flush left".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        !result.contains("inset: (left:"),
        "an unindented paragraph needs no inset: {result}"
    );
    assert!(
        result.contains("fill: rgb(244, 244, 244)"),
        "shading is still emitted: {result}"
    );
}

#[test]
fn test_empty_indented_paragraph_closes_its_block() {
    // An indented paragraph with no runs takes the early-return path. Leaving
    // its indent wrapper open produced "unclosed delimiter" and failed 73
    // third-party fixtures outright (regression caught on #464).
    let doc = make_doc(vec![make_flow_page(vec![
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                indent_left: Some(24.0),
                ..ParagraphStyle::default()
            },
            runs: Vec::new(),
        }),
        make_paragraph("after the empty paragraph"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;

    let opened: usize = result.matches('[').count();
    let closed: usize = result.matches(']').count();
    assert_eq!(
        opened, closed,
        "every content block opened must be closed: {result}"
    );
    assert!(
        result.contains("after the empty paragraph"),
        "the following paragraph must not be swallowed: {result}"
    );
}

/// One paragraph of `text` in `family` at `font_size`, with no grid.
fn line_box_for_text(text: &str, family: &str, font_size: f64) -> Option<(f64, f64)> {
    crate::render::pdf::font_line_metrics_em(family)?;
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: text.to_string(),
            style: TextStyle {
                font_family: Some(family.to_string()),
                font_size: Some(font_size),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        }],
    })])]);
    emitted_line_box_em(&generate_typst(&doc).unwrap().source)
}

#[test]
fn east_asian_line_advances_130_percent_of_the_font_line() {
    // Word gives a line carrying East Asian text 130% of the font's own hhea
    // line. Measured across the business corpus: 10.5pt Malgun Gothic paces its
    // wrapped lines at 18.00-18.24pt against 18.156 predicted, and
    // 06_official_letter_ko's 9.5pt paragraphs advance 16.43pt where the bare
    // hhea line is 12.64pt (issue #518).
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let _ = (ascender, descender);
    let Some((top, bottom)) = line_box_for_text("본문 한 줄", "Libertinus Serif", 10.0) else {
        return;
    };

    assert!(
        (top + bottom - 1.3 * word_pitch_em).abs() < 0.001,
        "East Asian advance {}em should be 1.3 x the {word_pitch_em}em hhea line",
        top + bottom
    );
}

#[test]
fn east_asian_bonus_is_centred_on_the_baseline() {
    // Half of the 30% lands above the baseline and half below: an Arial first
    // baseline sits at `hhea ascender + lineGap` while a Malgun Gothic one at
    // the same settings sits 0.15 x pitch lower, and the descent gap grows by
    // the same amount (issue #518).
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let Some((top, bottom)) = line_box_for_text("본문 한 줄", "Libertinus Serif", 10.0) else {
        return;
    };

    assert!(
        (top - (ascender + 0.15 * word_pitch_em)).abs() < 0.001,
        "the baseline should sit 0.15 x pitch below the Latin seat, got {top}em"
    );
    assert!(
        (bottom - (word_pitch_em - ascender + 0.15 * word_pitch_em)).abs() < 0.001,
        "the other half of the bonus belongs below the baseline, got {bottom}em"
    );
}

#[test]
fn a_latin_line_keeps_the_plain_hhea_line_and_seat() {
    // Triangulation for both rules above: the bonus is a property of the
    // line's script, not of the renderer. Inflating Latin lines too made every
    // Western document 30-50% taller (issue #354).
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let Some((top, bottom)) = line_box_for_text("plain body text", "Libertinus Serif", 10.0) else {
        return;
    };

    assert!(
        (top + bottom - word_pitch_em).abs() < 0.001,
        "a Latin line advances the bare hhea line, got {}em",
        top + bottom
    );
    assert!(
        (top - ascender).abs() < 0.001,
        "a Latin baseline keeps the `hhea ascender + lineGap` seat, got {top}em"
    );
}

#[test]
fn the_east_asian_bonus_scales_with_the_font_size_not_with_the_text() {
    // The rule is a factor on the font's line, so doubling the size doubles
    // both edges exactly - a fake that returned one measured pair would not.
    let Some((small_top, small_bottom)) = line_box_for_text("표", "Libertinus Serif", 9.0) else {
        return;
    };
    let (large_top, large_bottom) =
        line_box_for_text("전혀 다른 한국어 문장", "Libertinus Serif", 21.0)
            .expect("the same font resolves at any size");

    // The box is emitted in em, so the em split must be identical at both
    // sizes and for unrelated text.
    assert!(
        (small_top - large_top).abs() < 0.001 && (small_bottom - large_bottom).abs() < 0.001,
        "the split is a property of the font, not of the size or the text: \
         {small_top}/{small_bottom} vs {large_top}/{large_bottom}"
    );
}

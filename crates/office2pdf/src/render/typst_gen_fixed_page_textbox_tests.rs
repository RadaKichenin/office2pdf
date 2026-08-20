use super::*;

fn assert_powerpoint_grid_words_in_order(source: &str, words: &[&str]) {
    let mut remainder = source;
    for word in words {
        let needle = format!("#o2p-pptx-word([{}]", escape_typst(word));
        let offset = remainder
            .find(&needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in:\n{source}"));
        remainder = &remainder[offset + needle.len()..];
    }
}

#[test]
fn test_fixed_page_text_box() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_text_box(100.0, 200.0, 300.0, 50.0, "Slide Title")],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert_powerpoint_grid_words_in_order(&output.source, &["Slide", "Title"]);
    assert!(output.source.contains("100pt"));
    assert!(output.source.contains("200pt"));
}

#[test]
fn test_fixed_page_text_box_uses_padding_and_center_vertical_align() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            100.0,
            200.0,
            300.0,
            50.0,
            Insets {
                top: 3.6,
                right: 7.2,
                bottom: 3.6,
                left: 7.2,
            },
            crate::ir::TextBoxVerticalAlign::Center,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: "Centered".to_string(),
                    style: TextStyle::default(),
                    href: None,
                    footnote: None,
                }],
            })],
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output
            .source
            .contains("inset: (top: 3.6pt, right: 7.2pt, bottom: 3.6pt, left: 7.2pt)")
    );
    assert!(output.source.contains("width: 285.6pt"));
    assert!(output.source.contains(
        "#context {\n    let text_box_slack_0 = calc.max(42.8pt - measure(text_box_content_0).height, 0pt)"
    ));
    assert!(output.source.contains("#v(text_box_slack_0 / 2)"));
    assert!(output.source.contains("let text_box_aligned_0 = ["));
}

#[test]
fn test_fixed_page_text_box_multiple_paragraphs_preserve_breaks() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 300.0,
            height: 100.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![
                    Block::Paragraph(Paragraph {
                        style: ParagraphStyle::default(),
                        runs: vec![Run {
                            text: "First item".to_string(),
                            style: TextStyle::default(),
                            href: None,
                            footnote: None,
                        }],
                    }),
                    Block::Paragraph(Paragraph {
                        style: ParagraphStyle::default(),
                        runs: vec![Run {
                            text: "Second item".to_string(),
                            style: TextStyle::default(),
                            href: None,
                            footnote: None,
                        }],
                    }),
                ],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert_powerpoint_grid_words_in_order(&output.source, &["First", "item", "Second", "item"]);
    assert_eq!(
        output
            .source
            .matches("#block(above: 0pt, below: 0pt)")
            .count(),
        2,
        "each paragraph should retain a separate block: {}",
        output.source
    );
}

#[test]
fn test_fixed_page_text_box_ordered_list_preserves_textbox_styling() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 300.0,
            height: 100.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    line_spacing: Some(LineSpacing::Proportional(1.5)),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: " First item".to_string(),
                                    style: TextStyle {
                                        font_size: Some(24.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: Some(1),
                        },
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    line_spacing: Some(LineSpacing::Proportional(1.5)),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: " Second item".to_string(),
                                    style: TextStyle {
                                        font_size: Some(24.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: None,
                        },
                    ],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1.".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(!output.source.contains("#enum("));
    assert_powerpoint_grid_words_in_order(
        &output.source,
        &["1.", "First", "item", "2.", "Second", "item"],
    );
    assert_eq!(
        output.source.matches("#text(size: 24pt)[").count(),
        4,
        "both markers and both item bodies retain their 24pt style: {}",
        output.source
    );
    assert!(!output.source.contains("\\\n2. Second item"));
    // A slide list paces on PowerPoint's line box, scaled by the declared
    // `a:lnSpc`: 1.2em at 24pt is 28.8pt, and 1.5 line spacing makes each
    // item's box 43.2pt — which is the advance, since nothing is emitted
    // between items that declare no spacing (issue #934). The percentage grows
    // the line from its top, so the face keeps its plain descent gap and the
    // ascent takes the whole 14.4pt the box gains (issue #1020), and the seat
    // lands on a whole point (issue #1074). The expectation is derived rather
    // than spelled out because the default face decides the share.
    let size_pt: f64 = 24.0;
    let (share_top, _) =
        crate::render::pdf::powerpoint_line_box_em(crate::defaults::TYPST_DEFAULT_FONT_FAMILY)
            .expect("the default family's line metrics must resolve");
    let seat_pt: f64 = ((1.5 * 1.2 - (1.2 - share_top)) * size_pt).round();
    let expected: String = format!(
        "#set text(top-edge: {}pt, bottom-edge: -{}pt)",
        crate::render::typst_gen::fmt::format_f64(seat_pt),
        crate::render::typst_gen::fmt::format_f64(1.5 * 1.2 * size_pt - seat_pt),
    );
    assert!(
        output.source.contains(&expected),
        "the item takes the 1.5-scaled PowerPoint line box `{expected}`: {}",
        output.source
    );
    assert!(
        output.source.contains("#set par(leading: 0pt)"),
        "the line box carries the advance, so the leading is zero: {}",
        output.source
    );
}

#[test]
fn test_fixed_page_text_box_compact_list_items_use_full_width_blocks() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle::default(),
                                runs: vec![Run {
                                    text: "Long first item that should wrap inside the fixed text box width".to_string(),
                                    style: TextStyle {
                                        font_size: Some(20.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: Some(1),
                        },
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle::default(),
                                runs: vec![Run {
                                    text: "Long second item that should also wrap inside the fixed text box width".to_string(),
                                    style: TextStyle {
                                        font_size: Some(20.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: None,
                        },
                    ],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1)".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                    no_wrap: false,
                auto_fit: false,
            text_rotation_deg: None,
            shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    // The items' blocks contribute no spacing of their own; the gap between
    // them is emitted between them instead (issue #934).
    assert_eq!(
        output
            .source
            .matches("#block(width: 320pt, above: 0pt, below: 0pt)[")
            .count(),
        2
    );
}

#[test]
fn test_fixed_page_text_box_compact_list_preserves_hanging_indent() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![ListItem {
                        content: vec![Paragraph {
                            style: ParagraphStyle {
                                indent_left: Some(36.0),
                                indent_first_line: Some(-36.0),
                                ..ParagraphStyle::default()
                            },
                            runs: vec![Run {
                                text: "Long first item that should wrap under the body text instead of the number".to_string(),
                                style: TextStyle {
                                    font_size: Some(20.0),
                                    ..TextStyle::default()
                                },
                                href: None,
                                footnote: None,
                            }],
                        }],
                        level: 0,
                        start_at: Some(1),
                    }],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1)".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                    no_wrap: false,
                auto_fit: false,
            text_rotation_deg: None,
            shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    assert!(
        output
            .source
            .contains("#grid(columns: (36pt, 1fr), gutter: 0pt,"),
        "Expected ordered hanging-indent list to use a marker/body grid, got:\n{}",
        output.source,
    );
    assert!(!output.source.contains("hanging-indent: 36pt"));
    assert!(!output.source.contains("tab_advance_1"));
}

#[test]
fn test_fixed_page_text_box_compact_list_preserves_marker_origin_offset() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![ListItem {
                        content: vec![Paragraph {
                            style: ParagraphStyle {
                                indent_left: Some(54.0),
                                indent_first_line: Some(-36.0),
                                ..ParagraphStyle::default()
                            },
                            runs: vec![Run {
                                text: "Marker origin should stay inset while wrapped lines align to the text column"
                                    .to_string(),
                                style: TextStyle {
                                    font_size: Some(20.0),
                                    ..TextStyle::default()
                                },
                                href: None,
                                footnote: None,
                            }],
                        }],
                        level: 0,
                        start_at: Some(1),
                    }],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1)".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                    no_wrap: false,
                auto_fit: false,
            text_rotation_deg: None,
            shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    assert!(
        output
            .source
            .contains("inset: (top: 0pt, right: 0pt, bottom: 0pt, left: 18pt)")
    );
    assert!(
        output
            .source
            .contains("#grid(columns: (36pt, 1fr), gutter: 0pt,")
    );
}

#[test]
fn test_fixed_page_text_box_compact_bulleted_list_uses_custom_marker_style() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Unordered,
                    items: vec![ListItem {
                        content: vec![Paragraph {
                            style: ParagraphStyle {
                                indent_left: Some(22.5),
                                indent_first_line: Some(-22.5),
                                ..ParagraphStyle::default()
                            },
                            runs: vec![Run {
                                text: "Symbol bullet".to_string(),
                                style: TextStyle {
                                    font_family: Some("Pretendard".to_string()),
                                    font_size: Some(14.0),
                                    ..TextStyle::default()
                                },
                                href: None,
                                footnote: None,
                            }],
                        }],
                        level: 0,
                        start_at: None,
                    }],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Unordered,
                            numbering_pattern: None,
                            full_numbering: false,
                            marker_text: Some("è".to_string()),
                            marker_style: Some(TextStyle {
                                font_family: Some("Wingdings".to_string()),
                                font_size: Some(14.0),
                                ..TextStyle::default()
                            }),
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    assert!(!output.source.contains("Wingdings"));
    assert!(output.source.contains("➔"));
    assert!(output.source.contains("tab_advance_1"));
    assert_powerpoint_grid_words_in_order(&output.source, &["Symbol", "bullet"]);
}

#[test]
fn test_escape_typst_escapes_leading_dash_list_prefix() {
    assert_eq!(escape_typst("- bullet"), "\\- bullet");
}

#[test]
fn test_fixed_page_text_box_dash_bullets_use_generic_list_path() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Unordered,
                    items: vec![
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    indent_left: Some(22.5),
                                    indent_first_line: Some(-22.5),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: "First dash bullet".to_string(),
                                    style: TextStyle {
                                        font_family: Some("Pretendard".to_string()),
                                        font_size: Some(14.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: None,
                        },
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    indent_left: Some(22.5),
                                    indent_first_line: Some(-22.5),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: "Second dash bullet".to_string(),
                                    style: TextStyle {
                                        font_family: Some("Pretendard".to_string()),
                                        font_size: Some(14.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: None,
                        },
                    ],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Unordered,
                            numbering_pattern: None,
                            full_numbering: false,
                            marker_text: Some("-".to_string()),
                            marker_style: Some(TextStyle {
                                font_family: Some("Pretendard".to_string()),
                                font_size: Some(14.0),
                                ..TextStyle::default()
                            }),
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    assert!(output.source.contains("#list("));
    assert!(output.source.contains("marker: ["));
    assert!(!output.source.contains("tab_advance_1"));
}

#[test]
fn test_fixed_page_text_box_compact_list_preserves_soft_line_breaks() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![ListItem {
                        content: vec![Paragraph {
                            style: ParagraphStyle::default(),
                            runs: vec![Run {
                                text: "Line 1\u{000B}Line 2".to_string(),
                                style: TextStyle {
                                    font_size: Some(20.0),
                                    ..TextStyle::default()
                                },
                                href: None,
                                footnote: None,
                            }],
                        }],
                        level: 0,
                        start_at: Some(1),
                    }],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1)".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();

    assert!(output.source.contains("#linebreak()"));
    assert!(output.source.contains("#set text(size: 20pt"));
    assert!(output.source.contains("leading: 13pt"));
}

#[test]
fn test_fixed_page_text_box_with_width_height() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_text_box(50.0, 60.0, 400.0, 100.0, "Sized box")],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(output.source.contains("400pt"));
    assert!(output.source.contains("100pt"));
}

#[test]
fn test_fixed_page_text_box_with_solid_fill() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 300.0,
            height: 50.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "White BG".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: Some(Color {
                    r: 255,
                    g: 255,
                    b: 255,
                }),
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("fill: rgb(255, 255, 255)"),
        "Expected white fill in output, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_with_fill_and_stroke() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 50.0,
            y: 80.0,
            width: 200.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Bordered".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: Some(Color {
                    r: 200,
                    g: 220,
                    b: 240,
                }),
                opacity: None,
                stroke: Some(BorderSide {
                    width: 1.0,
                    color: Color { r: 0, g: 0, b: 0 },
                    style: BorderLineStyle::Solid,
                    join: LineJoin::Round,
                }),
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("fill: rgb(200, 220, 240)"),
        "Expected fill color in output, got:\n{}",
        output.source,
    );
    assert!(
        output
            .source
            .contains("stroke: (paint: rgb(0, 0, 0), thickness: 1pt, join: \"round\")"),
        "Expected stroke in output, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_with_fill_and_opacity() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 10.0,
            y: 20.0,
            width: 150.0,
            height: 30.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Semi-transparent".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: Some(Color {
                    r: 255,
                    g: 255,
                    b: 255,
                }),
                opacity: Some(0.5),
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("fill: rgb(255, 255, 255, 128)"),
        "Expected fill with alpha in output, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_with_polygon_shape_kind() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 50.0,
            y: 80.0,
            width: 200.0,
            height: 60.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Arrow Tab".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets {
                    top: 3.6,
                    right: 7.2,
                    bottom: 3.6,
                    left: 7.2,
                },
                vertical_align: crate::ir::TextBoxVerticalAlign::Center,
                fill: Some(Color {
                    r: 0,
                    g: 37,
                    b: 154,
                }),
                opacity: None,
                stroke: None,
                shape_kind: Some(ShapeKind::Polygon {
                    vertices: vec![(0.0, 0.0), (0.85, 0.0), (1.0, 0.5), (0.85, 1.0), (0.0, 1.0)],
                }),
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    // Should contain #polygon for the shape background
    assert!(
        output.source.contains("#polygon("),
        "Expected polygon in output for non-rectangular text box, got:\n{}",
        output.source,
    );
    // The fill should be on the polygon, not the block
    assert!(
        output.source.contains("fill: rgb(0, 37, 154)"),
        "Expected fill color on polygon, got:\n{}",
        output.source,
    );
    // Should NOT have fill on the block itself
    let block_line = output
        .source
        .lines()
        .find(|l| l.contains("#block("))
        .expect("Expected #block line");
    assert!(
        !block_line.contains("fill:"),
        "Block should not have fill when shape_kind is set, got:\n{block_line}",
    );
}

#[test]
fn test_fixed_page_text_box_no_fill_no_stroke() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_text_box(10.0, 20.0, 150.0, 30.0, "Plain")],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("fill: white"),
        "Expected fill: white for no-background slide, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains("stroke:"),
        "Expected no stroke in output, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_uses_place_for_positioning() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_text_box(100.0, 200.0, 300.0, 50.0, "Positioned")],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(output.source.contains("place("));
}

#[test]
fn test_fixed_page_text_box_no_wrap_centered_text_uses_inline_box() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 220.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Centered Title".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("clip: false"),
        "Expected clip: false for no-wrap text box, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("#set align(center)"),
        "Expected centered alignment in output, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("#box["),
        "Expected inline no-wrap box in output, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_keeps_mixed_latin_cjk_searchable() {
    // The heading issue #664 measured: `panic 안전성` came out with a joiner
    // between every character and its space swapped for U+00A0, so searching
    // the PDF for the heading found nothing.
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 180.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "panic 안전성".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("panic 안전성"),
        "Expected the heading with an ordinary space, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_still_strips_the_kinsoku_marker() {
    // Triangulation: the zero-width kinsoku marker must keep being dropped. A
    // no-wrap box never takes that break, and letting U+200B through would
    // trade one invisible text-layer character for another.
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 180.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "\u{200B}제안\u{200B}개요".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("제안개요"),
        "Expected the marker stripped, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains('\u{200B}'),
        "No U+200B may reach the text layer, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_keeps_cjk_title_extractable() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 180.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "제안개요".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    // The enclosing `#box[` already forbids every break inside the paragraph,
    // so no joiner is needed — and one here would reach the PDF text layer and
    // make the title unsearchable (issue #664).
    assert!(
        output.source.contains("제안개요"),
        "Expected the title verbatim in output, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains('\u{2060}'),
        "No U+2060 WORD JOINER may reach the text layer, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains('\u{00A0}'),
        "No U+00A0 NO-BREAK SPACE may reach the text layer, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("#box["),
        "The no-wrap box is what suppresses the breaks, got:\n{}",
        output.source,
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_fixed_page_text_box_no_wrap_keeps_latin_text_extractable() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 180.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Test text".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    let extracted: String = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source))
        .into_iter()
        .map(|run| run.text)
        .collect();
    assert!(
        extracted.contains("Test text"),
        "Expected plain Latin no-wrap text to remain extractable, got {extracted:?}:\n{}",
        output.source
    );
    assert!(
        !output.source.contains('\u{2060}') && !output.source.contains('\u{00A0}'),
        "Expected no invisible joiners or non-breaking spaces for Latin no-wrap text, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_keeps_mixed_script_titles_unbroken() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 320.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "III. 기술부문".to_string(),
                        style: TextStyle {
                            font_size: Some(28.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    // The heading must stay unbreakable *and* readable: the enclosing `#box[`
    // supplies the first, so the text itself is emitted verbatim (issue #664).
    assert!(
        output.source.contains("III. 기술부문"),
        "Expected the mixed-script heading verbatim, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("#box["),
        "The no-wrap box is what keeps the heading unbroken, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains('\u{2060}') && !output.source.contains('\u{00A0}'),
        "No invisible joiner may reach the text layer, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_preserves_mixed_script_titles_across_runs() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 320.0,
            height: 40.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![
                        Run {
                            text: "III.".to_string(),
                            style: TextStyle {
                                font_size: Some(28.0),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                        Run {
                            text: " 기술부문".to_string(),
                            style: TextStyle {
                                font_size: Some(40.0),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                    ],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    // Split across runs, the heading still has to come out as one readable
    // string rather than a joiner-separated one (issue #664).
    assert!(
        output.source.contains("III.") && output.source.contains(" 기술부문"),
        "Expected both runs of the heading verbatim, the second keeping its
         ordinary leading space, got:\n{}",
        output.source,
    );
    assert!(
        !output.source.contains('\u{2060}') && !output.source.contains('\u{00A0}'),
        "No invisible joiner may reach the text layer, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_auto_fit_short_text_uses_scale_to_fit() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 120.0,
            width: 145.0,
            height: 12.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Center),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Server(Cloud VM)".to_string(),
                        style: TextStyle {
                            font_size: Some(18.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: true,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("let text_box_scale_width_0 = (145pt / calc.max(measure(text_box_raw_0).width, 1pt)) * 100%"),
        "Expected width scale calculation, got:\n{}",
        output.source,
    );
    assert!(
        output
            .source
            .contains("let text_box_scale_height_0 = (12pt / 21.599999999999998pt) * 100%"),
        "Expected estimated line-height scale calculation, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("let text_box_scale_0 = calc.min(100%, calc.min(text_box_scale_width_0, text_box_scale_height_0))"),
        "Expected combined width/height scale clamp, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains(
            "#scale(x: text_box_scale_0, y: text_box_scale_0, origin: top + left, reflow: true)["
        ),
        "Expected scale-to-fit wrapper, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("#align(center)["),
        "Expected center alignment wrapper, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_no_wrap_auto_fit_uses_scale_to_fit() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 295.0,
            y: 78.0,
            width: 143.16,
            height: 58.15,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![
                        Run {
                            text: "- ".to_string(),
                            style: TextStyle {
                                font_size: Some(41.99),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                        Run {
                            text: "목 차 ".to_string(),
                            style: TextStyle {
                                font_size: Some(41.99),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                        Run {
                            text: "-".to_string(),
                            style: TextStyle {
                                font_size: Some(41.99),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                    ],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: true,
                auto_fit: true,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#let text_box_raw_0 = ["),
        "Expected no-wrap auto-fit title to use raw single-line measurement, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("let text_box_scale_width_0 ="),
        "Expected no-wrap auto-fit title to compute width scale, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("let text_box_scale_height_0 ="),
        "Expected no-wrap auto-fit title to compute height scale, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains(
            "#scale(x: text_box_scale_0, y: text_box_scale_0, origin: top + left, reflow: true)["
        ),
        "Expected no-wrap auto-fit title to use scale-to-fit, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_mixed_font_header_uses_scale_to_fit() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 0.0,
            y: 2.4,
            width: 474.5,
            height: 57.9,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![
                        Run {
                            text: "3. 시스템 연동 방안".to_string(),
                            style: TextStyle {
                                font_size: Some(25.0),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                        Run {
                            text: "| 클라우드 기반 업무 시스템 연동".to_string(),
                            style: TextStyle {
                                font_size: Some(16.0),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                    ],
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Center,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#let text_box_raw_0 = ["),
        "Expected raw single-line content wrapper, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains(
            "#scale(x: text_box_scale_0, y: text_box_scale_0, origin: top + left, reflow: true)["
        ),
        "Expected mixed-font header to use scale-to-fit, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_mixed_font_header_with_tight_leading_uses_scale_to_fit() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 0.0,
            y: 2.4,
            width: 474.5,
            height: 57.9,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        line_spacing: Some(LineSpacing::Proportional(0.585)),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![
                        Run {
                            text: "3. 시스템 연동 방안".to_string(),
                            style: TextStyle {
                                font_size: Some(24.99),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                        Run {
                            text: "|  클라우드 기반 업무 시스템 연동".to_string(),
                            style: TextStyle {
                                font_size: Some(16.0),
                                ..TextStyle::default()
                            },
                            href: None,
                            footnote: None,
                        },
                    ],
                })],
                padding: Insets {
                    top: 3.6,
                    right: 7.2,
                    bottom: 3.6,
                    left: 7.2,
                },
                vertical_align: crate::ir::TextBoxVerticalAlign::Center,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("#let text_box_raw_0 = ["),
        "Expected mixed-font header to use raw single-line measurement, got:\n{}",
        output.source,
    );
    assert!(
        output
            .source
            .contains("let text_box_scale_0 = calc.min(100%, calc.min(text_box_scale_width_0, text_box_scale_height_0))"),
        "Expected mixed-font header to use combined scale-to-fit, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_wrapped_centered_paragraph_scales_to_fit_height() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 368.9,
            y: 376.8,
            width: 139.0,
            height: 58.5,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "업무 시스템의 URL 기준으로 문서의 암/복호화 지원".to_string(),
                        style: TextStyle {
                            font_size: Some(18.0),
                            color: Some(Color {
                                r: 255,
                                g: 255,
                                b: 255,
                            }),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets {
                    top: 3.6,
                    right: 7.2,
                    bottom: 3.6,
                    left: 7.2,
                },
                vertical_align: crate::ir::TextBoxVerticalAlign::Center,
                fill: Some(Color {
                    r: 0,
                    g: 120,
                    b: 185,
                }),
                opacity: None,
                stroke: Some(BorderSide {
                    color: Color {
                        r: 0,
                        g: 120,
                        b: 185,
                    },
                    width: 1.0,
                    style: BorderLineStyle::Solid,
                    join: LineJoin::Round,
                }),
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output
            .source
            .contains("#let text_box_raw_0 = block(width: 124.60000000000001pt)["),
        "Expected wrapped paragraph measurement block, got:\n{}",
        output.source,
    );
    assert!(
        output.source.contains("let text_box_scale_0 = calc.min(100%, (51.3pt / calc.max(measure(text_box_raw_0).height, 1pt)) * 100%)"),
        "Expected height-based wrapped paragraph scale clamp, got:\n{}",
        output.source,
    );
}

#[test]
fn test_fixed_page_text_box_ordered_grid_normalizes_marker_spacing() {
    use crate::ir::List;

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 100.0,
            y: 200.0,
            width: 320.0,
            height: 140.0,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::List(List {
                    kind: ListKind::Ordered,
                    items: vec![
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    indent_left: Some(36.0),
                                    indent_first_line: Some(-36.0),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: " Alpha".to_string(),
                                    style: TextStyle {
                                        font_size: Some(20.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: Some(1),
                        },
                        ListItem {
                            content: vec![Paragraph {
                                style: ParagraphStyle {
                                    indent_left: Some(36.0),
                                    indent_first_line: Some(-36.0),
                                    ..ParagraphStyle::default()
                                },
                                runs: vec![Run {
                                    text: "Beta".to_string(),
                                    style: TextStyle {
                                        font_size: Some(20.0),
                                        ..TextStyle::default()
                                    },
                                    href: None,
                                    footnote: None,
                                }],
                            }],
                            level: 0,
                            start_at: None,
                        },
                    ],
                    level_styles: BTreeMap::from([(
                        0,
                        ListLevelStyle {
                            kind: ListKind::Ordered,
                            numbering_pattern: Some("1.".to_string()),
                            full_numbering: false,
                            marker_text: None,
                            marker_style: None,
                        },
                    )]),
                })],
                padding: Insets::default(),
                vertical_align: crate::ir::TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);
    let output = generate_typst(&doc).unwrap();
    assert!(
        output
            .source
            .contains("#text(size: 20pt)[#o2p-pptx-word([1\\.]"),
        "the first marker keeps its 20pt style: {}",
        output.source
    );
    assert!(
        output
            .source
            .contains("#text(size: 20pt)[#o2p-pptx-word([2\\.]"),
        "the second marker keeps its 20pt style: {}",
        output.source
    );
    assert_eq!(
        output.source.matches("#o2p-pptx-space()").count(),
        2,
        "each marker owns exactly one normalized trailing space: {}",
        output.source
    );
}

// ----- PowerPoint's line model (issue #513) -----

/// A slide text box holding one paragraph of `text` in `family` at `size`.
fn slide_text_box_source(family: &str, size: f64, style: ParagraphStyle) -> Option<String> {
    crate::render::pdf::powerpoint_line_box_em(family)?;
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            100.0,
            100.0,
            400.0,
            200.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Top,
            vec![Block::Paragraph(Paragraph {
                style,
                runs: vec![Run {
                    text: text_for(family),
                    style: TextStyle {
                        font_family: Some(family.to_string()),
                        font_size: Some(size),
                        ..TextStyle::default()
                    },
                    href: None,
                    footnote: None,
                }],
            })],
        )],
    )]);
    Some(generate_typst(&doc).unwrap().source)
}

fn text_for(family: &str) -> String {
    format!("Slide body set in {family}")
}

#[test]
fn slide_text_takes_powerpoints_flat_1_2em_line() {
    // PowerPoint gives every line 1.2 times the font size whatever the font's
    // own metrics say. Measured on native exports from both platforms: one
    // wrapped Arial paragraph advances 20.3657pt over 14 gaps at 17pt
    // (1.1980em) and 28.8400pt over 18 at 24pt (1.2017em). Slide text used to
    // take Word's hhea pitch, which is up to 4% short per line (issue #513).
    let Some(source) = slide_text_box_source("Libertinus Serif", 18.0, ParagraphStyle::default())
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let (top, bottom) = emitted_slide_line_box_em(&source, 18.0)
        .unwrap_or_else(|| panic!("no line box emitted: {source}"));

    assert!(
        (top + bottom - 1.2).abs() < 0.001,
        "a slide line spans 1.2em, got {}em: {source}",
        top + bottom
    );
    assert!(
        source.contains("leading: 0pt"),
        "the advance is carried by the box, not by leading: {source}"
    );
}

#[test]
fn slide_baseline_takes_the_faces_share_of_the_line_box() {
    // The 1.2em line is shared in the proportion the face's own OS/2 `usWin*`
    // pair puts its baseline at — a paragraph naming one family and no
    // paragraph-mark face has only that one font on the line (issue #1176).
    // Seating it by Typst's normalised ascender instead left a bottom-anchored
    // box's last baseline flat on the inset with no descent gap at all (issue
    // #513), and halving the leading of a face that fits the box misses the
    // golden mocks' 28pt Arial titles by a whole point (issues #660, #1118).
    //
    // What lands on the page is that share rounded to a whole point, which is
    // where PowerPoint seats a baseline inside its line box; the share itself
    // is what the rounding is applied to, so it is what this asserts (#1074).
    let size_pt: f64 = 18.0;
    let Some(source) =
        slide_text_box_source("Libertinus Serif", size_pt, ParagraphStyle::default())
    else {
        return;
    };
    let (top, bottom) = emitted_slide_line_box_em(&source, size_pt).expect("line box emitted");
    let (share_top, share_bottom) =
        crate::render::pdf::powerpoint_line_box_em("Libertinus Serif").expect("metrics resolve");
    let expected_top: f64 = (share_top * size_pt).round() / size_pt;
    let expected_bottom: f64 = 1.2 - expected_top;

    assert!(
        (top - expected_top).abs() < 0.001 && (bottom - expected_bottom).abs() < 0.001,
        "expected {expected_top}/{expected_bottom}em, got {top}/{bottom}em: {source}"
    );
    assert!(
        (share_top + share_bottom - 1.2).abs() < 1e-9,
        "the share the rounding starts from spans the 1.2em line: \
         {share_top}/{share_bottom}"
    );
    assert!(
        bottom > 0.01,
        "the descent gap must be real: a bottom-anchored box keeps it below its \
         last baseline, and we used to drop it entirely: {source}"
    );
}

/// A face whose own line overflows the 1.2em box gets the box shared in its
/// own proportion — the reading of the share that no rounding can hide, since
/// halving a *negative* leading moves the baseline the wrong way outright.
///
/// Measured on a native PowerPoint 16.112 export of the #841 Contoso deck,
/// whose titles are set in Posterama Bold (hhea ascender 2134, descender -590
/// per 2048 upem, so its own line is 1.3301em against PowerPoint's 1.2em box).
/// Slide 1 sets that face at 50pt with no `<a:lnSpc>`, and its three baselines
/// pace exactly 60.00pt = 1.2em apart, so the box really is 1.2em there. The
/// export seats the first one 47.06pt below the frame's content top = 0.9411em;
/// the proportional share predicts 0.9401em and halving the leading 0.9770em,
/// which is 1.79pt low (issue #1020).
///
/// Posterama's `usWin*` pair is that same 2134/590, which is the pair the share
/// is really taken from — an hhea line gap plays no part in it either way
/// (issue #1176). This face is one where the two agree, so reading hhea below
/// is equivalent; the assertion under it says so rather than assuming it.
#[test]
fn an_overflowing_face_shares_the_line_box_in_its_own_proportion() {
    // New Computer Modern: hhea 1127/-290 per 1000 upem = a 1.417em line.
    let family: &str = "New Computer Modern";
    let Some((above, below)) = crate::render::pdf::powerpoint_line_box_em(family) else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let Some((word_top_em, descent_em, _)) = crate::render::pdf::font_line_metrics_em(family)
    else {
        return;
    };
    let ascent_em: f64 = crate::render::pdf::font_hhea_ascender_em(family)
        .expect("the same face resolved for the line box must report an ascender");
    assert!(
        (word_top_em - ascent_em).abs() < 1e-9,
        "this test reads the bare hhea descent out of Word's gap-inclusive \
         split, which only holds for a face with no hhea line gap: \
         {word_top_em} against {ascent_em}"
    );
    // And the hhea pair it just read has to be the `usWin*` one PowerPoint
    // measures, or the proportional share below is derived from the wrong
    // numbers even when it happens to agree (issue #1176).
    let usual_split: (f64, f64) =
        crate::render::pdf::powerpoint_line_box_split_em([(ascent_em, descent_em)])
            .expect("a positive ascent splits the line box");
    assert!(
        (usual_split.0 - above).abs() < 1e-9,
        "this face's hhea pair must be its usWin pair for the derivation below \
         to hold: {usual_split:?} against ({above}, {below})"
    );
    let natural_em: f64 = ascent_em + descent_em;
    assert!(
        natural_em > 1.2,
        "this test needs a face that overflows the box, got {natural_em}em"
    );

    assert!(
        (above + below - 1.2).abs() < 1e-9,
        "the split must still span the 1.2em line, got {above} + {below}"
    );
    let proportional: f64 = 1.2 * ascent_em / natural_em;
    assert!(
        (above - proportional).abs() < 0.001,
        "an overflowing face seats its baseline at {proportional}em, not {above}em"
    );
    let halved_leading: f64 = (1.2 + ascent_em - descent_em) / 2.0;
    assert!(
        above < halved_leading - 0.01,
        "halving this face's negative leading pushes the baseline down to \
         {halved_leading}em; the proportional share must sit above it, got \
         {above}em"
    );
}

/// PowerPoint seats a baseline a whole number of points below its line box's
/// top, and gives whatever is left of the 1.2em line to the descent gap.
///
/// Measured on native PowerPoint 16.112 exports of a one-factor probe deck:
/// four faces — Georgia (1.13623em, fits inside the box), Verdana (1.21533em),
/// Avenir Next LT Pro (1.21289em) and Posterama (1.33008em, all three overflow
/// it) — in bottom-anchored boxes with every inset zeroed, at 8, 11, 14, 18, 24,
/// 28, 32, 36, 40, 44, 48, 54, 72 and 100pt. All 56 cells land on
/// `1.2 x size - round(share x size)` within the export's 0.12pt half-grid, and
/// none within 0.12pt of the unrounded share. The seat's share of the em
/// therefore is *not* constant across sizes: Avenir Next LT Pro keeps 0.192em
/// under its baseline at 10pt and 0.2625em at 32pt (issue #1074).
///
/// Georgia is the one of the four that fits the box, and its cells need the same
/// proportional share the other three do — which is why the split no longer
/// branches on whether the face fits (issue #1118). What this asserts is the
/// rounding, which is common to every face.
#[test]
fn a_slide_line_seats_its_baseline_on_a_whole_point() {
    let family: &str = "Libertinus Serif";
    let Some((model_above_em, _)) = crate::render::pdf::powerpoint_line_box_em(family) else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let mut rounding_bit: bool = false;

    for size_pt in [9.0_f64, 10.0, 11.0, 13.0, 17.0, 23.0, 31.0, 50.0] {
        let Some(source) = slide_text_box_source(family, size_pt, ParagraphStyle::default()) else {
            return;
        };
        let (top_em, bottom_em) =
            emitted_slide_line_box_em(&source, size_pt).expect("line box emitted");
        let seat_pt: f64 = top_em * size_pt;
        let unrounded_pt: f64 = model_above_em * size_pt;

        assert!(
            (seat_pt - seat_pt.round()).abs() < 1e-6,
            "the baseline sits a whole number of points below the line top, got \
             {seat_pt}pt at {size_pt}pt: {source}"
        );
        assert!(
            (seat_pt - unrounded_pt.round()).abs() < 1e-6,
            "the seat is the split's own share rounded to a point: expected \
             {}pt, got {seat_pt}pt at {size_pt}pt",
            unrounded_pt.round()
        );
        assert!(
            ((top_em + bottom_em) * size_pt - 1.2 * size_pt).abs() < 1e-6,
            "the line still spans 1.2em, got {}pt at {size_pt}pt",
            (top_em + bottom_em) * size_pt
        );
        assert!(
            bottom_em > 0.0,
            "the descent gap a bottom-anchored box keeps must stay positive, got \
             {bottom_em}em at {size_pt}pt"
        );
        rounding_bit |= (unrounded_pt - unrounded_pt.round()).abs() > 0.05;
    }

    assert!(
        rounding_bit,
        "no probed size exercised the rounding, so this test would pass on the \
         unrounded model too"
    );
}

/// The #841 Contoso deck's footer band, against a native PowerPoint 16.112
/// export of the same file.
///
/// `slideLayout5` seats the `ftr` placeholder at 5751576 EMU with a 722376 EMU
/// height and the master's inherited 45720 EMU `bIns`, `anchor="b"`, and the
/// master's `lvl1pPr` sets it in 10pt bold `+mn-lt` — Avenir Next LT Pro. Its
/// `sldNum` neighbour shares the frame's bottom and prints on the same line.
/// The export puts both on 504.24pt; the unrounded share put us on 503.69,
/// 0.55pt high, on every slide that repeats the band (issue #1074).
///
/// Avenir Next LT Pro is a cloud font no CI host has, so the seat is computed
/// from the model rather than rendered: what a fixture would exercise is the
/// split, and the split is a pure function of the face's metrics.
#[test]
fn the_contoso_footer_band_lands_on_its_native_baseline() {
    // Avenir Next LT Pro Bold: hhea ascender 1972, descender -512, no line
    // gap, per 2048 upem.
    let (above_em, _) =
        crate::render::pdf::powerpoint_line_box_split_em([(1972.0 / 2048.0, 512.0 / 2048.0)])
            .expect("a positive ascent splits the line box");
    let size_pt: f64 = 10.0;
    let (_, below_em) =
        crate::render::typst_gen::text::powerpoint_percentage_line_box_em(above_em, size_pt, 1.0);
    const EMU_PER_PT: f64 = 12700.0;
    let content_bottom_pt: f64 = (5751576.0 + 722376.0 - 45720.0) / EMU_PER_PT;
    let baseline_pt: f64 = content_bottom_pt - below_em * size_pt;

    assert!(
        (baseline_pt - 504.24).abs() <= 0.24,
        "the bottom-anchored footer band must land on the native 504.24pt \
         baseline within the export's 0.24pt position grid, got {baseline_pt}pt"
    );
}

/// The same deck's slide-8 attribution, which is what tells the two ways of
/// rounding a `spcPct` seat apart.
///
/// `slideLayout8` seats its `idx="11"` body placeholder at 6336792 EMU with
/// `tIns="45720"`, `anchor="t"` and `<a:lnSpc><a:spcPct val="85000"/>`, and sets
/// it in 14pt Avenir Next LT Pro. The export puts `Heraclitus` on 513.60pt,
/// 11.04pt below the content top. Measuring the scaled seat back from the plain
/// line's *unrounded* gap gives 11; rounding the plain seat to a point first and
/// subtracting that gives 10, a whole point low. The deck's other three `spcPct`
/// samples agree with both, so this one carries the distinction alone.
#[test]
fn the_contoso_scaled_attribution_lands_on_its_native_baseline() {
    // Avenir Next LT Pro: hhea ascender 1972, descender -512, no line gap, per
    // 2048 upem.
    let (plain_above, _) =
        crate::render::pdf::powerpoint_line_box_split_em([(1972.0 / 2048.0, 512.0 / 2048.0)])
            .expect("a positive ascent splits the line box");
    let size_pt: f64 = 14.0;
    let (above, _) = crate::render::typst_gen::text::powerpoint_percentage_line_box_em(
        plain_above,
        size_pt,
        0.85,
    );
    const EMU_PER_PT: f64 = 12700.0;
    let content_top_pt: f64 = (6336792.0 + 45720.0) / EMU_PER_PT;
    let baseline_pt: f64 = content_top_pt + above * size_pt;

    assert!(
        (baseline_pt - 513.60).abs() <= 0.24,
        "the scaled attribution must land on the native 513.60pt baseline within \
         the export's 0.24pt position grid, got {baseline_pt}pt"
    );
}

/// The paragraph mark's face reaches the emitted line box.
///
/// PowerPoint's 1.2em box is shared by every font on the line and the mark —
/// the empty run `<a:endParaRPr>` describes — is one of them, so a mark set in
/// a deeper-descended face seats the text higher than the run's own share
/// would (issue #1176). The golden mocks' Korean titles are that case: Malgun
/// Gothic runs with bare marks, which fall to the theme's Calibri.
///
/// Both faces here are Typst's own embedded ones so the test does not depend on
/// what the host has installed, and the size is chosen as the first one where
/// the two models round to different points — a size where they agree would
/// pass without the mark reaching the box at all.
#[test]
fn the_paragraph_mark_face_moves_the_emitted_line_box() {
    let run_family: &str = "Libertinus Serif";
    let mark_family: &str = "DejaVu Sans Mono";
    let Some((alone_em, _)) = crate::render::pdf::powerpoint_line_box_em(run_family) else {
        return;
    };
    let Some((shared_em, _)) =
        crate::render::pdf::powerpoint_line_box_em_for_families(&[run_family, mark_family])
    else {
        return;
    };

    let size_pt: f64 = (8..=72)
        .map(f64::from)
        .find(|size| (alone_em * size).round() != (shared_em * size).round())
        .expect("the two faces must round apart at some slide size");

    let bare: String = slide_text_box_source(run_family, size_pt, ParagraphStyle::default())
        .expect("the run face resolves");
    let marked: String = slide_text_box_source(
        run_family,
        size_pt,
        ParagraphStyle {
            paragraph_mark_font_family: Some(mark_family.into()),
            ..ParagraphStyle::default()
        },
    )
    .expect("the run face resolves");

    let (bare_top_em, _) = emitted_slide_line_box_em(&bare, size_pt)
        .unwrap_or_else(|| panic!("no line box emitted: {bare}"));
    let (marked_top_em, _) = emitted_slide_line_box_em(&marked, size_pt)
        .unwrap_or_else(|| panic!("no line box emitted: {marked}"));

    assert_eq!(
        (bare_top_em * size_pt).round(),
        (alone_em * size_pt).round(),
        "a paragraph with no mark face keeps the run face's own share at \
         {size_pt}pt"
    );
    assert_eq!(
        (marked_top_em * size_pt).round(),
        (shared_em * size_pt).round(),
        "the mark's face must share the box at {size_pt}pt"
    );
}

#[test]
fn the_split_differs_between_fonts_while_the_line_does_not() {
    // Triangulation: the height is a property of PowerPoint and the split is a
    // property of the font, so two faces must agree on 1.2em and disagree on
    // where the baseline sits inside it.
    let Some(serif) = crate::render::pdf::powerpoint_line_box_em("Libertinus Serif") else {
        return;
    };
    let Some(mono) = crate::render::pdf::powerpoint_line_box_em("DejaVu Sans Mono") else {
        return;
    };

    assert!((serif.0 + serif.1 - 1.2).abs() < 0.000_001);
    assert!((mono.0 + mono.1 - 1.2).abs() < 0.000_001);
    assert!(
        (serif.0 - mono.0).abs() > 0.001,
        "two faces with different usWin ascent/descent should seat the baseline \
         differently: {serif:?} vs {mono:?}"
    );
    assert!(
        serif.0 > 0.0 && serif.1 > 0.0 && mono.0 > 0.0 && mono.1 > 0.0,
        "both edges are positive distances: {serif:?} {mono:?}"
    );
}

/// The line advance, in em, the generator emits for `percent` line spacing.
fn slide_line_advance_em(percent: f64) -> Option<f64> {
    let source = slide_text_box_source(
        "Libertinus Serif",
        18.0,
        ParagraphStyle {
            line_spacing: Some(LineSpacing::Proportional(percent)),
            ..ParagraphStyle::default()
        },
    )?;
    let (top, bottom) = emitted_slide_line_box_em(&source, 18.0)
        .unwrap_or_else(|| panic!("no line box emitted: {source}"));
    Some(top + bottom)
}

#[test]
fn a_slide_paragraph_scales_the_1_2em_line_by_its_own_line_spacing() {
    // `a:lnSpc` scales PowerPoint's line rather than replacing it: the advance
    // is `percent x 1.2em`. Carrying it as `par(leading)` instead did nothing
    // between single-line paragraphs — a slide's code block is one `<a:p>` per
    // line — so eight lines of Rust stacked into the height of three (#541).
    let Some(advance) = slide_line_advance_em(1.18) else {
        return; // no font book available (e.g. exotic CI sandbox)
    };

    assert!(
        (advance - 1.18 * 1.2).abs() < 0.001,
        "118% line spacing spans {}em, expected {}em",
        advance,
        1.18 * 1.2
    );
}

#[test]
fn slide_line_spacing_scales_proportionally() {
    // Triangulation: a second percentage must land on its own multiple of
    // 1.2em, so the advance cannot be a constant that happens to fit 118%.
    let (Some(at_118), Some(at_125), Some(at_100)) = (
        slide_line_advance_em(1.18),
        slide_line_advance_em(1.25),
        slide_line_advance_em(1.0),
    ) else {
        return;
    };

    assert!((at_125 - 1.25 * 1.2).abs() < 0.001, "125% spans {at_125}em");
    assert!((at_100 - 1.2).abs() < 0.001, "100% spans {at_100}em");
    assert!(
        at_125 > at_118 && at_118 > at_100,
        "advance must rise with the percentage: {at_100} / {at_118} / {at_125}"
    );
}

#[test]
fn slide_line_spacing_keeps_the_descent_gap_and_moves_the_ascent() {
    // A percentage resizes the line from its top: the gap the face keeps below
    // its baseline is the same whatever the percentage, and the ascent side
    // absorbs the whole change.
    //
    // Measured on native PowerPoint 16.112 exports. Arial 38pt in a plain box
    // drops its first baseline 36.96pt below the content top and 30.00pt under
    // `<a:spcPct val="85000">` — a 6.96pt loss against the 6.84pt the line
    // itself loses, so the descent gap moved by 0.12pt, half of the export's
    // 0.24pt position grid. Posterama Bold behaves the same across the #841
    // Contoso deck's five title sizes. Scaling both sides instead left every
    // one of those titles 1.8-3.7pt low (issues #1020, #1024).
    //
    // Both sides land on a whole-point seat (issue #1074), so each keeps the
    // face's own unrounded gap to within half a point rather than keeping the
    // identical gap — asserted against that gap, in points.
    let family: &str = "Libertinus Serif";
    let size_pt: f64 = 18.0;
    let Some(plain) = slide_text_box_source(family, size_pt, ParagraphStyle::default()) else {
        return;
    };
    let Some(scaled) = slide_text_box_source(
        family,
        size_pt,
        ParagraphStyle {
            line_spacing: Some(LineSpacing::Proportional(0.85)),
            ..ParagraphStyle::default()
        },
    ) else {
        return;
    };
    let (_plain_top, plain_bottom) =
        emitted_slide_line_box_em(&plain, size_pt).expect("line box emitted");
    let (scaled_top, scaled_bottom) =
        emitted_slide_line_box_em(&scaled, size_pt).expect("line box emitted");
    let (share_top, share_bottom) =
        crate::render::pdf::powerpoint_line_box_em(family).expect("metrics resolve");
    let gap_pt: f64 = share_bottom * size_pt;

    assert!(
        ((scaled_bottom - share_bottom) * size_pt).abs() <= 0.5,
        "the descent gap must survive the percentage to within the whole-point \
         seat: {}pt against the face's {gap_pt}pt",
        scaled_bottom * size_pt
    );
    assert!(
        ((plain_bottom - share_bottom) * size_pt).abs() <= 0.5,
        "the plain line keeps the same gap to the same tolerance: {}pt against \
         {gap_pt}pt",
        plain_bottom * size_pt
    );
    assert!(
        ((scaled_top - (share_top - 0.15 * 1.2)) * size_pt).abs() <= 0.5,
        "the ascent must absorb the whole 0.18em the line loses: {scaled_top}em \
         against the face's {share_top}em"
    );
    assert!(
        (scaled_top * size_pt - (scaled_top * size_pt).round()).abs() < 1e-6,
        "the scaled line seats its baseline on a whole point too: {}pt",
        scaled_top * size_pt
    );
    assert!(
        (scaled_top + scaled_bottom - 0.85 * 1.2).abs() < 0.001,
        "the line still spans its percentage of 1.2em: {scaled_top}/{scaled_bottom}"
    );
    assert!(
        scaled.contains("leading: 0pt"),
        "the advance is carried by the box, not by leading: {scaled}"
    );
}

/// The #841 Contoso deck's slide-18 footer title, against a native PowerPoint
/// 16.112 export of the same file.
///
/// `slideLayout18` seats the title placeholder at 5367528 EMU with a 1490472
/// EMU height, `tIns="137160"`, the inherited 45720 EMU `bIns`, and
/// `anchor="ctr"`; its `lvl1pPr` declares `<a:lnSpc><a:spcPct val="85000"/>`
/// and the slide sets the run in 30pt Posterama. The export puts
/// `SPØRSMÅL OG SVAR` on 492.48pt; we put it on 494.53, exactly 2.05pt low
/// (issue #1020).
///
/// Posterama is a cloud font no CI host has, so the seat is computed from the
/// model rather than rendered: what the fixture would exercise is the split,
/// and the split is a pure function of the face's metrics.
#[test]
fn the_contoso_footer_title_lands_on_its_native_baseline() {
    // Posterama Bold: hhea ascender 2134, descender -590 per 2048 upem.
    let (plain_above, plain_below) =
        crate::render::pdf::powerpoint_line_box_split_em([(2134.0 / 2048.0, 590.0 / 2048.0)])
            .expect("a positive ascent splits the line box");
    assert!((plain_above + plain_below - 1.2).abs() < 1e-9);

    let size_pt: f64 = 30.0;
    let (above, below) = crate::render::typst_gen::text::powerpoint_percentage_line_box_em(
        plain_above,
        size_pt,
        0.85,
    );
    const EMU_PER_PT: f64 = 12700.0;
    let content_top_pt: f64 = (5367528.0 + 137160.0) / EMU_PER_PT;
    let content_height_pt: f64 = (1490472.0 - 137160.0 - 45720.0) / EMU_PER_PT;
    let line_pt: f64 = (above + below) * size_pt;
    let baseline_pt: f64 = content_top_pt + (content_height_pt - line_pt) / 2.0 + above * size_pt;

    assert!(
        (baseline_pt - 492.48).abs() <= 0.24,
        "the centred footer title must land on the native 492.48pt baseline \
         within the export's 0.24pt position grid, got {baseline_pt}pt"
    );
}

/// The same deck's top-anchored titles, which move the other way from the
/// centred one and so pin the ascent term on its own.
///
/// `slideLayout2` gives the title `tIns="338328"` at the same 5367528 EMU
/// origin and inherits the master's `anchor="t"`; the slide sets 38pt
/// Posterama. The export puts `Mirjam Nilsson` on 478.32pt against our
/// 480.84 — 2.52pt low, the deviation issue #1024 reports for the same
/// placeholder chain on slides 13 and 14.
#[test]
fn the_contoso_top_anchored_title_lands_on_its_native_baseline() {
    let (plain_above, _) =
        crate::render::pdf::powerpoint_line_box_split_em([(2134.0 / 2048.0, 590.0 / 2048.0)])
            .expect("a positive ascent splits the line box");
    let size_pt: f64 = 38.0;
    let (above, _) = crate::render::typst_gen::text::powerpoint_percentage_line_box_em(
        plain_above,
        size_pt,
        0.85,
    );
    const EMU_PER_PT: f64 = 12700.0;
    let content_top_pt: f64 = (5367528.0 + 338328.0) / EMU_PER_PT;
    let baseline_pt: f64 = content_top_pt + above * size_pt;

    assert!(
        (baseline_pt - 478.32).abs() <= 0.24,
        "the top-anchored title must land on the native 478.32pt baseline \
         within the export's 0.24pt position grid, got {baseline_pt}pt"
    );
}

/// The deck's slide-13 and slide-14 titles, the two issue #1024 measures.
///
/// Neither slide states any geometry of its own: both title shapes carry an
/// empty `<p:spPr/>` and a bare `<a:bodyPr rtlCol="0"/>`, so the frame, the
/// `tIns` and the `<a:lnSpc><a:spcPct val="85000"/>` all come from the layout's
/// title placeholder — `slideLayout13` seats it at y=0 with `tIns="685800"`,
/// `slideLayout14` at y=0 with `tIns="704088"`, both inheriting the master's
/// top anchor — and the slides set 38pt Posterama. Two layouts differing only
/// in their inset keep the assertion on the seat below the content top rather
/// than on one absolute number.
///
/// The native PowerPoint 16.112 export puts `PROSJEKT` on 83.04pt and
/// `NØKKELTALL` on 84.48pt. Splitting the box evenly and scaling both of its
/// sides by the percentage put them on 85.56 and 87.00 — the +2.52pt #1024
/// reports.
///
/// Posterama is a cloud font no CI host has, so the seat is computed from the
/// model rather than rendered: what a fixture would exercise here is the split,
/// and the split is a pure function of the face's metrics.
#[test]
fn the_contoso_inherited_slide_titles_land_on_their_native_baselines() {
    // Posterama Bold: hhea ascender 2134, descender -590 per 2048 upem.
    let (plain_above, _) =
        crate::render::pdf::powerpoint_line_box_split_em([(2134.0 / 2048.0, 590.0 / 2048.0)])
            .expect("a positive ascent splits the line box");
    let size_pt: f64 = 38.0;
    let (above, _) = crate::render::typst_gen::text::powerpoint_percentage_line_box_em(
        plain_above,
        size_pt,
        0.85,
    );
    const EMU_PER_PT: f64 = 12700.0;

    for (slide, top_inset_emu, native_pt) in [(13, 685800.0, 83.04), (14, 704088.0, 84.48)] {
        let baseline_pt: f64 = top_inset_emu / EMU_PER_PT + above * size_pt;

        assert!(
            (baseline_pt - native_pt).abs() <= 0.24,
            "slide {slide}'s inherited title must land on the native {native_pt}pt \
             baseline within the export's 0.24pt position grid, got {baseline_pt}pt"
        );
    }
}

#[test]
fn a_slide_paragraph_declares_its_own_block_spacing() {
    // A slide paragraph's gaps are its own `a:spcBef`/`a:spcAft` and nothing
    // else. Leaving them unset let Typst's 1.2em `block.spacing` default in,
    // which put 13pt between the lines of a code block declaring no spacing
    // (issue #513).
    let Some(source) = slide_text_box_source("Libertinus Serif", 11.0, ParagraphStyle::default())
    else {
        return;
    };

    assert!(
        source.contains("above: 0pt, below: 0pt"),
        "an unspaced slide paragraph pins both gaps to zero: {source}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn consecutive_slide_paragraphs_keep_powerpoints_full_line_advance() {
    // The unstyled case takes Typst's embedded default. Calibri verifies a
    // named Office family and the synthetic name forces the final embedded
    // fallback after every named candidate misses.
    for family in [None, Some("Calibri"), Some("Office2Pdf Missing Test Face")] {
        let paragraphs = ["Paragraph one", "Paragraph two", "Paragraph three"]
            .into_iter()
            .map(|text| {
                Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        line_spacing: Some(LineSpacing::Proportional(1.0)),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: text.to_string(),
                        style: TextStyle {
                            font_family: family.map(str::to_string),
                            font_size: Some(18.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })
            })
            .collect();
        let doc = make_doc(vec![make_fixed_page(
            960.0,
            540.0,
            vec![make_fixed_text_box(
                54.0,
                54.0,
                800.0,
                250.0,
                Insets::default(),
                crate::ir::TextBoxVerticalAlign::Top,
                paragraphs,
            )],
        )]);
        let output = generate_typst(&doc).unwrap();
        let mut baselines: Vec<f64> = crate::render::pdf::compiled_text_runs(&output.source, 0)
            .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source))
            .into_iter()
            .filter(|run| run.text.starts_with("Paragraph"))
            .map(|run| run.baseline_pt)
            .collect();
        baselines.sort_by(f64::total_cmp);

        assert_eq!(
            baselines.len(),
            3,
            "expected three paragraphs: {baselines:?}"
        );
        for gap in baselines.windows(2).map(|pair| pair[1] - pair[0]) {
            assert!(
                (gap - 21.6).abs() < 0.01,
                "18pt slide paragraphs in {family:?} should advance by 1.2em (21.6pt), \
                 got {gap}pt; baselines={baselines:?}\n{}",
                output.source
            );
        }
    }
}

/// A hard break between two sizes clears the preceding line: the advance is
/// that line's descent plus the following line's seat, not the following
/// line's whole 1.2em box.
///
/// #683 read one gap of a two-line block and concluded the following line owns
/// the whole box — 12.00pt for 12.5pt over 10pt. A nine-line probe of the same
/// pair paces it at 12.96pt and its 10pt-over-12.5pt partner at 14.04pt, where
/// the whole-box reading gives 12.00 and 15.00; the two sum to `1.2 x 22.5`
/// either way, so only the boundary between them was ever in question. The
/// half-point tolerances below are what separates the two models, and the
/// 0.36pt inside them is the paragraph-wide seat share tracked in #1252.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn powerpoint_hard_break_advance_clears_the_line_above_it() {
    let family = "Arial";
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            72.0,
            72.0,
            400.0,
            120.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Top,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![
                    Run {
                        text: "Large\n".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(12.5),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                    Run {
                        text: "Small\u{000B}".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(10.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                    Run {
                        text: "Large".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(12.5),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                ],
            })],
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    let runs = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source));
    let mut baselines: Vec<f64> = runs
        .iter()
        .filter(|run| run.text.contains("Large") || run.text.contains("Small"))
        .map(|run| run.baseline_pt)
        .collect();
    baselines.sort_by(f64::total_cmp);
    baselines.dedup_by(|left, right| (*left - *right).abs() < 0.01);

    assert_eq!(baselines.len(), 3, "expected three lines: {baselines:?}");
    let (top_em, _) = crate::render::pdf::powerpoint_line_box_em(family)
        .expect("the Arial-compatible line metrics must resolve");
    // The paragraph's largest size decides its line box, and PowerPoint seats
    // the baseline a whole number of points below its top (issue #1074).
    let expected_first_baseline = 72.0 + (top_em * 12.5).round();
    assert!(
        (baselines[0] - expected_first_baseline).abs() < 0.01,
        "hard-break boxing must preserve the first line's top seating: \
         expected {expected_first_baseline}, got {baselines:?}\n{}",
        output.source
    );
    assert!(
        (baselines[1] - baselines[0] - 12.96).abs() < 0.5,
        "the 12.5pt line's descent must carry the 10pt line to the export's \
         12.96pt, not to its own 12.00pt box: {baselines:?}\n{}",
        output.source
    );
    assert!(
        (baselines[2] - baselines[1] - 14.04).abs() < 0.5,
        "the 10pt line's descent must carry the 12.5pt line to the export's \
         14.04pt, not to its own 15.00pt box: {baselines:?}\n{}",
        output.source
    );
}

/// A proportional `<a:lnSpc>` scales the box both sides of the break are cut
/// from, so the hard-break stack must carry that paragraph-level multiplier.
///
/// The export paces this 12.5pt-over-10pt pair at 19.50pt under `val="150000"`
/// against 12.96pt plain — 1.5x to within its own dither. A following line
/// owning the whole scaled box would give 18.00pt, and the paragraph carrying
/// no multiplier at all 12.60pt, so this stack has to land well clear of both.
///
/// It lands 1.00pt short of the export, which is two open defects and not this
/// one: the paragraph-wide seat share of #1252, and the percentage split of
/// #1254. The corrected 0.97238em share (#1176) makes the second of those
/// exactly measurable here — scaling *both* sides by the percentage predicts
/// `4.5 + 15 = 19.50pt`, the export's own figure, where holding the descent
/// fixed and resizing from the top gives the 18.50pt below. The same
/// both-sides rule misses the 85% Arial 38pt cell of #1024 by a whole point,
/// so it is not a change to make from one cell.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn powerpoint_hard_break_preserves_proportional_line_spacing() {
    let family = "Arial";
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            72.0,
            72.0,
            400.0,
            120.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Top,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle {
                    line_spacing: Some(LineSpacing::Proportional(1.5)),
                    ..ParagraphStyle::default()
                },
                runs: vec![
                    Run {
                        text: "Large\n".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(12.5),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                    Run {
                        text: "Small".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(10.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                ],
            })],
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    let runs = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source));
    let large_baseline = runs
        .iter()
        .find(|run| run.text.contains("Large"))
        .expect("Large run")
        .baseline_pt;
    let small_baseline = runs
        .iter()
        .find(|run| run.text.contains("Small"))
        .expect("Small run")
        .baseline_pt;

    let advance_pt: f64 = small_baseline - large_baseline;
    assert!(
        (advance_pt - 19.5).abs() < 1.05,
        "the scaled box must carry the break to within #1252 and #1254's \
         residual of the export's 19.5pt: {large_baseline}, {small_baseline}\n{}",
        output.source
    );
    assert!(
        advance_pt > 18.25,
        "the break must be paced by the *paragraph's* scaled box, not by the \
         18pt the 10pt line's own scaled box would give: {advance_pt}pt\n{}",
        output.source
    );
}

/// The previous line's bottom edge can only be replaced when the explicit
/// segment fits one physical line. Otherwise the replacement would also alter
/// every ordinary wrapped line inside that segment (#683).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn powerpoint_hard_break_keeps_normal_edges_when_the_segment_soft_wraps() {
    let family = "Arial";
    let font_size_pt = 12.5;
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            72.0,
            72.0,
            45.0,
            140.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Top,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![
                    Run {
                        text: "Large words wrap\n".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(font_size_pt),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                    Run {
                        text: "Small".to_string(),
                        style: TextStyle {
                            font_family: Some(family.to_string()),
                            font_size: Some(10.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    },
                ],
            })],
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    let mut large_baselines: Vec<f64> = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source))
        .into_iter()
        .filter(|run| run.text.contains("Large") || run.text.contains("words"))
        .map(|run| run.baseline_pt)
        .collect();
    large_baselines.sort_by(f64::total_cmp);
    large_baselines.dedup_by(|left, right| (*left - *right).abs() < 0.01);

    assert!(
        large_baselines.len() >= 2,
        "the probe must soft-wrap before its explicit break: {large_baselines:?}\n{}",
        output.source
    );
    for gap in large_baselines.windows(2).map(|pair| pair[1] - pair[0]) {
        assert!(
            (gap - 1.2 * font_size_pt).abs() < 0.01,
            "ordinary wrapped lines must retain their 15pt pitch, got {gap}: \
             {large_baselines:?}\n{}",
            output.source
        );
    }
}

/// PowerPoint rounds every nominal glyph advance to the nearest 1/8pt before
/// it decides where a line ends. Ten 17pt Libertinus `o` glyphs are a stable,
/// environment-free edge case: their exact Typst advances fit this box, while
/// their PowerPoint-grid advances exceed it. The tenth word must therefore
/// move to a second line (issue #661).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn slide_text_wraps_on_powerpoints_one_eighth_point_advance_grid() {
    let family = "Libertinus Serif";
    let font_size_pt = 17.0;
    let word_advance_pt = crate::render::pdf::text_advance_em(family, false, "o")
        .expect("the embedded Libertinus Serif face must resolve")
        * font_size_pt;
    let space_advance_pt = crate::render::pdf::text_advance_em(family, false, " ")
        .expect("the embedded Libertinus Serif space must resolve")
        * font_size_pt;
    let quantize = |advance_pt: f64| (advance_pt * 8.0).round() / 8.0;
    let exact_line_pt = 10.0 * word_advance_pt + 9.0 * space_advance_pt;
    let grid_line_pt = 10.0 * quantize(word_advance_pt) + 9.0 * quantize(space_advance_pt);
    assert!(
        grid_line_pt > exact_line_pt + 0.25,
        "probe must straddle the 1/8pt grid: exact={exact_line_pt}, grid={grid_line_pt}"
    );

    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            72.0,
            72.0,
            (exact_line_pt + grid_line_pt) / 2.0,
            100.0,
            Insets::default(),
            crate::ir::TextBoxVerticalAlign::Center,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: "o o o o o o o o o o".to_string(),
                    style: TextStyle {
                        font_family: Some(family.to_string()),
                        font_size: Some(font_size_pt),
                        ..TextStyle::default()
                    },
                    href: None,
                    footnote: None,
                }],
            })],
        )],
    )]);
    let output = generate_typst(&doc).unwrap();
    let mut baselines: Vec<f64> = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source))
        .into_iter()
        .filter(|run| run.text.contains('o'))
        .map(|run| run.baseline_pt)
        .collect();
    baselines.sort_by(f64::total_cmp);
    baselines.dedup_by(|left, right| (*left - *right).abs() < 0.01);

    assert_eq!(
        baselines.len(),
        2,
        "the grid-rounded tenth word should wrap: {baselines:?}\n{}",
        output.source
    );
    let (top_em, bottom_em) = crate::render::pdf::powerpoint_line_box_em(family)
        .expect("the embedded Libertinus Serif line metrics must resolve");
    let line_height_pt = (top_em + bottom_em) * font_size_pt;
    // PowerPoint seats the baseline a whole number of points below its line
    // box's top; only the centring offset stays fractional (issue #1074).
    let expected_first_baseline_pt =
        72.0 + (100.0 - 2.0 * line_height_pt) / 2.0 + (top_em * font_size_pt).round();
    assert!(
        (baselines[0] - expected_first_baseline_pt).abs() < 0.01,
        "advance-grid boxes must preserve the active line box during vertical centring: \
         got {}, expected {expected_first_baseline_pt}; {baselines:?}\n{}",
        baselines[0],
        output.source
    );
    assert!(
        (baselines[1] - baselines[0] - line_height_pt).abs() < 0.01,
        "grid-rounded lines must retain PowerPoint's 1.2em advance: {baselines:?}\n{}",
        output.source
    );
}

/// A box barely one line tall does not scale its text unless the file asked
/// for it (issue #898).
///
/// `single_line_fit_paragraph` folded the paragraph onto one line and scaled
/// it whenever the box was short, whatever `auto_fit` said. The deck in #841
/// gives its sensitivity label a 67.4 x 9.6pt box holding 8pt text and no
/// `<a:normAutofit/>`; we rendered it at 0.61x — an effective 4.9pt — where
/// the reference keeps 8pt and lets it overflow.
#[test]
fn a_short_box_without_autofit_does_not_scale_its_text() {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![FixedElement {
            x: 5.0,
            y: 525.0,
            width: 67.4,
            height: 9.6,
            kind: FixedElementKind::TextBox(crate::ir::TextBoxData {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Sensitivity: Internal".to_string(),
                        style: TextStyle {
                            font_size: Some(8.0),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                padding: Insets::default(),
                vertical_align: TextBoxVerticalAlign::Top,
                fill: None,
                opacity: None,
                stroke: None,
                shape_kind: None,
                no_wrap: false,
                auto_fit: false,
                text_rotation_deg: None,
                shape_rotation_deg: None,
            }),
        }],
    )]);

    let output = generate_typst(&doc).unwrap();

    assert!(
        !output.source.contains("text_box_scale_"),
        "nothing asked for the text to shrink:\n{}",
        output.source
    );
}

/// A slide text box carrying one paragraph, for the trailing letter-space
/// probes below. The box spans the #841 deck's `sldNum` placeholder: 59.76pt
/// wide with PowerPoint's default 7.2pt side insets, so its content measure is
/// 45.36pt and the centre a centred line takes sits 29.88pt from the slide's
/// left edge.
fn tracked_slide_line_source(
    text: &str,
    letter_spacing: Option<f64>,
    alignment: Option<Alignment>,
) -> String {
    let doc = make_doc(vec![make_fixed_page(
        960.0,
        540.0,
        vec![make_fixed_text_box(
            0.0,
            453.0,
            59.76,
            56.88,
            Insets {
                top: 3.6,
                right: 7.2,
                bottom: 3.6,
                left: 7.2,
            },
            crate::ir::TextBoxVerticalAlign::Bottom,
            vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle {
                    alignment,
                    ..ParagraphStyle::default()
                },
                runs: vec![Run {
                    text: text.to_string(),
                    style: TextStyle {
                        font_size: Some(10.0),
                        bold: Some(true),
                        letter_spacing,
                        ..TextStyle::default()
                    },
                    href: None,
                    footnote: None,
                }],
            })],
        )],
    )]);
    generate_typst(&doc).unwrap().source
}

/// PowerPoint measures a line's width with one letter-space after *every*
/// glyph, the last one included, and centres the line on that width. Typst
/// drops the tracking of a shaped item's final glyph, so a centred tracked
/// line came out half a letter-space to the right of where PowerPoint puts it.
///
/// Measured with `scripts/probe_harness.py --backend office` on the #841
/// Contoso deck, whose `sldNum` placeholder is a centred 10pt bold Avenir Next
/// LT Pro field on a 45.36pt measure centred at 29.88pt. Varying only the
/// field's `a:rPr/@spc` and tracing the glyph origin out of native PowerPoint
/// 16.112 exports:
///
/// | `spc` | `5` origin | implied width |
/// | ---: | ---: | ---: |
/// | 0 | 26.63pt | 6.50pt |
/// | 100 | 26.13pt | 7.50pt |
/// | 200 | 25.63pt | 8.50pt |
/// | 400 | 24.63pt | 10.50pt |
/// | 800 | 22.63pt | 14.50pt |
///
/// One glyph, so one letter-space: the origin moves exactly half a point per
/// point of `spc`, which is the trailing space halved by centring and nothing
/// else (issue #1075).
#[test]
fn a_centred_tracked_slide_line_carries_a_trailing_letter_space() {
    let source: String = tracked_slide_line_source("5", Some(1.0), Some(Alignment::Center));

    assert!(
        source.contains("[5]#h(1pt)"),
        "the centred line must be measured with the letter-space that follows \
         its last glyph: {source}"
    );
}

/// Triangulation on the factor: the trailing space is the run's own tracking,
/// not a constant. The same probe at `spc="400"` moves the origin 2pt, four
/// times what `spc="100"` moves it.
#[test]
fn a_centred_tracked_slide_line_trails_its_own_letter_space() {
    let source: String = tracked_slide_line_source("5", Some(4.0), Some(Alignment::Center));

    assert!(
        source.contains("[5]#h(4pt)"),
        "the trailing space is the run's tracking: {source}"
    );
}

/// Right alignment consumes the whole trailing space rather than half of it,
/// which falls out of measuring the line the same way. Same probe deck with
/// the field's paragraph forced to `algn="r"`: the origin moves a full point
/// per point of `spc` (45.95pt at 0, 44.95pt at 100, 43.95pt at 200, 41.95pt
/// at 400), against a 52.56pt content right edge.
#[test]
fn a_right_aligned_tracked_slide_line_carries_a_trailing_letter_space() {
    let source: String = tracked_slide_line_source("5", Some(1.0), Some(Alignment::Right));

    assert!(
        source.contains("[5]#h(1pt)"),
        "a right-aligned line is placed by the same measured width: {source}"
    );
}

/// A left-aligned line starts at the content edge whatever its tracking, so
/// the trailing space cannot move it — and emitting one would only risk an
/// extra wrap. The same deck's `ftr` placeholder is left-aligned and tracked
/// at `spc="200"`, and starts at 69.84pt in both the native export and ours.
#[test]
fn a_left_aligned_tracked_slide_line_trails_nothing() {
    let source: String = tracked_slide_line_source("5", Some(1.0), Some(Alignment::Left));

    assert!(
        !source.contains("]#h("),
        "a left-aligned line is not placed by its width: {source}"
    );
}

/// An untracked centred line has no letter-space to trail. PowerPoint writes
/// `spc="0"` routinely, so keying this on the attribute's presence would put a
/// spurious `0pt` spacer on whole decks.
#[test]
fn an_untracked_centred_slide_line_trails_nothing() {
    for spacing in [None, Some(0.0)] {
        let source: String = tracked_slide_line_source("5", spacing, Some(Alignment::Center));

        assert!(
            !source.contains("]#h("),
            "no tracking, no trailing space (spacing {spacing:?}): {source}"
        );
    }
}

/// The two lines of a tracked slide title, built twice: page 1 puts them in
/// one paragraph split by a PPTX `<a:br/>` marker, page 2 gives each line a
/// paragraph of its own. Both boxes have the same geometry, so a line must land
/// on the same origin either way — PowerPoint measures a hard-broken line
/// exactly as it measures a paragraph's only line.
///
/// `trailing_break` reproduces the shape the #841 deck's slide-13 title has:
/// a `<a:br/>` after the last visible line, which puts an empty third line at
/// the end of the paragraph.
#[cfg(not(target_arch = "wasm32"))]
fn tracked_two_line_title_source(
    first_spacing: Option<f64>,
    second_spacing: Option<f64>,
    alignment: Alignment,
    trailing_break: bool,
) -> String {
    let run = |text: &str, letter_spacing: Option<f64>| Run {
        text: text.to_string(),
        style: TextStyle {
            font_family: Some("Arial".to_string()),
            font_size: Some(38.0),
            bold: Some(true),
            letter_spacing,
            ..TextStyle::default()
        },
        href: None,
        footnote: None,
    };
    let paragraph = |runs: Vec<Run>| {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                alignment: Some(alignment),
                ..ParagraphStyle::default()
            },
            runs,
        })
    };
    let slide = |content: Vec<Block>| {
        make_fixed_page(
            960.0,
            540.0,
            vec![make_fixed_text_box(
                60.0,
                40.0,
                600.0,
                200.0,
                Insets::default(),
                crate::ir::TextBoxVerticalAlign::Top,
                content,
            )],
        )
    };

    let second: String = match trailing_break {
        true => "RAPPORT\u{000B}".to_string(),
        false => "RAPPORT".to_string(),
    };
    let doc = make_doc(vec![
        slide(vec![paragraph(vec![
            run("PROSJEKT\u{000B}", first_spacing),
            run(&second, second_spacing),
        ])]),
        slide(vec![
            paragraph(vec![run("PROSJEKT", first_spacing)]),
            paragraph(vec![run("RAPPORT", second_spacing)]),
        ]),
    ]);
    generate_typst(&doc).unwrap().source
}

/// The left edge of the line that carries `word`, on `page_index`.
#[cfg(not(target_arch = "wasm32"))]
fn compiled_line_origin_pt(source: &str, page_index: usize, word: &str) -> f64 {
    crate::render::pdf::compiled_text_runs(source, page_index)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{source}"))
        .iter()
        .filter(|run| run.text.contains(word))
        .map(|run| run.left_pt)
        .reduce(f64::min)
        .unwrap_or_else(|| panic!("no run carries {word:?} on page {page_index}:\n{source}"))
}

/// PowerPoint measures *every* line of a paragraph with a letter-space after
/// its last glyph, not just the paragraph's last one, and places a centred or
/// right-aligned line from that width. #1120 trailed the space once per
/// paragraph, which left slide 13 of the #841 deck — one centred 38pt title
/// paragraph broken by `a:br` at `spc="300"` — half a letter-space right of a
/// native PowerPoint 16.112 export on both of its lines: `PROSJEKT` at
/// 131.198pt against 129.720pt and `RAPPORTSTATUS` at 54.461pt against
/// 53.086pt, while the single-line slide number on the same slide matched to
/// 0.04pt (issue #1174).
///
/// The two lines are tracked differently on purpose: the space a line reserves
/// is the one its own last run declares, not the paragraph's or its
/// neighbour's.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_hard_broken_tracked_slide_line_reserves_its_own_trailing_letter_space() {
    for alignment in [Alignment::Center, Alignment::Right] {
        let source: String = tracked_two_line_title_source(Some(3.0), Some(1.0), alignment, false);

        for word in ["PROSJEKT", "RAPPORT"] {
            let broken_pt: f64 = compiled_line_origin_pt(&source, 0, word);
            let separate_pt: f64 = compiled_line_origin_pt(&source, 1, word);

            assert!(
                (broken_pt - separate_pt).abs() <= 0.01,
                "{alignment:?} {word:?}: a hard-broken line must land where the same \
                 line as its own paragraph lands, got {broken_pt}pt against \
                 {separate_pt}pt\n{source}"
            );
        }
    }
}

/// The #841 deck's title paragraph ends with a `<a:br/>`, so its last line
/// carries no glyph at all and the paragraph-final space #1120 emits lands on
/// that empty line. Both *visible* lines must still reserve their own.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_trailing_hard_break_does_not_absorb_a_visible_line_letter_space() {
    let source: String =
        tracked_two_line_title_source(Some(3.0), Some(3.0), Alignment::Center, true);

    for word in ["PROSJEKT", "RAPPORT"] {
        let broken_pt: f64 = compiled_line_origin_pt(&source, 0, word);
        let separate_pt: f64 = compiled_line_origin_pt(&source, 1, word);

        assert!(
            (broken_pt - separate_pt).abs() <= 0.01,
            "{word:?}: an empty trailing line must not take the reserve off the \
             line above it, got {broken_pt}pt against {separate_pt}pt\n{source}"
        );
    }
}

/// A left-aligned line starts at the content edge whatever its width, and an
/// untracked one has no letter-space to reserve. Neither may gain a spacer:
/// one that reaches a hard-broken line would only widen it towards a wrap.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_untracked_or_left_aligned_hard_broken_paragraph_trails_nothing() {
    for (first, second, alignment) in [
        (Some(3.0), Some(3.0), Alignment::Left),
        (None, None, Alignment::Center),
        (Some(0.0), Some(0.0), Alignment::Center),
    ] {
        let source: String = tracked_two_line_title_source(first, second, alignment, false);

        assert!(
            !source.contains("]#h("),
            "no reserve is due here (spacing {first:?}/{second:?}, \
             {alignment:?}): {source}"
        );
    }
}

/// The #841 Contoso deck's slide number, against a native PowerPoint 16.112
/// export of the same file.
///
/// `slideLayout5` seats the `sldNum` placeholder at x=1 EMU with a 758952 EMU
/// width, and the master's inherited 91440 EMU side insets leave a 45.36pt
/// measure centred at 29.88pt. The master's `lvl1pPr` sets the field in 10pt
/// bold `+mn-lt` — Avenir Next LT Pro, whose digits advance 1323/2048 em — at
/// `spc="100"`. The export puts `5` on 26.13pt and `10` on 22.38pt; measuring
/// the line without its trailing letter-space put us on 26.65 and 22.92.
///
/// Avenir Next LT Pro is a cloud font no CI host has, so the origin is
/// computed from the metrics rather than rendered: what a fixture would
/// exercise is the width, and the width is a pure function of the advances,
/// the tracking, and the glyph count.
#[test]
fn the_contoso_slide_number_lands_on_its_native_origin() {
    const EMU_PER_PT: f64 = 12700.0;
    let content_left_pt: f64 = (1.0 + 91440.0) / EMU_PER_PT;
    let content_right_pt: f64 = (1.0 + 758952.0 - 91440.0) / EMU_PER_PT;
    let centre_pt: f64 = (content_left_pt + content_right_pt) / 2.0;
    // Avenir Next LT Pro Bold: every digit advances 1323 units per 2048 upem.
    let digit_pt: f64 = 1323.0 / 2048.0 * 10.0;
    let tracking_pt: f64 = 1.0;

    for (glyphs, native_origin_pt) in [(1.0_f64, 26.13_f64), (2.0, 22.38)] {
        let width_pt: f64 = glyphs * (digit_pt + tracking_pt);
        let origin_pt: f64 = centre_pt - width_pt / 2.0;

        assert!(
            (origin_pt - native_origin_pt).abs() <= 0.12,
            "the centred slide number must land on the native {native_origin_pt}pt \
             origin within the export's 0.12pt half-grid, got {origin_pt}pt for \
             {glyphs} glyph(s)"
        );
    }
}

/// Where the rotated-box probes below seat their element on the slide. Off
/// both the slide's midlines, so a pivot on the box centre and a pivot on the
/// slide centre cannot coincide.
#[cfg(not(target_arch = "wasm32"))]
const TURNED_BOX_X_PT: f64 = 120.0;
#[cfg(not(target_arch = "wasm32"))]
const TURNED_BOX_Y_PT: f64 = 40.0;

/// One text box on a 960 x 540pt slide, with `apply` free to turn it.
#[cfg(not(target_arch = "wasm32"))]
fn turned_slide_text_box(
    width_pt: f64,
    height_pt: f64,
    apply: impl FnOnce(&mut crate::ir::TextBoxData),
) -> crate::ir::Document {
    let mut element: FixedElement = make_fixed_text_box(
        TURNED_BOX_X_PT,
        TURNED_BOX_Y_PT,
        width_pt,
        height_pt,
        Insets::default(),
        crate::ir::TextBoxVerticalAlign::Top,
        vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Turned".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
    );
    let FixedElementKind::TextBox(text_box) = &mut element.kind else {
        unreachable!("make_fixed_text_box builds a text box");
    };
    apply(text_box);
    make_doc(vec![make_fixed_page(960.0, 540.0, vec![element])])
}

/// Every text run's page-space origin, in layout order.
#[cfg(not(target_arch = "wasm32"))]
fn placed_run_origins(doc: &crate::ir::Document) -> Vec<(f64, f64)> {
    let output = generate_typst(doc).expect("the slide generates");
    let runs = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source));
    assert!(
        !runs.is_empty(),
        "the slide carries text:\n{}",
        output.source
    );
    runs.iter()
        .map(|run| (run.left_pt, run.baseline_pt))
        .collect()
}

/// Turn `seat` clockwise about `centre` the way PowerPoint turns a shape,
/// evaluated here independently of the markup under test.
#[cfg(not(target_arch = "wasm32"))]
fn turned_about(seat: (f64, f64), centre: (f64, f64), rotation_deg: f64) -> (f64, f64) {
    let (sin, cos): (f64, f64) = rotation_deg.to_radians().sin_cos();
    let (dx, dy): (f64, f64) = (seat.0 - centre.0, seat.1 - centre.1);
    (
        centre.0 + cos * dx - sin * dy,
        centre.1 + sin * dx + cos * dy,
    )
}

/// `<a:xfrm rot>` on a shape turns its text box about the box's own centre,
/// whatever the box measures.
///
/// Typst resolves `origin: center` against the frame it lays the body out in,
/// and that frame is clamped to the region. A box taller than the slide
/// therefore pivoted on the slide's midpoint and the whole box landed
/// translated — the clamp #1032 took out of the picture path, still in the
/// text path (issue #1078).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_oversized_turned_text_box_pivots_on_its_own_centre() {
    const WIDTH_PT: f64 = 400.0;
    const HEIGHT_PT: f64 = 856.8;
    const ROTATION_DEG: f64 = 30.0;

    let unturned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(WIDTH_PT, HEIGHT_PT, |_| {}));
    let turned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(WIDTH_PT, HEIGHT_PT, |text_box| {
            text_box.shape_rotation_deg = Some(ROTATION_DEG);
        }));
    assert_eq!(
        unturned.len(),
        turned.len(),
        "the turn lays the content out unchanged and moves the result"
    );

    let centre: (f64, f64) = (
        TURNED_BOX_X_PT + WIDTH_PT / 2.0,
        TURNED_BOX_Y_PT + HEIGHT_PT / 2.0,
    );
    for (index, (seat, got)) in unturned.iter().zip(turned.iter()).enumerate() {
        let want: (f64, f64) = turned_about(*seat, centre, ROTATION_DEG);
        assert!(
            (want.0 - got.0).abs() < 0.05 && (want.1 - got.1).abs() < 0.05,
            "run {index} sits at {got:?}, a turn about the box centre seats it at {want:?}"
        );
    }
}

/// The same clamp under `<a:bodyPr vert>`: the content lays out in a box with
/// the dimensions swapped, so a box wider than the slide is tall becomes a
/// laid-out block taller than the region and the turn pivots on the slide
/// (issue #1078).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn an_oversized_vertical_text_box_pivots_on_its_own_centre() {
    const WIDTH_PT: f64 = 700.0;
    const HEIGHT_PT: f64 = 200.0;
    const ROTATION_DEG: f64 = 270.0;

    // The control is that swapped box standing unturned on the same seat,
    // which is exactly what the vertical path lays out before turning it.
    let unturned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(HEIGHT_PT, WIDTH_PT, |_| {}));
    let turned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(WIDTH_PT, HEIGHT_PT, |text_box| {
            text_box.text_rotation_deg = Some(ROTATION_DEG);
        }));
    assert_eq!(
        unturned.len(),
        turned.len(),
        "the turn lays the content out unchanged and moves the result"
    );

    // Turn about the swapped box's centre, then re-centre that box on the
    // element's own width x height region — the box geometry stays unrotated.
    let centre: (f64, f64) = (
        TURNED_BOX_X_PT + HEIGHT_PT / 2.0,
        TURNED_BOX_Y_PT + WIDTH_PT / 2.0,
    );
    let recentre: (f64, f64) = ((WIDTH_PT - HEIGHT_PT) / 2.0, (HEIGHT_PT - WIDTH_PT) / 2.0);
    for (index, (seat, got)) in unturned.iter().zip(turned.iter()).enumerate() {
        let pivoted: (f64, f64) = turned_about(*seat, centre, ROTATION_DEG);
        let want: (f64, f64) = (pivoted.0 + recentre.0, pivoted.1 + recentre.1);
        assert!(
            (want.0 - got.0).abs() < 0.05 && (want.1 - got.1).abs() < 0.05,
            "run {index} sits at {got:?}, PowerPoint seats it at {want:?}"
        );
    }
}

/// A box that fits the region never met the clamp, so the corner pivot has to
/// leave it exactly where the centre pivot did — otherwise the fix would move
/// every rotated label on the corpus.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_turned_text_box_inside_the_slide_keeps_its_seat() {
    const WIDTH_PT: f64 = 300.0;
    const HEIGHT_PT: f64 = 120.0;
    const ROTATION_DEG: f64 = 45.0;

    let unturned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(WIDTH_PT, HEIGHT_PT, |_| {}));
    let turned: Vec<(f64, f64)> =
        placed_run_origins(&turned_slide_text_box(WIDTH_PT, HEIGHT_PT, |text_box| {
            text_box.shape_rotation_deg = Some(ROTATION_DEG);
        }));

    let centre: (f64, f64) = (
        TURNED_BOX_X_PT + WIDTH_PT / 2.0,
        TURNED_BOX_Y_PT + HEIGHT_PT / 2.0,
    );
    for (index, (seat, got)) in unturned.iter().zip(turned.iter()).enumerate() {
        let want: (f64, f64) = turned_about(*seat, centre, ROTATION_DEG);
        assert!(
            (want.0 - got.0).abs() < 0.05 && (want.1 - got.1).abs() < 0.05,
            "run {index} sits at {got:?}, a turn about the box centre seats it at {want:?}"
        );
    }
}

/// The baselines a slide text box seats one hard-broken line per entry of
/// `sizes_pt` on, in reading order. `wraps` picks the `<a:bodyPr wrap>` the box
/// declares: the two settings take different code paths, and each has had this
/// same advance wrong for its own reason (issues #1115, #1172).
#[cfg(not(target_arch = "wasm32"))]
fn hard_broken_slide_baselines(family: &str, sizes_pt: &[f64], wraps: bool) -> Vec<f64> {
    let mut runs: Vec<Run> = Vec::new();
    for (index, size_pt) in sizes_pt.iter().enumerate() {
        if index > 0 {
            // What the parser emits for `<a:br/>`: the break marker on its
            // own run, with no run properties of its own.
            runs.push(Run {
                text: "\u{000B}".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            });
        }
        runs.push(Run {
            text: format!("Hxg{index}"),
            style: TextStyle {
                font_family: Some(family.to_string()),
                font_size: Some(*size_pt),
                ..TextStyle::default()
            },
            href: None,
            footnote: None,
        });
    }

    let mut text_box = make_fixed_text_box(
        72.0,
        72.0,
        400.0,
        200.0,
        Insets::default(),
        crate::ir::TextBoxVerticalAlign::Top,
        vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs,
        })],
    );
    if let FixedElementKind::TextBox(ref mut data) = text_box.kind {
        data.no_wrap = !wraps;
    }
    let doc = make_doc(vec![make_fixed_page(960.0, 540.0, vec![text_box])]);
    let output = generate_typst(&doc).unwrap();

    let mut baselines: Vec<f64> = crate::render::pdf::compiled_text_runs(&output.source, 0)
        .unwrap_or_else(|error| panic!("compile failed: {error}\n{}", output.source))
        .into_iter()
        .filter(|run| run.text.contains("Hxg"))
        .map(|run| run.baseline_pt)
        .collect();
    baselines.sort_by(f64::total_cmp);
    baselines.dedup_by(|left, right| (*left - *right).abs() < 0.001);
    assert_eq!(
        baselines.len(),
        sizes_pt.len(),
        "expected {} hard-broken lines for {sizes_pt:?}: {baselines:?}\n{}",
        sizes_pt.len(),
        output.source
    );
    baselines
}

/// A slide's hard-broken lines advance by the run's own 1.2em box, whatever
/// the size — in a box that wraps and in one that does not.
///
/// `<a:br/>` reaches the IR as a run carrying no size of its own, so the
/// paragraph has no size every run agrees on and emits no `#set text(size:)`.
/// The line box's `em` edges then resolved against Typst's 11pt default rather
/// than against the size they were computed from, pinning every hard-broken
/// line under 11pt to a flat `1.2 x 11pt` = 13.20pt — 89% too far apart for a
/// 6pt caption (issue #1115).
///
/// A wrapping box paces the same lines through a measured `#stack` instead,
/// which floored each line box at 10pt and so advanced every line under 10pt a
/// flat 12.00pt (issue #1172). Native PowerPoint 16 paces a nine-line column at
/// `1.2 x size` at every size probed — 6, 6.5, 8, 8.5, 9, 9.2, 9.5, 9.8, 10.5,
/// 11.5, 12.25 and 20pt, in a text box and in a table cell alike.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_hard_broken_slide_line_advances_by_its_own_font_size() {
    const FAMILY: &str = "Arial";
    const LINE_COUNT: usize = 4;

    for wraps in [false, true] {
        for font_size_pt in [6.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0] {
            let baselines: Vec<f64> =
                hard_broken_slide_baselines(FAMILY, &[font_size_pt; LINE_COUNT], wraps);
            let expected_advance_pt: f64 = 1.2 * font_size_pt;
            for gap in baselines.windows(2).map(|pair| pair[1] - pair[0]) {
                assert!(
                    (gap - expected_advance_pt).abs() < 0.01,
                    "a {font_size_pt}pt hard-broken line must advance \
                     {expected_advance_pt}pt in a box that wraps={wraps}, \
                     got {gap}pt: {baselines:?}"
                );
            }
        }
    }
}

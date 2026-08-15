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
        "#set text(top-edge: {}em, bottom-edge: -{}em)",
        crate::render::typst_gen::fmt::format_f64(seat_pt / size_pt),
        crate::render::typst_gen::fmt::format_f64((1.5 * 1.2 * size_pt - seat_pt) / size_pt),
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
        output.source.contains("stroke: 1pt + rgb(0, 0, 0)"),
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
    let (top, bottom) =
        emitted_line_box_em(&source).unwrap_or_else(|| panic!("no line box emitted: {source}"));

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
fn slide_baseline_splits_the_lines_extra_leading_evenly() {
    // The glyphs take hhea `ascent + descent`; whatever the 1.2em line has
    // left over is split evenly above and below them, seating the baseline at
    // `(1.2 + ascent - descent) / 2`. A proportional split on OS/2
    // `usWinAscent` was tried and measured 0.45-1.12pt low on every frame of
    // `08_marketing_report_en` (issues #513, #660).
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
    let (top, bottom) = emitted_line_box_em(&source).expect("line box emitted");
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
/// own proportion, because there is no leading left to halve.
///
/// Measured on a native PowerPoint 16.112 export of the #841 Contoso deck,
/// whose titles are set in Posterama Bold (hhea ascender 2134, descender -590
/// per 2048 upem, so its own line is 1.3301em against PowerPoint's 1.2em box).
/// Slide 1 sets that face at 50pt with no `<a:lnSpc>`, and its three baselines
/// pace exactly 60.00pt = 1.2em apart, so the box really is 1.2em there. The
/// export seats the first one 47.06pt below the frame's content top = 0.9411em;
/// the proportional share predicts 0.9401em and the even split 0.9770em, which
/// is 1.79pt low (issue #1020).
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
    let even: f64 = (1.2 + ascent_em - descent_em) / 2.0;
    assert!(
        above < even - 0.01,
        "the even split's negative half-leading pushes the baseline down to \
         {even}em; the proportional share must sit above it, got {above}em"
    );
}

/// PowerPoint seats a baseline a whole number of points below its line box's
/// top, and gives whatever is left of the 1.2em line to the descent gap.
///
/// Measured on native PowerPoint 16.112 exports of a one-factor probe deck:
/// four faces spanning the split's two branches — Georgia (1.13623em, fits the
/// box), Verdana (1.21533em), Avenir Next LT Pro (1.21289em) and Posterama
/// (1.33008em, all three overflow) — in bottom-anchored boxes with every inset
/// zeroed, at 8, 11, 14, 18, 24, 28, 32, 36, 40, 44, 48, 54, 72 and 100pt. All
/// 56 cells land on `1.2 x size - round(share x size)` within the export's
/// 0.12pt half-grid, and none within 0.12pt of the unrounded share. The seat's
/// share of the em therefore is *not* constant across sizes: Avenir Next LT Pro
/// keeps 0.192em under its baseline at 10pt and 0.2625em at 32pt (issue #1074).
///
/// Georgia's cells need the *proportional* share to fit, not the even split a
/// face that fits the box currently gets — a separate root cause tracked in
/// #1118. What this asserts is the rounding, which is common to both.
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
        let (top_em, bottom_em) = emitted_line_box_em(&source).expect("line box emitted");
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
    // Avenir Next LT Pro Bold: hhea ascender 1972, descender -512 per 2048 upem.
    let (above_em, _) =
        crate::render::pdf::powerpoint_line_box_split_em(1972.0 / 2048.0, 512.0 / 2048.0)
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
    // Avenir Next LT Pro: hhea ascender 1972, descender -512 per 2048 upem.
    let (plain_above, _) =
        crate::render::pdf::powerpoint_line_box_split_em(1972.0 / 2048.0, 512.0 / 2048.0)
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
        "two faces with different hhea ascent/descent should seat the baseline \
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
    let (top, bottom) =
        emitted_line_box_em(&source).unwrap_or_else(|| panic!("no line box emitted: {source}"));
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
    let (_plain_top, plain_bottom) = emitted_line_box_em(&plain).expect("line box emitted");
    let (scaled_top, scaled_bottom) = emitted_line_box_em(&scaled).expect("line box emitted");
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
        crate::render::pdf::powerpoint_line_box_split_em(2134.0 / 2048.0, 590.0 / 2048.0)
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
        crate::render::pdf::powerpoint_line_box_split_em(2134.0 / 2048.0, 590.0 / 2048.0)
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
        crate::render::pdf::powerpoint_line_box_split_em(2134.0 / 2048.0, 590.0 / 2048.0)
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

/// A hard break starts a PowerPoint line whose own largest run sets the full
/// 1.2em advance. Typst normally combines the preceding line's descent with
/// the following line's ascent, which is wrong when the sizes differ (#683).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn powerpoint_hard_break_advance_uses_the_following_lines_font_size() {
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
        (baselines[1] - baselines[0] - 12.0).abs() < 0.01,
        "the 10pt following line must own a 12pt advance: {baselines:?}\n{}",
        output.source
    );
    assert!(
        (baselines[2] - baselines[1] - 15.0).abs() < 0.01,
        "the 12.5pt following line must own a 15pt advance: {baselines:?}\n{}",
        output.source
    );
}

/// A proportional `<a:lnSpc>` scales the following line's complete box; the
/// hard-break correction must preserve that paragraph-level multiplier.
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

    assert!(
        (small_baseline - large_baseline - 18.0).abs() < 0.01,
        "the 1.5 x 1.2em box of the 10pt following line must advance 18pt: \
         {large_baseline}, {small_baseline}\n{}",
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

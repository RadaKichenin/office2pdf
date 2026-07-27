use super::*;

#[test]
fn test_table_cell_with_multiple_paragraphs() {
    let multi_para_cell = TableCell {
        content: vec![
            Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: "First para".to_string(),
                    style: TextStyle::default(),
                    href: None,
                    footnote: None,
                }],
            }),
            Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: "Second para".to_string(),
                    style: TextStyle::default(),
                    href: None,
                    footnote: None,
                }],
            }),
        ],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![multi_para_cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("First para"),
        "Expected First para in: {result}"
    );
    assert!(
        result.contains("Second para"),
        "Expected Second para in: {result}"
    );
}

#[test]
fn test_table_cell_simple_list_uses_compact_fixed_text_layout() {
    let list = List {
        kind: ListKind::Unordered,
        items: vec![
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "First item".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                }],
                level: 0,
                start_at: None,
            },
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Second item".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                }],
                level: 0,
                start_at: None,
            },
        ],
        level_styles: BTreeMap::new(),
    };
    let cell = TableCell {
        content: vec![Block::List(list)],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#stack(dir: ttb"),
        "Expected compact stack-based list layout in: {result}"
    );
    assert!(
        !result.contains("#list("),
        "Compact table-cell lists should not use Typst list layout in: {result}"
    );
    assert!(result.contains("First item"));
    assert!(result.contains("Second item"));
}

#[test]
fn test_table_cell_simple_list_treats_default_and_explicit_left_as_same_style() {
    let list = List {
        kind: ListKind::Unordered,
        items: vec![
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle {
                        alignment: Some(Alignment::Left),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "First item".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                }],
                level: 0,
                start_at: None,
            },
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Second item".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                }],
                level: 0,
                start_at: None,
            },
        ],
        level_styles: BTreeMap::new(),
    };
    let cell = TableCell {
        content: vec![Block::List(list)],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#stack(dir: ttb"),
        "Expected compact stack-based list layout when only left-alignment explicitness differs: {result}"
    );
    assert!(
        !result.contains("#list("),
        "Equivalent left-alignment styles should not force Typst list layout in: {result}"
    );
}

#[test]
fn test_table_cell_compact_list_adds_inter_item_spacing_from_line_spacing() {
    let list = List {
        kind: ListKind::Unordered,
        items: vec![
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle {
                        line_spacing: Some(LineSpacing::Proportional(1.5)),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "First item".to_string(),
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
            ListItem {
                content: vec![Paragraph {
                    style: ParagraphStyle {
                        line_spacing: Some(LineSpacing::Proportional(1.5)),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Second item".to_string(),
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
        level_styles: BTreeMap::new(),
    };
    let cell = TableCell {
        content: vec![Block::List(list)],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#set par(leading: 12pt)"),
        "Expected paragraph leading derived from PPT line spacing in: {result}"
    );
    assert!(
        result.contains("#stack(dir: ttb, spacing: 12pt"),
        "Compact table-cell lists should add inter-item spacing derived from PPT line spacing in: {result}"
    );
}

#[test]
fn test_east_asian_table_cell_snaps_to_the_document_grid() {
    // Under a grid the author actually turned on, East Asian cell text snaps
    // to it exactly as body text does. The box is still emitted as a fixed box
    // with zero leading so auto-height rows are not left short (issue #396).
    //
    // No fixture in the business corpus reaches this branch: their `w:docGrid`
    // elements carry the `default` type, so their rows are sized from the East
    // Asian line alone — 03_meeting_minutes_ko's 25.44pt rows decompose to
    // 3.5pt cell margins, a 16.43pt line for 9.5pt Malgun, its 1.5pt `w:after`
    // and a 0.5pt border, with no 18pt slot anywhere (issue #518). Uses a
    // Typst-embedded font so the test is environment-free.
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    // One 18pt grid line, since the East Asian line fits inside it.
    let grid_em: f64 = 18.0 / font_size;
    // The slot's slack accrues below the baseline: the ascent stays the
    // constant it would be without a grid (issue #518).
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let bottom_em: f64 = grid_em - top_em;
    // What the same cell would emit with no grid in force.
    let ungridded_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "회의 안건".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let mut page = match make_flow_page(vec![Block::Table(table)]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(bottom_em)
        )),
        "Korean cell must fill the 18pt grid line box: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "cell line box uses zero leading (box already equals the full line): {result}"
    );
    assert!(
        !result.contains(&format!(
            "bottom-edge: -{}em",
            format_f64(ungridded_bottom_em)
        )),
        "Korean cell must take the grid slot, not its own East Asian line: {result}"
    );
}

#[test]
fn test_latin_table_cell_uses_natural_line_height() {
    // Latin cells likewise fill the font's full hhea line box (Word single
    // spacing = hhea line), not Typst's glyph-tight default (issues #385,
    // #396).
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender;
    let bottom_em: f64 = word_pitch_em - top_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Agenda".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(bottom_em)
        )),
        "Latin cell must fill the full hhea line box: {result}"
    );
}

/// Word puts every cell in a table row on one baseline. Choosing the
/// grid-snapped line box per cell, from that cell's own text, gave a Korean
/// label a taller box than its numeric neighbours and split the row across two
/// baselines 4.29pt apart (issue #498). The grid is a property of the section,
/// not of a cell's content.
#[test]
fn mixed_script_row_shares_one_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let grid_em: f64 = 18.0 / font_size;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let bottom_em: f64 = grid_em - top_em;
    let make_cell = |text: &str| TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            // A Korean month label beside a numeric column, as in the research
            // report fixture.
            cells: vec![make_cell("2024년 1월"), make_cell("380")],
            height: None,
        }],
        column_widths: vec![120.0, 80.0],
        ..Table::default()
    };
    let mut page = match make_flow_page(vec![Block::Table(table)]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    let grid_box = format!(
        "top-edge: {}em, bottom-edge: -{}em",
        format_f64(top_em),
        format_f64(bottom_em)
    );
    assert_eq!(
        result.matches(&grid_box).count(),
        2,
        "both cells in the row must take the same grid line box: {result}"
    );
}

/// Triangulation for [`mixed_script_row_shares_one_line_box`]: the rule is
/// "the row decides", not "always snap". A row with no East Asian text keeps
/// the font's own hhea line even when the section declares a grid.
#[test]
fn latin_only_row_under_a_grid_keeps_the_font_line() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let hhea_top_em: f64 = ascender;
    let hhea_bottom_em: f64 = word_pitch_em - hhea_top_em;
    let make_cell = |text: &str| TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_cell("Aug 3"), make_cell("380")],
            height: None,
        }],
        column_widths: vec![120.0, 80.0],
        ..Table::default()
    };
    let mut page = match make_flow_page(vec![Block::Table(table)]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    assert_eq!(
        result
            .matches(&format!(
                "top-edge: {}em, bottom-edge: -{}em",
                format_f64(hhea_top_em),
                format_f64(hhea_bottom_em)
            ))
            .count(),
        2,
        "a Latin-only row must keep the font's hhea line under a grid: {result}"
    );
}

/// Word snaps a grid row's line *plus* the paragraph's `w:spacing w:after`,
/// so the gap lives inside the line box and must not also be emitted after the
/// runs. Adding it outside made every grid-scoped row 1.06pt too tall
/// (issues #500, #503).
#[test]
fn grid_cell_absorbs_space_after_into_the_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    // The East Asian line plus a 1.5pt gap still fits one 18pt grid line.
    let grid_em: f64 = 18.0 / font_size;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(1.5),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "2024년 1월".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![160.0],
        ..Table::default()
    };
    let mut page = match make_flow_page(vec![Block::Table(table)]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let doc = make_doc(vec![Page::Flow(page)]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(grid_em - top_em)
        )),
        "line plus w:after should snap to one 18pt grid line: {result}"
    );
    assert!(
        !result.contains("#v(1.5pt)"),
        "the absorbed gap must not also be emitted after the runs: {result}"
    );
}

/// Triangulation: without a grid there is no snap to absorb the gap into, so
/// the paragraph's `w:spacing w:after` must still be emitted.
#[test]
fn ungridded_cell_still_emits_space_after() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(1.5),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "Aug 3".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(10.0),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![160.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#v(1.5pt)"),
        "an unsnapped cell keeps its declared gap: {result}"
    );
}

/// Word counts a border's width in the row height; Typst draws our per-cell
/// strokes without reserving space for them. Each horizontal border is shared
/// between the rows either side, so a cell takes half (issues #500, #503).
#[test]
fn cell_border_width_joins_the_inset() {
    let border = CellBorder {
        top: Some(BorderSide {
            width: 0.5,
            color: Color::new(0, 0, 0),
            style: BorderLineStyle::Solid,
        }),
        bottom: Some(BorderSide {
            width: 0.5,
            color: Color::new(0, 0, 0),
            style: BorderLineStyle::Solid,
        }),
        left: None,
        right: None,
    };
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Aug 3".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(border),
        padding: Some(Insets {
            top: 3.5,
            right: 5.0,
            bottom: 3.5,
            left: 5.0,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![160.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("top: 3.75pt") && result.contains("bottom: 3.75pt"),
        "half of each 0.5pt border should join the 3.5pt inset: {result}"
    );
}

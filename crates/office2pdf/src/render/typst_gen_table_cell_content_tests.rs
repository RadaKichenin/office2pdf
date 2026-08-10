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

/// Excel seats a bottom-aligned cell's line box on the descender line: the
/// last line's descent bottom rests on the row's bottom inset edge, with all
/// slack above. The East Asian row model splits its 0.3-line bonus evenly
/// around the baseline, so a bottom-aligned Korean cell floated 0.15 lines
/// above where Excel prints it (issue #618). The removed surplus moves into
/// leading so multi-line baseline-to-baseline advance is unchanged.
#[test]
fn bottom_aligned_spreadsheet_cell_seats_its_line_box_on_the_descender() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    // What the same cell emits when the box stays symmetric.
    let symmetric_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    // The sub-baseline surplus the descender seat removes from the box.
    let leading_pt: f64 = ((1.3 * word_pitch_em - top_em - descender) * font_size).max(0.0);
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "급여 총액".to_string(),
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
            height: Some(20.0),
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(descender)
        )),
        "bottom-aligned spreadsheet cell must end its line box at the descender: {result}"
    );
    assert!(
        result.contains(&format!("#set par(leading: {}pt)", format_f64(leading_pt))),
        "the removed sub-baseline surplus must move into leading: {result}"
    );
    assert!(
        !result.contains(&format!(
            "bottom-edge: -{}em",
            format_f64(symmetric_bottom_em)
        )),
        "the symmetric East Asian box must not survive under bottom alignment: {result}"
    );
}

/// Triangulation: an explicitly centred cell in the same spreadsheet keeps the
/// symmetric box — the East Asian surplus is even around the baseline, so
/// centre alignment already matches Excel and must not move (issue #618).
#[test]
fn center_aligned_spreadsheet_cell_keeps_the_symmetric_line_box() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let symmetric_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "급여 총액".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        vertical_align: Some(CellVerticalAlign::Center),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: Some(20.0),
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(symmetric_bottom_em)
        )),
        "a centred spreadsheet cell keeps the symmetric East Asian box: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "a centred spreadsheet cell keeps zero leading: {result}"
    );
    assert!(
        !result.contains(&format!("bottom-edge: -{}em", format_f64(descender))),
        "a centred spreadsheet cell must not be re-seated on the descender: {result}"
    );
}

/// Regression: a bottom-aligned East Asian spreadsheet cell in an AUTO-height
/// row keeps the symmetric line box and zero leading. In auto rows the
/// renderer sizes the row from the content, whose intrinsic height was
/// calibrated against Excel GT (#396/#411/#498) with the symmetric box; only
/// fixed rows have slack for alignment to distribute, and only they were
/// measured in #618.
#[test]
fn bottom_aligned_spreadsheet_cell_in_auto_height_row_keeps_the_symmetric_line_box() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let symmetric_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "급여 총액".to_string(),
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
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: -{}em",
            format_f64(top_em),
            format_f64(symmetric_bottom_em)
        )),
        "an auto-height row keeps the symmetric East Asian box: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "an auto-height row keeps zero leading: {result}"
    );
    assert!(
        !result.contains(&format!("bottom-edge: -{}em", format_f64(descender))),
        "an auto-height row must not be re-seated on the descender: {result}"
    );
}

/// Triangulation: a Word-style table (no descender seating) keeps its current
/// emission even for bottom-aligned cells — Word GT has not verified that
/// seating, so DOCX/PPTX output must stay byte-identical (issue #618).
#[test]
fn bottom_aligned_word_table_cell_keeps_the_symmetric_line_box() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let symmetric_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
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
        vertical_align: Some(CellVerticalAlign::Bottom),
        ..TableCell::default()
    };
    // A fixed row height, so this guards the table's seating flag itself
    // rather than passing trivially through the fixed-row gate.
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: Some(20.0),
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
            format_f64(symmetric_bottom_em)
        )),
        "a Word table keeps the symmetric East Asian box under bottom alignment: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "a Word table keeps zero leading: {result}"
    );
    assert!(
        !result.contains(&format!("bottom-edge: -{}em", format_f64(descender))),
        "a Word table cell must not be re-seated on the descender: {result}"
    );
}

/// Two stacked `<w:p>` in one `<w:tc>` are separated by the first paragraph's
/// `w:spacing w:after` alone. Each paragraph's `#block` wrapper must therefore
/// carry `above: 0pt, below: 0pt`: sibling blocks otherwise pick up Typst's
/// ambient default block spacing (1.2em at the document size — the +13.2pt of
/// issue #625), which Word does not have. The gap is then exactly the fixed
/// line box's advance plus the explicit `#v(1.5pt)`.
#[test]
fn stacked_cell_paragraphs_zero_the_default_block_spacing() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let make_para = |text: &str, space_after: Option<f64>| -> Block {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after,
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(9.5),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })
    };
    let cell = TableCell {
        content: vec![
            make_para("Hanbit Tech Co., Ltd.", Some(1.5)),
            make_para("CEO Lee Jun-seo (seal)", None),
        ],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![225.65],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert_eq!(
        result.matches("#block(above: 0pt, below: 0pt)[").count(),
        2,
        "both stacked cell paragraphs must zero the default block spacing: {result}"
    );
    assert!(
        !result.contains("#block()["),
        "no stacked cell paragraph may leave Typst's default block spacing in force: {result}"
    );
    assert_eq!(
        result.matches("#v(1.5pt)").count(),
        1,
        "the declared w:after is the only inter-paragraph gap: {result}"
    );
}

/// Triangulation: with no `w:spacing w:after` at all, stacked cell paragraphs
/// stack flush. `space_after: None` here means Word resolves no gap — the
/// parser already folds `w:docDefaults` and style-chain `w:after` into
/// `space_after` (docx_styles.rs), so a `None` reaching codegen is a document
/// whose effective `w:after` is absent, which Word lays out as 0.
#[test]
fn stacked_cell_paragraphs_without_w_after_stack_flush() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let make_para = |text: &str| -> Block {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(9.5),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })
    };
    let cell = TableCell {
        content: vec![make_para("First line"), make_para("Second line")],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![225.65],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert_eq!(
        result.matches("#block(above: 0pt, below: 0pt)[").count(),
        2,
        "paragraphs without w:after still suppress the default block spacing: {result}"
    );
    assert!(
        !result.contains("#v("),
        "no gap may be synthesized when the document declares none: {result}"
    );
}

/// A single-paragraph cell has no sibling block to leak spacing against, so
/// its emission must stay byte-identical to before the #625 fix: the plain
/// `#block()` wrapper, the fixed line box, and the trailing `#v(w:after)`
/// inside it (which Word counts into the row height — the contract fixture's
/// header row is exact today and must stay so).
#[test]
fn single_paragraph_cell_emission_is_unchanged() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(1.5),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "Party A".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(9.5),
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
        column_widths: vec![225.65],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#block()["),
        "a lone cell paragraph keeps its exact pre-fix wrapper: {result}"
    );
    assert!(
        !result.contains("above: 0pt"),
        "a lone cell paragraph gains no spacing parameters: {result}"
    );
    assert!(
        result.contains("#v(1.5pt)"),
        "the trailing w:after stays inside the block for the row height: {result}"
    );
}

/// A cell paragraph carrying its own `w:spacing w:line` now gets a fixed line
/// box of the declared multiple, so it also gets the block-spacing
/// suppression: the box carries the whole advance, and Typst's own gap on top
/// of it would be counted twice.
///
/// It used to get neither — `word_cell_line_box` bailed on `line_spacing`, so
/// the paragraph fell back to Typst's line model and the suppression had to be
/// gated off it, or the stack collapsed onto itself. Fixing that bail
/// (issue #727) is what lets both apply here.
#[test]
fn line_spaced_stacked_cell_paragraphs_take_a_scaled_line_box() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let make_para = |text: &str| -> Block {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                line_spacing: Some(LineSpacing::Proportional(1.5)),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(9.5),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })
    };
    let cell = TableCell {
        content: vec![
            make_para("Hanbit Tech Co., Ltd."),
            make_para("CEO Lee Jun-seo (seal)"),
        ],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![225.65],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let (ascender_em, _descender_em, word_pitch_em) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif").expect("checked above");
    assert!(
        result.contains(&format!(
            "#set text(top-edge: {}em, bottom-edge: -{}em)",
            format_f64(ascender_em),
            format_f64(1.5 * word_pitch_em - ascender_em)
        )),
        "each paragraph takes a box 1.5 x Word's line: {result}"
    );
    assert_eq!(
        result.matches("above: 0pt, below: 0pt").count(),
        2,
        "and the box carrying the advance means the wrapper contributes none: {result}"
    );
}

/// Word lays an empty `<w:p>` in a table cell out as one full blank line: the
/// paragraph mark still occupies its line box. The cell path emitted nothing
/// at all for it — no wrapper, no line box, no strut — so the spacer had zero
/// height and the stack only looked right while Typst's ambient block spacing
/// happened to stand in for it (issue #625). The empty paragraph must hold one
/// line box of its own, sized like its neighbours'.
#[test]
fn an_empty_cell_paragraph_holds_one_full_line_box() {
    let Some((_ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 9.5;
    let line_box_height_pt: f64 = word_pitch_em * font_size;
    let make_para = |text: &str| -> Block {
        Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_after: Some(1.5),
                ..ParagraphStyle::default()
            },
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
        })
    };
    let cell = TableCell {
        content: vec![
            make_para("Hanbit Tech Co., Ltd."),
            Block::Paragraph(Paragraph {
                style: ParagraphStyle {
                    space_after: Some(1.5),
                    ..ParagraphStyle::default()
                },
                runs: vec![],
            }),
            make_para("CEO Lee Jun-seo (seal)"),
        ],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![225.65],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "#box(width: 0pt, height: {}pt)",
            format_f64(line_box_height_pt)
        )),
        "the empty spacer paragraph must hold one line box sized like its \
         neighbours': {result}"
    );
    assert_eq!(
        result.matches("#v(1.5pt)").count(),
        3,
        "every paragraph's own w:after still separates it from the next: {result}"
    );
}

/// Word's `w:ind` offsets a paragraph's column wherever the paragraph sits,
/// and the cell path never emitted it: the invoice template of issue #841 puts
/// its `Title` style — `<w:ind w:left="101"/>`, 5.05pt — in the first cell of a
/// layout table, and a native export offsets it exactly that far from the
/// column's other paragraphs while we rendered it flush (issue #938).
#[test]
fn cell_paragraph_carries_its_left_indent() {
    let indented = Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            indent_left: Some(5.05),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "FAKTURA".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    });
    let flush = Block::Paragraph(Paragraph {
        style: ParagraphStyle::default(),
        runs: vec![Run {
            text: "DATO".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    });
    let table = Table {
        rows: vec![TableRow {
            cells: vec![TableCell {
                content: vec![indented, flush],
                ..TableCell::default()
            }],
            height: None,
        }],
        column_widths: vec![176.95],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let indented_block: &str = result
        .split("FAKTURA")
        .next()
        .and_then(|before| before.rfind("#block(").map(|at| &before[at..]))
        .expect("the indented paragraph opens a block");
    assert!(
        indented_block.contains("inset: (left: 5.05pt, right: 0pt)"),
        "the indented cell paragraph carries its w:ind as an inset: {result}"
    );
    assert_eq!(
        result.matches("inset: (left:").count(),
        1,
        "only the indented paragraph gets an inset; the flush one keeps none: {result}"
    );
}

/// A right indent narrows the column the cell text wraps in, so it has to
/// reach the same inset rather than being dropped (issue #938).
#[test]
fn cell_paragraph_carries_its_right_indent() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![TableCell {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        indent_right: Some(12.0),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Narrowed".to_string(),
                        style: TextStyle::default(),
                        href: None,
                        footnote: None,
                    }],
                })],
                ..TableCell::default()
            }],
            height: None,
        }],
        column_widths: vec![176.95],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("inset: (left: 0pt, right: 12pt)"),
        "the cell paragraph carries its right indent as an inset: {result}"
    );
}

/// A cell paragraph's `w:spacing w:line` scales its line box, exactly as it
/// scales a body paragraph's. `word_cell_line_box` bailed on any declared line
/// spacing, so the multiple never applied inside a cell and the paragraph fell
/// back to Typst's own advance — short of Word's by the whole difference
/// (issue #727).
#[test]
fn a_line_spaced_cell_paragraph_scales_its_line_box() {
    let Some((ascender_em, descender_em, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let make_cell = |line_spacing: Option<LineSpacing>| -> TableCell {
        TableCell {
            content: vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle {
                    line_spacing,
                    ..ParagraphStyle::default()
                },
                runs: vec![Run {
                    text: "Signature".to_string(),
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
        }
    };
    let render = |line_spacing: Option<LineSpacing>| -> String {
        let table = Table {
            rows: vec![TableRow {
                cells: vec![make_cell(line_spacing)],
                height: None,
            }],
            column_widths: vec![200.0],
            ..Table::default()
        };
        let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
        generate_typst(&doc).unwrap().source
    };

    // Unspaced: Word's single line, which the metric pair sums to directly.
    let single: String = render(None);
    assert!(
        single.contains(&format!(
            "#set text(top-edge: {}em, bottom-edge: -{}em)",
            format_f64(ascender_em),
            format_f64(word_pitch_em - ascender_em)
        )),
        "an unspaced cell keeps Word's single line: {single}"
    );

    // 1.5 lines: the same box, scaled — the bottom edge carries the surplus.
    let spaced: String = render(Some(LineSpacing::Proportional(1.5)));
    assert!(
        spaced.contains(&format!(
            "#set text(top-edge: {}em, bottom-edge: -{}em)",
            format_f64(ascender_em),
            format_f64(1.5 * word_pitch_em - ascender_em)
        )),
        "a 1.5-line cell advances 1.5 x Word's line: {spaced}"
    );
    let _ = descender_em;
}

/// `w:lineRule="exact"` states the advance outright, so the box is that many
/// points tall whatever the font asks for (issue #727).
#[test]
fn an_exactly_spaced_cell_paragraph_takes_the_stated_advance() {
    let Some((ascender_em, _descender_em, _pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let table = Table {
        rows: vec![TableRow {
            cells: vec![TableCell {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        line_spacing: Some(LineSpacing::Exact(18.0)),
                        ..ParagraphStyle::default()
                    },
                    runs: vec![Run {
                        text: "Exact".to_string(),
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
            }],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let source = generate_typst(&doc).unwrap().source;

    assert!(
        source.contains(&format!(
            "#set text(top-edge: {}em, bottom-edge: -{}em)",
            format_f64(ascender_em),
            format_f64(18.0 / font_size - ascender_em)
        )),
        "an exact rule states the advance outright: {source}"
    );
}

/// A grid-snapped row folds the paragraph's `w:spacing w:after` into its line
/// box, so the caller must not emit it again. That holds for a line-spaced
/// paragraph too, now that one resolves a box at all: gating the absorption on
/// `line_spacing` would emit the gap twice (issue #727).
#[test]
fn a_grid_snapped_line_spaced_cell_emits_its_space_after_once() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                line_spacing: Some(LineSpacing::Proportional(1.5)),
                space_after: Some(1.5),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "계약자".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(9.5),
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
        column_widths: vec![225.65],
        ..Table::default()
    };
    let mut page = match make_flow_page(vec![Block::Table(table)]) {
        Page::Flow(flow) => flow,
        _ => unreachable!(),
    };
    page.line_grid_pitch = Some(18.0);
    page.line_grid_snaps_lines = true;
    let source = generate_typst(&make_doc(vec![Page::Flow(page)]))
        .unwrap()
        .source;

    assert_eq!(
        source.matches("#v(1.5pt)").count(),
        0,
        "the grid-snapped box already carries the gap: {source}"
    );
}

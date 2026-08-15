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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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

/// The row's line box keys on the face its lines are set in, not on the
/// script of its characters — the rule the body line took in issue #643
/// (issue #814).
///
/// Measured on a native export: twelve rows of `10_research_report_ko`
/// relabelled `2025-01`..`2025-12` — every cell Latin-only, every `w:rFonts`
/// slot Malgun Gothic — keep Word's 25.44pt row pitch exactly, where the bare
/// hhea line would pitch them at 21.64pt.
#[test]
fn latin_only_row_in_east_asian_face_keeps_the_east_asian_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let font_size: f64 = 9.5;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    let make_cell = |text: &str| TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Malgun Gothic".to_string()),
                    east_asian_font_family: Some("Malgun Gothic".to_string()),
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
            minimum_height: None,
            cells: vec![make_cell("2025-01"), make_cell("464")],
            height: None,
        }],
        column_widths: vec![120.0, 80.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let east_asian_box = format!(
        "top-edge: {}em, bottom-edge: -{}em",
        format_f64(top_em),
        format_f64(bottom_em)
    );
    assert_eq!(
        result.matches(&east_asian_box).count(),
        2,
        "both Latin-only cells set in a CJK face must take the East Asian box: {result}"
    );
}

/// Triangulation for [`latin_only_row_in_east_asian_face_keeps_the_east_asian_line_box`]:
/// the face keys the line *height*, but only East Asian *text* snaps to a
/// document grid — the same asymmetry the body path measured (issues #354,
/// #643). A Latin-only row in a CJK face keeps its own 1.3-line advance under
/// an active grid rather than being stretched to the grid pitch.
#[test]
fn latin_only_row_in_east_asian_face_does_not_snap_to_the_grid() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let font_size: f64 = 9.5;
    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let natural_bottom_em: f64 = 1.3 * word_pitch_em - top_em;
    let grid_bottom_em: f64 = 18.0 / font_size - top_em;
    let make_cell = |text: &str| TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle {
                    font_family: Some("Malgun Gothic".to_string()),
                    east_asian_font_family: Some("Malgun Gothic".to_string()),
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
            minimum_height: None,
            cells: vec![make_cell("2025-01"), make_cell("464")],
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
                format_f64(top_em),
                format_f64(natural_bottom_em)
            ))
            .count(),
        2,
        "the row keeps its own East Asian line under the grid: {result}"
    );
    assert!(
        !result.contains(&format!("bottom-edge: -{}em", format_f64(grid_bottom_em))),
        "a Latin-only row must not be stretched to the grid pitch: {result}"
    );
}

/// End-to-end pin for issue #814's probe fixture: `10_research_report_ko`
/// with its twelve `2025년 N월` row labels relabelled `2025-01`..`2025-12` —
/// the one patched factor — whose native Word export keeps the East Asian
/// 25.44pt row pitch on every relabelled row. Every cell in the table names
/// Malgun Gothic in all four `w:rFonts` slots, so all 31 rows must share the
/// East Asian line box even where a row's every character is ASCII.
#[test]
fn research_report_probe_rows_share_the_east_asian_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let data = include_bytes!(
        "../../../../tests/fixtures/docx/issue_814_latin_row_in_cjk_face_probe.docx"
    );
    let (doc, _warnings) = crate::parser::Parser::parse(
        &crate::parser::docx::DocxParser,
        data,
        &crate::config::ConvertOptions::default(),
    )
    .expect("the probe document parses");
    let result = generate_typst(&doc).unwrap().source;

    let top_em: f64 = ascender + 0.15 * word_pitch_em;
    let east_asian_box = format!(
        "top-edge: {}em, bottom-edge: -{}em",
        format_f64(top_em),
        format_f64(1.3 * word_pitch_em - top_em)
    );
    let bare_box = format!(
        "top-edge: {}em, bottom-edge: -{}em",
        format_f64(ascender),
        format_f64(word_pitch_em - ascender)
    );
    assert!(
        result.matches(&east_asian_box).count() >= 31 * 5,
        "all 31 rows x 5 columns must share the East Asian line box; found {}",
        result.matches(&east_asian_box).count()
    );
    assert!(
        !result.contains(&bare_box),
        "a relabelled Latin-only row must not fall back to the bare hhea box"
    );
}

/// A spreadsheet row set in an East Asian face keeps the bare hhea box —
/// the face check issue #814 gave a Word table row must not reach a sheet,
/// because Excel's own box is the bare line for *any* script (issue #1060,
/// measured; see [`spreadsheet_rows_share_one_line_box_whatever_script`]).
#[test]
fn latin_only_spreadsheet_row_in_east_asian_face_keeps_the_hhea_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let font_size: f64 = 10.0;
    let east_asian_top_em: f64 = ascender + 0.15 * word_pitch_em;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "2025-01".to_string(),
                style: TextStyle {
                    font_family: Some("Malgun Gothic".to_string()),
                    east_asian_font_family: Some("Malgun Gothic".to_string()),
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
            minimum_height: None,
            cells: vec![cell],
            height: Some(20.0),
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let boxes: Vec<(f64, f64)> = cell_line_boxes_em(&result);
    assert_eq!(boxes.len(), 1, "one cell, one line box: {result}");
    assert!(
        (boxes[0].0 + boxes[0].1 - word_pitch_em).abs() < 1e-9,
        "a sheet's Latin-only row keeps the bare hhea line: {boxes:?}"
    );
    assert!(
        !result.contains(&format!("top-edge: {}em", format_f64(east_asian_top_em))),
        "the East Asian ascent excess must not reach a sheet's Latin row: {result}"
    );
}

/// Every distinct line-box ascent `source` sets, as written.
///
/// Only the `#set text` form: an eojeol frame re-emits the same ascent
/// resolved to points at its own token size (issue #626), which exists on
/// Korean text alone and would read as a line-box difference here.
fn distinct_top_edges(source: &str) -> std::collections::BTreeSet<&str> {
    const MARKER: &str = "#set text(top-edge: ";
    source
        .match_indices(MARKER)
        .map(|(index, _)| {
            let value: &str = &source[index + MARKER.len()..];
            &value[..value.find(',').unwrap_or(value.len())]
        })
        .collect()
}

/// Every line advance `source` sets, as `(ascent em, descent em, leading pt,
/// size pt)` in emission order.
///
/// The descender seat trims the box below the baseline and moves the trimmed
/// surplus into leading, so the quantity that stays invariant across seats is
/// the *advance*: `(ascent + descent) x size + leading`.
fn cell_line_advances(source: &str) -> Vec<(f64, f64, f64, f64)> {
    const MARKER: &str = "#set text(top-edge: ";
    source
        .match_indices(MARKER)
        .filter_map(|(index, _)| {
            let rest: &str = &source[index + MARKER.len()..];
            let (top, rest) = rest.split_once("em, bottom-edge: ")?;
            let (bottom, rest) = rest.split_once("em)\n#set par(leading: ")?;
            let (leading, rest) = rest.split_once("pt)")?;
            let (_, rest) = rest.split_once("size: ")?;
            let (size, _) = rest.split_once("pt")?;
            Some((
                top.parse::<f64>().ok()?,
                -bottom.parse::<f64>().ok()?,
                leading.parse::<f64>().ok()?,
                size.parse::<f64>().ok()?,
            ))
        })
        .collect()
}

/// Every line box `source` sets, as `(ascent em, descent em)` pairs in
/// emission order.
///
/// A sheet cell's ascent and descent are no longer a fixed split of the line:
/// the seat that puts the baseline where Excel prints it redistributes the box
/// around the baseline, keyed to the row's track (issue #1063). What stays
/// invariant is the box's *height*, so the assertions that used to pin the
/// split read the pair and check the sum.
fn cell_line_boxes_em(source: &str) -> Vec<(f64, f64)> {
    const MARKER: &str = "#set text(top-edge: ";
    source
        .match_indices(MARKER)
        .filter_map(|(index, _)| {
            let rest: &str = &source[index + MARKER.len()..];
            let (top, rest) = rest.split_once("em, bottom-edge: ")?;
            let (bottom, _) = rest.split_once("em)")?;
            // The emitted descent is negative downward; report it positive.
            Some((top.parse::<f64>().ok()?, -bottom.parse::<f64>().ok()?))
        })
        .collect()
}

/// A spreadsheet row's line box does not vary with the script of its
/// characters, and the box it keeps is the *bare* hhea line.
///
/// Measured on a native Excel-for-Mac export of the probe workbook committed
/// as `tests/fixtures/xlsx/issue_1060_sheet_row_line_box_probe.xlsx`, whose
/// paired blocks differ only in that script — same face (Malgun Gothic, the
/// workbook's Normal font too), size, row-height mode, column and vertical
/// alignment. All four pairs print 0.00pt apart: auto rows at a 20.00pt track
/// each with equal seats, `ht=36` top-aligned rows seated identically, and
/// `ht=36` centred rows seated identically. Our text-keyed gate seated the
/// Korean top-aligned row 2.79pt low — `0.15 x` Malgun's 1.330078em hhea
/// pitch at 14pt (issue #1060).
///
/// So the two candidate gates are not the choice: Excel's invariant is the
/// bare line, which a 1.3-factor box contradicts outright — 14pt auto rows
/// print 20.00pt tracks that a 24.20pt East Asian box does not fit. Extending
/// issue #814's face check to a sheet would have made both rows *equally*
/// wrong instead; a sheet row takes no East Asian box at all.
#[test]
fn spreadsheet_rows_share_one_line_box_whatever_script() {
    fn sheet_row_source(text: &str) -> String {
        let cell = TableCell {
            content: vec![Block::Paragraph(Paragraph {
                style: ParagraphStyle::default(),
                runs: vec![Run {
                    text: text.to_string(),
                    style: TextStyle {
                        font_family: Some("Malgun Gothic".to_string()),
                        east_asian_font_family: Some("Malgun Gothic".to_string()),
                        font_size: Some(14.0),
                        ..TextStyle::default()
                    },
                    href: None,
                    footnote: None,
                }],
            })],
            vertical_align: Some(CellVerticalAlign::Top),
            ..TableCell::default()
        };
        let table = Table {
            rows: vec![TableRow {
                minimum_height: None,
                cells: vec![cell],
                // Tall enough that the track keeps per-cell alignment, so the
                // box's ascent shows in the seat instead of being centred away.
                height: Some(36.0),
            }],
            column_widths: vec![200.0],
            default_vertical_align: Some(CellVerticalAlign::Top),
            seats_bottom_aligned_text_on_descender: true,
            border_paint_model: TableBorderPaintModel::CenteredStroke,
            prints_gridlines: false,
            prints_headings: false,
            ..Table::default()
        };
        let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
        generate_typst(&doc).unwrap().source
    }

    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let korean: String = sheet_row_source("가나다라마 01");
    let latin: String = sheet_row_source("Latin only row 01");

    let bare_ascent: String = format!("{}em", format_f64(ascender));
    assert_eq!(
        distinct_top_edges(&korean),
        distinct_top_edges(&latin),
        "the two rows differ only in script, so their line boxes must agree"
    );
    assert_eq!(
        distinct_top_edges(&korean),
        std::collections::BTreeSet::from([bare_ascent.as_str()]),
        "Excel seats both on the bare hhea ascent, not {}em: {korean}",
        format_f64(ascender + 0.15 * word_pitch_em)
    );
}

/// End-to-end pin for the probe workbook behind issue #1060: 26 rows of
/// Malgun Gothic in paired Korean and Latin-only blocks, auto and `ht=36`,
/// bottom, top and centre aligned. Excel prints every pair 0.00pt apart, so
/// no row of the sheet may take Word's East Asian box.
///
/// The pin is the box's *height*, not its ascent: a row's seat is keyed to its
/// track since issue #1063, so the probe's `ht=36` rows and its auto rows
/// legitimately split the same line differently. Script invariance itself is
/// pinned by [`spreadsheet_rows_share_one_line_box_whatever_script`], which
/// compares a Korean row against its Latin twin directly.
#[test]
fn sheet_row_line_box_probe_takes_the_bare_hhea_line_for_every_row() {
    let Some((_ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    let data =
        include_bytes!("../../../../tests/fixtures/xlsx/issue_1060_sheet_row_line_box_probe.xlsx");
    let (doc, _warnings) = crate::parser::Parser::parse(
        &crate::parser::xlsx::XlsxParser,
        data,
        &crate::config::ConvertOptions::default(),
    )
    .expect("the probe workbook parses");
    let result = generate_typst(&doc).unwrap().source;

    let advances: Vec<(f64, f64, f64, f64)> = cell_line_advances(&result);
    assert!(
        !advances.is_empty(),
        "the probe sheet emits line boxes: {result}"
    );
    for (top_em, bottom_em, leading_pt, size_pt) in advances {
        let advance_pt: f64 = (top_em + bottom_em) * size_pt + leading_pt;
        assert!(
            (advance_pt - word_pitch_em * size_pt).abs() < 1e-9,
            "every probe row advances by the bare hhea line, not Word's East \
             Asian box: {advance_pt}pt against {}pt at {size_pt}pt",
            word_pitch_em * size_pt
        );
    }
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
///
/// A sheet's own line carries no such surplus since issue #1060 — the box is
/// the face's bare hhea line for any script, which already ends at the
/// descender — so the seat is now an identity and what this pins is that the
/// East Asian box does not come back below a bottom-aligned sheet cell.
#[test]
fn bottom_aligned_spreadsheet_cell_seats_its_line_box_on_the_descender() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender;
    // What the same cell would emit under Word's East Asian box.
    let east_asian_top_em: f64 = ascender + 0.15 * word_pitch_em;
    let symmetric_bottom_em: f64 = 1.3 * word_pitch_em - east_asian_top_em;
    // Excel rests the descent on the row's own bottom boundary, which is one
    // cell inset below where Typst puts the box's bottom edge (issue #1063);
    // the descent it rests there is a whole number of points.
    let default_padding_bottom_pt: f64 = 5.0;
    let seated_bottom_em: f64 =
        ((descender * font_size).round() - default_padding_bottom_pt) / font_size;
    // The sub-baseline surplus the descender seat removes from the box.
    let leading_pt: f64 = ((word_pitch_em - top_em - seated_bottom_em) * font_size).max(0.0);
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
            minimum_height: None,
            cells: vec![cell],
            // Tall enough to hold visibly more than the 10pt line: a track
            // the line fills alone is the tight regime of issue #839, where
            // every cell centres the row's one box instead.
            height: Some(30.0),
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains(&format!(
            "top-edge: {}em, bottom-edge: {}em",
            format_f64(top_em),
            format_f64(-seated_bottom_em)
        )),
        "bottom-aligned spreadsheet cell must rest its descent on the row's \
         own bottom boundary: {result}"
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
/// symmetric box — the descender seat is a bottom-alignment treatment and must
/// not reach it (issue #618). Its box is the bare hhea line like every other
/// sheet cell's (issue #1060); centring never showed the East Asian surplus
/// anyway, because that surplus was even around the baseline.
#[test]
fn center_aligned_spreadsheet_cell_keeps_the_symmetric_line_box() {
    let Some((ascender, descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return;
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender;
    let symmetric_bottom_em: f64 = word_pitch_em - top_em;
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
            minimum_height: None,
            cells: vec![cell],
            height: Some(20.0),
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let boxes: Vec<(f64, f64)> = cell_line_boxes_em(&result);
    assert_eq!(boxes.len(), 1, "one cell, one line box: {result}");
    assert!(
        (boxes[0].0 + boxes[0].1 - word_pitch_em).abs() < 1e-9,
        "a centred spreadsheet cell keeps the whole line — the descender seat \
         would have trimmed it: {boxes:?}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "a centred spreadsheet cell keeps zero leading: {result}"
    );
    // The bare line ends at the descender by construction, so the seat is
    // indistinguishable here; what a re-seat would still show is the East
    // Asian box it was written to trim.
    assert_eq!(
        descender, symmetric_bottom_em,
        "the bare hhea box already ends at the descender"
    );
    assert!(
        !result.contains(&format!(
            "top-edge: {}em",
            format_f64(ascender + 0.15 * word_pitch_em)
        )),
        "a centred spreadsheet cell takes no East Asian ascent: {result}"
    );
}

/// Regression: a bottom-aligned East Asian spreadsheet cell in an AUTO-height
/// row keeps the symmetric line box and zero leading. In auto rows the
/// renderer sizes the row from the content, so the box *is* the row height;
/// only fixed rows have slack for alignment to distribute, and only they were
/// measured in #618.
///
/// That height is the face's bare hhea line since issue #1060: the same
/// Malgun Gothic face Excel prints a 14pt auto row at 20.00pt in, which
/// Word's 24.20pt East Asian box does not fit.
#[test]
fn bottom_aligned_spreadsheet_cell_in_auto_height_row_keeps_the_symmetric_line_box() {
    let Some((ascender, _descender, word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    let top_em: f64 = ascender;
    let symmetric_bottom_em: f64 = word_pitch_em - top_em;
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
            minimum_height: None,
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![200.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
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
        "an auto-height row keeps the symmetric box: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "an auto-height row keeps zero leading: {result}"
    );
    assert!(
        !result.contains(&format!(
            "top-edge: {}em",
            format_f64(ascender + 0.15 * word_pitch_em)
        )),
        "an auto-height row must not grow by the East Asian ascent: {result}"
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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
                minimum_height: None,
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
            minimum_height: None,
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
            minimum_height: None,
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

/// Excel prints every cell of a single-line sheet row on one baseline: the
/// native export of `09_expense_report_en` puts a `vertical="bottom"` amount
/// column and its `vertical="center"` neighbours all at y=143.00 in a 14pt
/// track, because the track has no room for the alignments to differ.
/// Honouring the declared alignments split the row 0.50pt (issue #839): a
/// tight row must anchor every cell on its one centred line.
#[test]
fn mixed_alignment_tight_sheet_row_seats_every_cell_on_one_baseline() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let font_size: f64 = 10.0;
    let make_cell = |text: &str, vertical_align: Option<CellVerticalAlign>| TableCell {
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
        vertical_align,
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            minimum_height: None,
            // A number-format column beside a centred text column, as in the
            // expense report's data rows.
            cells: vec![
                make_cell("1,240.00 €", None),
                make_cell("Airfare", Some(CellVerticalAlign::Center)),
            ],
            height: Some(14.0),
        }],
        column_widths: vec![84.0, 72.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert_eq!(
        result.matches("align: horizon").count(),
        2,
        "both cells — the bottom-defaulted number included — must anchor on \
         the row's one centred line: {result}"
    );
    // The table-level default emission is `align: bottom,` — the check that
    // no *cell* anchors bottom keys on the cell parameter's closing paren.
    assert!(
        !result.contains("align: bottom)"),
        "no cell of a tight row may keep a bottom anchor of its own: {result}"
    );
}

/// The tight row's one line resolves one metric family for every cell: reading
/// each cell's own face gave a Korean cell and its Latin neighbour boxes of
/// different heights, so their anchors still split by the box difference —
/// `04_payroll_ko`'s `E-1021` column sat 0.25pt off its Korean neighbours
/// (issue #839).
///
/// Which family that is keys on the row's characters, and the cell order must
/// not decide it: the Hangul picks Malgun Gothic whichever column carries it.
/// The row's *box* is Malgun's bare hhea line either way (issue #1060).
#[test]
fn tight_sheet_row_resolves_one_metric_family_for_every_cell() {
    let Some((malgun_ascender, _malgun_descender, malgun_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Malgun Gothic")
    else {
        return; // Malgun Gothic not installed
    };
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let font_size: f64 = 10.0;
    let make_cell = |text: &str, family: &str| TableCell {
        content: vec![Block::Paragraph(Paragraph {
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
        })],
        ..TableCell::default()
    };
    let make_row = |cells: Vec<TableCell>| TableRow {
        minimum_height: None,
        cells,
        height: Some(14.0),
    };
    let korean = || make_cell("김민준", "Malgun Gothic");
    let latin = || make_cell("E-1021", "Libertinus Serif");
    let table = Table {
        rows: vec![
            // A Korean name cell beside a Latin employee-number cell, as in
            // the payroll's data rows, then the same row reversed.
            make_row(vec![korean(), latin()]),
            make_row(vec![latin(), korean()]),
        ],
        column_widths: vec![72.0, 72.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let boxes: Vec<(f64, f64)> = cell_line_boxes_em(&result);
    assert_eq!(boxes.len(), 4, "four cells, four line boxes: {result}");
    for line_box in &boxes {
        assert!(
            (line_box.0 - boxes[0].0).abs() < 1e-9 && (line_box.1 - boxes[0].1).abs() < 1e-9,
            "every cell of both rows must take the same box: {boxes:?}"
        );
        assert!(
            (line_box.0 + line_box.1 - malgun_pitch_em).abs() < 1e-9,
            "and that box is the row face's bare hhea line: {boxes:?}"
        );
    }
    assert!(
        malgun_ascender > 0.0,
        "the row face's metrics must resolve for the comparison to mean anything"
    );
    assert_eq!(
        result.matches("align: horizon").count(),
        4,
        "every cell must anchor on its row's one centred line: {result}"
    );
}

/// Triangulation: the collapse is the tight row's, not every fixed row's. A
/// cell spanning several tracks has real room, and Excel honours its declared
/// alignment there — the merge must keep it while its single-track
/// neighbours join the row line (issue #839).
#[test]
fn row_spanning_cell_keeps_its_declared_alignment_in_a_tight_row() {
    if crate::render::pdf::font_line_metrics_em("Libertinus Serif").is_none() {
        return; // no font book available (e.g. exotic CI sandbox)
    }
    let font_size: f64 = 10.0;
    let make_cell =
        |text: &str, row_span: u32, vertical_align: Option<CellVerticalAlign>| TableCell {
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
            row_span,
            vertical_align,
            ..TableCell::default()
        };
    let table = Table {
        rows: vec![
            TableRow {
                minimum_height: None,
                cells: vec![
                    make_cell("Merged", 2, Some(CellVerticalAlign::Bottom)),
                    make_cell("A", 1, None),
                ],
                height: Some(14.0),
            },
            TableRow {
                minimum_height: None,
                cells: vec![make_cell("B", 1, None)],
                height: Some(14.0),
            },
        ],
        column_widths: vec![72.0, 72.0],
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("align: bottom)"),
        "a cell spanning two tracks keeps its declared bottom seat: {result}"
    );
    assert_eq!(
        result.matches("align: horizon").count(),
        2,
        "the merge's single-track neighbours join the row's centred line: {result}"
    );
}

/// Excel lays a printed sheet out in whole device points: a row's line box is
/// its face's `hhea` ascent and descent each rounded to a point, centred in
/// the row's track with the odd leftover point given to the space *above* the
/// line (issue #1063).
///
/// The table is what four native Excel-for-Mac probe exports measured
/// (`/Volumes/T7/scratch/issue-1063/probe`): a track-height sweep at Arial 10,
/// a font-size sweep in 40pt and 60pt tracks, and the two rows of
/// `09_expense_report_en` the issue reports. Arial's `hhea` numbers are
/// written out rather than read from a face, so the assertion holds on a
/// runner with no Arial installed.
#[test]
fn sheet_cell_line_seat_reproduces_the_native_excel_probe() {
    const ARIAL_ASCENT_EM: f64 = (1854.0 + 67.0) / 2048.0;
    const ARIAL_DESCENT_EM: f64 = 434.0 / 2048.0;

    // (track height pt, font size pt, baseline below the track's top edge pt)
    let measured: [(f64, f64, f64); 28] = [
        // Arial 10, one row height per point of the sweep.
        (12.0, 10.0, 10.0),
        (13.0, 10.0, 11.0),
        (14.0, 10.0, 11.0),
        (15.0, 10.0, 12.0),
        (16.0, 10.0, 12.0),
        (17.0, 10.0, 13.0),
        (18.0, 10.0, 13.0),
        (20.0, 10.0, 14.0),
        (22.0, 10.0, 15.0),
        (23.0, 10.0, 16.0),
        (25.0, 10.0, 17.0),
        (30.0, 10.0, 19.0),
        (40.0, 10.0, 24.0),
        // Font-size sweep in a 40pt track.
        (40.0, 8.0, 23.0),
        (40.0, 12.0, 24.0),
        (40.0, 16.0, 26.0),
        (40.0, 20.0, 28.0),
        (40.0, 24.0, 29.0),
        (40.0, 28.0, 30.0),
        (40.0, 32.0, 32.0),
        // Font-size sweep in a 60pt track.
        (60.0, 8.0, 33.0),
        (60.0, 10.0, 34.0),
        (60.0, 14.0, 35.0),
        (60.0, 18.0, 37.0),
        (60.0, 24.0, 39.0),
        (60.0, 30.0, 41.0),
        (60.0, 36.0, 43.0),
        (60.0, 44.0, 46.0),
    ];

    for (track_pt, font_size_pt, expected_pt) in measured {
        let seated_pt: f64 = sheet_cell_baseline_from_track_top_pt(
            track_pt,
            ARIAL_ASCENT_EM,
            ARIAL_DESCENT_EM,
            font_size_pt,
        );
        assert!(
            (seated_pt - expected_pt).abs() < 1e-9,
            "Arial {font_size_pt}pt in a {track_pt}pt track: Excel prints the \
             baseline {expected_pt}pt below the track top, seated {seated_pt}pt"
        );
    }
}

/// The expense report's data rows: a 14pt track of Arial 10 whose cells Excel
/// prints at y=143.00, 11.00pt below the track's top boundary. Our own seat
/// centred the line in the cell's *inset* box instead of the track and used
/// unrounded metrics, landing 0.62pt high (issue #1063).
#[test]
fn fixed_track_sheet_cell_seats_its_centred_line_on_the_track() {
    const FAMILY: &str = "Libertinus Serif";
    let Some((ascent_em, descent_em, pitch_em)) = crate::render::pdf::font_line_metrics_em(FAMILY)
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size_pt: f64 = 10.0;
    let track_pt: f64 = 14.0;
    let padding = Insets {
        top: 1.0,
        right: 3.0,
        bottom: 1.5,
        left: 3.0,
    };
    let table = Table {
        rows: vec![TableRow {
            minimum_height: None,
            cells: vec![TableCell {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Airfare".to_string(),
                        style: TextStyle {
                            font_family: Some(FAMILY.to_string()),
                            font_size: Some(font_size_pt),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                vertical_align: Some(CellVerticalAlign::Center),
                ..TableCell::default()
            }],
            height: Some(track_pt),
        }],
        column_widths: vec![72.0],
        default_cell_padding: Some(padding),
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    // Typst centres the fixed line box in the cell's inset content area, so
    // the emitted ascent is what decides where the baseline lands.
    let content_mid_pt: f64 = (padding.top + (track_pt - padding.bottom)) / 2.0;
    let baseline_pt: f64 =
        sheet_cell_baseline_from_track_top_pt(track_pt, ascent_em, descent_em, font_size_pt);
    let expected_top_em: f64 = pitch_em / 2.0 + (baseline_pt - content_mid_pt) / font_size_pt;
    let needle: String = format!("top-edge: {}em", format_f64(expected_top_em));
    assert!(
        result.contains(&needle),
        "the centred line must seat its baseline {baseline_pt}pt below the \
         track top, which needs `{needle}`: {result}"
    );
}

/// The expense report's title: Arial Bold 14 bottom-aligned in a 23pt track,
/// printed with its descender resting on the row's own bottom boundary — not
/// on the boundary less the cell's bottom inset, which seated it 1.47pt high
/// (issue #1063). The descent Excel rests there is a whole number of points.
#[test]
fn bottom_aligned_sheet_cell_rests_its_descender_on_the_row_boundary() {
    const FAMILY: &str = "Libertinus Serif";
    let Some((_ascent_em, descent_em, _pitch_em)) =
        crate::render::pdf::font_line_metrics_em(FAMILY)
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size_pt: f64 = 14.0;
    let padding = Insets {
        top: 1.0,
        right: 3.0,
        bottom: 1.5,
        left: 3.0,
    };
    let table = Table {
        rows: vec![TableRow {
            minimum_height: None,
            cells: vec![TableCell {
                content: vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle::default(),
                    runs: vec![Run {
                        text: "Business Trip Expense Report".to_string(),
                        style: TextStyle {
                            font_family: Some(FAMILY.to_string()),
                            font_size: Some(font_size_pt),
                            bold: Some(true),
                            ..TextStyle::default()
                        },
                        href: None,
                        footnote: None,
                    }],
                })],
                vertical_align: Some(CellVerticalAlign::Bottom),
                ..TableCell::default()
            }],
            height: Some(23.0),
        }],
        column_widths: vec![72.0],
        default_cell_padding: Some(padding),
        default_vertical_align: Some(CellVerticalAlign::Bottom),
        seats_bottom_aligned_text_on_descender: true,
        border_paint_model: TableBorderPaintModel::CenteredStroke,
        prints_gridlines: false,
        prints_headings: false,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    // Typst rests the box's bottom edge on the inset content bottom, so the
    // emitted descent has to be short of Excel's by that inset.
    let descent_pt: f64 = (descent_em * font_size_pt).round();
    let expected_bottom_em: f64 = (descent_pt - padding.bottom) / font_size_pt;
    let needle: String = format!("bottom-edge: {}em", format_f64(-expected_bottom_em));
    assert!(
        result.contains(&needle),
        "the descender must rest on the row boundary, {descent_pt}pt below the \
         baseline, which needs `{needle}`: {result}"
    );
}

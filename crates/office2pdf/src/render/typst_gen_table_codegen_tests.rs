use super::*;
use crate::ir::{BorderSide, CellBorder, Insets, Table, TableCell, TableRow};

/// Helper to create a table cell with plain text.
pub(super) fn make_text_cell(text: &str) -> TableCell {
    TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: text.to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    }
}

#[test]
fn test_table_simple_2x2() {
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![make_text_cell("A1"), make_text_cell("B1")],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("A2"), make_text_cell("B2")],
                height: None,
            },
        ],
        column_widths: vec![100.0, 200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("#table("), "Expected #table( in: {result}");
    assert!(
        result.contains("columns: (100pt, 200pt)"),
        "Expected column widths in: {result}"
    );
    assert!(result.contains("A1"), "Expected A1 in: {result}");
    assert!(result.contains("B1"), "Expected B1 in: {result}");
    assert!(result.contains("A2"), "Expected A2 in: {result}");
    assert!(result.contains("B2"), "Expected B2 in: {result}");
}

#[test]
fn test_table_with_default_cell_padding() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("Padded")],
            height: None,
        }],
        column_widths: vec![100.0],
        header_row_count: 0,
        non_repeating_header_row_count: 0,
        alignment: None,
        default_cell_padding: Some(Insets {
            top: 2.0,
            right: 3.0,
            bottom: 1.0,
            left: 4.0,
        }),
        use_content_driven_row_heights: false,
        default_vertical_align: None,
        seats_bottom_aligned_text_on_descender: false,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("inset: (top: 2pt, right: 3pt, bottom: 1pt, left: 4pt)"),
        "Expected table inset in: {result}"
    );
}

#[test]
fn test_table_cell_with_padding_override() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Inset".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        padding: Some(Insets {
            top: 5.0,
            right: 2.0,
            bottom: 3.0,
            left: 6.0,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: None,
        }],
        column_widths: vec![100.0],
        header_row_count: 0,
        non_repeating_header_row_count: 0,
        alignment: None,
        default_cell_padding: Some(Insets {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        }),
        use_content_driven_row_heights: false,
        default_vertical_align: None,
        seats_bottom_aligned_text_on_descender: false,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("table.cell(inset: (top: 5pt, right: 2pt, bottom: 3pt, left: 6pt))"),
        "Expected cell inset override in: {result}"
    );
}

#[test]
fn test_table_alignment_center_wraps_table() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("Centered table")],
            height: None,
        }],
        column_widths: vec![100.0],
        header_row_count: 0,
        non_repeating_header_row_count: 0,
        alignment: Some(Alignment::Center),
        default_cell_padding: None,
        use_content_driven_row_heights: false,
        default_vertical_align: None,
        seats_bottom_aligned_text_on_descender: false,
        paints_borders_inside_boundary: false,
        prints_gridlines: false,
        prints_headings: false,
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#align(center)["),
        "Expected center wrapper in: {result}"
    );
    assert!(
        result.contains("#table("),
        "Expected table inside wrapper in: {result}"
    );
}

#[test]
fn test_table_with_repeating_header_rows_uses_table_header() {
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![make_text_cell("Header 1"), make_text_cell("Header 2")],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("Body 1"), make_text_cell("Body 2")],
                height: None,
            },
        ],
        column_widths: vec![100.0, 100.0],
        header_row_count: 1,
        non_repeating_header_row_count: 0,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("table.header("),
        "Expected table.header wrapper in: {result}"
    );
    assert!(
        result.contains("Header 1") && result.contains("Body 1"),
        "Expected header and body cell content in: {result}"
    );
}

#[test]
fn test_table_with_colspan() {
    let merged_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Merged".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        col_span: 2,
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![merged_cell],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("A2"), make_text_cell("B2")],
                height: None,
            },
        ],
        column_widths: vec![100.0, 200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("colspan: 2"),
        "Expected colspan: 2 in: {result}"
    );
    assert!(result.contains("Merged"), "Expected Merged in: {result}");
}

#[test]
fn test_table_with_rowspan() {
    let tall_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Tall".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        row_span: 2,
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![tall_cell, make_text_cell("B1")],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("B2")],
                height: None,
            },
        ],
        column_widths: vec![100.0, 200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("rowspan: 2"),
        "Expected rowspan: 2 in: {result}"
    );
    assert!(result.contains("Tall"), "Expected Tall in: {result}");
}

#[test]
fn test_table_with_explicit_row_sizes_and_cell_vertical_align() {
    let centered_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Centered".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        vertical_align: Some(CellVerticalAlign::Center),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![centered_cell, make_text_cell("B1")],
                height: Some(36.0),
            },
            TableRow {
                cells: vec![make_text_cell("A2"), make_text_cell("B2")],
                height: None,
            },
        ],
        column_widths: vec![100.0, 100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("rows: (36pt, auto)"),
        "Expected explicit Typst row sizes in: {result}"
    );
    assert!(
        result.contains("align: horizon"),
        "Expected centered vertical alignment in: {result}"
    );
}

#[test]
fn test_table_with_content_driven_row_heights_omits_explicit_rows() {
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![make_text_cell("A1"), make_text_cell("B1")],
                height: Some(36.0),
            },
            TableRow {
                cells: vec![make_text_cell("A2"), make_text_cell("B2")],
                height: Some(48.0),
            },
        ],
        column_widths: vec![100.0, 100.0],
        use_content_driven_row_heights: true,
        default_vertical_align: None,
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        !result.contains("rows: ("),
        "Content-driven row-height tables should not emit exact Typst row sizes: {result}"
    );
}

#[test]
fn test_table_with_colspan_and_rowspan() {
    let big_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Big".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        col_span: 2,
        row_span: 2,
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![big_cell, make_text_cell("C1")],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("C2")],
                height: None,
            },
            TableRow {
                cells: vec![
                    make_text_cell("A3"),
                    make_text_cell("B3"),
                    make_text_cell("C3"),
                ],
                height: None,
            },
        ],
        column_widths: vec![100.0, 100.0, 100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("colspan: 2"),
        "Expected colspan: 2 in: {result}"
    );
    assert!(
        result.contains("rowspan: 2"),
        "Expected rowspan: 2 in: {result}"
    );
    assert!(result.contains("Big"), "Expected Big in: {result}");
}

#[test]
fn test_table_with_background_color() {
    let colored_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Colored".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        background: Some(Color::new(200, 200, 200)),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![colored_cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("fill: rgb(200, 200, 200)"),
        "Expected fill color in: {result}"
    );
    assert!(result.contains("Colored"), "Expected Colored in: {result}");
}

#[test]
fn test_table_with_cell_borders() {
    let bordered_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Bordered".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            bottom: Some(BorderSide {
                width: 2.0,
                color: Color::new(255, 0, 0),
                style: BorderLineStyle::Solid,
            }),
            left: None,
            right: None,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![bordered_cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("stroke:"), "Expected stroke in: {result}");
    assert!(
        result.contains("Bordered"),
        "Expected Bordered in: {result}"
    );
}

#[test]
fn test_table_with_partial_cell_borders_does_not_fill_missing_grid_lines() {
    let header_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Header".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        border: Some(CellBorder {
            top: None,
            bottom: Some(BorderSide {
                width: 1.0,
                color: Color::black(),
                style: BorderLineStyle::Solid,
            }),
            left: None,
            right: None,
        }),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![
            TableRow {
                cells: vec![header_cell],
                height: None,
            },
            TableRow {
                cells: vec![make_text_cell("Body")],
                height: None,
            },
        ],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("stroke: none"),
        "Expected table default stroke to be disabled so unbordered cells stay unbordered: {result}"
    );
    assert!(
        result.contains("stroke: (bottom: 1pt + rgb(0, 0, 0))"),
        "Expected explicit bottom border to remain on the header cell: {result}"
    );
}

#[test]
fn test_table_with_styled_text_in_cell() {
    let styled_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Bold cell".to_string(),
                style: TextStyle {
                    bold: Some(true),
                    font_size: Some(14.0),
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
            cells: vec![styled_cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("weight: \"bold\""),
        "Expected bold in table cell: {result}"
    );
    assert!(
        result.contains("size: 14pt"),
        "Expected font size in table cell: {result}"
    );
}

#[test]
fn test_table_cell_paragraph_preserves_right_alignment() {
    let right_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                alignment: Some(Alignment::Right),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "N".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("greek"), right_cell],
            height: None,
        }],
        column_widths: vec![100.0, 100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#block(width: 100%)") && result.contains("#set align(right)"),
        "Expected table cell paragraph to preserve right alignment: {result}"
    );
}

#[test]
fn test_table_cell_paragraph_preserves_spacing() {
    let spaced_cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                space_before: Some(2.0),
                space_after: Some(3.0),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "Header".to_string(),
                style: TextStyle::default(),
                href: None,
                footnote: None,
            }],
        })],
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![spaced_cell],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#v(2pt)") && result.contains("#v(3pt)"),
        "Expected table cell paragraph spacing to be preserved: {result}"
    );
}

#[test]
fn test_table_cell_word_line_box() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle {
                line_box: Some(LineBox {
                    ascent_em: 1.3125,
                    descent_em: 0.4375,
                }),
                space_after: Some(8.0),
                ..ParagraphStyle::default()
            },
            runs: vec![Run {
                text: "Word line box".to_string(),
                style: TextStyle::default(),
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
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#set text(top-edge: 1.3125em, bottom-edge: -0.4375em)"),
        "Expected Word-compatible text edges in: {result}"
    );
    assert!(
        result.contains("#set par(leading: 0pt)"),
        "Expected Word-compatible line stacking in: {result}"
    );
    assert!(
        result.contains("#v(8pt)"),
        "Expected space-after in: {result}"
    );
}

#[test]
fn test_table_empty_cells() {
    let empty_cell = TableCell::default();
    let table = Table {
        rows: vec![TableRow {
            cells: vec![empty_cell, make_text_cell("Has text")],
            height: None,
        }],
        column_widths: vec![100.0, 100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("#table("), "Expected #table( in: {result}");
    assert!(
        result.contains("Has text"),
        "Expected Has text in: {result}"
    );
}

#[test]
fn test_table_no_column_widths() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("A"), make_text_cell("B")],
            height: None,
        }],
        column_widths: vec![],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(result.contains("#table("), "Expected #table( in: {result}");
    assert!(result.contains("A"), "Expected A in: {result}");
    assert!(result.contains("B"), "Expected B in: {result}");
}

#[path = "typst_gen_table_border_tests.rs"]
mod border_tests;

#[path = "typst_gen_table_cell_content_tests.rs"]
mod cell_content_tests;

#[test]
fn test_table_special_chars_in_cells() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("Price: $100 #items")],
            height: None,
        }],
        column_widths: vec![200.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("\\$") && result.contains("\\#"),
        "Expected escaped special chars in: {result}"
    );
}

#[test]
fn test_table_in_flow_page_with_paragraphs() {
    let table = Table {
        rows: vec![TableRow {
            cells: vec![make_text_cell("Cell")],
            height: None,
        }],
        column_widths: vec![100.0],
        ..Table::default()
    };
    let doc = make_doc(vec![make_flow_page(vec![
        make_paragraph("Before table"),
        Block::Table(table),
        make_paragraph("After table"),
    ])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("Before table"),
        "Expected Before table in: {result}"
    );
    assert!(result.contains("#table("), "Expected #table( in: {result}");
    assert!(
        result.contains("After table"),
        "Expected After table in: {result}"
    );
}

#[test]
fn test_generate_space_before_after() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Paragraph(Paragraph {
        style: ParagraphStyle {
            space_before: Some(12.0),
            space_after: Some(6.0),
            ..ParagraphStyle::default()
        },
        runs: vec![Run {
            text: "Spaced paragraph".to_string(),
            style: TextStyle::default(),
            href: None,
            footnote: None,
        }],
    })])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        result.contains("12pt") || result.contains("above"),
        "Expected space_before in: {result}"
    );
}

/// Excel rests a bottom-aligned cell's last-line descender on the bottom
/// inset edge. The spill wrapper used to hardcode a `horizon` anchor and an
/// ambient-sized `1.3em` box, so an unwrapped title in a tall row floated at
/// the row's vertical centre instead (issue #618). The clip box and the
/// in-flow strut must both be sized from the paragraph's own line box at the
/// run's font size, not the ambient text size.
#[test]
fn bottom_aligned_spill_cell_anchors_its_line_box_at_the_bottom() {
    let Some((ascender, descender, _word_pitch_em)) =
        crate::render::pdf::font_line_metrics_em("Libertinus Serif")
    else {
        return; // no font book available (e.g. exotic CI sandbox)
    };
    let font_size: f64 = 10.0;
    // With the descender seated on the inset edge, the single line spans
    // ascent-to-descender at the run's own size.
    let line_box_height_pt: f64 = (ascender + descender) * font_size;
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Warehouse Inventory".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(font_size),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        spill_width: Some(200.0),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: Some(23.0),
        }],
        column_widths: vec![60.0],
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
            "#place(left + bottom, box(width: 200pt, height: {}pt, clip: true)[",
            format_f64(line_box_height_pt)
        )),
        "bottom cell's spill box must anchor at the bottom, sized to its own line: {result}"
    );
    assert!(
        result.contains(&format!(
            "])#box(width: 0pt, height: {}pt)",
            format_f64(line_box_height_pt)
        )),
        "the in-flow strut must hold the same line height in points: {result}"
    );
    assert!(
        !result.contains("horizon"),
        "a bottom-aligned spill cell must not be vertically centred: {result}"
    );
}

/// Triangulation for the spill anchor: an explicitly centred cell measures
/// correct today, so its emission must stay exactly as it was — a `horizon`
/// anchor with the ambient-sized box and strut.
#[test]
fn center_aligned_spill_cell_keeps_the_centered_wrapper() {
    let cell = TableCell {
        content: vec![Block::Paragraph(Paragraph {
            style: ParagraphStyle::default(),
            runs: vec![Run {
                text: "Centered spill".to_string(),
                style: TextStyle {
                    font_family: Some("Libertinus Serif".to_string()),
                    font_size: Some(10.0),
                    ..TextStyle::default()
                },
                href: None,
                footnote: None,
            }],
        })],
        spill_width: Some(200.0),
        vertical_align: Some(CellVerticalAlign::Center),
        ..TableCell::default()
    };
    let table = Table {
        rows: vec![TableRow {
            cells: vec![cell],
            height: Some(23.0),
        }],
        column_widths: vec![60.0],
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
        result.contains("#place(left + horizon, box(width: 200pt, height: 1.3em, clip: true)["),
        "an explicitly centred spill cell keeps its current wrapper: {result}"
    );
    assert!(
        result.contains("])#box(width: 0pt, height: 1.3em)"),
        "an explicitly centred spill cell keeps its current strut: {result}"
    );
}

// ---------------------------------------------------------------------------
// Printed headings (issue #623)
//
// `<printOptions headings="1"/>` prints the row-number gutter and the
// column-letter strip on every page. The XLSX parser materializes them in the
// IR (gutter as the first column, letter strip as `rows[0]`) and sets
// `prints_headings`; codegen must re-emit that first row as a repeating
// `table.header` so the letters appear on every paginated page, above any
// print-title headers.
// ---------------------------------------------------------------------------

/// The augmented shape the XLSX parser hands codegen: strip row first, gutter
/// cells at each row's front.
fn headings_table(extra_rows: Vec<TableRow>) -> Table {
    let mut rows: Vec<TableRow> = vec![TableRow {
        cells: vec![
            TableCell::default(),
            make_text_cell("A"),
            make_text_cell("B"),
        ],
        height: Some(13.0),
    }];
    rows.extend(extra_rows);
    Table {
        rows,
        column_widths: vec![23.0, 120.0, 110.0],
        prints_headings: true,
        ..Table::default()
    }
}

fn gutter_row(number: &str, first: &str, second: &str) -> TableRow {
    TableRow {
        cells: vec![
            make_text_cell(number),
            make_text_cell(first),
            make_text_cell(second),
        ],
        height: Some(13.0),
    }
}

#[test]
fn test_prints_headings_emits_the_letter_strip_as_a_repeating_header() {
    let table = headings_table(vec![gutter_row("1", "x", "y"), gutter_row("2", "z", "w")]);
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let header_start: usize = result
        .find("table.header(repeat: true,")
        .expect("the letter strip must repeat on every page");
    let header_end: usize = result[header_start..]
        .find("),")
        .map(|offset| header_start + offset)
        .expect("header block must close");
    let header_block: &str = &result[header_start..header_end];
    assert!(
        header_block.contains("[A]") && header_block.contains("[B]"),
        "the strip header must carry the column letters: {result}"
    );
    assert!(
        !header_block.contains("[1]"),
        "gutter numbers flow with the body rows, not the strip: {result}"
    );
    let body: &str = &result[header_end..];
    assert!(
        body.contains("[1]") && body.contains("[2]"),
        "gutter numbers must follow in the body: {result}"
    );
    assert!(
        result.contains("columns: (23pt, 120pt, 110pt)"),
        "the gutter track must lead the column list: {result}"
    );
    assert!(
        result.contains("rows: (13pt, 13pt, 13pt)"),
        "the strip row's track must lead the row list: {result}"
    );
}

#[test]
fn test_prints_headings_keeps_print_title_headers_below_the_strip() {
    let mut table = headings_table(vec![
        gutter_row("1", "Title", "Title2"),
        gutter_row("2", "x", "y"),
    ]);
    // One print-title row (counted AFTER the strip row).
    table.header_row_count = 1;
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    let strip_pos: usize = result
        .find("table.header(repeat: true,")
        .expect("strip header must exist");
    let title_pos: usize = result
        .find("table.header(level: 2,")
        .expect("print-title header must sit at the next level");
    assert!(
        strip_pos < title_pos,
        "the strip repeats above the print-title rows: {result}"
    );
    assert!(
        result[title_pos..].contains("Title"),
        "the title row must live in the level-2 header: {result}"
    );
}

#[test]
fn test_prints_headings_shifts_lead_and_title_header_levels() {
    let mut table = headings_table(vec![
        gutter_row("1", "Lead", "Lead2"),
        gutter_row("2", "Title", "Title2"),
        gutter_row("3", "x", "y"),
    ]);
    // One non-repeating lead row above one print-title row.
    table.non_repeating_header_row_count = 1;
    table.header_row_count = 1;
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("table.header(repeat: true,"),
        "strip header must exist: {result}"
    );
    assert!(
        result.contains("table.header(repeat: false, level: 2,"),
        "the lead block must move to level 2 under the strip: {result}"
    );
    assert!(
        result.contains("table.header(level: 3,"),
        "the print-title block must move to level 3: {result}"
    );
}

#[test]
fn test_prints_headings_off_emits_no_repeating_strip() {
    let mut table = headings_table(vec![gutter_row("1", "x", "y")]);
    table.prints_headings = false;
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;
    assert!(
        !result.contains("table.header("),
        "without the flag no header block may appear: {result}"
    );
}

/// The gray of the 1pt rules between heading cells (issue #623 GT model).
const HEADING_RULE_GRAY: crate::ir::Color = crate::ir::Color {
    r: 86,
    g: 86,
    b: 86,
};

fn solid_side(width: f64, color: crate::ir::Color) -> BorderSide {
    BorderSide {
        width,
        color,
        style: crate::ir::BorderLineStyle::Solid,
    }
}

/// The bordered shape the XLSX parser's heading augmentation hands codegen:
/// gray rules between heading cells, black 1pt separators against the data
/// grid (strip bottom, gutter right), boundary-band border regime.
fn separator_bordered_headings_table(data_cell: TableCell, prints_gridlines: bool) -> Table {
    let gray = || solid_side(1.0, HEADING_RULE_GRAY);
    let black_separator = || solid_side(1.0, crate::ir::Color::black());
    Table {
        rows: vec![
            TableRow {
                cells: vec![
                    TableCell {
                        border: Some(CellBorder {
                            bottom: Some(gray()),
                            right: Some(gray()),
                            ..CellBorder::default()
                        }),
                        ..TableCell::default()
                    },
                    TableCell {
                        border: Some(CellBorder {
                            bottom: Some(black_separator()),
                            right: Some(gray()),
                            ..CellBorder::default()
                        }),
                        ..make_text_cell("A")
                    },
                ],
                height: Some(13.0),
            },
            TableRow {
                cells: vec![
                    TableCell {
                        border: Some(CellBorder {
                            top: Some(gray()),
                            bottom: Some(gray()),
                            right: Some(black_separator()),
                            ..CellBorder::default()
                        }),
                        ..make_text_cell("1")
                    },
                    data_cell,
                ],
                height: Some(13.0),
            },
        ],
        column_widths: vec![23.0, 120.0],
        prints_headings: true,
        prints_gridlines,
        paints_borders_inside_boundary: true,
        ..Table::default()
    }
}

/// The `table.header(repeat: true, ...)` slice of the generated source.
fn strip_header_block(result: &str) -> &str {
    let header_start: usize = result
        .find("table.header(repeat: true,")
        .expect("strip header must exist");
    let header_end: usize = result[header_start..]
        .find("\n  ),")
        .map(|offset| header_start + offset)
        .expect("header block must close");
    &result[header_start..header_end]
}

#[test]
fn test_prints_headings_strip_bottom_separator_repeats_over_a_tying_body_border() {
    // The first body row declares a 1pt solid black top border — the same
    // conflict rank as the strip's black separator. The tie must resolve to
    // the strip side (the #619 repeating-header rule, applied at the strip's
    // own bottom boundary): a band left on the body row's top would vanish
    // on pages 2+, where only the header repeats (issue #623 adversarial
    // review, finding 1).
    let data_cell = TableCell {
        border: Some(CellBorder {
            top: Some(solid_side(1.0, crate::ir::Color::black())),
            ..CellBorder::default()
        }),
        ..make_text_cell("x")
    };
    let table = separator_bordered_headings_table(data_cell, false);
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        strip_header_block(&result).contains("#place(bottom + left"),
        "the strip|data separator band must live inside the repeating header: {result}"
    );
}

#[test]
fn test_prints_headings_strip_adopts_a_heavier_body_border_into_the_repeat() {
    // A strictly heavier body declaration (2pt medium) outranks the strip's
    // 1pt separator; the #619 mechanism adopts it into the header side so
    // the heavier band repeats on every page alongside the body's own copy.
    let data_cell = TableCell {
        border: Some(CellBorder {
            top: Some(solid_side(2.0, crate::ir::Color::black())),
            ..CellBorder::default()
        }),
        ..make_text_cell("x")
    };
    let table = separator_bordered_headings_table(data_cell, false);
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        strip_header_block(&result).contains("stroke: 2pt"),
        "the header must adopt the body's heavier band so repeats carry it: {result}"
    );
}

#[test]
fn test_prints_headings_excludes_the_heading_exterior_from_gridline_seeding() {
    // gridLines=1 AND headings=1: GT rules the heading exterior — the strip
    // row's top and the gutter column's left — as the 1pt black print FRAME,
    // not as gridlines (issue #623 evidence, nft-sheet-0002 trace: black
    // bands [54,538]x[72,73] and [54,55]x[72,710]). Gridline seeding stays
    // excluded there; the frame band replaces it rather than stacking, so
    // every frame boundary carries exactly one band. Data-area seeding
    // (#622) is unchanged where the frame does not own the boundary.
    let table = separator_bordered_headings_table(make_text_cell("x"), true);
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        result.contains("#place(top + left"),
        "the strip-top frame band must paint: {result}"
    );
    // 10 black bands, each boundary painted once: corner top+left, letter
    // top+right (frame over the last column's gray rule) + bottom separator,
    // gutter left + right separator + bottom (frame over the last row's gray
    // rule), data cell right+bottom (frame replacing the #622 closure seeds).
    assert_eq!(
        result.matches("rgb(0, 0, 0)").count(),
        10,
        "the frame and separators paint exactly once per boundary — no \
         doubled bands and no leftover gridline seeds: {result}"
    );
}

#[test]
fn test_prints_headings_paints_the_black_exterior_frame_without_gridlines() {
    // GT (issue #623): with headings on, a 1pt black frame encloses the
    // heading bands and the data grid — the corner box's top and left edges
    // ARE the frame. The frame is gated on prints_headings alone; gridlines
    // are off here, so every exterior band below is the frame's.
    let table = separator_bordered_headings_table(make_text_cell("x"), false);
    let doc = make_doc(vec![make_flow_page(vec![Block::Table(table)])]);
    let result = generate_typst(&doc).unwrap().source;

    assert!(
        strip_header_block(&result).contains("#place(top + left"),
        "the strip-top frame edge must live inside the repeating header so \
         pages 2+ carry it: {result}"
    );
    // Same 10-band census as the gridlines-on test: the frame does not
    // depend on gridline seeding.
    assert_eq!(
        result.matches("rgb(0, 0, 0)").count(),
        10,
        "frame top/left/right/bottom plus the strip and gutter separators \
         paint exactly once per boundary: {result}"
    );
}

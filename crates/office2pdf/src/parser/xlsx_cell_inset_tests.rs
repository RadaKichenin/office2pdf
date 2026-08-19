//! The horizontal box Excel lays a cell's text out in (issue #1157).
//!
//! Measured against the ten native Excel for Mac exports under
//! `tests/golden_mocks/business/expected/xlsx/`, one `fill_text` per cell in a
//! `mutool draw -F trace` of both sides, matched by string and baseline. Each
//! run is classified by how far it moves when the right inset alone changes:
//! a left-aligned run does not move, a centred one moves half a point, a
//! right-aligned one a whole point.
//!
//! | run class | n | comparable edge | error at 3/3 | error at 3/2 |
//! | --- | ---: | --- | ---: | ---: |
//! | left-aligned | 139 | pen origin | median 0.000 | median 0.000 |
//! | centred | 570 | run centre | mean +0.012 | mean +0.512 |
//! | right-aligned | 135 | pen end | mean -0.994 | mean +0.006 |
//!
//! The right-aligned figures are the ones that carry a correction, because
//! Excel rounds every glyph advance to a whole point: its last glyph is laid
//! down `round(adv) - adv` short of where the true advance would end it, and
//! we place the same glyph on the unrounded advance. Subtracting that per-run
//! offset — read off the GT's own trace, not fitted — leaves +0.006pt (sd
//! 0.086) at a 2pt right inset against -0.994pt at 3pt: one whole point out.
//!
//! The centred figures carry no correction and want none. Excel centres the
//! run on the *column*, not in that asymmetric box, so a symmetric split of
//! the same 5pt total is what puts our centred runs on its own — which is why
//! issue #657's warning that an asymmetric pair moves every centred run by
//! half the difference is right, and only its conclusion that both sides are
//! therefore 3pt is not.

use super::*;

/// A one-cell workbook whose cell carries `horizontal`.
fn workbook_with_cell_alignment(
    horizontal: umya_spreadsheet::HorizontalAlignmentValues,
) -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    {
        let sheet = book.get_sheet_mut(&0).unwrap();
        let cell = sheet.get_cell_mut("B3");
        cell.set_value("2026");
        cell.get_style_mut()
            .get_alignment_mut()
            .set_horizontal(horizontal);
    }
    let mut cursor = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut cursor).unwrap();
    cursor.into_inner()
}

/// The insets the workbook's one value-bearing cell is laid out with: its own
/// when it states them, otherwise the table default it inherits.
fn first_cell_padding(data: &[u8]) -> Insets {
    let (doc, _warnings) = XlsxParser
        .parse(data, &ConvertOptions::default())
        .expect("workbook should parse");
    let sheet = get_sheet_page(&doc, 0);
    let default: Insets = sheet
        .table
        .default_cell_padding
        .expect("a sheet states the cell padding its table is laid out with");
    sheet
        .table
        .rows
        .iter()
        .flat_map(|row| row.cells.iter())
        .find(|cell| !cell.content.is_empty())
        .expect("the workbook has a value-bearing cell")
        .padding
        .unwrap_or(default)
}

#[test]
fn test_left_aligned_cell_starts_three_points_inside_its_column() {
    let data = workbook_with_cell_alignment(umya_spreadsheet::HorizontalAlignmentValues::Left);

    let padding: Insets = first_cell_padding(&data);

    assert!(
        (padding.left - 3.0).abs() < 0.01,
        "Excel starts a left-aligned value 3pt inside the column, got {}",
        padding.left
    );
}

#[test]
fn test_right_aligned_cell_ends_two_points_inside_its_column() {
    let data = workbook_with_cell_alignment(umya_spreadsheet::HorizontalAlignmentValues::Right);

    let padding: Insets = first_cell_padding(&data);

    assert!(
        (padding.right - 2.0).abs() < 0.01,
        "Excel ends a right-aligned value 2pt inside the column, got {}",
        padding.right
    );
}

#[test]
fn test_centred_cell_stays_on_its_columns_own_centre() {
    let data = workbook_with_cell_alignment(umya_spreadsheet::HorizontalAlignmentValues::Center);

    let padding: Insets = first_cell_padding(&data);

    assert!(
        (padding.left - padding.right).abs() < 0.01,
        "an asymmetric box moves a centred run off the column's centre by half \
         the difference, got left {} right {}",
        padding.left,
        padding.right
    );
    assert!(
        ((padding.left + padding.right) - 5.0).abs() < 0.01,
        "a centred cell splits Excel's own 5pt total, got {}",
        padding.left + padding.right
    );
}

#[test]
fn test_sheet_table_carries_excels_cell_text_box() {
    let data = build_xlsx_bytes("Sheet1", &[("B3", "2026")]);
    let (doc, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("workbook should parse");

    let source: String = crate::render::typst_gen::generate_typst(&doc)
        .expect("sheet should generate Typst")
        .source;

    assert!(
        source.contains("inset: (top: 1pt, right: 2pt, bottom: 1.5pt, left: 3pt)"),
        "the measured text box should reach the renderer, got:\n{source}"
    );
}

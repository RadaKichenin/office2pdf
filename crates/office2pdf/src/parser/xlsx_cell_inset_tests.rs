//! The horizontal box Excel lays a cell's text out in (issues #1157, #1165).
//!
//! The box's left edge steps with the cell's own font — 37 rows of a
//! one-factor native Excel for Mac probe, tabulated on `cell_left_inset_pt`,
//! which the sweep below asserts on the families whose digit advance the
//! reference table carries. The figures under it fix the Calibri 11 pair the
//! step reduces to at the workbook default.
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

#[test]
fn test_sheet_default_padding_follows_the_workbook_normal_fonts_column_unit() {
    let data = build_xlsx_with_normal_font("Arial", 32.0);
    let (doc, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("workbook should parse");
    let sheet = get_sheet_page(&doc, 0);
    let padding = sheet
        .table
        .default_cell_padding
        .expect("the table carries its Normal-font cell box");

    assert_eq!(padding.left, 6.0);
    assert_eq!(padding.right, 5.0);
    assert!(
        sheet.table.rows[0].cells[0].padding.is_none(),
        "an unstyled cell inherits the Normal-font table box instead of repeating it"
    );
}

/// A one-cell workbook whose cell states `family`, `size_pt`, and `alignment`.
fn workbook_with_cell_font_and_alignment(
    family: &str,
    size_pt: f64,
    alignment: umya_spreadsheet::HorizontalAlignmentValues,
) -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    {
        let sheet = book.get_sheet_mut(&0).unwrap();
        let cell = sheet.get_cell_mut("B3");
        cell.set_value("2026");
        let style = cell.get_style_mut();
        style.get_alignment_mut().set_horizontal(alignment);
        let font = style.get_font_mut();
        font.set_name(family);
        font.set_size(size_pt);
    }
    let mut cursor = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut cursor).unwrap();
    cursor.into_inner()
}

/// A one-cell workbook whose cell states `family` at `size_pt`, left-aligned.
fn workbook_with_cell_font(family: &str, size_pt: f64) -> Vec<u8> {
    workbook_with_cell_font_and_alignment(
        family,
        size_pt,
        umya_spreadsheet::HorizontalAlignmentValues::Left,
    )
}

/// Every row of the issue #1165 probe whose family the reference digit table
/// carries, so the expectation is the same on any machine. Century Gothic and
/// Segoe UI are measured in the module doc but resolve through the live font
/// set, which CI does not ship.
const MEASURED_LEFT_INSETS: &[(&str, f64, f64)] = &[
    ("Calibri", 6.0, 2.0),
    ("Calibri", 7.0, 2.0),
    ("Calibri", 8.0, 2.0),
    ("Calibri", 9.0, 3.0),
    ("Calibri", 10.0, 3.0),
    ("Calibri", 11.0, 3.0),
    ("Calibri", 12.0, 3.0),
    ("Calibri", 14.0, 3.0),
    ("Calibri", 16.0, 3.0),
    ("Calibri", 17.0, 4.0),
    ("Calibri", 20.0, 4.0),
    ("Calibri", 24.0, 4.0),
    ("Calibri", 25.0, 5.0),
    ("Calibri", 28.0, 5.0),
    ("Calibri", 32.0, 5.0),
    ("Calibri", 33.0, 6.0),
    ("Calibri", 36.0, 6.0),
    ("Arial", 8.0, 2.0),
    ("Arial", 10.0, 3.0),
    ("Arial", 12.0, 3.0),
    ("Arial", 14.0, 3.0),
    ("Arial", 16.0, 4.0),
    ("Arial", 18.0, 4.0),
    ("Arial", 20.0, 4.0),
    ("Arial", 24.0, 5.0),
    ("Arial", 32.0, 6.0),
    ("Times New Roman", 11.0, 3.0),
    ("Times New Roman", 16.0, 3.0),
    ("Times New Roman", 18.0, 4.0),
    ("Verdana", 10.0, 3.0),
    ("Verdana", 11.0, 3.0),
];

#[test]
fn test_left_inset_steps_with_the_cell_fonts_own_column_unit() {
    for &(family, size_pt, expected_left) in MEASURED_LEFT_INSETS {
        let padding: Insets = first_cell_padding(&workbook_with_cell_font(family, size_pt));

        assert!(
            (padding.left - expected_left).abs() < 0.01,
            "{family} {size_pt} starts {expected_left}pt inside its column in Excel's own \
             export, got {}",
            padding.left,
        );
    }
}

#[test]
fn test_right_inset_steps_one_point_behind_the_cell_fonts_left_inset() {
    for &(family, size_pt, expected_left) in MEASURED_LEFT_INSETS {
        let data = workbook_with_cell_font_and_alignment(
            family,
            size_pt,
            umya_spreadsheet::HorizontalAlignmentValues::Right,
        );
        let padding: Insets = first_cell_padding(&data);
        let expected_right: f64 = expected_left - 1.0;

        assert!(
            (padding.right - expected_right).abs() < 0.01,
            "{family} {size_pt} ends {expected_right}pt inside its column in Excel's own \
             export, got {}",
            padding.right,
        );
    }
}

#[test]
fn test_a_title_cell_starts_further_in_than_the_body_line_below_it() {
    // The shape the issue reported: one column, a body line and a title, the
    // title's origin further right than the body's. Both sizes are probe rows,
    // and their 3pt step is the one the reported workbook's column B carries.
    let body: Insets = first_cell_padding(&workbook_with_cell_font("Arial", 14.0));
    let title: Insets = first_cell_padding(&workbook_with_cell_font("Arial", 32.0));

    assert!(
        (title.left - body.left - 3.0).abs() < 0.01,
        "Excel starts a 32pt title three points right of a 14pt body line in the same \
         column, got {} against {}",
        title.left,
        body.left,
    );
}

#[test]
fn test_the_cell_font_drives_the_inset_not_the_workbook_normal_font() {
    // The workbook Normal font stays Calibri 11, whose inset is 3pt; only the
    // cell's own font is larger.
    let padding: Insets = first_cell_padding(&workbook_with_cell_font("Arial", 32.0));

    assert!(
        (padding.left - 6.0).abs() < 0.01,
        "the cell's own Arial 32 takes a 6pt inset whatever the Normal font is, got {}",
        padding.left,
    );
}

#[test]
fn test_a_centred_cell_holds_the_column_centre_at_any_cell_font() {
    let mut book = umya_spreadsheet::new_file();
    {
        let sheet = book.get_sheet_mut(&0).unwrap();
        let cell = sheet.get_cell_mut("B3");
        cell.set_value("2026");
        let style = cell.get_style_mut();
        style
            .get_alignment_mut()
            .set_horizontal(umya_spreadsheet::HorizontalAlignmentValues::Center);
        style.get_font_mut().set_size(32.0);
    }
    let mut cursor = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(&book, &mut cursor).unwrap();

    let padding: Insets = first_cell_padding(&cursor.into_inner());

    assert!(
        (padding.left - padding.right).abs() < 0.01,
        "a centred run sits on the column's own centre whatever its font size, got \
         left {} right {}",
        padding.left,
        padding.right,
    );
    assert!(
        ((padding.left + padding.right) - 9.0).abs() < 0.01,
        "a centred Calibri 32 cell keeps the font-sized 5pt/4pt box total, got left {} \
         right {}",
        padding.left,
        padding.right,
    );
}

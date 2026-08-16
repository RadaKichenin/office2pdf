#![cfg(not(target_arch = "wasm32"))] // native-only integration tests (fs, qpdf, criterion)
//! Integration tests for XLSX fixtures.
//!
//! Each real-world `.xlsx` file in `tests/fixtures/xlsx/` gets two tests:
//! - **smoke**: `convert()` → valid PDF (or graceful error — no panic)
//! - **structure**: parse → assert expected IR content

mod common;

use std::path::PathBuf;

use office2pdf::config::ConvertOptions;
use office2pdf::internal::Parser;
use office2pdf::internal::XlsxParser;
use office2pdf::internal::generate_typst;
use office2pdf::ir::{
    Alignment, Block, BorderLineStyle, Color, HFInline, Page, SheetPage, TableCell,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/xlsx")
        .join(name)
}

fn load_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).expect("fixture file should exist")
}

/// Smoke-test helper: conversion must not panic.
fn assert_produces_valid_pdf(name: &str) {
    let path = fixture_path(name);
    match office2pdf::convert(&path) {
        Ok(result) => {
            assert!(!result.pdf.is_empty(), "PDF output should not be empty");
            assert!(
                result.pdf.starts_with(b"%PDF"),
                "output should start with PDF magic bytes"
            );
            common::validate_pdf_with_qpdf(&result.pdf);
        }
        Err(e) => {
            eprintln!("[WARN] {name}: conversion error (non-panic): {e}");
        }
    }
}

/// Parse an XLSX fixture and return the sheet pages.
fn sheet_pages(name: &str) -> Vec<SheetPage> {
    let data = load_fixture(name);
    let (doc, _warnings) = XlsxParser.parse(&data, &ConvertOptions::default()).unwrap();
    doc.pages
        .into_iter()
        .filter_map(|p| match p {
            Page::Sheet(sp) => Some(sp),
            _ => None,
        })
        .collect()
}

fn sheet_names(pages: &[SheetPage]) -> Vec<&str> {
    pages.iter().map(|p| p.name.as_str()).collect()
}

fn total_rows(pages: &[SheetPage]) -> usize {
    pages.iter().map(|p| p.table.rows.len()).sum()
}

fn has_cell_border(pages: &[SheetPage]) -> bool {
    pages.iter().any(|p| {
        p.table
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .any(|c| c.border.is_some())
    })
}

fn has_merged_cells(pages: &[SheetPage]) -> bool {
    pages.iter().any(|p| {
        p.table
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .any(|c| c.col_span > 1 || c.row_span > 1)
    })
}

fn has_formatted_text(pages: &[SheetPage]) -> bool {
    pages.iter().any(|p| {
        p.table.rows.iter().flat_map(|r| r.cells.iter()).any(|c| {
            c.content.iter().any(|b| match b {
                Block::Paragraph(para) => para.runs.iter().any(|r| {
                    r.style.bold == Some(true)
                        || r.style.italic == Some(true)
                        || r.style.color.is_some()
                }),
                _ => false,
            })
        })
    })
}

fn table_cell_text(cell: &TableCell) -> String {
    cell.content
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => Some(
                paragraph
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect()
}

fn sheet_page_named<'a>(pages: &'a [SheetPage], name: &str) -> &'a SheetPage {
    pages
        .iter()
        .find(|page| page.name == name)
        .unwrap_or_else(|| {
            let available = pages
                .iter()
                .map(|page| page.name.as_str())
                .collect::<Vec<_>>();
            panic!("missing sheet page {name}; available pages: {available:?}")
        })
}

// ---------------------------------------------------------------------------
// PR #186 contributor acceptance fixture
// ---------------------------------------------------------------------------

const PR_186_FIXTURE: &str = "pr_186_contributor_acceptance.xlsx";

#[test]
fn smoke_pr_186_contributor_acceptance_fixture() {
    assert_produces_valid_pdf(PR_186_FIXTURE);
}

#[test]
fn structure_pr_186_contributor_acceptance_supported_behavior() {
    let pages = sheet_pages(PR_186_FIXTURE);
    let statement = sheet_page_named(&pages, "Statement Landscape");
    let statement_pages = pages
        .iter()
        .filter(|page| page.name == "Statement Landscape")
        .collect::<Vec<_>>();
    let executive = sheet_page_named(&pages, "Executive Portrait");

    assert_eq!(statement.table.rows.len(), 4);
    assert_eq!(
        statement_pages
            .iter()
            .map(|page| page.table.column_widths.len())
            .sum::<usize>(),
        4
    );
    assert!(
        (statement.table.column_widths[1] - 120.0).abs() < 0.01,
        "20 Carlito character units should convert to 120pt"
    );

    let expected_alignments = [
        Alignment::Left,
        Alignment::Center,
        Alignment::Right,
        Alignment::Justify,
    ];
    let alignment_cells = statement_pages
        .iter()
        .flat_map(|page| page.table.rows[1].cells.iter());
    for (cell, expected) in alignment_cells.zip(expected_alignments) {
        let paragraph = match &cell.content[0] {
            Block::Paragraph(paragraph) => paragraph,
            _ => panic!("alignment cell should contain a paragraph"),
        };
        assert_eq!(paragraph.style.alignment, Some(expected));
    }

    assert!(
        !table_cell_text(&statement.table.rows[2].cells[1]).is_empty(),
        "the text-valued General cell should be retained"
    );
    let top_border = statement.table.rows[3].cells[0]
        .border
        .as_ref()
        .and_then(|border| border.top.as_ref())
        .expect("A4 should have a top border");
    assert_eq!(top_border.style, BorderLineStyle::Double);

    assert!((statement.margins.top - 28.8).abs() < 0.01);
    assert!((statement.margins.bottom - 36.0).abs() < 0.01);
    assert!((statement.margins.left - 21.6).abs() < 0.01);
    assert!((statement.margins.right - 43.2).abs() < 0.01);
    assert!((executive.margins.top - 54.0).abs() < 0.01);
    assert!((executive.margins.bottom - 54.0).abs() < 0.01);
    assert!((executive.margins.left - 50.4).abs() < 0.01);
    assert!((executive.margins.right - 50.4).abs() < 0.01);
}

#[test]
fn acceptance_pr_186_contributor_acceptance_six_digit_colors() {
    let pages = sheet_pages(PR_186_FIXTURE);
    let statement = sheet_page_named(&pages, "Statement Landscape");
    let header_cell = &statement.table.rows[0].cells[0];

    assert_eq!(
        header_cell.background,
        Some(Color::new(0xD9, 0xEA, 0xF7)),
        "six-digit header fill should survive XLSX parsing"
    );
    let paragraph = match &header_cell.content[0] {
        Block::Paragraph(paragraph) => paragraph,
        _ => panic!("header cell should contain a paragraph"),
    };
    assert_eq!(
        paragraph.runs[0].style.color,
        Some(Color::new(0x16, 0x32, 0x4F)),
        "six-digit header font color should survive XLSX parsing"
    );
}

#[test]
fn acceptance_pr_186_contributor_acceptance_carlito_fallback() {
    let data = load_fixture(PR_186_FIXTURE);
    let (document, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("fixture should parse");
    let output = generate_typst(&document).expect("fixture should generate Typst");

    assert!(
        output
            .source
            .contains(r#"font: ("Carlito", "Calibri", "Liberation Sans", "Arimo", "Arial")"#),
        "Carlito should retain a sans-serif fallback chain: {}",
        output.source
    );
}

#[test]
fn acceptance_pr_186_contributor_acceptance_numeric_general_alignment() {
    let pages = sheet_pages(PR_186_FIXTURE);
    let statement = sheet_page_named(&pages, "Statement Landscape");
    let general_row = &statement.table.rows[2];

    let numeric = match &general_row.cells[0].content[0] {
        Block::Paragraph(paragraph) => paragraph,
        _ => panic!("numeric cell should contain a paragraph"),
    };
    let numeric_looking_text = match &general_row.cells[1].content[0] {
        Block::Paragraph(paragraph) => paragraph,
        _ => panic!("text cell should contain a paragraph"),
    };
    assert_eq!(numeric.style.alignment, Some(Alignment::Right));
    assert_eq!(numeric_looking_text.style.alignment, None);
}

#[test]
fn acceptance_pr_186_contributor_acceptance_page_setup() {
    let pages = sheet_pages(PR_186_FIXTURE);
    let statement = sheet_page_named(&pages, "Statement Landscape");
    let executive = sheet_page_named(&pages, "Executive Portrait");

    assert!((statement.size.width - 612.0).abs() < 0.01);
    assert!((statement.size.height - 396.0).abs() < 0.01);
    assert!((executive.size.width - 522.0).abs() < 0.01);
    assert!((executive.size.height - 756.0).abs() < 0.01);
}

#[test]
fn acceptance_pr_186_contributor_acceptance_print_pagination() {
    let pages = sheet_pages(PR_186_FIXTURE);
    let statement_pages = pages
        .iter()
        .filter(|page| page.name == "Statement Landscape")
        .collect::<Vec<_>>();

    assert_eq!(
        statement_pages.len(),
        2,
        "the four statement columns should print as two horizontal pages"
    );
    assert_eq!(
        statement_pages[0].table.column_widths,
        [156.0, 120.0, 144.0]
    );
    assert_eq!(statement_pages[1].table.column_widths, [144.0]);
    assert_eq!(
        table_cell_text(&statement_pages[0].table.rows[0].cells[2]),
        "Explicit Right"
    );
    assert_eq!(
        table_cell_text(&statement_pages[1].table.rows[0].cells[0]),
        "Explicit Justify"
    );
}

#[test]
fn acceptance_pr_186_contributor_acceptance_double_border_rendering() {
    let data = load_fixture(PR_186_FIXTURE);
    let (document, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("fixture should parse");
    let output = generate_typst(&document).expect("fixture should generate Typst");

    assert!(!output.source.contains("dash: \"dashed\""));
    // Overlay offsets track the cell padding, which is 3pt each side since
    // issue #657 (it was 2pt, and the left inset put every left-aligned run
    // 1pt left of Excel). Since issue #619 the double paints as two
    // boundary-anchored 1pt bands [B-1, B] and [B+1, B+2], and each band runs
    // 1pt past its end boundary, so the run is the 6pt padding backout plus 1.
    assert!(output.source.contains(
        "#place(top + left, dx: -3pt, dy: -2pt, line(length: 100% + 7pt, angle: 0deg, stroke: 1pt + rgb(0, 0, 0)))"
    ));
    assert!(output.source.contains(
        "#place(top + left, dx: -3pt, dy: 0pt, line(length: 100% + 7pt, angle: 0deg, stroke: 1pt + rgb(0, 0, 0)))"
    ));
}

// ---------------------------------------------------------------------------
// any_sheets.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_any_sheets() {
    assert_produces_valid_pdf("any_sheets.xlsx");
}

#[test]
fn structure_any_sheets() {
    // any_sheets.xlsx declares 4 sheets: Visible, Hidden (state="hidden"),
    // VeryHidden (state="veryHidden") and Chart. The hidden pair is out
    // because Excel never prints a hidden sheet (issue #1065) — and empty of
    // cells besides. `Chart` is a chartsheet, and Excel prints one of those as
    // a page of its own (issue #1099), so two pages are left.
    let pages = sheet_pages("any_sheets.xlsx");
    assert_eq!(sheet_names(&pages), vec!["Visible", "Chart"]);
}

/// A chartsheet prints as one page carrying its chart alone.
///
/// Measured on Excel for Mac 16.100 exports of this fixture's `Chart` sheet:
/// one page, the chart filling it, and — since the chartsheet declares no
/// `<pageSetup>` — landscape, where the same workbook's `Visible` worksheet
/// (equally without one) exports portrait.
#[test]
fn structure_any_sheets_chartsheet_pages_its_chart_full_page() {
    let pages = sheet_pages("any_sheets.xlsx");
    let chart_page = pages
        .iter()
        .find(|page| page.name == "Chart")
        .expect("the chartsheet should contribute a page");

    // No `<pageSetup>`: Letter (the schema's paperSize default, issue #717)
    // turned on its side by the chartsheet's own landscape default.
    assert_eq!(
        (chart_page.size.width, chart_page.size.height),
        (792.0, 612.0)
    );
    // The chartsheet's own `<pageMargins>`: 0.7" sides, 0.75" top and bottom.
    assert_eq!(
        (
            chart_page.margins.left,
            chart_page.margins.right,
            chart_page.margins.top,
            chart_page.margins.bottom,
        ),
        (50.4, 50.4, 54.0, 54.0)
    );
    // A chartsheet holds no cells at all.
    assert!(chart_page.table.rows.is_empty());

    assert_eq!(chart_page.charts.len(), 1);
    let placement = chart_page.charts[0]
        .placement
        .expect("a chartsheet's chart is placed, not flowed after the grid");
    assert_eq!((placement.x_offset_pt, placement.y_offset_pt), (0.0, 0.0));
    assert_eq!((placement.width, placement.height), (691.2, 504.0));

    // The chart belongs to the chartsheet, not to the worksheet before it: a
    // native export of `Visible` alone draws no chart at all.
    let worksheet_page = pages
        .iter()
        .find(|page| page.name == "Visible")
        .expect("the visible worksheet should still page");
    assert!(worksheet_page.charts.is_empty());
}

/// Two chartsheet packages whose parts collide by filename, which is how the
/// worksheet-only rels lookup went wrong in the first place.
///
/// `chart_sheet.xlsx` has no `xl/worksheets/_rels/` at all, so its chartsheet
/// resolved to nothing and its chart fell through to the first worksheet as an
/// orphan. `SimpleScatterChart.xlsx` has both `xl/worksheets/sheet1.xml` and
/// `xl/chartsheets/sheet1.xml`, so the chartsheet read the *worksheet's* rels
/// and printed a second copy of the worksheet's chart instead of its own.
#[test]
fn structure_chartsheet_reads_its_own_drawing_relationships() {
    let pages = sheet_pages("chart_sheet.xlsx");
    assert_eq!(sheet_names(&pages), vec!["Sheet1", "Chart1"]);
    assert!(pages[0].charts.is_empty());
    assert_eq!(pages[1].charts.len(), 1);

    let scatter = sheet_pages("SimpleScatterChart.xlsx");
    assert_eq!(sheet_names(&scatter), vec!["Sheet1", "Chart1"]);
    // The worksheet's chart keeps its own anchor; the chartsheet's fills the
    // page, so the two placements can no longer be the same box.
    let worksheet_placement = scatter[0].charts[0]
        .placement
        .expect("the worksheet chart is anchored");
    let chartsheet_placement = scatter[1].charts[0]
        .placement
        .expect("the chartsheet chart is placed full-page");
    assert_eq!(
        (chartsheet_placement.width, chartsheet_placement.height),
        (691.2, 504.0)
    );
    assert_ne!(
        (worksheet_placement.width, worksheet_placement.height),
        (chartsheet_placement.width, chartsheet_placement.height)
    );
}

// ---------------------------------------------------------------------------
// issue_1065_hidden_sheet_probe.xlsx
// ---------------------------------------------------------------------------

/// The probe workbook Excel for Mac 16.112 itself authored for issue #1065:
/// a visible `Summary` sheet and a populated `state="hidden"` `Data` sheet,
/// with `paperSize="9"` written into both worksheets afterwards so the paper
/// comes from the file rather than from whichever printer exported it.
///
/// A native export of it is one page — Excel prints no hidden sheet — and so
/// is ours; the hidden sheet's rows reach neither the page list nor the IR.
#[test]
fn structure_issue_1065_probe_pages_only_the_visible_sheet() {
    let pages = sheet_pages("issue_1065_hidden_sheet_probe.xlsx");
    assert_eq!(sheet_names(&pages), vec!["Summary"]);

    let printed_text: String = pages
        .iter()
        .flat_map(|page| page.table.rows.iter())
        .flat_map(|row| row.cells.iter())
        .map(table_cell_text)
        .collect::<Vec<String>>()
        .join(" ");
    assert!(
        printed_text.contains("Regional revenue"),
        "the visible sheet's title should print"
    );
    assert!(
        !printed_text.contains("N-01"),
        "the hidden sheet's lookup rows should not print"
    );
}

// ---------------------------------------------------------------------------
// 123233_charts.xlsx
// ---------------------------------------------------------------------------

/// The workbook's four `data_Page1_1_*` sheets are `state="hidden"` scratch
/// data behind the charts on `Page1_1`. Excel prints the one visible sheet;
/// paging the hidden four added four pages no reference export has
/// (issue #1065).
#[test]
fn structure_charts_123233_pages_only_the_visible_sheet() {
    let pages = sheet_pages("123233_charts.xlsx");
    let names = sheet_names(&pages);
    // `Page1_1` is wider than its paper, so it splits across pages of its own.
    assert!(
        names.iter().all(|name| *name == "Page1_1"),
        "only the visible sheet should page, got {names:?}"
    );
}

// ---------------------------------------------------------------------------
// date.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_date() {
    assert_produces_valid_pdf("date.xlsx");
}

#[test]
fn structure_date() {
    let pages = sheet_pages("date.xlsx");
    assert!(!pages.is_empty(), "should have at least one sheet");
    assert!(total_rows(&pages) > 0, "should have data rows");
}

// ---------------------------------------------------------------------------
// merge_cells.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_merge_cells() {
    assert_produces_valid_pdf("merge_cells.xlsx");
}

#[test]
fn structure_merge_cells() {
    let pages = sheet_pages("merge_cells.xlsx");
    assert!(
        has_merged_cells(&pages),
        "should have cells with col_span > 1 or row_span > 1"
    );
}

// ---------------------------------------------------------------------------
// SH001-Table.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_sh001_table() {
    assert_produces_valid_pdf("SH001-Table.xlsx");
}

#[test]
fn structure_sh001_table() {
    let pages = sheet_pages("SH001-Table.xlsx");
    assert!(!pages.is_empty(), "should have at least one sheet");
    assert!(total_rows(&pages) > 0, "should have data rows");
}

// ---------------------------------------------------------------------------
// SH002-TwoTablesTwoSheets.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_sh002_two_tables_two_sheets() {
    assert_produces_valid_pdf("SH002-TwoTablesTwoSheets.xlsx");
}

#[test]
fn structure_sh002_two_tables_two_sheets() {
    let pages = sheet_pages("SH002-TwoTablesTwoSheets.xlsx");
    assert!(pages.len() >= 2, "should have >= 2 sheets");
    let names = sheet_names(&pages);
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "sheet names should be unique");
}

// ---------------------------------------------------------------------------
// SH106-Formatted.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_sh106_formatted() {
    assert_produces_valid_pdf("SH106-Formatted.xlsx");
}

#[test]
fn structure_sh106_formatted() {
    let pages = sheet_pages("SH106-Formatted.xlsx");
    assert!(
        has_formatted_text(&pages),
        "should have formatted text (bold/italic/color)"
    );
}

// ---------------------------------------------------------------------------
// SH109-CellWithBorder.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_sh109_cell_with_border() {
    assert_produces_valid_pdf("SH109-CellWithBorder.xlsx");
}

#[test]
fn structure_sh109_cell_with_border() {
    let pages = sheet_pages("SH109-CellWithBorder.xlsx");
    assert!(has_cell_border(&pages), "should have cells with borders");
}

// ---------------------------------------------------------------------------
// temperature.xlsx
// ---------------------------------------------------------------------------

#[test]
fn smoke_temperature() {
    assert_produces_valid_pdf("temperature.xlsx");
}

#[test]
fn structure_temperature() {
    let pages = sheet_pages("temperature.xlsx");
    assert!(!pages.is_empty(), "should have at least one sheet");
    assert!(total_rows(&pages) > 0, "should have data rows");
}

// ===========================================================================
// PDF text content verification
// ===========================================================================

/// Helper: convert an XLSX fixture to PDF and extract text.
fn pdf_text(name: &str) -> String {
    let path = fixture_path(name);
    let result = office2pdf::convert(&path).expect("conversion should succeed");
    common::extract_pdf_text(&result.pdf)
}

// ---------------------------------------------------------------------------
// temperature.xlsx — text content
// ---------------------------------------------------------------------------

#[test]
fn text_content_temperature() {
    let text = pdf_text("temperature.xlsx");
    assert!(
        text.contains("celsius"),
        "PDF should contain 'celsius' label"
    );
    assert!(
        text.contains("fahrenheit"),
        "PDF should contain 'fahrenheit' label"
    );
}

// ---------------------------------------------------------------------------
// SH001-Table.xlsx — text content
// ---------------------------------------------------------------------------

#[test]
fn text_content_sh001_table() {
    let text = pdf_text("SH001-Table.xlsx");
    // This is a simple table with single-character headers and numeric data
    assert!(!text.is_empty(), "PDF should contain extracted text");
    // Check for numeric data that should be present
    assert!(
        text.contains('1') && text.contains('2') && text.contains('3'),
        "PDF should contain numeric data from the table"
    );
}

// ---------------------------------------------------------------------------
// SH002-TwoTablesTwoSheets.xlsx — text content
// ---------------------------------------------------------------------------

#[test]
fn text_content_sh002_two_tables_two_sheets() {
    let text = pdf_text("SH002-TwoTablesTwoSheets.xlsx");
    assert!(!text.is_empty(), "PDF should contain extracted text");
    // Both sheets have different content; verify we have data from at least one
    assert!(
        text.contains('1') || text.contains('a') || text.contains('q'),
        "PDF should contain data from the sheets"
    );
}

// ===========================================================================
// Third-party fixtures — smoke tests (must not panic)
// ===========================================================================

/// Generate a pair of smoke + basic-structure tests for an XLSX fixture.
macro_rules! xlsx_fixture_tests {
    ($test_name:ident, $file:expr) => {
        paste::paste! {
            #[test]
            fn [<smoke_ $test_name>]() {
                assert_produces_valid_pdf($file);
            }

            #[test]
            fn [<structure_ $test_name>]() {
                let data = load_fixture($file);
                match XlsxParser.parse(&data, &ConvertOptions::default()) {
                    Ok((doc, _)) => {
                        let _ = doc.pages.len();
                    }
                    Err(e) => {
                        eprintln!("[WARN] {}: parse error (non-panic): {e}", $file);
                    }
                }
            }
        }
    };
}

// --- CC0 (Public Domain) ---------------------------------------------------

xlsx_fixture_tests!(ffc, "ffc.xlsx");
xlsx_fixture_tests!(hundred_customers, "100-customers.xlsx");
xlsx_fixture_tests!(thousand_customers, "1000-customers.xlsx");

// --- Apache POI (Apache 2.0) -----------------------------------------------

xlsx_fixture_tests!(charts_123233, "123233_charts.xlsx");
xlsx_fixture_tests!(booleans, "Booleans.xlsx");
xlsx_fixture_tests!(chart_sheet, "chart_sheet.xlsx");
xlsx_fixture_tests!(comments, "comments.xlsx");
xlsx_fixture_tests!(excel_pivot_table, "ExcelPivotTableSample.xlsx");
xlsx_fixture_tests!(excel_tables, "ExcelTables.xlsx");
xlsx_fixture_tests!(formatting, "Formatting.xlsx");
xlsx_fixture_tests!(group_test, "GroupTest.xlsx");
xlsx_fixture_tests!(header_footer_test, "headerFooterTest.xlsx");
xlsx_fixture_tests!(inline_string, "InlineString.xlsx");
xlsx_fixture_tests!(picture, "picture.xlsx");
xlsx_fixture_tests!(right_to_left, "right-to-left.xlsx");
xlsx_fixture_tests!(sample_ss, "SampleSS.xlsx");
xlsx_fixture_tests!(shared_formulas, "shared_formulas.xlsx");
xlsx_fixture_tests!(sheet_tab_colors, "SheetTabColors.xlsx");
xlsx_fixture_tests!(simple_monthly_budget, "simple-monthly-budget.xlsx");
xlsx_fixture_tests!(simple_scatter_chart, "SimpleScatterChart.xlsx");
xlsx_fixture_tests!(themes, "Themes.xlsx");
xlsx_fixture_tests!(with_chart, "WithChart.xlsx");
xlsx_fixture_tests!(with_drawing, "WithDrawing.xlsx");
xlsx_fixture_tests!(with_more_various_data, "WithMoreVariousData.xlsx");
xlsx_fixture_tests!(with_text_box, "WithTextBox.xlsx");
xlsx_fixture_tests!(with_various_data, "WithVariousData.xlsx");

// --- Repo-authored regression fixtures --------------------------------------

xlsx_fixture_tests!(theme_color_drawing, "theme_color_drawing.xlsx");
xlsx_fixture_tests!(
    issue_1065_hidden_sheet_probe,
    "issue_1065_hidden_sheet_probe.xlsx"
);

// --- MIT: Open-Xml-PowerTools (Microsoft) ----------------------------------

xlsx_fixture_tests!(
    sh003_date_first_col,
    "SH003-TableWithDateInFirstColumn.xlsx"
);
xlsx_fixture_tests!(sh004_offset_location, "SH004-TableAtOffsetLocation.xlsx");
xlsx_fixture_tests!(sh005_shared_strings, "SH005-Table-With-SharedStrings.xlsx");
xlsx_fixture_tests!(sh006_no_shared_strings, "SH006-Table-No-SharedStrings.xlsx");
xlsx_fixture_tests!(sh007_one_cell, "SH007-One-Cell-Table.xlsx");
xlsx_fixture_tests!(sh008_tall_row, "SH008-Table-With-Tall-Row.xlsx");
xlsx_fixture_tests!(sh101_simple_formats, "SH101-SimpleFormats.xlsx");
xlsx_fixture_tests!(sh102_9x9, "SH102-9-x-9.xlsx");
xlsx_fixture_tests!(sh103_no_shared_string, "SH103-No-SharedString.xlsx");
xlsx_fixture_tests!(sh104_with_shared_string, "SH104-With-SharedString.xlsx");
xlsx_fixture_tests!(sh105_no_shared_string2, "SH105-No-SharedString.xlsx");
xlsx_fixture_tests!(sh107_formatted_table, "SH107-9-x-9-Formatted-Table.xlsx");
xlsx_fixture_tests!(
    sh108_simple_formatted_cell,
    "SH108-SimpleFormattedCell.xlsx"
);

// --- Upstream parse failures (umya-spreadsheet) ------------------------------
// Related: #97
// These files fail with parse errors in umya-spreadsheet. All are handled
// gracefully — no panics, no crashes. Documented as known upstream limitations.

// ZipError: specified file not found in archive
xlsx_fixture_tests!(tdf121887, "libreoffice/tdf121887.xlsx");
xlsx_fixture_tests!(tdf131575, "libreoffice/tdf131575.xlsx");
xlsx_fixture_tests!(tdf76115, "libreoffice/tdf76115.xlsx");
xlsx_fixture_tests!(poi_49609, "poi/49609.xlsx");
xlsx_fixture_tests!(poi_56278, "poi/56278.xlsx");
xlsx_fixture_tests!(poi_59021, "poi/59021.xlsx");

// IoError: Invalid checksum
xlsx_fixture_tests!(forcepoint107, "libreoffice/forcepoint107.xlsx");

// ZipError: invalid Zip archive (Could not find EOCD)
xlsx_fixture_tests!(deep_data, "poi/deep-data.xlsx");

// --- Upstream panics caught by catch_unwind (umya-spreadsheet) ----------------
// Related: #97
// These files trigger panics inside umya-spreadsheet (arithmetic overflow,
// unwrap on None). catch_unwind prevents process crashes. Documented as
// known upstream limitations.

// attempt to subtract with overflow
xlsx_fixture_tests!(
    functions_excel_2010,
    "libreoffice/functions-excel-2010.xlsx"
);
xlsx_fixture_tests!(poi_51710, "poi/51710.xlsx");

// called Option::unwrap() on a None value
xlsx_fixture_tests!(poi_64450, "poi/64450.xlsx");

// attempt to multiply with overflow
xlsx_fixture_tests!(
    formula_eval_test_data_copy,
    "poi/FormulaEvalTestData_Copy.xlsx"
);

// --- Upstream panic safety (patched umya-spreadsheet) ------------------------
// Related: #83

/// Files that previously panicked in umya-spreadsheet now convert successfully
/// after the fork fix (developer0hye/umya-spreadsheet fix/panic-safety-v2).
///
/// All 21 previously-panicking files now produce valid PDFs.
#[test]
fn previously_panicking_files_now_convert() {
    let cases: &[&str] = &[
        // --- Phase 1 fixes (PR #90) ---
        // FileNotFound panics (7 files)
        "libreoffice/chart_hyperlink.xlsx",
        "libreoffice/hyperlink.xlsx",
        "libreoffice/tdf130959.xlsx",
        "libreoffice/test_115192.xlsx",
        "poi/47504.xlsx",
        "poi/bug63189.xlsx",
        "poi/ConditionalFormattingSamples.xlsx",
        // ParseFloatError / boolean cell (1 file)
        "libreoffice/check-boolean.xlsx",
        // unwrap() on None (2 files)
        "libreoffice/tdf100709.xlsx",
        "poi/sample-beta.xlsx",
        // dataBar end element (2 files)
        "libreoffice/tdf162948.xlsx",
        "poi/NewStyleConditionalFormattings.xlsx",
        // --- Phase 2 fixes ---
        // Backslash zip paths from Windows tools (3 files)
        "libreoffice/tdf131575.xlsx",
        "libreoffice/tdf76115.xlsx",
        "poi/49609.xlsx",
        // Missing optional styles.xml (3 files)
        "poi/56278.xlsx",
        "libreoffice/tdf121887.xlsx",
        "poi/59021.xlsx",
        // Arithmetic overflow in formula parsing (2 files)
        "libreoffice/functions-excel-2010.xlsx",
        "poi/FormulaEvalTestData_Copy.xlsx",
        // Missing XML attributes (1 file)
        "poi/64450.xlsx",
    ];
    for name in cases {
        let path = fixture_path(name);
        if !path.exists() {
            eprintln!("Skipping {name}: fixture not available");
            continue;
        }
        assert_produces_valid_pdf(name);
    }
}

// --- MIT: calamine (Rust) --------------------------------------------------

xlsx_fixture_tests!(date_1904, "date_1904.xlsx");
xlsx_fixture_tests!(empty_sheet, "empty_sheet.xlsx");
xlsx_fixture_tests!(errors, "errors.xlsx");
xlsx_fixture_tests!(pivots, "pivots.xlsx");
xlsx_fixture_tests!(richtext_namespaced, "richtext-namespaced.xlsx");
xlsx_fixture_tests!(column_row_ranges, "column_row_ranges.xlsx");
xlsx_fixture_tests!(table_multiple, "table-multiple.xlsx");
xlsx_fixture_tests!(formula_issue, "formula.issue.xlsx");
xlsx_fixture_tests!(header_row, "header-row.xlsx");

// --- Encrypted / password-protected fixtures --------------------------------

#[test]
fn encrypted_xlsx_returns_unsupported_encryption() {
    let path = fixture_path("poi/protected_passtika.xlsx");
    let err = office2pdf::convert(&path).unwrap_err();
    assert!(
        matches!(err, office2pdf::error::ConvertError::UnsupportedEncryption),
        "Expected UnsupportedEncryption for protected_passtika.xlsx, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Worksheet drawings (issue #238)
// ---------------------------------------------------------------------------

#[test]
fn with_drawing_renders_anchored_images() {
    let pages = sheet_pages("poi/WithDrawing.xlsx");
    let total_images: usize = pages.iter().map(|sp| sp.images.len()).sum();
    assert!(
        total_images >= 3,
        "the drawing anchors five pictures (jpeg/png plus metafiles); at least \
         the raster ones must be extracted, got {total_images}"
    );
    let first = pages
        .iter()
        .flat_map(|sp| sp.images.iter())
        .next()
        .expect("at least one image");
    assert!(!first.image.data.is_empty(), "image bytes must be loaded");
    assert!(
        first.image.width.unwrap_or(0.0) > 10.0,
        "anchor geometry must produce a real width, got {:?}",
        first.image.width
    );
}

/// Excel writes `a:blip` as a start element with children whenever the picture
/// carries an alpha or recolour effect. The fixture is an Excel for Mac export
/// whose only picture is spelled that way — `<a:blip r:embed="rId1">
/// <a:alphaModFix amt="70000"/></a:blip>` — and its native export draws the
/// photo, so dropping it left the anchored band empty (issue #1066).
#[test]
fn picture_with_effect_bearing_blip_is_still_drawn() {
    let pages = sheet_pages("issue_1066_blip_effect_picture.xlsx");
    let images: Vec<_> = pages.iter().flat_map(|sp| sp.images.iter()).collect();
    assert_eq!(images.len(), 1, "the drawing anchors one picture");
    assert!(
        !images[0].image.data.is_empty(),
        "the blip's r:embed must resolve to the media bytes"
    );
    assert!(
        images[0].image.width.unwrap_or(0.0) > 10.0 && images[0].image.height.unwrap_or(0.0) > 10.0,
        "the anchor must produce a real size, got {:?} x {:?}",
        images[0].image.width,
        images[0].image.height
    );
}

/// `<a:alphaModFix amt="70000"/>` on that same blip is a transparency effect:
/// Excel draws the picture at 70% strength over whatever the worksheet shows
/// beneath it, and its native export wraps the bitmap in a soft mask so the
/// `#C0392B` bar prints as that colour composited onto white. Emitting the raw
/// bitmap printed a saturated block against a washed-out ground truth (issue
/// #1103).
#[test]
fn picture_alpha_mod_fix_is_baked_into_the_bitmap() {
    let pages = sheet_pages("issue_1066_blip_effect_picture.xlsx");
    let images: Vec<_> = pages.iter().flat_map(|sp| sp.images.iter()).collect();
    assert_eq!(images.len(), 1, "the drawing anchors one picture");

    let decoded = image::load_from_memory(&images[0].image.data)
        .expect("the anchored picture must stay a decodable bitmap")
        .into_rgba8();
    // The source is a flat three-bar PNG: #C0392B, #27AE60, #2980B9 on white,
    // every pixel fully opaque. At 70% each of them composites onto the white
    // page as 0.7 * channel + 0.3 * 255 - the red bar as (211, 116, 107).
    let red_bar: Vec<&image::Rgba<u8>> = decoded
        .pixels()
        .filter(|pixel| pixel[0] == 192 && pixel[1] == 57 && pixel[2] == 43)
        .collect();
    assert!(
        !red_bar.is_empty(),
        "the source bar colour must survive the alpha bake unchanged"
    );
    for pixel in &red_bar {
        assert_eq!(
            pixel[3], 179,
            "70% of an opaque pixel is 178.5, rounded to 179"
        );
    }
}

/// A drawing anchor spans the rows Excel *prints*, not the heights the
/// worksheet declares. The same fixture's Excel for Mac export draws its
/// picture 105.00pt tall with its top 90.00pt below the first row — seven and
/// six 15pt tracks — while the sheet declares `defaultRowHeight="18"` and
/// `ht="16"`, and Excel honours neither: it recomputes 16pt rows from the
/// Calibri 12 Normal font and prints them at the 15pt track that font's grid
/// compacts to (issue #1102).
///
/// The picture's width is left out on purpose. Excel's columns here measure
/// 65.00pt in the app and in the export alike, matching ours, and the 11.00pt
/// the export is narrower comes from a `to` `colOff` that overruns its column
/// — a separate rule, tracked in #1149.
#[test]
fn a_picture_anchor_spans_the_printed_row_track() {
    let pages = sheet_pages("issue_1066_blip_effect_picture.xlsx");
    let images: Vec<_> = pages.iter().flat_map(|sp| sp.images.iter()).collect();
    assert_eq!(images.len(), 1, "the drawing anchors one picture");
    let height: f64 = images[0].image.height.expect("a two-cell anchor sizes");
    assert!(
        (height - 105.0).abs() < 0.01,
        "seven printed 15pt tracks, got {height}"
    );
    assert!(
        (images[0].y_offset_pt - 90.0).abs() < 0.01,
        "six printed 15pt tracks above the anchor row, got {}",
        images[0].y_offset_pt
    );
}

/// Excel cuts a blocked cell's unwrapped line at the cell's own gridline, not
/// at the inset its text starts from. The same fixture holds `Wrapping paper`
/// in `B4` against a value in `C4`, and its Excel for Mac export prints
/// `Wrapping pa`: the next `p` begins on the column boundary and is left
/// undrawn. Sizing the clip box from the *content* edge instead pushed that
/// boundary a whole left inset further right, which is exactly the room the
/// extra glyph needed (issue #1105).
#[test]
fn a_blocked_cell_clips_its_line_at_the_column_gridline() {
    let data = load_fixture("issue_1066_blip_effect_picture.xlsx");
    let (document, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("fixture should parse");
    let source = generate_typst(&document)
        .expect("fixture should generate Typst")
        .source;

    // 65pt columns laid out from a 3pt left inset: the gridline is 62pt past
    // the point the line starts at.
    assert!(
        source.contains("place(left + horizon, box(width: 62pt,"),
        "a blocked cell's clip must end on its column's right gridline: {source}"
    );
    assert!(
        !source.contains("place(left + horizon, box(width: 65pt,"),
        "a whole-column clip box overhangs that gridline by the left inset: {source}"
    );
}

// ---------------------------------------------------------------------------
// Worksheet drawing scheme colors resolve against the theme (issue #430)
// ---------------------------------------------------------------------------

#[test]
fn theme_color_drawing_resolves_scheme_fills() {
    use office2pdf::ir::Color;

    let pages = sheet_pages("theme_color_drawing.xlsx");
    let boxes: Vec<_> = pages.iter().flat_map(|sp| sp.text_boxes.iter()).collect();
    assert_eq!(boxes.len(), 3, "fixture drawing holds three text boxes");

    // accent1 straight from the workbook theme.
    assert_eq!(boxes[0].fill, Some(Color::new(68, 114, 196)));
    // "accent1, lighter 60%": lumMod 40% + lumOff 60% in HSL space.
    assert_eq!(boxes[1].fill, Some(Color::new(180, 199, 231)));
    // "accent6, darker 25%": lumMod 75%.
    assert_eq!(boxes[2].fill, Some(Color::new(84, 130, 53)));
}

// ---------------------------------------------------------------------------
// Worksheet drawing anchor geometry (issue #460)
// ---------------------------------------------------------------------------

#[test]
fn theme_color_drawing_anchors_span_six_eighteen_point_rows() {
    // The three shapes are `xdr:twoCellAnchor`s spanning rows 3..9 of a sheet
    // whose rows measure 18pt in the worksheet, so Excel draws them
    // 6 * 18 = 108pt tall - measured at 108pt on the native export, whose
    // printed grid keeps that 18pt because this workbook's Normal font
    // resolves through a per-script theme scheme and so does not compact
    // (issues #460, #1102). Resolving the anchor through a track that rounded
    // 18pt to 17pt left every shape 6pt short.
    let pages = sheet_pages("theme_color_drawing.xlsx");
    let boxes: Vec<_> = pages.iter().flat_map(|sp| sp.text_boxes.iter()).collect();
    assert_eq!(boxes.len(), 3, "fixture drawing holds three text boxes");
    for text_box in &boxes {
        assert!(
            (text_box.height - 108.0).abs() < 0.5,
            "anchor should span six 18pt rows, got {}",
            text_box.height
        );
    }
}

// ---------------------------------------------------------------------------
// Worksheet text boxes (issue #240)
// ---------------------------------------------------------------------------

#[test]
fn with_text_box_renders_anchored_text() {
    use office2pdf::ir::{Alignment, Color};

    let pages = sheet_pages("poi/WithTextBox.xlsx");
    let boxes: Vec<_> = pages.iter().flat_map(|sp| sp.text_boxes.iter()).collect();
    assert_eq!(boxes.len(), 1, "the drawing holds one text box");

    let text_box = boxes[0];
    assert_eq!(text_box.paragraphs.len(), 3);
    assert!(
        text_box.width > 50.0,
        "anchor width, got {}",
        text_box.width
    );

    let para = &text_box.paragraphs[0];
    assert_eq!(para.runs[0].text, "Line 1");
    assert_eq!(para.style.alignment, None, "algn=l maps to default/left");
    // This `xdr:txBody` declares no `a:spcBef`/`a:spcAft`, so its paragraphs
    // must carry an explicit zero rather than being left unset — unset lets
    // the renderer's own default block spacing in, which doubled the pitch
    // between lines (issue #656).
    for (index, paragraph) in text_box.paragraphs.iter().enumerate() {
        assert_eq!(
            paragraph.style.space_before,
            Some(0.0),
            "paragraph {index} must state its space before"
        );
        assert_eq!(
            paragraph.style.space_after,
            Some(0.0),
            "paragraph {index} must state its space after"
        );
    }
    assert_eq!(para.runs[0].style.color, Some(Color::new(0xFF, 0, 0)));

    assert_eq!(
        text_box.paragraphs[1].style.alignment,
        Some(Alignment::Center)
    );
    assert_eq!(
        text_box.paragraphs[1]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>(),
        "Line 2"
    );
    assert_eq!(
        text_box.paragraphs[2].style.alignment,
        Some(Alignment::Right)
    );
    assert_eq!(
        text_box.paragraphs[2].runs[0].style.color,
        Some(Color::new(0, 0, 0xFF))
    );
}

// ---------------------------------------------------------------------------
// Embedded charts on chart-only sheets (issue #239)
// ---------------------------------------------------------------------------

#[test]
fn with_chart_renders_embedded_chart() {
    // WithChart.xlsx puts its chart on a sheet with no cells; that sheet was
    // skipped entirely, dropping the chart (same class as #238's image-only
    // sheets, which the fix did not extend to charts).
    let pages = sheet_pages("poi/WithChart.xlsx");
    let total_charts: usize = pages.iter().map(|sp| sp.charts.len()).sum();
    assert!(
        total_charts >= 1,
        "the embedded chart must be extracted, got {total_charts}"
    );
    let chart = pages
        .iter()
        .flat_map(|sp| sp.charts.iter())
        .next()
        .expect("a chart");
    assert!(
        !chart.chart.series.is_empty(),
        "chart must carry its series data"
    );
}

// ---------------------------------------------------------------------------
// Repository workbook — multi-sheet Korean analysis workbook
// ---------------------------------------------------------------------------

/// Ten-sheet Korean workbook describing this repository. Unlike the focused
/// fixtures above it combines the XLSX features that co-occur in real reporting
/// documents on the same printed page: merged banner rows, an Excel table with
/// `tableStyleInfo` stripes, data bars, colour scales, icon sets, hyperlinks,
/// three chart kinds, cell comments, mixed portrait/landscape print setup, and
/// `fitToPage` print scaling.
const REPOSITORY_WORKBOOK_FIXTURE: &str = "office2pdf_repository_workbook.xlsx";

#[test]
fn smoke_repository_workbook_fixture() {
    assert_produces_valid_pdf(REPOSITORY_WORKBOOK_FIXTURE);
}

#[test]
fn structure_repository_workbook_keeps_every_sheet_in_workbook_order() {
    let pages = sheet_pages(REPOSITORY_WORKBOOK_FIXTURE);

    let mut names: Vec<&str> = Vec::new();
    for page in &pages {
        if names.last() != Some(&page.name.as_str()) {
            names.push(page.name.as_str());
        }
    }

    assert_eq!(
        names,
        [
            "00_개요",
            "01_대시보드",
            "02_기능_매트릭스",
            "03_CLI_옵션",
            "04_라이브러리_API",
            "05_모듈_인벤토리",
            "06_릴리스_이력",
            "07_검증_품질게이트",
            "08_의존성",
            "09_리스크_로드맵",
        ],
        "every worksheet must survive parsing, in workbook order"
    );
}

#[test]
fn structure_repository_workbook_preserves_print_orientation_per_sheet() {
    let pages = sheet_pages(REPOSITORY_WORKBOOK_FIXTURE);

    let is_landscape = |sheet: &str| -> bool {
        let page = sheet_page_named(&pages, sheet);
        page.size.width > page.size.height
    };

    // The overview, dashboard, and library-API sheets print portrait; the wide
    // inventory and matrix sheets print landscape.
    assert!(!is_landscape("00_개요"));
    assert!(!is_landscape("01_대시보드"));
    assert!(!is_landscape("04_라이브러리_API"));
    assert!(is_landscape("02_기능_매트릭스"));
    assert!(is_landscape("05_모듈_인벤토리"));
    assert!(is_landscape("09_리스크_로드맵"));
}

#[test]
fn structure_repository_workbook_extracts_every_dashboard_chart_with_data() {
    let pages = sheet_pages(REPOSITORY_WORKBOOK_FIXTURE);

    let charts: Vec<&office2pdf::ir::Chart> = pages
        .iter()
        .filter(|page| page.name == "01_대시보드")
        .flat_map(|page| page.charts.iter().map(|sheet_chart| &sheet_chart.chart))
        .collect();

    assert_eq!(charts.len(), 3, "the dashboard anchors three charts");

    for chart in &charts {
        let title: &str = chart.title.as_deref().unwrap_or_default();
        assert!(
            !chart.categories.is_empty(),
            "chart {title:?} must carry its category labels"
        );
        assert!(
            chart.series.iter().any(|series| !series.values.is_empty()),
            "chart {title:?} must carry its cached series values"
        );
    }
}

/// End-to-end: `&A` reaches the rendered header as the worksheet name.
///
/// `check-boolean.xlsx` declares `<oddHeader>&C&"Times New Roman,Regular"&12&A`.
/// `&A` is the section's only content, so while the code was discarded the
/// section came out empty and the header paragraph was dropped altogether —
/// the sheet printed with no header at all where Excel prints "Sheet1"
/// (issue #690). Its footer, `Page &P`, survives throughout because `&P` was
/// already implemented, and is asserted here to keep the two apart.
///
/// Issue #690 cites `libreoffice/page_scale.xlsx`, which carries the very same
/// header string but has no `<sheetData>` rows. A sheet with no used range
/// takes the `empty_workbook_page` path, which deliberately carries neither
/// header nor footer onto the blank page it prints (issue #632) — so that
/// fixture renders wholly blank and cannot witness this defect either way.
/// This one has the same header and two data rows.
#[test]
fn structure_check_boolean_header_prints_the_sheet_name() {
    let pages = sheet_pages("libreoffice/check-boolean.xlsx");
    let page = pages
        .first()
        .expect("check-boolean has at least one sheet page");
    assert_eq!(page.name, "Sheet1");

    let header = page
        .header
        .as_ref()
        .expect("a header whose only content is &A must still be emitted");
    let header_text: String = header
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.elements.iter())
        .filter_map(|element| match element {
            HFInline::Run(run) => Some(run.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(header_text, "Sheet1");

    let footer = page
        .footer
        .as_ref()
        .expect("check-boolean declares a footer");
    assert!(
        footer
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.elements.iter())
            .any(|element| matches!(element, HFInline::PageNumber(_))),
        "the footer's &P must still resolve"
    );
}

/// A cached formula string keeps the leading spaces `xml:space="preserve"`
/// protects (issue #719).
///
/// Tests rows 44-46 are `t="str"` cells whose cached `<v xml:space="preserve">`
/// carries 1, 2 and 4 leading spaces — the indent a `?` placeholder reserves,
/// which Excel bakes into the cached display text and which cannot be
/// recomputed from the formula.
///
/// Every cell is scanned rather than a fixed column index: this sheet prints
/// headings, so a row-number gutter column is prepended (issue #623) and
/// column A is not `cells[0]`.
#[test]
fn structure_number_format_tests_keeps_preserved_leading_spaces() {
    let pages = sheet_pages("poi/NumberFormatTests.xlsx");
    let texts: Vec<String> = pages
        .iter()
        .filter(|page| page.name == "Tests")
        .flat_map(|page| page.table.rows.iter())
        .flat_map(|row| row.cells.iter())
        .map(table_cell_text)
        .collect();

    for expected in [" 1,234,567", "  1,234,567", "    1,234,567"] {
        assert!(
            texts.iter().any(|text| text == expected),
            "expected a cell of exactly {expected:?}; leading spaces were trimmed"
        );
    }
}

/// A missing typewriter face must retain fixed pitch through the complete
/// XLSX-to-Typst path instead of reaching Typst's proportional body default.
#[test]
fn number_format_tests_emits_a_monospace_fallback_chain() {
    let data = load_fixture("poi/NumberFormatTests.xlsx");
    let (document, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("fixture should parse");
    let source = generate_typst(&document)
        .expect("fixture should generate Typst")
        .source;

    assert!(
        source.contains(
            r#"font: ("Lucida Sans Typewriter", "DejaVu Sans Mono", "Noto Sans Mono", "Liberation Mono", "Cousine")"#,
        ),
        "Lucida Sans Typewriter should keep a monospace fallback chain"
    );
}

/// A cell's leading spaces must survive *rendering*, not just parsing: Typst
/// drops a space that opens a markup line, so the single-space cell above
/// rendered flush left even though its text was intact (issue #752).
#[test]
fn number_format_tests_renders_a_single_leading_space_as_an_indent() {
    let data = load_fixture("poi/NumberFormatTests.xlsx");
    let (document, _warnings) = XlsxParser
        .parse(&data, &ConvertOptions::default())
        .expect("fixture should parse");
    let source = generate_typst(&document)
        .expect("fixture should generate Typst")
        .source;

    // Two- and four-space runs already emitted a code-mode string, which
    // markup cannot collapse; the lone space fell through to a literal.
    for spaces in [" ", "  ", "    "] {
        assert!(
            source.contains(&format!("[#\"{spaces}\";1,234,567]")),
            "a {}-space indent must survive as a code-mode string",
            spaces.len()
        );
    }
}

/// A literal-text number format renders its literal, not the raw number
/// (issue #750).
///
/// `check-boolean.xlsx` styles both cells with `numFmt` 165,
/// `"TRUE";"TRUE";"FALSE"` — sections that carry no numeric placeholder at all.
/// A1 is `t="b"` and resolves through the cell type; A2 is `t="n"` holding 2
/// and must take the format's positive section.
#[test]
fn structure_check_boolean_renders_literal_text_number_format() {
    let pages = sheet_pages("libreoffice/check-boolean.xlsx");
    let texts: Vec<String> = pages
        .iter()
        .flat_map(|page| page.table.rows.iter())
        .flat_map(|row| row.cells.iter())
        .map(table_cell_text)
        .filter(|text| !text.is_empty())
        .collect();

    assert_eq!(
        texts.iter().filter(|text| *text == "TRUE").count(),
        2,
        "both cells carry numFmt 165 and print TRUE; got {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == "2"),
        "the raw number must not leak past the literal format; got {texts:?}"
    );
}

#[test]
fn smoke_merged_row_overflows_page_column() {
    assert_produces_valid_pdf("merged_row_overflows_page_column.xlsx");
}

/// A merged row spanning a sheet wide enough to split horizontally used to keep
/// the whole merge's width as its spill width on every page-column, so its text
/// painted a single line far past the printable edge — off the paper on this
/// fixture, losing that ink entirely (#631).
#[test]
fn structure_merged_row_overflow_clamps_spill_to_its_page_column() {
    let pages = sheet_pages("merged_row_overflows_page_column.xlsx");
    assert!(
        pages.len() >= 2,
        "the sheet is wider than one page and must split into column groups; got {}",
        pages.len()
    );

    for (index, page) in pages.iter().enumerate() {
        let group_width: f64 = page.table.column_widths.iter().sum();
        for row in &page.table.rows {
            for cell in &row.cells {
                let Some(spill) = cell.spill_width else {
                    continue;
                };
                assert!(
                    spill <= group_width + 0.001,
                    "page {index}: spill width {spill}pt exceeds the {group_width}pt \
                     the page-column actually carries",
                );
            }
        }
    }
}

#[test]
fn smoke_spill_reach_print_range() {
    assert_produces_valid_pdf("spill_reach_print_range.xlsx");
}

/// Excel extends a sheet's printed range to every column that unwrapped cell
/// text visibly overflows into, and only to those (issue #718). Probe-measured
/// on `NumberFormatTests.xlsx`: deleting the trailing styled `<col>` run, the
/// row-level `customFormat`, the pane, and the selections each left the extra
/// printed column in place, while deleting the rows whose D-column text paints
/// past the column edge removed it; lengthening that text made Excel print
/// additional columns and horizontal spill pages (14 pages against the
/// workbook's baseline 9).
///
/// The fixture's five sheets vary one factor each around the same grid
/// (Verdana 10; columns 20/18.33/14.66/13 chars; default width 10.6640625):
/// - `SpillOne`: 17-char text in the 13-char last column reaches one column
///   past the used range.
/// - `Wrapped`: the same long text as `LongSpill`, under `wrapText`, never
///   spills.
/// - `Fits`: 9-char text fits its column.
/// - `LongSpill`: 44-char text reaches three columns past the used range.
/// - `Numeric`: a wide number never spills.
#[test]
fn structure_spill_reach_extends_printed_columns() {
    let pages = sheet_pages("spill_reach_print_range.xlsx");
    // LongSpill's extension makes the grid wider than one page, so it splits
    // into two column groups — the same horizontal spill pages the native
    // export emits for an overflow reaching past the printable width.
    assert_eq!(
        sheet_names(&pages),
        vec![
            "SpillOne",
            "Wrapped",
            "Fits",
            "LongSpill",
            "LongSpill",
            "Numeric"
        ],
        "only the reach past the printable width may add pages"
    );

    // Each sheet prints headings, so every page group carries one row-heading
    // column ahead of its spreadsheet columns: 1 + used range (4) + extension.
    let column_counts: Vec<usize> = pages
        .iter()
        .map(|page| page.table.column_widths.len())
        .collect();
    assert_eq!(
        column_counts,
        vec![6, 5, 5, 6, 3, 5],
        "printed columns must extend exactly to the overflow's reach \
         (SpillOne +1, LongSpill +3 split across its column groups) and stay \
         at the used range everywhere else"
    );

    // The extension columns carry no `<col>` record, so they take the sheet's
    // declared default width (10.6640625 chars of Verdana 10 -> ~64pt, the
    // width the native export prints for column E).
    let spill_one_extension: f64 = *pages[0].table.column_widths.last().unwrap();
    assert!(
        (spill_one_extension - 64.0).abs() < 1.5,
        "extension column must take the default column width; got {spill_one_extension}pt"
    );
}

/// The `customFormat` row style paints the printed cells the row extends
/// over, including spill-reached columns holding no cell at all — Excel's
/// export fills the whole A1:E1 band (461pt on `NumberFormatTests.xlsx`,
/// issue #718), not just the populated A1:D1.
#[test]
fn structure_row_custom_format_fill_covers_spill_reached_column() {
    let pages = sheet_pages("spill_reach_print_range.xlsx");
    // rows[0] is the printed column-heading row; rows[1] is spreadsheet row 1,
    // whose customFormat style fills A1:D1 — and must also fill the reached E1.
    let header_cells = &pages[0].table.rows[1].cells;
    assert_eq!(header_cells.len(), 6, "header row spans the extended grid");
    assert_eq!(
        header_cells.last().unwrap().background,
        Some(Color {
            r: 255,
            g: 192,
            b: 0
        }),
        "the row-level customFormat fill must cover the spill-reached column"
    );

    // Rows without a customFormat style leave the extension column unfilled.
    let data_cells = &pages[0].table.rows[2].cells;
    assert_eq!(data_cells.len(), 6);
    assert_eq!(
        data_cells.last().unwrap().background,
        None,
        "plain rows must not inherit a fill in the extension column"
    );
}

/// The KPI tracker's A11 footnote is 99 characters of Arial 9 sitting in a
/// 156pt column, so it paints past the used range — but only as far as the
/// grid it already has. Excel prints the workbook on one page (issue #1054);
/// pricing that line a third too wide walked the printed range three columns
/// past F, which pushed the grid over the 487pt printable width and emitted a
/// second, empty column group.
#[test]
fn structure_kpi_tracker_note_overflow_keeps_the_grid_on_one_page() {
    let pages = sheet_pages("../../golden_mocks/business/sources/xlsx/10_kpi_tracker_en.xlsx");
    assert_eq!(
        sheet_names(&pages),
        vec!["KPI"],
        "the sheet must not split into horizontal column groups"
    );
    // printOptions omits headings, so the page is the A:F used range itself.
    assert_eq!(
        pages[0].table.column_widths.len(),
        6,
        "the printed range stays at the used range A:F, got widths {:?}",
        pages[0].table.column_widths
    );
}

/// `ExcelTables.xlsx` styles its `G1:I4` table `TableStyleLight1`. A native
/// Excel-for-Mac export prints that style as a `#D9D9D9` band over the 1st and
/// 3rd body rows, three full-width black 1pt rules — above the header, under
/// it, and at the table's foot — and a bold header row (issue #1080).
#[test]
fn structure_light1_table_style_bands_rules_and_bolds_its_header() {
    let pages = sheet_pages("ExcelTables.xlsx");
    let rows = &pages[0].table.rows;
    // The table occupies G1:I4, so column G is grid index 6 and its header
    // sits on the first printed row.
    let cell_at = |row: usize, col: usize| -> &TableCell { &rows[row].cells[col] };
    let header = cell_at(0, 6);
    assert_eq!(
        header.content.len(),
        1,
        "G1 must still hold the header label"
    );

    let stripe = Some(Color {
        r: 0xd9,
        g: 0xd9,
        b: 0xd9,
    });
    assert_eq!(cell_at(1, 6).background, stripe, "G2 is the first band");
    assert_eq!(cell_at(2, 6).background, None, "G3 sits between the bands");
    assert_eq!(cell_at(3, 6).background, stripe, "G4 is the second band");
    assert_eq!(
        cell_at(1, 3).background,
        None,
        "column D is outside the table"
    );

    let header_border = header.border.as_ref().expect("G1 carries the two rules");
    for (side, name) in [
        (&header_border.top, "above the header"),
        (&header_border.bottom, "under the header"),
    ] {
        let side = side.as_ref().unwrap_or_else(|| panic!("rule {name}"));
        assert_eq!(side.color, Color { r: 0, g: 0, b: 0 }, "rule {name}");
        assert_eq!(side.width, 1.0, "rule {name} is a 1pt band");
        assert_eq!(side.style, BorderLineStyle::Solid, "rule {name}");
    }
    let foot_border = cell_at(3, 6)
        .border
        .as_ref()
        .expect("G4 carries the foot rule");
    assert!(
        foot_border.bottom.is_some(),
        "the table's last row is ruled at its foot"
    );
    assert!(
        foot_border.top.is_none(),
        "no rule runs between the body rows"
    );
    assert!(
        cell_at(2, 6).border.is_none(),
        "an interior body row carries no rule"
    );

    let header_run = match &header.content[0] {
        Block::Paragraph(paragraph) => &paragraph.runs[0],
        other => panic!("header holds a paragraph, got {other:?}"),
    };
    assert_eq!(
        header_run.style.bold,
        Some(true),
        "the table style prints its header row bold"
    );
}

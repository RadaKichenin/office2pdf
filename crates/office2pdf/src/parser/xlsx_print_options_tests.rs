use super::*;

// `printOptions` follows `sheetData` in CT_Worksheet — the real
// NumberFormatTests fixture writes `<printOptions headings="1"
// gridLines="1"/>` between the sheet data and the page margins.
const PRINTS_GRIDLINES: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>
  <printOptions headings="1" gridLines="1"/>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#;

const NO_PRINT_OPTIONS: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#;

// Excel also writes `printOptions` for centering alone; that must not turn
// gridlines on.
const CENTERED_ONLY: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
  <printOptions horizontalCentered="1"/>
</worksheet>"#;

// CT_Worksheet orders `customSheetViews` before the sheet-level
// `printOptions`, and each custom view nests its own `<printOptions>`
// (CT_CustomSheetView). A first-match scan reads the view's options instead
// of the sheet's.
const CUSTOM_VIEW_THEN_SHEET_LEVEL: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>
  <customSheetViews>
    <customSheetView guid="{3C29A897-4F3B-4A0B-9A5C-2D53E1F1F001}" scale="85">
      <selection activeCell="A1" sqref="A1"/>
      <printOptions horizontalCentered="1"/>
      <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
    </customSheetView>
  </customSheetViews>
  <printOptions headings="1" gridLines="1"/>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#;

const CUSTOM_VIEW_GRIDLINES_ONLY: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>
  <customSheetViews>
    <customSheetView guid="{3C29A897-4F3B-4A0B-9A5C-2D53E1F1F001}" printArea="1">
      <printOptions gridLines="1"/>
      <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
    </customSheetView>
  </customSheetViews>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#;

#[test]
fn reads_grid_lines_after_sheet_data() {
    assert!(worksheet_prints_gridlines(PRINTS_GRIDLINES));
}

#[test]
fn custom_view_print_options_do_not_shadow_the_sheet_level_element() {
    assert!(worksheet_prints_gridlines(CUSTOM_VIEW_THEN_SHEET_LEVEL));
}

#[test]
fn custom_view_grid_lines_do_not_leak_to_the_sheet() {
    assert!(!worksheet_prints_gridlines(CUSTOM_VIEW_GRIDLINES_ONLY));
}

#[test]
fn explicit_grid_lines_set_false_vetoes_grid_lines() {
    // ECMA-376 §18.3.1.70: gridlines print only when `gridLines` AND
    // `gridLinesSet` (default true) are both true.
    let vetoed = PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="1" gridLinesSet="0""#);
    assert!(!worksheet_prints_gridlines(&vetoed));
    let vetoed_spelt = PRINTS_GRIDLINES.replace(
        r#"gridLines="1""#,
        r#"gridLines="true" gridLinesSet="false""#,
    );
    assert!(!worksheet_prints_gridlines(&vetoed_spelt));
    let confirmed =
        PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="1" gridLinesSet="1""#);
    assert!(worksheet_prints_gridlines(&confirmed));
}

#[test]
fn absent_or_unrelated_print_options_stay_off() {
    assert!(!worksheet_prints_gridlines(NO_PRINT_OPTIONS));
    assert!(!worksheet_prints_gridlines(CENTERED_ONLY));
}

#[test]
fn accepts_the_boolean_spelt_out() {
    let spelt = PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="true""#);
    assert!(worksheet_prints_gridlines(&spelt));
    let off = PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="0""#);
    assert!(!worksheet_prints_gridlines(&off));
}

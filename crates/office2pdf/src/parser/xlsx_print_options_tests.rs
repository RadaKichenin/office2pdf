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
// gridlines or headings on.
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

const CUSTOM_VIEW_FLAGS_ONLY: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>
  <customSheetViews>
    <customSheetView guid="{3C29A897-4F3B-4A0B-9A5C-2D53E1F1F001}" printArea="1">
      <printOptions gridLines="1" headings="1"/>
      <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
    </customSheetView>
  </customSheetViews>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</worksheet>"#;

#[test]
fn reads_grid_lines_and_headings_after_sheet_data() {
    let options: SheetPrintOptions = worksheet_print_options(PRINTS_GRIDLINES);
    assert!(options.prints_gridlines);
    assert!(options.prints_headings);
}

#[test]
fn custom_view_print_options_do_not_shadow_the_sheet_level_element() {
    let options: SheetPrintOptions = worksheet_print_options(CUSTOM_VIEW_THEN_SHEET_LEVEL);
    assert!(options.prints_gridlines);
    assert!(options.prints_headings);
}

#[test]
fn custom_view_flags_do_not_leak_to_the_sheet() {
    let options: SheetPrintOptions = worksheet_print_options(CUSTOM_VIEW_FLAGS_ONLY);
    assert!(!options.prints_gridlines);
    assert!(!options.prints_headings);
}

#[test]
fn explicit_grid_lines_set_false_vetoes_grid_lines_but_not_headings() {
    // ECMA-376 §18.3.1.70: gridlines print only when `gridLines` AND
    // `gridLinesSet` (default true) are both true. `headings` has no such
    // conjunction attribute in CT_PrintOptions, so the veto must not touch it.
    let vetoed = PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="1" gridLinesSet="0""#);
    let options: SheetPrintOptions = worksheet_print_options(&vetoed);
    assert!(!options.prints_gridlines);
    assert!(options.prints_headings);

    let vetoed_spelt = PRINTS_GRIDLINES.replace(
        r#"gridLines="1""#,
        r#"gridLines="true" gridLinesSet="false""#,
    );
    assert!(!worksheet_print_options(&vetoed_spelt).prints_gridlines);

    let confirmed =
        PRINTS_GRIDLINES.replace(r#"gridLines="1""#, r#"gridLines="1" gridLinesSet="1""#);
    assert!(worksheet_print_options(&confirmed).prints_gridlines);
}

#[test]
fn absent_print_options_stay_off() {
    assert_eq!(
        worksheet_print_options(NO_PRINT_OPTIONS),
        SheetPrintOptions::default()
    );
}

#[test]
fn horizontal_centering_reads_on_its_own() {
    // `horizontalCentered` centres the printed grid between the margins
    // (issue #1110) and says nothing about gridlines or headings.
    let options: SheetPrintOptions = worksheet_print_options(CENTERED_ONLY);
    assert!(options.centers_horizontally);
    assert!(!options.prints_gridlines);
    assert!(!options.prints_headings);

    let spelt = CENTERED_ONLY.replace(r#"horizontalCentered="1""#, r#"horizontalCentered="true""#);
    assert!(worksheet_print_options(&spelt).centers_horizontally);

    let off = CENTERED_ONLY.replace(r#"horizontalCentered="1""#, r#"horizontalCentered="0""#);
    assert_eq!(worksheet_print_options(&off), SheetPrintOptions::default());

    // Excel's own writer spells every flag out, centering included; a sheet
    // that declares it false must not be centred.
    let all_false = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
  <printOptions headings="false" gridLines="false" gridLinesSet="true" horizontalCentered="false" verticalCentered="false"/>
</worksheet>"#;
    assert_eq!(
        worksheet_print_options(all_false),
        SheetPrintOptions::default()
    );
}

#[test]
fn custom_view_horizontal_centering_does_not_leak_to_the_sheet() {
    // CUSTOM_VIEW_THEN_SHEET_LEVEL centres the saved view, not the sheet.
    let options: SheetPrintOptions = worksheet_print_options(CUSTOM_VIEW_THEN_SHEET_LEVEL);
    assert!(!options.centers_horizontally);
}

#[test]
fn accepts_the_boolean_spelt_out() {
    let spelt = PRINTS_GRIDLINES
        .replace(r#"gridLines="1""#, r#"gridLines="true""#)
        .replace(r#"headings="1""#, r#"headings="true""#);
    let options: SheetPrintOptions = worksheet_print_options(&spelt);
    assert!(options.prints_gridlines);
    assert!(options.prints_headings);

    let off = PRINTS_GRIDLINES
        .replace(r#"gridLines="1""#, r#"gridLines="0""#)
        .replace(r#"headings="1""#, r#"headings="0""#);
    assert_eq!(worksheet_print_options(&off), SheetPrintOptions::default());
}

#[test]
fn headings_and_grid_lines_flag_independently() {
    let headings_only = PRINTS_GRIDLINES.replace(r#"gridLines="1""#, "");
    let options: SheetPrintOptions = worksheet_print_options(&headings_only);
    assert!(!options.prints_gridlines);
    assert!(options.prints_headings);

    let grid_lines_only = PRINTS_GRIDLINES.replace(r#"headings="1""#, "");
    let options: SheetPrintOptions = worksheet_print_options(&grid_lines_only);
    assert!(options.prints_gridlines);
    assert!(!options.prints_headings);
}

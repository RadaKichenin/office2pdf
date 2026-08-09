use super::*;

const FITTING: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetPr><pageSetUpPr fitToPage="1"/></sheetPr>
  <sheetData/>
  <pageSetup paperSize="9" scale="100" fitToWidth="1" fitToHeight="0"/>
</worksheet>"#;

const NOT_FITTING: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetPr><pageSetUpPr/></sheetPr>
  <sheetData/>
  <pageSetup paperSize="9" scale="100" fitToWidth="1" fitToHeight="0"/>
</worksheet>"#;

const NO_SHEET_PR: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
  <pageSetup paperSize="9" fitToWidth="1"/>
</worksheet>"#;

/// The Gantt template from issue #841: `fitToPage` is set and `fitToWidth` is
/// left off the `<pageSetup>` entirely.
const FITTING_WITHOUT_FIT_TO_WIDTH: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetPr><tabColor theme="7"/><pageSetUpPr fitToPage="1"/></sheetPr>
  <sheetData/>
  <pageSetup paperSize="9" scale="48" fitToHeight="0" orientation="landscape"/>
</worksheet>"#;

#[test]
fn reads_the_flag_that_gates_the_fit_attributes() {
    assert_eq!(worksheet_fit_to_width(FITTING), Some(1));
}

#[test]
fn fit_to_width_alone_does_not_ask_excel_to_scale() {
    // Both sheets carry `fitToWidth="1"`; only the first asks to be scaled.
    assert_eq!(worksheet_fit_to_width(NOT_FITTING), None);
    assert_eq!(worksheet_fit_to_width(NO_SHEET_PR), None);
}

#[test]
fn accepts_the_boolean_spelt_out() {
    let spelt = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="true""#);
    assert_eq!(worksheet_fit_to_width(&spelt), Some(1));
    let off = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="0""#);
    assert_eq!(worksheet_fit_to_width(&off), None);
}

/// ECMA-376 defaults `fitToWidth` to 1, so a `fitToPage` sheet that omits the
/// attribute still asks to be scaled onto one page wide (issue #850).
#[test]
fn an_omitted_fit_to_width_defaults_to_one_page() {
    assert_eq!(
        worksheet_fit_to_width(FITTING_WITHOUT_FIT_TO_WIDTH),
        Some(1)
    );
}

/// A declared count is honoured as written, so the default above cannot be a
/// hardcoded 1.
#[test]
fn a_declared_fit_to_width_is_read_as_written() {
    let two = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="2""#);
    assert_eq!(worksheet_fit_to_width(&two), Some(2));
    let three = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="3""#);
    assert_eq!(worksheet_fit_to_width(&three), Some(3));
}

/// `fitToWidth="0"` is Excel's "as many pages wide as it takes" — the width is
/// unconstrained, so nothing is scaled in that direction.
#[test]
fn an_explicit_zero_fit_to_width_leaves_the_width_unbounded() {
    let zero = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="0""#);
    assert_eq!(worksheet_fit_to_width(&zero), Some(0));
}

/// `<pageSetup>` is a sibling that follows `<sheetData>`, so the scan cannot
/// stop at the cells the way the `fitToPage` probe used to.
#[test]
fn reads_page_setup_declared_after_the_cells() {
    let with_cells = FITTING_WITHOUT_FIT_TO_WIDTH.replace(
        "<sheetData/>",
        r#"<sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>"#,
    );
    assert_eq!(worksheet_fit_to_width(&with_cells), Some(1));
}

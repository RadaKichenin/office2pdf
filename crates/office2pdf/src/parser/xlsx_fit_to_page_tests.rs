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

#[test]
fn reads_the_flag_that_gates_the_fit_attributes() {
    assert!(worksheet_fits_to_page(FITTING));
}

#[test]
fn fit_to_width_alone_does_not_ask_excel_to_scale() {
    // Both sheets carry `fitToWidth="1"`; only the first asks to be scaled.
    assert!(!worksheet_fits_to_page(NOT_FITTING));
    assert!(!worksheet_fits_to_page(NO_SHEET_PR));
}

#[test]
fn accepts_the_boolean_spelt_out() {
    let spelt = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="true""#);
    assert!(worksheet_fits_to_page(&spelt));
    let off = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="0""#);
    assert!(!worksheet_fits_to_page(&off));
}

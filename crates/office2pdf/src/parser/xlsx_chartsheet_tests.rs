use super::*;

/// A chartsheet part with whatever print settings the caller wants, in the
/// element order `CT_Chartsheet` fixes.
fn chartsheet_xml(print_settings: &str) -> String {
    format!(
        r#"<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetPr/>
  <sheetViews><sheetView zoomScale="88" workbookViewId="0" zoomToFit="1"/></sheetViews>
  {print_settings}
  <drawing r:id="rId1"/>
</chartsheet>"#
    )
}

#[test]
fn a_chartsheet_stating_nothing_prints_letter_landscape() {
    // `paperSize` unstated means the schema's default of 1, US Letter (issue
    // #717); `orientation` unstated means landscape for a chartsheet.
    let setup = parse_chartsheet_print_setup(&chartsheet_xml(""));
    assert_eq!((setup.size.width, setup.size.height), (792.0, 612.0));
}

#[test]
fn an_explicit_default_orientation_still_prints_landscape() {
    // "default" is the schema's own value for the attribute, so it has to read
    // the same as an absent one.
    let setup =
        parse_chartsheet_print_setup(&chartsheet_xml(r#"<pageSetup orientation="default"/>"#));
    assert_eq!((setup.size.width, setup.size.height), (792.0, 612.0));
}

#[test]
fn a_chartsheet_asking_for_portrait_gets_it() {
    let setup = parse_chartsheet_print_setup(&chartsheet_xml(
        r#"<pageSetup paperSize="9" orientation="portrait"/>"#,
    ));
    assert_eq!((setup.size.width, setup.size.height), (595.28, 841.89));
}

#[test]
fn a_declared_paper_size_turns_on_its_side_with_the_chartsheet() {
    // A5 (code 11) is 419.53 x 595.28 upright.
    let setup = parse_chartsheet_print_setup(&chartsheet_xml(r#"<pageSetup paperSize="11"/>"#));
    assert_eq!((setup.size.width, setup.size.height), (595.28, 419.53));
}

#[test]
fn declared_margins_are_read_in_inches() {
    let setup = parse_chartsheet_print_setup(&chartsheet_xml(
        r#"<pageMargins left="1.5" right="0.5" top="1" bottom="0.25" header="0.3" footer="0.3"/>"#,
    ));
    assert_eq!(
        (
            setup.margins.left,
            setup.margins.right,
            setup.margins.top,
            setup.margins.bottom
        ),
        (108.0, 36.0, 72.0, 18.0)
    );
}

#[test]
fn absent_margins_fall_back_to_excels_defaults() {
    let setup = parse_chartsheet_print_setup(&chartsheet_xml(""));
    assert_eq!(
        (
            setup.margins.left,
            setup.margins.right,
            setup.margins.top,
            setup.margins.bottom
        ),
        (50.4, 50.4, 54.0, 54.0)
    );
}

#[test]
fn only_the_chartsheets_of_a_workbook_are_reported() {
    // `chart_sheet.xlsx` declares Sheet1, Sheet2, Chart1 and Sheet3, of which
    // only Chart1 resolves through a `chartsheet` relationship.
    let data = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/xlsx/chart_sheet.xlsx"),
    )
    .expect("fixture should exist");
    let setups = chartsheet_print_setups(&data);
    assert_eq!(setups.keys().collect::<Vec<&String>>(), vec!["Chart1"]);
}

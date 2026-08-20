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
    assert_eq!(worksheet_fit_to_page(FITTING), Some((1, 0)));
}

#[test]
fn fit_to_width_alone_does_not_ask_excel_to_scale() {
    // Both sheets carry `fitToWidth="1"`; only the first asks to be scaled.
    assert_eq!(worksheet_fit_to_page(NOT_FITTING), None);
    assert_eq!(worksheet_fit_to_page(NO_SHEET_PR), None);
}

#[test]
fn accepts_the_boolean_spelt_out() {
    let spelt = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="true""#);
    assert_eq!(worksheet_fit_to_page(&spelt), Some((1, 0)));
    let off = FITTING.replace(r#"fitToPage="1""#, r#"fitToPage="0""#);
    assert_eq!(worksheet_fit_to_page(&off), None);
}

/// ECMA-376 defaults `fitToWidth` to 1, so a `fitToPage` sheet that omits the
/// attribute still asks to be scaled onto one page wide (issue #850).
#[test]
fn an_omitted_fit_to_width_defaults_to_one_page() {
    assert_eq!(
        worksheet_fit_to_page(FITTING_WITHOUT_FIT_TO_WIDTH),
        Some((1, 0))
    );
}

/// A declared count is honoured as written, so the default above cannot be a
/// hardcoded 1.
#[test]
fn a_declared_fit_to_width_is_read_as_written() {
    let two = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="2""#);
    assert_eq!(worksheet_fit_to_page(&two), Some((2, 0)));
    let three = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="3""#);
    assert_eq!(worksheet_fit_to_page(&three), Some((3, 0)));
}

/// The reported college-budget workbook of issue #1181: `fitToPage` is set and
/// `<pageSetup>` names neither `fitToWidth` nor `fitToHeight`.
const FITTING_WITHOUT_EITHER_BOUND: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetPr codeName="Sheet1"><tabColor theme="4"/><pageSetUpPr fitToPage="1"/></sheetPr>
  <sheetData/>
  <pageSetup paperSize="8" scale="85" orientation="portrait"/>
</worksheet>"#;

/// ECMA-376 §18.3.1.63 defaults `fitToHeight` to 1 exactly as it defaults
/// `fitToWidth`, so a `fitToPage` sheet naming neither asks to be squeezed onto
/// a single page both ways — which is what Excel's own export of the reported
/// workbook does (issue #1181).
#[test]
fn an_omitted_fit_to_height_defaults_to_one_page() {
    assert_eq!(
        worksheet_fit_to_page(FITTING_WITHOUT_EITHER_BOUND),
        Some((1, 1))
    );
}

/// A declared row count is honoured as written, so the default above cannot be
/// a hardcoded 1.
#[test]
fn a_declared_fit_to_height_is_read_as_written() {
    for pages_tall in [2, 3, 7] {
        let declared = FITTING_WITHOUT_EITHER_BOUND.replace(
            r#"scale="85""#,
            &format!(r#"scale="85" fitToHeight="{pages_tall}""#),
        );
        assert_eq!(worksheet_fit_to_page(&declared), Some((1, pages_tall)));
    }
}

/// `fitToHeight="0"` is Excel's "as many pages tall as it takes", and it has to
/// stay distinguishable from the absent attribute above: the audited
/// fit-to-width workbooks all declare it and must keep spilling down the page.
#[test]
fn an_explicit_zero_fit_to_height_leaves_the_height_unbounded() {
    let zero =
        FITTING_WITHOUT_EITHER_BOUND.replace(r#"scale="85""#, r#"scale="85" fitToHeight="0""#);
    assert_eq!(worksheet_fit_to_page(&zero), Some((1, 0)));
}

/// `fitToWidth="0"` is Excel's "as many pages wide as it takes" — the width is
/// unconstrained, so nothing is scaled in that direction.
#[test]
fn an_explicit_zero_fit_to_width_leaves_the_width_unbounded() {
    let zero = FITTING.replace(r#"fitToWidth="1""#, r#"fitToWidth="0""#);
    assert_eq!(worksheet_fit_to_page(&zero), Some((0, 0)));
}

/// `<pageSetup>` is a sibling that follows `<sheetData>`, so the scan cannot
/// stop at the cells the way the `fitToPage` probe used to.
#[test]
fn reads_page_setup_declared_after_the_cells() {
    let with_cells = FITTING_WITHOUT_FIT_TO_WIDTH.replace(
        "<sheetData/>",
        r#"<sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>"#,
    );
    assert_eq!(worksheet_fit_to_page(&with_cells), Some((1, 0)));
}

/// `headerFooter/@scaleWithDoc` defaults to 1, so Excel shrinks the header and
/// footer with a scaled sheet. Only an explicit false opts out (issue #940).
#[test]
fn a_header_footer_scales_with_the_document_unless_it_says_otherwise() {
    assert!(
        worksheet_header_footer_scales_with_doc(FITTING),
        "a part with no <headerFooter> at all still scales"
    );
    let stated = FITTING.replace(
        "<sheetData/>",
        r#"<sheetData/><headerFooter differentFirst="1"><oddFooter>&amp;L&amp;8x</oddFooter></headerFooter>"#,
    );
    assert!(
        worksheet_header_footer_scales_with_doc(&stated),
        "an omitted scaleWithDoc defaults to 1"
    );
    for opted_out in [r#"scaleWithDoc="0""#, r#"scaleWithDoc="false""#] {
        let off = stated.replace("<headerFooter ", &format!("<headerFooter {opted_out} "));
        assert!(
            !worksheet_header_footer_scales_with_doc(&off),
            "an explicit false opts out: {opted_out}"
        );
    }
}

/// A saved custom view nests its own `<headerFooter>`; reading that one would
/// let a view's opt-out shadow the sheet's own setting (issue #940).
#[test]
fn a_custom_views_header_footer_does_not_shadow_the_sheets() {
    let with_view = FITTING.replace(
        "<sheetData/>",
        r#"<sheetData/><customSheetViews><customSheetView guid="{0}"><headerFooter scaleWithDoc="0"/></customSheetView></customSheetViews>"#,
    );
    assert!(worksheet_header_footer_scales_with_doc(&with_view));
}

use std::collections::HashSet;

use super::cond_fmt_raw::{worksheet_has_direct_child, worksheets_where};

/// Worksheet names whose package parts carry no paper-setting state.
///
/// umya-spreadsheet maps both a fully pristine worksheet and an initialised
/// setup with no `paperSize` to code zero. Excel distinguishes them: a sheet
/// with neither sheet-level `<pageSetup>` nor `<pageMargins>` follows the
/// application's current paper, while either element initialises the OOXML
/// paper default. Preserve that provenance from the package before umya
/// collapses it (issue #1382).
pub(crate) fn pristine_paper_sheets(data: &[u8]) -> HashSet<String> {
    worksheets_where(data, worksheet_has_pristine_paper_state)
}

/// Whether one worksheet has neither sheet-level paper-state element.
///
/// Only direct children of `<worksheet>` count. Saved custom views can nest
/// their own page setup, and that view-local state must not change the sheet's
/// paper selection. Malformed parts fail closed to the established Letter
/// fallback instead of silently changing paper.
fn worksheet_has_pristine_paper_state(worksheet_xml: &str) -> bool {
    worksheet_has_direct_child(worksheet_xml, &[b"pageSetup", b"pageMargins"]) == Some(false)
}

#[cfg(test)]
mod tests {
    use super::worksheet_has_pristine_paper_state;

    #[test]
    fn direct_page_state_initialises_paper_but_custom_view_state_does_not() {
        for element in ["<pageMargins/>", "<pageSetup orientation=\"portrait\"/>"] {
            let xml = format!("<worksheet>{element}</worksheet>");
            assert!(!worksheet_has_pristine_paper_state(&xml));
        }

        assert!(worksheet_has_pristine_paper_state(
            "<worksheet><customSheetViews><customSheetView><pageMargins/></customSheetView></customSheetViews></worksheet>"
        ));
    }
}

use std::collections::HashSet;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// Worksheet names whose package parts carry no paper-setting state.
///
/// umya-spreadsheet maps both a fully pristine worksheet and an initialised
/// setup with no `paperSize` to code zero. Excel distinguishes them: a sheet
/// with neither sheet-level `<pageSetup>` nor `<pageMargins>` follows the
/// application's current paper, while either element initialises the OOXML
/// paper default. Preserve that provenance from the package before umya
/// collapses it (issue #1382).
pub(crate) fn pristine_paper_sheets(data: &[u8]) -> HashSet<String> {
    let mut pristine: HashSet<String> = HashSet::new();
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return pristine;
    };
    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return pristine;
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return pristine;
    };

    let relationships = parse_relationships(&relationships_xml);
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        if worksheet_has_pristine_paper_state(&worksheet_xml) {
            pristine.insert(sheet_name);
        }
    }
    pristine
}

/// Whether one worksheet has neither sheet-level paper-state element.
///
/// Only direct children of `<worksheet>` count. Saved custom views can nest
/// their own page setup, and that view-local state must not change the sheet's
/// paper selection. Malformed parts fail closed to the established Letter
/// fallback instead of silently changing paper.
fn worksheet_has_pristine_paper_state(worksheet_xml: &str) -> bool {
    let mut reader = Reader::from_str(worksheet_xml);
    let mut depth: usize = 0;
    let mut saw_worksheet: bool = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) => {
                if depth == 0 && element.local_name().as_ref() == b"worksheet" {
                    saw_worksheet = true;
                } else if depth == 1
                    && matches!(element.local_name().as_ref(), b"pageSetup" | b"pageMargins")
                {
                    return false;
                }
                depth += 1;
            }
            Ok(Event::Empty(ref element)) => {
                if depth == 1
                    && matches!(element.local_name().as_ref(), b"pageSetup" | b"pageMargins")
                {
                    return false;
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) => return saw_worksheet,
            Err(_) => return false,
            _ => {}
        }
    }
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

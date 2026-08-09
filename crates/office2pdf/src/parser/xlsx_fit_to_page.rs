use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// ECMA-376's default for `<pageSetup fitToWidth>`, used when the attribute is
/// absent. Excel omits it from a sheet that fits onto one page wide, which is
/// the most common shape of all (issue #850).
const DEFAULT_FIT_TO_WIDTH: u32 = 1;

/// How many pages wide each sheet asks to be scaled onto, keyed by sheet name.
///
/// A sheet appears only when `<sheetPr><pageSetUpPr fitToPage="1"/>` is set:
/// `fitToWidth` and `fitToHeight` mean nothing on their own, because ECMA-376
/// gates both on that flag and Excel writes `fitToWidth="1"` into sheets that
/// print at 100% simply because the attribute defaults there. Only the pair
/// asks Excel to scale (issue #530).
///
/// The value is `fitToWidth` as declared, or [`DEFAULT_FIT_TO_WIDTH`] when the
/// attribute is absent. Zero is preserved and means "as many pages wide as it
/// takes", leaving the width unconstrained.
///
/// umya-spreadsheet models `<pageSetup>` but not `<sheetPr>`, and it cannot
/// tell an absent `fitToWidth` from an explicit zero — both read back as 0 —
/// so both are read from the archive directly.
pub(crate) fn sheets_fit_to_width(data: &[u8]) -> HashMap<String, u32> {
    let mut fitting: HashMap<String, u32> = HashMap::new();
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return fitting;
    };
    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return fitting;
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return fitting;
    };

    let relationships = parse_relationships(&relationships_xml);
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        if let Some(pages_wide) = worksheet_fit_to_width(&worksheet_xml) {
            fitting.insert(sheet_name, pages_wide);
        }
    }
    fitting
}

/// `fitToWidth` for one worksheet part, or `None` when it does not fit to page.
///
/// `<pageSetUpPr>` precedes `<sheetData>` and `<pageSetup>` follows it, so the
/// whole part is scanned rather than stopping at the cells.
fn worksheet_fit_to_width(worksheet_xml: &str) -> Option<u32> {
    let mut reader = Reader::from_str(worksheet_xml);
    let mut fits_to_page: bool = false;
    let mut declared_pages_wide: Option<u32> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element) | Event::Empty(ref element)) => {
                match element.local_name().as_ref() {
                    b"pageSetUpPr" => {
                        fits_to_page = element.attributes().flatten().any(|attribute| {
                            attribute.key.local_name().as_ref() == b"fitToPage"
                                && matches!(attribute.value.as_ref(), b"1" | b"true")
                        });
                    }
                    b"pageSetup" => {
                        declared_pages_wide = element
                            .attributes()
                            .flatten()
                            .find(|attribute| attribute.key.local_name().as_ref() == b"fitToWidth")
                            .and_then(|attribute| {
                                std::str::from_utf8(attribute.value.as_ref())
                                    .ok()
                                    .and_then(|value| value.trim().parse::<u32>().ok())
                            });
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    fits_to_page.then(|| declared_pages_wide.unwrap_or(DEFAULT_FIT_TO_WIDTH))
}

#[cfg(test)]
#[path = "xlsx_fit_to_page_tests.rs"]
mod tests;

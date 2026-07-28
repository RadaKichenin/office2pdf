use std::collections::HashSet;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// Names of the sheets whose `<sheetPr><pageSetUpPr fitToPage="1"/>` is set.
///
/// `fitToWidth` and `fitToHeight` on `<pageSetup>` mean nothing on their own:
/// ECMA-376 gates both on this flag, and Excel writes `fitToWidth="1"` into
/// sheets that print at 100% simply because the attribute defaults there. Only
/// the pair asks Excel to scale, which is why the flag is read rather than
/// assumed (issue #530).
///
/// umya-spreadsheet models `<pageSetup>` but not `<sheetPr>`, so the flag is
/// read from the archive directly.
pub(crate) fn sheets_fitting_to_page(data: &[u8]) -> HashSet<String> {
    let mut fitting: HashSet<String> = HashSet::new();
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
        if worksheet_fits_to_page(&worksheet_xml) {
            fitting.insert(sheet_name);
        }
    }
    fitting
}

fn worksheet_fits_to_page(worksheet_xml: &str) -> bool {
    let mut reader = Reader::from_str(worksheet_xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element) | Event::Empty(ref element)) => {
                if element.local_name().as_ref() == b"pageSetUpPr" {
                    return element.attributes().flatten().any(|attribute| {
                        attribute.key.local_name().as_ref() == b"fitToPage"
                            && matches!(attribute.value.as_ref(), b"1" | b"true")
                    });
                }
                // `sheetPr` is the first child of `worksheet`; once the cells
                // begin there is no page setup left to find.
                if element.local_name().as_ref() == b"sheetData" {
                    return false;
                }
            }
            Ok(Event::Eof) | Err(_) => return false,
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "xlsx_fit_to_page_tests.rs"]
mod tests;

use std::collections::HashSet;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// Names of the sheets whose `<printOptions gridLines="1"/>` is set.
///
/// The flag strictly gates printed gridlines: native Excel GT of a sheet
/// without it contains zero gridline primitives, while a flagged sheet rules
/// every cell boundary of the printed range (issue #622). umya-spreadsheet's
/// `PrintOptions` models only the centering attributes, so the flag is read
/// from the archive directly, like `sheets_fitting_to_page`.
pub(crate) fn sheets_printing_gridlines(data: &[u8]) -> HashSet<String> {
    let mut printing: HashSet<String> = HashSet::new();
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return printing;
    };
    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return printing;
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return printing;
    };

    let relationships = parse_relationships(&relationships_xml);
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        if worksheet_prints_gridlines(&worksheet_xml) {
            printing.insert(sheet_name);
        }
    }
    printing
}

/// Whether the worksheet's `<printOptions>` asks for printed gridlines.
///
/// Unlike `<pageSetUpPr>`, `<printOptions>` follows `<sheetData>` in
/// CT_Worksheet, so the scan must run to the end of the document. The
/// `<customSheetViews>` subtree is skipped: CT_Worksheet places it before the
/// sheet-level `<printOptions>`, and each CT_CustomSheetView nests its own
/// `<printOptions>`, which describes that saved view only — a first-match
/// scan would read the view's options and shadow the sheet's.
fn worksheet_prints_gridlines(worksheet_xml: &str) -> bool {
    let mut reader = Reader::from_str(worksheet_xml);
    // Element depth inside the skipped `<customSheetViews>` subtree; 0 means
    // the scan is at sheet level.
    let mut skipped_subtree_depth: usize = 0;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) => {
                if skipped_subtree_depth > 0 {
                    skipped_subtree_depth += 1;
                } else if element.local_name().as_ref() == b"customSheetViews" {
                    skipped_subtree_depth = 1;
                } else if element.local_name().as_ref() == b"printOptions" {
                    return print_options_request_gridlines(element);
                }
            }
            Ok(Event::Empty(ref element)) => {
                if skipped_subtree_depth == 0 && element.local_name().as_ref() == b"printOptions" {
                    return print_options_request_gridlines(element);
                }
            }
            Ok(Event::End(_)) => {
                skipped_subtree_depth = skipped_subtree_depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => return false,
            _ => {}
        }
    }
}

/// ECMA-376 §18.3.1.70: gridlines print only when `gridLines` and
/// `gridLinesSet` (default true) are both true — an explicit
/// `gridLinesSet="0"` vetoes `gridLines="1"`.
fn print_options_request_gridlines(element: &BytesStart<'_>) -> bool {
    let mut grid_lines: bool = false;
    let mut grid_lines_set: bool = true;
    for attribute in element.attributes().flatten() {
        let is_on: bool = matches!(attribute.value.as_ref(), b"1" | b"true");
        match attribute.key.local_name().as_ref() {
            b"gridLines" => grid_lines = is_on,
            b"gridLinesSet" => grid_lines_set = is_on,
            _ => {}
        }
    }
    grid_lines && grid_lines_set
}

#[cfg(test)]
#[path = "xlsx_print_options_tests.rs"]
mod tests;

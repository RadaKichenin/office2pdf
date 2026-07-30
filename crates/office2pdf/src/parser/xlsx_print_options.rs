use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// Print flags one worksheet's `<printOptions>` requests (ECMA-376
/// §18.3.1.70).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SheetPrintOptions {
    /// `gridLines` ∧ `gridLinesSet`: print the gridline hairline on every
    /// cell boundary of the printed range (issue #622).
    pub(crate) prints_gridlines: bool,
    /// `headings`: print the row-number gutter and column-letter strip on
    /// every page (issue #623). Unlike `gridLines`, the spec defines no
    /// conjunction attribute for headings — CT_PrintOptions has no
    /// `headingsSet`.
    pub(crate) prints_headings: bool,
}

/// Per-sheet `<printOptions>` flags, keyed by sheet name.
///
/// The flags strictly gate printing: native Excel GT of a sheet without
/// them contains zero gridline/heading primitives, while a flagged sheet
/// rules every cell boundary (issue #622) and prints the heading gutter and
/// strip on every page (issue #623). umya-spreadsheet's `PrintOptions`
/// models only the centering attributes, so the flags are read from the
/// archive directly, like `sheets_fitting_to_page`.
pub(crate) fn sheets_print_options(data: &[u8]) -> HashMap<String, SheetPrintOptions> {
    let mut options_by_sheet: HashMap<String, SheetPrintOptions> = HashMap::new();
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return options_by_sheet;
    };
    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return options_by_sheet;
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return options_by_sheet;
    };

    let relationships = parse_relationships(&relationships_xml);
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        let sheet_options: SheetPrintOptions = worksheet_print_options(&worksheet_xml);
        if sheet_options != SheetPrintOptions::default() {
            options_by_sheet.insert(sheet_name, sheet_options);
        }
    }
    options_by_sheet
}

/// The flags the worksheet's own `<printOptions>` requests.
///
/// Unlike `<pageSetUpPr>`, `<printOptions>` follows `<sheetData>` in
/// CT_Worksheet, so the scan must run to the end of the document. The
/// `<customSheetViews>` subtree is skipped: CT_Worksheet places it before the
/// sheet-level `<printOptions>`, and each CT_CustomSheetView nests its own
/// `<printOptions>`, which describes that saved view only — a first-match
/// scan would read the view's options and shadow the sheet's.
fn worksheet_print_options(worksheet_xml: &str) -> SheetPrintOptions {
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
                    return parse_print_options_element(element);
                }
            }
            Ok(Event::Empty(ref element)) => {
                if skipped_subtree_depth == 0 && element.local_name().as_ref() == b"printOptions" {
                    return parse_print_options_element(element);
                }
            }
            Ok(Event::End(_)) => {
                skipped_subtree_depth = skipped_subtree_depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => return SheetPrintOptions::default(),
            _ => {}
        }
    }
}

/// ECMA-376 §18.3.1.70: gridlines print only when `gridLines` and
/// `gridLinesSet` (default true) are both true — an explicit
/// `gridLinesSet="0"` vetoes `gridLines="1"`. `headings` stands alone.
fn parse_print_options_element(element: &BytesStart<'_>) -> SheetPrintOptions {
    let mut grid_lines: bool = false;
    let mut grid_lines_set: bool = true;
    let mut headings: bool = false;
    for attribute in element.attributes().flatten() {
        let is_on: bool = matches!(attribute.value.as_ref(), b"1" | b"true");
        match attribute.key.local_name().as_ref() {
            b"gridLines" => grid_lines = is_on,
            b"gridLinesSet" => grid_lines_set = is_on,
            b"headings" => headings = is_on,
            _ => {}
        }
    }
    SheetPrintOptions {
        prints_gridlines: grid_lines && grid_lines_set,
        prints_headings: headings,
    }
}

#[cfg(test)]
#[path = "xlsx_print_options_tests.rs"]
mod tests;

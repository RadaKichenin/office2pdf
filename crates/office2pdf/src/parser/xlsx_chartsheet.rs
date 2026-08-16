//! Print geometry of a chartsheet — a sheet that is one full-page chart.
//!
//! ECMA-376 gives `CT_Chartsheet` no cells at all: it carries print settings
//! and a `<drawing>` and nothing else. Excel prints it as a page of its own,
//! so the paper and the margins have to come from the chartsheet part rather
//! than from the worksheet stub umya-spreadsheet builds for it, which records
//! neither (issue #1099).

use std::collections::{HashMap, HashSet};

use crate::ir::{Margins, PageSize};

use super::xlsx_drawing::{
    parse_rels_by_type, parse_rels_targets, parse_workbook_sheet_rids, read_zip_entry_string,
    resolve_relative_xl_path,
};

/// The paper one chartsheet prints on.
pub(super) struct ChartsheetPrintSetup {
    pub(super) size: PageSize,
    pub(super) margins: Margins,
}

/// Every chartsheet in the workbook, keyed by the name `xl/workbook.xml` gives
/// it — the same key the drawing maps use.
pub(super) fn chartsheet_print_setups(data: &[u8]) -> HashMap<String, ChartsheetPrintSetup> {
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return HashMap::new();
    };

    let workbook_xml: String = read_zip_entry_string(&mut archive, "xl/workbook.xml");
    let workbook_rels_xml: String =
        read_zip_entry_string(&mut archive, "xl/_rels/workbook.xml.rels");
    let rid_to_target: HashMap<String, String> = parse_rels_targets(&workbook_rels_xml);
    // A sheet is a chartsheet by its relationship type, not by where its part
    // happens to sit: `xl/workbook.xml` states only a name and an rId.
    let chartsheet_paths: HashSet<String> = parse_rels_by_type(&workbook_rels_xml, "chartsheet")
        .iter()
        .map(|target| resolve_relative_xl_path("xl", target))
        .collect();

    let mut result: HashMap<String, ChartsheetPrintSetup> = HashMap::new();
    for (sheet_name, sheet_rid) in parse_workbook_sheet_rids(&workbook_xml) {
        let Some(sheet_target) = rid_to_target.get(&sheet_rid) else {
            continue;
        };
        let sheet_path: String = resolve_relative_xl_path("xl", sheet_target);
        if !chartsheet_paths.contains(&sheet_path) {
            continue;
        }
        let chartsheet_xml: String = read_zip_entry_string(&mut archive, &sheet_path);
        result.insert(sheet_name, parse_chartsheet_print_setup(&chartsheet_xml));
    }
    result
}

/// Read one chartsheet part's `<pageSetup>` and `<pageMargins>`.
///
/// A chartsheet that states neither still prints, so both fall back to what
/// Excel does with an unstated setting.
pub(super) fn parse_chartsheet_print_setup(xml: &str) -> ChartsheetPrintSetup {
    let mut paper_size: u32 = 0;
    // `CT_CsPageSetup/@orientation` defaults to "default", and Excel resolves
    // that to landscape for a chartsheet. Measured on Excel for Mac 16.100:
    // `tests/fixtures/xlsx/any_sheets.xlsx` declares no `<pageSetup>` in
    // either sheet, and its chartsheet exports 842x595 where its worksheet
    // exports 595x842 on the same machine and printer.
    let mut landscape: bool = true;
    let mut margins: Margins = super::DEFAULT_PRINT_MARGINS;

    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(ref element))
            | Ok(quick_xml::events::Event::Empty(ref element)) => {
                match element.local_name().as_ref() {
                    b"pageSetup" => {
                        for attr in element.attributes().flatten() {
                            let Ok(value) = attr.unescape_value() else {
                                continue;
                            };
                            match attr.key.local_name().as_ref() {
                                b"paperSize" => paper_size = value.parse().unwrap_or(0),
                                b"orientation" => landscape = value.as_ref() != "portrait",
                                _ => {}
                            }
                        }
                    }
                    b"pageMargins" => {
                        for attr in element.attributes().flatten() {
                            let Some(points) = attr
                                .unescape_value()
                                .ok()
                                .and_then(|value| value.parse::<f64>().ok())
                                .map(|inches| inches * 72.0)
                            else {
                                continue;
                            };
                            match attr.key.local_name().as_ref() {
                                b"left" => margins.left = points,
                                b"right" => margins.right = points,
                                b"top" => margins.top = points,
                                b"bottom" => margins.bottom = points,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let portrait: PageSize = super::worksheet_paper_size(paper_size);
    let size: PageSize = if landscape {
        PageSize {
            width: portrait.height,
            height: portrait.width,
        }
    } else {
        portrait
    };
    ChartsheetPrintSetup { size, margins }
}

#[cfg(test)]
#[path = "xlsx_chartsheet_tests.rs"]
mod tests;

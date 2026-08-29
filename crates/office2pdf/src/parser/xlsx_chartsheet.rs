//! Print geometry of a chartsheet — a sheet that is one chart and nothing
//! else.
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

/// Where a chartsheet's chart prints, relative to the printable area's
/// top-left corner.
pub(super) struct ChartsheetChartBox {
    pub(super) x_offset_pt: f64,
    pub(super) y_offset_pt: f64,
    pub(super) width: f64,
    pub(super) height: f64,
}

/// The deterministic gap Excel leaves between a printed chartsheet's near
/// edges and the printable area, in points. The far edges reuse it as the
/// stable page-only estimate described by [`printed_chart_box`].
///
/// Measured on 58 Excel for Mac 16.100 exports of the `Chart` sheet of
/// `tests/fixtures/xlsx/any_sheets.xlsx` — four papers, both orientations and
/// each margin swept in 0.01in steps — reading the chart area's `fill_path`
/// out of `mutool draw -F trace` (issue #1147).
const CHART_INSET_PT: f64 = 4.0;

/// The box a chartsheet's chart prints in.
///
/// Excel does not fill the printable area with it. The near edges are exact:
/// every one of the 58 exports starts the chart at `floor(margin) + 4`, the
/// margin snapping to a whole point first exactly as a worksheet's does
/// (issue #1191), so a 0.7in margin of 50.4pt puts the chart's left edge on
/// 54 rather than on 50.4.
///
/// The far edges only average that 4pt. The original 58-export sweep lands on
/// Excel's internal grid rather than following a constant page inset: 0.5in
/// margins on A4 landscape leave 10.19pt of width over where Letter portrait
/// leaves 1.54pt. This converter deliberately keeps that unexplained residual
/// as the page-only model's error term (issue #1221).
///
/// The far edges consequently take the near edges' exact 4pt inset on the
/// whole point: a deterministic, symmetric page rule. Against the sweep, that
/// sizes each axis 0.89pt out on average and 6.19pt at worst — 0.44pt per edge,
/// since the near ones are exact — where filling the printable area exactly
/// was 8.84pt out per axis. A fitted 5pt far inset improves only that sweep, so
/// it is not a general rule.
///
/// A margin Excel cannot print into is not modelled: a chartsheet asking for
/// zero margins exported against the printer's hardware minimum instead
/// (19pt and 18pt on the measured machine), which a PDF has no equivalent of.
pub(super) fn printed_chart_box(setup: &ChartsheetPrintSetup) -> ChartsheetChartBox {
    let left: f64 = setup.margins.left.floor() + CHART_INSET_PT;
    let top: f64 = setup.margins.top.floor() + CHART_INSET_PT;
    let right: f64 = (setup.size.width - setup.margins.right).floor() - CHART_INSET_PT;
    let bottom: f64 = (setup.size.height - setup.margins.bottom).floor() - CHART_INSET_PT;
    ChartsheetChartBox {
        x_offset_pt: left - setup.margins.left,
        y_offset_pt: top - setup.margins.top,
        width: (right - left).max(0.0),
        height: (bottom - top).max(0.0),
    }
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

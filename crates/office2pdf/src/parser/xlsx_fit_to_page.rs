use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// ECMA-376's default for `<pageSetup fitToWidth>` and `<pageSetup
/// fitToHeight>`, used when either attribute is absent (§18.3.1.63).
///
/// Excel omits `fitToWidth` from a sheet that fits onto one page wide, which
/// is the most common shape of all (issue #850). A sheet that omits
/// `fitToHeight` is asking for the same bound in the row direction, and Excel
/// honours it: the reported college-budget workbook names neither attribute
/// and its native export is one A3 page (issue #1181).
const DEFAULT_FIT_TO_PAGES: u32 = 1;

/// What one worksheet's page setup asks for when it fits to page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SheetFitToPage {
    /// `fitToWidth` as declared, or [`DEFAULT_FIT_TO_PAGES`] when absent. Zero
    /// means "as many pages wide as it takes", leaving the width unconstrained.
    pub(crate) pages_wide: u32,
    /// `fitToHeight` as declared, or [`DEFAULT_FIT_TO_PAGES`] when absent.
    /// Zero means "as many pages tall as it takes", leaving the height
    /// unconstrained — the shape the audited fit-to-width workbooks all use.
    pub(crate) pages_tall: u32,
    /// `headerFooter/@scaleWithDoc`, which defaults to `1` — Excel shrinks the
    /// header and footer with the sheet unless the file opts out
    /// (ECMA-376 §18.3.1.46, issue #940).
    pub(crate) header_footer_scales_with_doc: bool,
}

/// What each sheet's page setup asks for, keyed by sheet name.
///
/// A sheet appears only when `<sheetPr><pageSetUpPr fitToPage="1"/>` is set:
/// `fitToWidth` and `fitToHeight` mean nothing on their own, because ECMA-376
/// gates both on that flag and Excel writes `fitToWidth="1"` into sheets that
/// print at 100% simply because the attribute defaults there. Only the pair
/// asks Excel to scale (issue #530).
///
/// umya-spreadsheet models `<pageSetup>` but not `<sheetPr>`, and it cannot
/// tell an absent `fitToWidth` or `fitToHeight` from an explicit zero — both
/// read back as 0 — so all three are read from the archive directly.
/// `<headerFooter>`'s
/// `scaleWithDoc` is read in the same pass for the same reason: the struct
/// umya exposes carries only the section strings.
pub(crate) fn sheets_fit_to_page(data: &[u8]) -> HashMap<String, SheetFitToPage> {
    let mut fitting: HashMap<String, SheetFitToPage> = HashMap::new();
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
        if let Some((pages_wide, pages_tall)) = worksheet_fit_to_page(&worksheet_xml) {
            fitting.insert(
                sheet_name,
                SheetFitToPage {
                    pages_wide,
                    pages_tall,
                    header_footer_scales_with_doc: worksheet_header_footer_scales_with_doc(
                        &worksheet_xml,
                    ),
                },
            );
        }
    }
    fitting
}

/// `(fitToWidth, fitToHeight)` for one worksheet part, or `None` when it does
/// not fit to page.
///
/// `<pageSetUpPr>` precedes `<sheetData>` and `<pageSetup>` follows it, so the
/// whole part is scanned rather than stopping at the cells.
fn worksheet_fit_to_page(worksheet_xml: &str) -> Option<(u32, u32)> {
    let mut reader = Reader::from_str(worksheet_xml);
    let mut fits_to_page: bool = false;
    let mut declared_pages_wide: Option<u32> = None;
    let mut declared_pages_tall: Option<u32> = None;
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
                        declared_pages_wide = page_count_attribute(element, b"fitToWidth");
                        declared_pages_tall = page_count_attribute(element, b"fitToHeight");
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    fits_to_page.then(|| {
        (
            declared_pages_wide.unwrap_or(DEFAULT_FIT_TO_PAGES),
            declared_pages_tall.unwrap_or(DEFAULT_FIT_TO_PAGES),
        )
    })
}

/// One `<pageSetup>` page-count attribute, or `None` when it is absent or not
/// a number. An absent attribute has to stay distinguishable from a declared
/// zero: zero leaves that direction unconstrained where absence asks for one
/// page.
fn page_count_attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Option<u32> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.local_name().as_ref() == name)
        .and_then(|attribute| {
            std::str::from_utf8(attribute.value.as_ref())
                .ok()
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
}

/// Whether the worksheet's `<headerFooter>` scales with the sheet.
///
/// `scaleWithDoc` defaults to `1`, so a part with no `<headerFooter>` at all —
/// or one that omits the attribute — scales. Only an explicit false opts out.
/// The `<customSheetViews>` subtree is skipped for the same reason
/// `worksheet_print_options` skips it: each CT_CustomSheetView nests its own
/// `<headerFooter>`, describing that saved view rather than the sheet.
fn worksheet_header_footer_scales_with_doc(worksheet_xml: &str) -> bool {
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
                } else if element.local_name().as_ref() == b"headerFooter" {
                    return scales_with_doc(element);
                }
            }
            Ok(Event::Empty(ref element)) => {
                if skipped_subtree_depth == 0 && element.local_name().as_ref() == b"headerFooter" {
                    return scales_with_doc(element);
                }
            }
            Ok(Event::End(_)) => {
                skipped_subtree_depth = skipped_subtree_depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    true
}

/// `scaleWithDoc` on a `<headerFooter>` element: true unless explicitly false.
fn scales_with_doc(element: &quick_xml::events::BytesStart<'_>) -> bool {
    !element.attributes().flatten().any(|attribute| {
        attribute.key.local_name().as_ref() == b"scaleWithDoc"
            && matches!(attribute.value.as_ref(), b"0" | b"false")
    })
}

#[cfg(test)]
#[path = "xlsx_fit_to_page_tests.rs"]
mod tests;

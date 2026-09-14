//! Which worksheets write a `<sheetFormatPr>` element at all.
//!
//! umya-spreadsheet hands every worksheet a `SheetFormatProperties`, so the
//! parsed model cannot tell a sheet that declared the element without
//! `baseColWidth` from one that never wrote the element. Excel for Mac
//! prints them differently: the absent element prices default columns at
//! the application's own 10-character base, the present one at the ECMA
//! 8-character base (issue #1656). Preserve that provenance from the package
//! before umya collapses it.

use std::collections::HashSet;

use super::cond_fmt_raw::{worksheet_has_direct_child, worksheets_where};

/// Worksheet names whose package parts have no sheet-level `<sheetFormatPr>`.
///
/// A malformed part is not reported, so it keeps the ECMA base the
/// `baseColWidth` attribute model was measured on.
pub(crate) fn sheets_without_format_properties(data: &[u8]) -> HashSet<String> {
    worksheets_where(data, worksheet_lacks_format_properties)
}

fn worksheet_lacks_format_properties(worksheet_xml: &str) -> bool {
    worksheet_has_direct_child(worksheet_xml, &[b"sheetFormatPr"]) == Some(false)
}

#[cfg(test)]
mod tests {
    use super::worksheet_lacks_format_properties;

    #[test]
    fn a_worksheet_that_never_wrote_the_element_lacks_it() {
        assert!(worksheet_lacks_format_properties(
            r#"<worksheet><dimension ref="A1:E101"/><sheetViews><sheetView workbookViewId="0"/></sheetViews><sheetData/></worksheet>"#
        ));
    }

    #[test]
    fn a_declared_element_counts_whether_empty_or_attribute_less() {
        for element in [
            r#"<sheetFormatPr defaultRowHeight="15"/>"#,
            r#"<sheetFormatPr baseColWidth="10" defaultRowHeight="15"/>"#,
            "<sheetFormatPr></sheetFormatPr>",
        ] {
            let xml =
                format!("<worksheet><dimension ref=\"A1\"/>{element}<sheetData/></worksheet>");
            assert!(
                !worksheet_lacks_format_properties(&xml),
                "{element} declares the element"
            );
        }
    }

    #[test]
    fn a_malformed_part_keeps_the_declared_element_model() {
        assert!(!worksheet_lacks_format_properties(
            "<worksheet><sheetData><row r=\"1\"></worksheet>"
        ));
        assert!(!worksheet_lacks_format_properties("<sheetData/>"));
    }
}

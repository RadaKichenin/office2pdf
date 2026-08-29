//! Printed points reserved by worksheet row-boundary flags.
//!
//! The crates.io v2 release of umya-spreadsheet reads `thickBot` but drops
//! `thickTop`, so the two flags are read together from the raw worksheet XML.
//! This keeps the published crate on the same dependency API as its packaged
//! build while preserving both sides of Excel's printed-boundary rule
//! (issue #1228).

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};

/// Extra printed points by 1-indexed row number.
pub(crate) type RowBoundaryPoints = HashMap<u32, u8>;

/// Every sheet's row-boundary points, keyed by worksheet name.
pub(crate) type SheetRowBoundaryPoints = HashMap<String, RowBoundaryPoints>;

fn attr_value(reader: &Reader<&[u8]>, element: &BytesStart<'_>, name: &[u8]) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.local_name().as_ref() == name)
        .and_then(|attribute| {
            attribute
                .decode_and_unescape_value(reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        })
}

fn attr_is_true(reader: &Reader<&[u8]>, element: &BytesStart<'_>, name: &[u8]) -> bool {
    attr_value(reader, element, name)
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

/// Parse the extra printed points carried by one worksheet's `<row>` flags.
///
/// A non-custom row contributes one point to its own track for `thickTop` and
/// one point to the following row's track for `thickBot`. A custom row's
/// declared height already includes both boundaries, so its flags contribute
/// nothing further.
fn parse_worksheet_row_boundary_points(xml: &str) -> RowBoundaryPoints {
    let mut points: RowBoundaryPoints = HashMap::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if element.local_name().as_ref() == b"row" =>
            {
                let Some(row_idx) =
                    attr_value(&reader, &element, b"r").and_then(|value| value.parse::<u32>().ok())
                else {
                    continue;
                };
                if attr_is_true(&reader, &element, b"customHeight") {
                    continue;
                }
                if attr_is_true(&reader, &element, b"thickTop") {
                    *points.entry(row_idx).or_default() += 1;
                }
                if attr_is_true(&reader, &element, b"thickBot")
                    && let Some(next_row_idx) = row_idx.checked_add(1)
                {
                    *points.entry(next_row_idx).or_default() += 1;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    points
}

/// Extract every worksheet's row-boundary points from an XLSX package.
pub(crate) fn extract_row_boundary_points(data: &[u8]) -> SheetRowBoundaryPoints {
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return HashMap::new();
    };
    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return HashMap::new();
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return HashMap::new();
    };
    let relationships = parse_relationships(&relationships_xml);

    let mut result: SheetRowBoundaryPoints = HashMap::new();
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        let points = parse_worksheet_row_boundary_points(&worksheet_xml);
        if !points.is_empty() {
            result.insert(sheet_name, points);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_flags_reserve_current_top_and_previous_bottom() {
        let points = parse_worksheet_row_boundary_points(
            r#"<worksheet><sheetData>
                <row r="1" thickTop="1" thickBot="1"/>
                <row r="2" thickTop="true"/>
                <row r="3"/>
            </sheetData></worksheet>"#,
        );

        assert_eq!(points, HashMap::from([(1, 1), (2, 2)]));
    }

    #[test]
    fn custom_height_flags_add_no_second_reservation() {
        let points = parse_worksheet_row_boundary_points(
            r#"<worksheet><sheetData>
                <row r="1" customHeight="1" thickTop="1" thickBot="1"/>
                <row r="2" customHeight="true" thickTop="true"/>
            </sheetData></worksheet>"#,
        );

        assert!(points.is_empty());
    }
}

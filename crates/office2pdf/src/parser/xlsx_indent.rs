//! Cell alignment indent levels, read straight from the package.
//!
//! umya-spreadsheet's `Alignment` models `horizontal`, `vertical`, `wrapText`
//! and `textRotation` only — it never reads `indent`, so the level is taken
//! here from `xl/styles.xml` and joined to each cell through the `s` index its
//! worksheet writes (issue #1109).
//!
//! Two semantics of the join are probe-measured on native Excel for Mac, both
//! against the same one-cell workbook re-exported per variant:
//!
//! - `applyAlignment="false"` beside an indent does **not** suppress it. The
//!   reported workbook of #1068 is written that way throughout, its default
//!   `cellXfs[0]` included.
//! - A `cellXfs` entry with no `<alignment>` of its own does **not** inherit
//!   the indent of the `cellStyleXfs` entry its `xfId` names; that cell prints
//!   flush.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::cond_fmt_raw::{
    parse_relationships, parse_sheet_relationships, read_zip_text, worksheet_path,
};
use super::xlsx_cells::{CellPos, parse_cell_ref};

/// Indent levels of one sheet's cells, keyed by (column, row), both 1-indexed.
/// Only cells that carry a level of their own are stored.
pub(crate) type CellIndentLevels = HashMap<CellPos, u32>;

/// Every sheet's indent levels, keyed by worksheet name.
pub(crate) type SheetCellIndents = HashMap<String, CellIndentLevels>;

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

fn attr_number<T: std::str::FromStr>(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    name: &[u8],
) -> Option<T> {
    attr_value(reader, element, name).and_then(|value| value.parse::<T>().ok())
}

/// The indent level of every `cellXfs` entry, by index.
///
/// `cellStyleXfs` carries the same element names ahead of it in the part, so
/// the walk only records entries inside `<cellXfs>`.
fn parse_style_indents(xml: &str) -> Vec<u32> {
    let mut indents: Vec<u32> = Vec::new();
    let mut in_cell_xfs = false;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) if element.local_name().as_ref() == b"cellXfs" => {
                in_cell_xfs = true;
            }
            Ok(Event::End(element)) if element.local_name().as_ref() == b"cellXfs" => break,
            Ok(Event::Start(element) | Event::Empty(element))
                if in_cell_xfs && element.local_name().as_ref() == b"xf" =>
            {
                indents.push(0);
            }
            Ok(Event::Start(element) | Event::Empty(element))
                if in_cell_xfs && element.local_name().as_ref() == b"alignment" =>
            {
                if let (Some(entry), Some(indent)) = (
                    indents.last_mut(),
                    attr_number::<u32>(&reader, &element, b"indent"),
                ) {
                    *entry = indent;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    indents
}

/// A `<col>` run's style, over the column numbers it covers.
struct ColumnStyleRun {
    first_col: u32,
    last_col: u32,
    style: usize,
}

/// The indent level of every cell in one worksheet that has one.
///
/// A `<c>` with no `s` of its own resolves through its row's format and then
/// its column's, which is how a whole indented band is written when every
/// cell in it shares one style.
fn parse_worksheet_indents(xml: &str, xf_indents: &[u32]) -> CellIndentLevels {
    let mut levels: CellIndentLevels = HashMap::new();
    let mut column_runs: Vec<ColumnStyleRun> = Vec::new();
    let mut row_style: Option<usize> = None;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if element.local_name().as_ref() == b"col" =>
            {
                if let (Some(first_col), Some(last_col), Some(style)) = (
                    attr_number::<u32>(&reader, &element, b"min"),
                    attr_number::<u32>(&reader, &element, b"max"),
                    attr_number::<usize>(&reader, &element, b"style"),
                ) {
                    column_runs.push(ColumnStyleRun {
                        first_col,
                        last_col,
                        style,
                    });
                }
            }
            Ok(Event::Start(element) | Event::Empty(element))
                if element.local_name().as_ref() == b"row" =>
            {
                // ECMA-376 §18.3.1.73: the row's `s` only formats its cells
                // when `customFormat` says so.
                row_style = attr_value(&reader, &element, b"customFormat")
                    .filter(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                    .and_then(|_| attr_number::<usize>(&reader, &element, b"s"));
            }
            Ok(Event::End(element)) if element.local_name().as_ref() == b"row" => {
                row_style = None;
            }
            Ok(Event::Start(element) | Event::Empty(element))
                if element.local_name().as_ref() == b"c" =>
            {
                let Some(position) = attr_value(&reader, &element, b"r")
                    .and_then(|reference| parse_cell_ref(&reference))
                else {
                    continue;
                };
                let style: Option<usize> = attr_number::<usize>(&reader, &element, b"s")
                    .or(row_style)
                    .or_else(|| {
                        column_runs
                            .iter()
                            .find(|run| (run.first_col..=run.last_col).contains(&position.0))
                            .map(|run| run.style)
                    });
                let indent: u32 = style
                    .and_then(|index| xf_indents.get(index).copied())
                    .unwrap_or(0);
                if indent > 0 {
                    levels.insert(position, indent);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    levels
}

/// Every sheet's cell indent levels, keyed by worksheet name so the result
/// joins to umya's sheet collection.
pub(crate) fn extract_cell_indents(data: &[u8]) -> SheetCellIndents {
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return HashMap::new();
    };
    let Some(styles_xml) = read_zip_text(&mut archive, "xl/styles.xml") else {
        return HashMap::new();
    };
    let xf_indents: Vec<u32> = parse_style_indents(&styles_xml);
    if xf_indents.iter().all(|indent| *indent == 0) {
        return HashMap::new();
    }

    let Some(workbook_xml) = read_zip_text(&mut archive, "xl/workbook.xml") else {
        return HashMap::new();
    };
    let Some(relationships_xml) = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels") else {
        return HashMap::new();
    };
    let relationships = parse_relationships(&relationships_xml);

    let mut result: SheetCellIndents = HashMap::new();
    for (sheet_name, relationship_id) in parse_sheet_relationships(&workbook_xml) {
        let Some(target) = relationships.get(&relationship_id) else {
            continue;
        };
        let Some(worksheet_xml) = read_zip_text(&mut archive, &worksheet_path(target)) else {
            continue;
        };
        let levels = parse_worksheet_indents(&worksheet_xml, &xf_indents);
        if !levels.is_empty() {
            result.insert(sheet_name, levels);
        }
    }
    result
}

//! Banded-row shading for Excel tables (`xl/tables/*.xml`).
//!
//! A table's stripes live only in `<tableStyleInfo>`. They are not recoverable
//! from the cell records: every body cell of the audited table carries a
//! `cellXfs` entry with `fillId="0"`, so reading styles alone leaves all 171
//! rows white (issue #532).

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::xlsx_cells::parse_column_letters;
use super::xlsx_drawing::{parse_rels_by_type, read_zip_entry_string, resolve_relative_xl_path};
use crate::ir::Color;
use crate::parser::xml_util::get_attr_str;

/// Rows of one table that Excel paints with the style's stripe fill.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RowStripes {
    start_col: u32,
    end_col: u32,
    /// First body row — the table's first row plus its header rows.
    first_body_row: u32,
    end_row: u32,
    fill: Color,
}

impl RowStripes {
    /// The stripe fill at `(col, row)`, or `None` where the table paints none.
    ///
    /// Excel shades the first body row and every second one after it, so the
    /// band alternates from the top of the body rather than from row 1.
    pub(super) fn fill_at(&self, col: u32, row: u32) -> Option<Color> {
        if !(self.start_col..=self.end_col).contains(&col)
            || !(self.first_body_row..=self.end_row).contains(&row)
        {
            // A header row sits above `first_body_row`, so this bound has to
            // be checked before the offset below is computed.
            return None;
        }
        (row - self.first_body_row)
            .is_multiple_of(2)
            .then_some(self.fill)
    }
}

/// The stripe fill for each sheet that declares one, keyed by sheet name.
pub(super) type SheetRowStripes = HashMap<String, Vec<RowStripes>>;

/// Excel's built-in table styles derive their banding from the theme accents.
///
/// `TableStyleMedium<N>` runs in bands of seven: one dark style followed by
/// accent 1 through 6. Its stripe is the accent at a 20% tint, which is the
/// `DCE6F2` Excel paints for `TableStyleMedium2` against a `4F81BD` accent 1.
///
/// TODO(unverified): the Light and Dark families use different tints that no
/// fixture here pins down, so they are left unstriped rather than guessed at.
fn stripe_fill_for_style(style_name: &str, accents: &[Color]) -> Option<Color> {
    let index: usize = style_name.strip_prefix("TableStyleMedium")?.parse().ok()?;
    let within_band: usize = (index - 1) % 7;
    // The band's first style is the dark one, which carries no accent.
    let accent: Color = *accents.get(within_band.checked_sub(1)?)?;
    Some(tint(accent, 0.8))
}

/// Lighten `color` by moving each channel `amount` of the way to white, the
/// way OOXML's `tint` attribute does.
fn tint(color: Color, amount: f64) -> Color {
    let lighten = |channel: u8| -> u8 {
        (f64::from(channel) + (255.0 - f64::from(channel)) * amount).round() as u8
    };
    Color::new(lighten(color.r), lighten(color.g), lighten(color.b))
}

/// Read a table part's stripe definition.
///
/// `None` when the table declares no row stripes, its range is unreadable, or
/// its style is one whose banding we cannot resolve.
fn parse_table_part(xml: &str, accents: &[Color]) -> Option<RowStripes> {
    let mut reader = Reader::from_str(xml);
    let mut range: Option<(u32, u32, u32, u32)> = None;
    let mut header_rows: u32 = 0;
    let mut fill: Option<Color> = None;

    loop {
        let event = reader.read_event();
        let element = match event {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => e.clone(),
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        match element.local_name().as_ref() {
            b"table" => {
                range = get_attr_str(&element, b"ref")
                    .as_deref()
                    .and_then(parse_range);
                header_rows = get_attr_str(&element, b"headerRowCount")
                    .and_then(|value| value.parse::<u32>().ok())
                    // ECMA-376 defaults headerRowCount to 1.
                    .unwrap_or(1);
            }
            b"tableStyleInfo"
                if get_attr_str(&element, b"showRowStripes").as_deref() == Some("1") =>
            {
                fill = get_attr_str(&element, b"name")
                    .as_deref()
                    .and_then(|name| stripe_fill_for_style(name, accents));
            }
            _ => {}
        }
    }

    let (start_col, start_row, end_col, end_row) = range?;
    let fill: Color = fill?;
    let first_body_row: u32 = start_row + header_rows;
    (first_body_row <= end_row).then_some(RowStripes {
        start_col,
        end_col,
        first_body_row,
        end_row,
        fill,
    })
}

/// Parse an `A4:H175`-style range into `(start_col, start_row, end_col, end_row)`.
fn parse_range(reference: &str) -> Option<(u32, u32, u32, u32)> {
    let (start, end) = reference.split_once(':')?;
    let (start_col, start_row) = parse_cell_reference(start)?;
    let (end_col, end_row) = parse_cell_reference(end)?;
    Some((start_col, start_row, end_col, end_row))
}

/// Parse an `A4`-style reference into a 1-based `(column, row)`.
fn parse_cell_reference(reference: &str) -> Option<(u32, u32)> {
    let split: usize = reference.find(|c: char| c.is_ascii_digit())?;
    let (letters, digits) = reference.split_at(split);
    Some((parse_column_letters(letters)?, digits.parse().ok()?))
}

/// The theme's accent 1 through 6, in order, for resolving built-in styles.
fn theme_accents(theme_xml: &str) -> Vec<Color> {
    let scheme = crate::parser::drawingml::parse_theme_color_scheme(theme_xml);
    (1..=6)
        .map_while(|index| scheme.get(&format!("accent{index}")).copied())
        .collect()
}

/// Collect every sheet's banded-row shading from the workbook.
pub(super) fn extract_row_stripes(data: &[u8]) -> SheetRowStripes {
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return SheetRowStripes::new();
    };
    let workbook_xml: String = read_zip_entry_string(&mut archive, "xl/workbook.xml");
    let workbook_rels_xml: String =
        read_zip_entry_string(&mut archive, "xl/_rels/workbook.xml.rels");
    let rid_to_target = super::xlsx_drawing::parse_rels_targets(&workbook_rels_xml);

    let theme_path: String = parse_rels_by_type(&workbook_rels_xml, "theme")
        .first()
        .map(|target| resolve_relative_xl_path("xl", target))
        .unwrap_or_else(|| "xl/theme/theme1.xml".to_string());
    let theme_xml: String = read_zip_entry_string(&mut archive, &theme_path);
    let accents: Vec<Color> = theme_accents(&theme_xml);
    if accents.is_empty() {
        return SheetRowStripes::new();
    }

    let mut stripes: SheetRowStripes = HashMap::new();
    for (sheet_name, sheet_rid) in super::xlsx_drawing::parse_workbook_sheet_rids(&workbook_xml) {
        let Some(sheet_target) = rid_to_target.get(&sheet_rid) else {
            continue;
        };
        let sheet_path: String = format!("xl/{sheet_target}");
        let filename: &str = sheet_path.rsplit('/').next().unwrap_or(&sheet_path);
        let rels_path: String = format!("xl/worksheets/_rels/{filename}.rels");
        let rels_xml: String = read_zip_entry_string(&mut archive, &rels_path);
        if rels_xml.is_empty() {
            continue;
        }
        let mut sheet_stripes: Vec<RowStripes> = Vec::new();
        for target in parse_rels_by_type(&rels_xml, "table") {
            let table_path: String = resolve_relative_xl_path("xl/worksheets", &target);
            let table_xml: String = read_zip_entry_string(&mut archive, &table_path);
            if table_xml.is_empty() {
                continue;
            }
            if let Some(parsed) = parse_table_part(&table_xml, &accents) {
                sheet_stripes.push(parsed);
            }
        }
        if !sheet_stripes.is_empty() {
            stripes.insert(sheet_name, sheet_stripes);
        }
    }
    stripes
}

#[cfg(test)]
#[path = "xlsx_tables_tests.rs"]
mod tests;

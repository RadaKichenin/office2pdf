//! The paint a built-in table style lays over an Excel table (`xl/tables/*.xml`).
//!
//! A table's styling lives only in `<tableStyleInfo>`. None of it is recoverable
//! from the cell records: every body cell of the audited table carries a
//! `cellXfs` entry with `fillId="0"` and no border, so reading styles alone
//! leaves all 171 rows white (issue #532) and every rule missing (issue #1080).

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::xlsx_cells::parse_column_letters;
use super::xlsx_drawing::{parse_rels_by_type, read_zip_entry_string, resolve_relative_xl_path};
use crate::ir::{BorderLineStyle, BorderSide, CellBorder, Color};
use crate::parser::xml_util::get_attr_str;

/// One table's range together with the paint its built-in style lays over it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TableStyleRange {
    start_col: u32,
    end_col: u32,
    /// The table's first row, which is its first header row when it has one.
    start_row: u32,
    /// First body row — the table's first row plus its header rows.
    first_body_row: u32,
    end_row: u32,
    stripe: Option<Color>,
    rule: Option<Color>,
}

impl TableStyleRange {
    /// The stripe fill at `(col, row)`, or `None` where the table paints none.
    ///
    /// Excel shades the first body row and every second one after it, so the
    /// band alternates from the top of the body rather than from row 1.
    pub(super) fn fill_at(&self, col: u32, row: u32) -> Option<Color> {
        let fill: Color = self.stripe?;
        if !(self.start_col..=self.end_col).contains(&col)
            || !(self.first_body_row..=self.end_row).contains(&row)
        {
            // A header row sits above `first_body_row`, so this bound has to
            // be checked before the offset below is computed.
            return None;
        }
        (row - self.first_body_row)
            .is_multiple_of(2)
            .then_some(fill)
    }

    /// The style's own rules at `(col, row)`.
    ///
    /// A `TableStyleLight1` table is ruled in three places and nowhere else:
    /// above its header row, under it, and at the foot of its last row. Each
    /// is a 1pt band spanning the table's full width, which is the paint
    /// issue #619 measured every thin Excel border to be.
    pub(super) fn border_at(&self, col: u32, row: u32) -> Option<CellBorder> {
        let rule: Color = self.rule?;
        if !(self.start_col..=self.end_col).contains(&col)
            || !(self.start_row..=self.end_row).contains(&row)
        {
            return None;
        }
        let side = || BorderSide {
            width: 1.0,
            color: rule,
            style: BorderLineStyle::Solid,
        };
        let top: Option<BorderSide> = (row == self.start_row).then(side);
        // The second rule closes the header, so a table declaring
        // `headerRowCount="0"` — where `first_body_row` is `start_row` — has
        // no row to hang it on and prints only the outer two.
        let bottom: Option<BorderSide> =
            (row + 1 == self.first_body_row || row == self.end_row).then(side);
        (top.is_some() || bottom.is_some()).then_some(CellBorder {
            top,
            bottom,
            left: None,
            right: None,
        })
    }

    /// Whether `(col, row)` sits in a header row this style prints bold.
    ///
    /// Gated on the same `rule` as [`Self::border_at`]: a style family whose
    /// header treatment is unresolved leaves the header alone entirely rather
    /// than bolding it into a header that is otherwise unpainted.
    pub(super) fn bolds_header_at(&self, col: u32, row: u32) -> bool {
        self.rule.is_some()
            && (self.start_col..=self.end_col).contains(&col)
            && (self.start_row..self.first_body_row).contains(&row)
    }
}

/// The style paint for each sheet that declares one, keyed by sheet name.
pub(super) type SheetTableStyles = HashMap<String, Vec<TableStyleRange>>;

/// The theme slots a built-in table style resolves its colours against.
struct StylePalette {
    /// `accent1`..`accent6`, in order.
    accents: Vec<Color>,
    /// `lt1`, the background the accent-less styles shade their band out of.
    light: Color,
    /// `dk1`, which those same styles rule in.
    dark: Color,
}

/// What a built-in table style paints over its range.
///
/// A family whose banding is measured but whose header treatment is not — the
/// `TableStyleMedium` mapping of issue #532, tracked in issue #1125 — resolves a
/// `stripe` and leaves `rule` `None`, which leaves its header row alone too.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TableStylePaint {
    /// The banded-row fill, painted where `showRowStripes` asks for it.
    stripe: Option<Color>,
    /// The colour of the rule above the header row, under it, and at the
    /// table's foot. A style that draws these also prints its header bold.
    rule: Option<Color>,
}

/// Excel's built-in table styles derive their paint from the theme palette.
///
/// Each family runs in bands of seven: one accent-less style followed by
/// accent 1 through 6.
///
/// `TableStyleMedium<N>` stripes in the accent at a 20% tint, which is the
/// `DCE6F2` Excel paints for `TableStyleMedium2` against a `4F81BD` accent 1.
///
/// `TableStyleLight1`..`7` — the family's first band — stripe the same way and
/// add the three rules and the bold header row. A native Excel-for-Mac export
/// of the audited table prints `Light1` in `#D9D9D9` over black rules and
/// `Light2` in `#DCE6F1` over `4F81BD` ones, so the accent-less member takes
/// its band from `lt1` shaded 15% and its rule from `dk1` (issue #1080).
///
/// TODO(unverified): the same probe shows `Light8`..`21` and the Dark family
/// painting something else entirely — `Light8` fills its header row solid and
/// rules every row, `Light15` boxes the whole table — so they stay unresolved
/// rather than guessed at.
fn built_in_table_style(style_name: &str, palette: &StylePalette) -> Option<TableStylePaint> {
    let (index, rules): (usize, bool) = match style_name.strip_prefix("TableStyleMedium") {
        Some(digits) => (digits.parse().ok()?, false),
        None => {
            let digits: &str = style_name.strip_prefix("TableStyleLight")?;
            let index: usize = digits.parse().ok()?;
            // Only the first band of the Light family is measured.
            (index <= 7).then_some((index, true))?
        }
    };

    let within_band: usize = (index.checked_sub(1)?) % 7;
    let Some(accent_index) = within_band.checked_sub(1) else {
        // The band opens on the accent-less style. In the Medium family that
        // is the dark style, whose paint is not measured; in the Light family
        // it is the neutral one.
        return rules.then_some(TableStylePaint {
            stripe: Some(shade(palette.light, 0.15)),
            rule: Some(palette.dark),
        });
    };
    let accent: Color = *palette.accents.get(accent_index)?;
    Some(TableStylePaint {
        stripe: Some(tint(accent, 0.8)),
        rule: rules.then_some(accent),
    })
}

/// Lighten `color` by moving each channel `amount` of the way to white, the
/// way OOXML's `tint` attribute does.
fn tint(color: Color, amount: f64) -> Color {
    let lighten = |channel: u8| -> u8 {
        (f64::from(channel) + (255.0 - f64::from(channel)) * amount).round() as u8
    };
    Color::new(lighten(color.r), lighten(color.g), lighten(color.b))
}

/// Darken `color` by moving each channel `amount` of the way to black, the way
/// a negative OOXML `tint` does.
fn shade(color: Color, amount: f64) -> Color {
    let darken = |channel: u8| -> u8 {
        (f64::from(channel) * (1.0 - amount))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::new(darken(color.r), darken(color.g), darken(color.b))
}

/// Read a table part's style paint.
///
/// `None` when the range is unreadable, or when the style resolves to nothing
/// the table would print — an unmapped style family, or a mapped one whose
/// only paint is a stripe the table turned off.
fn parse_table_part(xml: &str, palette: &StylePalette) -> Option<TableStyleRange> {
    let mut reader = Reader::from_str(xml);
    let mut range: Option<(u32, u32, u32, u32)> = None;
    let mut header_rows: u32 = 0;
    let mut paint: Option<TableStylePaint> = None;
    let mut shows_row_stripes: bool = false;

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
            b"tableStyleInfo" => {
                // `showRowStripes` governs the banding alone: a `Light1` table
                // that turns it off still prints its three rules and its bold
                // header, as the native export of that variant shows.
                shows_row_stripes =
                    get_attr_str(&element, b"showRowStripes").as_deref() == Some("1");
                paint = get_attr_str(&element, b"name")
                    .as_deref()
                    .and_then(|name| built_in_table_style(name, palette));
            }
            _ => {}
        }
    }

    let (start_col, start_row, end_col, end_row) = range?;
    let paint: TableStylePaint = paint?;
    let first_body_row: u32 = start_row + header_rows;
    let stripe: Option<Color> = paint
        .stripe
        .filter(|_| shows_row_stripes && first_body_row <= end_row);
    (stripe.is_some() || paint.rule.is_some()).then_some(TableStyleRange {
        start_col,
        end_col,
        start_row,
        first_body_row,
        end_row,
        stripe,
        rule: paint.rule,
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

/// The theme slots the built-in styles resolve against.
///
/// A workbook whose theme part is missing or unreadable still prints an
/// accent-less style, because Excel's default `lt1`/`dk1` are plain white and
/// black; only the accented members go unresolved.
fn style_palette(theme_xml: &str) -> StylePalette {
    let scheme = crate::parser::drawingml::parse_theme_color_scheme(theme_xml);
    StylePalette {
        accents: (1..=6)
            .map_while(|index| scheme.get(&format!("accent{index}")).copied())
            .collect(),
        light: scheme.get("lt1").copied().unwrap_or(Color::white()),
        dark: scheme.get("dk1").copied().unwrap_or(Color::black()),
    }
}

/// Collect every sheet's table-style paint from the workbook.
pub(super) fn extract_table_styles(data: &[u8]) -> SheetTableStyles {
    let Ok(mut archive) = crate::parser::open_zip(data) else {
        return SheetTableStyles::new();
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
    let palette: StylePalette = style_palette(&theme_xml);

    let mut styles: SheetTableStyles = HashMap::new();
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
        let mut sheet_styles: Vec<TableStyleRange> = Vec::new();
        for target in parse_rels_by_type(&rels_xml, "table") {
            let table_path: String = resolve_relative_xl_path("xl/worksheets", &target);
            let table_xml: String = read_zip_entry_string(&mut archive, &table_path);
            if table_xml.is_empty() {
                continue;
            }
            if let Some(parsed) = parse_table_part(&table_xml, &palette) {
                sheet_styles.push(parsed);
            }
        }
        if !sheet_styles.is_empty() {
            styles.insert(sheet_name, sheet_styles);
        }
    }
    styles
}

#[cfg(test)]
#[path = "xlsx_tables_tests.rs"]
mod tests;

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
use crate::ir::{BorderLineStyle, BorderSide, CellBorder, Color, LineJoin};
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
    rule: Option<TableRule>,
    header: Option<HeaderPaint>,
}

impl TableStyleRange {
    /// The fill at `(col, row)`, or `None` where the table paints none.
    ///
    /// A header row takes the style's own fill. Below it Excel shades the
    /// first body row and every second one after it, so the band alternates
    /// from the top of the body rather than from row 1.
    pub(super) fn fill_at(&self, col: u32, row: u32) -> Option<Color> {
        if !(self.start_col..=self.end_col).contains(&col)
            || !(self.start_row..=self.end_row).contains(&row)
        {
            // A header row sits above `first_body_row`, so this bound has to
            // be checked before the offset below is computed.
            return None;
        }
        if row < self.first_body_row {
            return self.header.map(|header| header.fill);
        }
        let fill: Color = self.stripe?;
        (row - self.first_body_row)
            .is_multiple_of(2)
            .then_some(fill)
    }

    /// The style's own rules at `(col, row)`.
    ///
    /// Every rule is a 1pt band, which is the paint issue #619 measured every
    /// thin Excel border to be. How far they run is the family's to say — see
    /// [`RuleExtent`].
    pub(super) fn border_at(&self, col: u32, row: u32) -> Option<CellBorder> {
        let rule: TableRule = self.rule?;
        if !(self.start_col..=self.end_col).contains(&col)
            || !(self.start_row..=self.end_row).contains(&row)
        {
            return None;
        }
        let side = || BorderSide {
            width: 1.0,
            color: rule.color,
            style: BorderLineStyle::Solid,
            join: LineJoin::Round,
        };
        let closes_row: bool = match rule.extent {
            // The second rule closes the header, so a table declaring
            // `headerRowCount="0"` — where `first_body_row` is `start_row` —
            // has no row to hang it on and prints only the outer two.
            RuleExtent::HeaderAndFoot => row + 1 == self.first_body_row || row == self.end_row,
            RuleExtent::EveryRowAndOuterEdges => true,
        };
        let draws_verticals: bool = matches!(rule.extent, RuleExtent::EveryRowAndOuterEdges);
        let border = CellBorder {
            top: (row == self.start_row).then(side),
            bottom: closes_row.then(side),
            left: (draws_verticals && col == self.start_col).then(side),
            right: (draws_verticals && col == self.end_col).then(side),
        };
        let CellBorder {
            top,
            bottom,
            left,
            right,
        } = &border;
        (top.is_some() || bottom.is_some() || left.is_some() || right.is_some()).then_some(border)
    }

    /// Whether `(col, row)` sits in a header row this style prints bold.
    ///
    /// Gated on the same `rule` as [`Self::border_at`]: a style family whose
    /// header treatment is unresolved leaves the header alone entirely rather
    /// than bolding it into a header that is otherwise unpainted.
    pub(super) fn bolds_header_at(&self, col: u32, row: u32) -> bool {
        self.rule.is_some() && self.is_header_cell(col, row)
    }

    /// The ink a header row's runs take, where the style states one.
    ///
    /// Only a style that fills the header states one — a Medium table prints
    /// white on its accent band. A Light header sits on the sheet's own
    /// background, and the export leaves its runs black.
    pub(super) fn header_text_color_at(&self, col: u32, row: u32) -> Option<Color> {
        self.header
            .filter(|_| self.is_header_cell(col, row))
            .map(|header| header.text)
    }

    fn is_header_cell(&self, col: u32, row: u32) -> bool {
        (self.start_col..=self.end_col).contains(&col)
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
/// A family whose banding is measured but whose header and rules are not — the
/// `TableStyleMedium8`..`28` mapping of issue #532 — resolves a `stripe` and
/// leaves the other two `None`, which leaves its header row alone too.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TableStylePaint {
    /// The banded-row fill, painted where `showRowStripes` asks for it.
    stripe: Option<Color>,
    /// The rules the style lays over the table's row boundaries.
    rule: Option<TableRule>,
    /// The header row's own paint, where the style gives it one.
    header: Option<HeaderPaint>,
}

/// A style's rules: one colour, and how far through the table they run.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TableRule {
    color: Color,
    extent: RuleExtent,
}

/// How far a style's rules run.
#[derive(Debug, Clone, Copy, PartialEq)]
enum RuleExtent {
    /// Above the header row, under it, and at the table's foot — nothing
    /// between the body rows, and no verticals. The Light band's treatment.
    HeaderAndFoot,
    /// Every row boundary, plus a vertical down the table's own left and
    /// right edges. The Medium band's treatment; a native `TableStyleMedium2`
    /// export lays five horizontals over a four-row table and two verticals
    /// spanning its full height (issue #1125).
    EveryRowAndOuterEdges,
}

/// The paint a style lays over its header row.
///
/// The Medium band fills the row in its accent and sets the runs on it white;
/// the Light band fills nothing, so it carries no `HeaderPaint` at all and its
/// header keeps the sheet's own ink.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HeaderPaint {
    fill: Color,
    text: Color,
}

/// The tint a banded row takes off its style's accent.
const BAND_TINT: f64 = 0.8;
/// The tint the Medium band's rules take off the same accent.
const RULE_TINT: f64 = 0.4;
/// The shade an accent-less style bands with, off `lt1`. Negative, because a
/// neutral style darkens its background where an accented one lightens.
const NEUTRAL_BAND_TINT: f64 = -0.15;

/// Excel's built-in table styles derive their paint from the theme palette.
///
/// Each family runs in bands of seven: one accent-less style followed by
/// accent 1 through 6. Only each family's first band is measured, from native
/// Excel-for-Mac exports of one table restyled through every member.
///
/// `TableStyleLight1`..`7` band the body, rule above the header row, under it
/// and at the table's foot, and print the header bold. The accent-less member
/// bands out of `lt1` shaded 15% and rules in `dk1` — `#D9D9D9` over black —
/// while `Light2` bands in `#DCE6F1` and rules in the `4F81BD` accent itself
/// (issue #1080).
///
/// `TableStyleMedium1`..`7` band identically but paint far more around it: the
/// header row is filled in the accent with white bold runs on it, and the
/// rules — the accent at a 40% tint, `#95B3D7` against `4F81BD` — run at every
/// row boundary and down both outer edges. `Medium1` opens that band on the
/// dark style, filling and ruling in `dk1` (issue #1125).
///
/// TODO(unverified): the same probe shows every later band painting something
/// else entirely — `Light8` fills its header row solid and rules every row,
/// `Medium9` fills all four rows in two tints, `Medium15` boxes the table in
/// 2pt rules, `Medium22` bands in greys. The Medium ones keep the banding
/// issue #532 resolved for them and nothing more; the Light ones and the Dark
/// family stay unpainted rather than guessed at.
fn built_in_table_style(style_name: &str, palette: &StylePalette) -> Option<TableStylePaint> {
    let (index, is_medium): (usize, bool) = match style_name.strip_prefix("TableStyleMedium") {
        Some(digits) => (digits.parse().ok()?, true),
        None => (
            style_name.strip_prefix("TableStyleLight")?.parse().ok()?,
            false,
        ),
    };

    let within_band: usize = (index.checked_sub(1)?) % 7;
    let accent: Option<Color> = match within_band.checked_sub(1) {
        Some(accent_index) => Some(*palette.accents.get(accent_index)?),
        // The band opens on the accent-less style: the neutral one in the
        // Light family, the dark one in the Medium family.
        None => None,
    };

    if index > 7 {
        return match (is_medium, accent) {
            (true, Some(accent)) => Some(TableStylePaint {
                stripe: Some(tint(accent, BAND_TINT)),
                rule: None,
                header: None,
            }),
            _ => None,
        };
    }

    let (fill, rule_color): (Color, Color) = match accent {
        Some(accent) if is_medium => (accent, tint(accent, RULE_TINT)),
        Some(accent) => (accent, accent),
        None => (palette.dark, palette.dark),
    };
    Some(TableStylePaint {
        stripe: Some(match accent {
            Some(accent) => tint(accent, BAND_TINT),
            None => tint(palette.light, NEUTRAL_BAND_TINT),
        }),
        rule: Some(TableRule {
            color: rule_color,
            extent: if is_medium {
                RuleExtent::EveryRowAndOuterEdges
            } else {
                RuleExtent::HeaderAndFoot
            },
        }),
        header: is_medium.then_some(HeaderPaint {
            fill,
            text: Color::white(),
        }),
    })
}

/// Excel's integer HLS range.
///
/// The OOXML note on `tint` names 255, but only 240 — the range Win32's
/// `RGBToHLS` has always used — reproduces a native export: a `4F81BD`
/// accent 1 tinted 0.8 prints `DCE6F1`, which 255 misses by a level
/// (`DBE5F1`) and a per-channel RGB tint misses the other way (`DCE6F2`,
/// issue #1125).
const HLS_MAX: i32 = 240;
const RGB_MAX: i32 = 255;

/// Move `color`'s luminance `amount` of the way to white, or to black where
/// `amount` is negative, the way OOXML's `tint` attribute does.
///
/// Measured against native Excel-for-Mac exports of one table restyled under
/// two Office themes: `4F81BD` gives `DCE6F1`/`95B3D7`, `5B9BD5` gives
/// `DDEBF7`/`9BC2E6` and `C0504D` gives `F2DCDB`/`DA9694` at 0.8 and 0.4, and
/// white gives `D9D9D9` at -0.15. The moved luminance truncates rather than
/// rounds — rounding misses four of those seven by a level.
fn tint(color: Color, amount: f64) -> Color {
    let (hue, luminance, saturation): (i32, i32, i32) = to_hls(color);
    let moved: f64 = if amount < 0.0 {
        f64::from(luminance) * (1.0 + amount)
    } else {
        f64::from(luminance) * (1.0 - amount) + f64::from(HLS_MAX) * amount
    };
    from_hls(hue, moved as i32, saturation)
}

/// Win32's `RGBToHLS`, in the integer arithmetic it is defined with.
fn to_hls(color: Color) -> (i32, i32, i32) {
    let (red, green, blue): (i32, i32, i32) =
        (i32::from(color.r), i32::from(color.g), i32::from(color.b));
    let brightest: i32 = red.max(green).max(blue);
    let darkest: i32 = red.min(green).min(blue);
    let sum: i32 = brightest + darkest;
    let span: i32 = brightest - darkest;
    let luminance: i32 = (sum * HLS_MAX + RGB_MAX) / (2 * RGB_MAX);
    if span == 0 {
        return (0, luminance, 0);
    }
    let saturation: i32 = if luminance <= HLS_MAX / 2 {
        (span * HLS_MAX + sum / 2) / sum
    } else {
        (span * HLS_MAX + (2 * RGB_MAX - sum) / 2) / (2 * RGB_MAX - sum)
    };
    let distance =
        |channel: i32| -> i32 { ((brightest - channel) * (HLS_MAX / 6) + span / 2) / span };
    let hue: i32 = if red == brightest {
        distance(blue) - distance(green)
    } else if green == brightest {
        HLS_MAX / 3 + distance(red) - distance(blue)
    } else {
        2 * HLS_MAX / 3 + distance(green) - distance(red)
    };
    (hue.rem_euclid(HLS_MAX), luminance, saturation)
}

/// Win32's `HLSToRGB`.
///
/// Its zero-saturation shortcut is left out on purpose: it truncates where the
/// general path rounds, which lands `D8D8D8` for the `lt1` shade a native
/// export bands `D9D9D9`. With no saturation the general path collapses to the
/// same single level anyway, so dropping the shortcut only changes the
/// rounding.
fn from_hls(hue: i32, luminance: i32, saturation: i32) -> Color {
    let upper: i32 = if luminance <= HLS_MAX / 2 {
        (luminance * (HLS_MAX + saturation) + HLS_MAX / 2) / HLS_MAX
    } else {
        luminance + saturation - (luminance * saturation + HLS_MAX / 2) / HLS_MAX
    };
    let lower: i32 = 2 * luminance - upper;
    let channel = |hue_offset: i32| -> u8 {
        let level: i32 = hue_level(lower, upper, hue + hue_offset);
        ((level * RGB_MAX + HLS_MAX / 2) / HLS_MAX).clamp(0, RGB_MAX) as u8
    };
    Color::new(channel(HLS_MAX / 3), channel(0), channel(-HLS_MAX / 3))
}

/// One channel's level, interpolated between `lower` and `upper` across the
/// hue wheel — Win32's `HueToRGB`.
fn hue_level(lower: i32, upper: i32, hue: i32) -> i32 {
    let hue: i32 = hue.rem_euclid(HLS_MAX);
    let sixth: i32 = HLS_MAX / 6;
    if hue < sixth {
        lower + ((upper - lower) * hue + HLS_MAX / 12) / sixth
    } else if hue < HLS_MAX / 2 {
        upper
    } else if hue < 2 * HLS_MAX / 3 {
        lower + ((upper - lower) * (2 * HLS_MAX / 3 - hue) + HLS_MAX / 12) / sixth
    } else {
        lower
    }
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
    (stripe.is_some() || paint.rule.is_some() || paint.header.is_some()).then_some(
        TableStyleRange {
            start_col,
            end_col,
            start_row,
            first_body_row,
            end_row,
            stripe,
            rule: paint.rule,
            header: paint.header,
        },
    )
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

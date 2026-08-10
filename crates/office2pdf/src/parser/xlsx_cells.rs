use std::collections::{HashMap, HashSet};

use crate::ir::{Block, Paragraph, ParagraphStyle, Run, TableRow};
use crate::parser::cond_fmt::build_cond_fmt_overrides;

use super::xlsx_style::{
    apply_rich_run_font, extract_cell_alignment, extract_cell_background, extract_cell_borders,
    extract_cell_text_style,
};
use crate::ir::{BorderSide, CellBorder, Color, Insets, TableCell};

/// A cell range within a sheet (1-indexed, inclusive).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CellRange {
    pub(crate) start_col: u32,
    pub(crate) start_row: u32,
    pub(crate) end_col: u32,
    pub(crate) end_row: u32,
}

/// A (column, row) coordinate pair (1-indexed).
pub(crate) type CellPos = (u32, u32);

/// Info about a merged cell region, keyed by its top-left coordinate.
pub(super) struct MergeInfo {
    pub(super) col_span: u32,
    pub(super) row_span: u32,
}

/// Convert Excel column width (character units) to points.
/// OOXML widths are expressed relative to the Normal font's column unit. The
/// stored width already incorporates Excel's cell padding adjustment, so
/// print geometry must not add padding again. Excel prints each declared
/// column at an integer point count: probe calibri11frac (issue #621) shows
/// width 10.6 at the 6pt Calibri-11 unit printing 64pt, not 63.6pt.
pub(super) fn column_width_to_pt(char_width: f64, column_unit_pt: f64) -> f64 {
    round_half_up_pt(char_width * column_unit_pt)
}

/// Round to the nearest integer point, halves upward. Excel's column metric
/// rounds half UP, not half-even: the Times New Roman 13 probe lands exactly
/// on 6.500pt and prints a 7pt unit (issue #621). Inputs are non-negative.
fn round_half_up_pt(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// Read the workbook's Normal font (the first `<font>` in `xl/styles.xml`)
/// straight from the archive; umya does not expose the stylesheet. Excel
/// derives all column print metrics from this font, not from cell fonts.
pub(super) fn extract_normal_font(data: &[u8]) -> Option<NormalFont> {
    use quick_xml::events::Event;
    use std::io::Read;

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let mut file = archive.by_name("xl/styles.xml").ok()?;
    let mut xml = String::new();
    file.read_to_string(&mut xml).ok()?;

    let mut reader = quick_xml::Reader::from_str(&xml);
    let mut in_first_font = false;
    let mut name: Option<String> = None;
    let mut size: Option<f64> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"font" => {
                in_first_font = true;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"font" => break,
            Ok(Event::Empty(ref e)) if in_first_font => {
                let val = e
                    .try_get_attribute("val")
                    .ok()
                    .flatten()
                    .and_then(|a| String::from_utf8(a.value.into_owned()).ok());
                match e.local_name().as_ref() {
                    b"name" => name = val,
                    b"sz" => size = val.and_then(|v| v.parse::<f64>().ok()),
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Some(NormalFont {
        family: name?,
        size_pt: size.unwrap_or(11.0),
    })
}

/// The workbook's Normal font: the `xl/styles.xml` font that cells with no
/// style of their own inherit, and the font Excel derives every column
/// print metric from.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct NormalFont {
    pub(super) family: String,
    pub(super) size_pt: f64,
}

/// Max digit advance of Calibri (and metrically identical Carlito), Excel's
/// default Normal font — the last-resort metric when a family is unknown to
/// the reference table and no real face resolves.
const CALIBRI_DIGIT_ADVANCE_EM: f64 = 0.506836;

/// Reference maximum digit advances (em over U+0030..=U+0039) of the faces
/// Excel itself ships, read from their `hmtx` tables by the issue #621 probe
/// tooling. These outrank live font resolution on purpose: the converting
/// machine may substitute a digit-incompatible face (Calibri → Liberation
/// Sans advances 0.556em against Calibri's 0.5068), which would shift column
/// geometry per machine, while Excel's own print metric always comes from the
/// face Excel resolves. The table also keeps wasm and font-less environments
/// on the exact native-Excel numbers.
pub(super) fn reference_digit_advance_em(family: &str) -> Option<f64> {
    match family.to_ascii_lowercase().as_str() {
        "calibri" | "carlito" => Some(CALIBRI_DIGIT_ADVANCE_EM),
        "arial" | "helvetica" | "liberation sans" => Some(0.556152),
        "verdana" => Some(0.635742),
        "courier new" => Some(0.600098),
        "times new roman" => Some(0.500000),
        "malgun gothic" | "맑은 고딕" => Some(0.550781),
        _ => None,
    }
}

/// Points Excel allots to one column character unit for the given Normal
/// font: `round_half_up(max digit advance × size)` — an INTEGER point count.
/// Measured on 17 one-factor native Excel-for-Mac probes (issue #621); the
/// probe set discriminates this model from every integer-96dpi-pixel model
/// (Calibri 10 → 5pt, where pixel-ceiling gave 7px = 5.25pt) and from other
/// rounding modes (Times New Roman 13 = 6.500 → 7 kills half-even; Calibri 9
/// and Verdana 11 kill truncation; Calibri 10 and Verdana 10 kill ceiling).
pub(super) fn column_unit_pt(family: &str, size_pt: f64) -> f64 {
    let digit_advance_em: f64 = reference_digit_advance_em(family)
        .or_else(|| crate::render::pdf::max_digit_advance_em(family))
        .unwrap_or(CALIBRI_DIGIT_ADVANCE_EM);
    round_half_up_pt(digit_advance_em * size_pt)
}

/// Width in points of a column with no `<col>` entry.
///
/// With no declared `defaultColWidth` either, Excel prints
/// `baseColWidth × unit + 5` points — not 8.43 character units — where
/// `baseColWidth` defaults to 8 when `sheetFormatPr` omits it too. Measured
/// by the issue #621 probes: no-baseColWidth workbooks print 45/53/61pt at
/// unit 5/6/7, and the round-3 probes calibri11base10/calibri11base12
/// (`<sheetFormatPr baseColWidth="10|12"/>`, no defaultColWidth, 6pt
/// Calibri-11 unit) print 65pt and 77pt default columns — killing the
/// ignore-baseColWidth model (53pt). When the sheet does declare
/// `defaultColWidth`, it outranks `baseColWidth` (ECMA-376 §18.3.1.81) and
/// is assumed to quantize like any declared width (the probes only covered
/// the absent case; declared widths quantize this way, so the declared
/// default is routed through the same rule).
pub(super) fn default_column_width_pt(
    declared_width_chars: Option<f64>,
    base_col_width_chars: Option<u32>,
    column_unit_pt: f64,
) -> f64 {
    match declared_width_chars {
        Some(width_chars) => round_half_up_pt(width_chars * column_unit_pt),
        None => f64::from(base_col_width_chars.unwrap_or(8)) * column_unit_pt + 5.0,
    }
}

/// The sheet's `defaultColWidth`, only when the file actually declares one.
/// umya reports 0.0 for an absent attribute, a width Excel never writes.
pub(super) fn declared_default_column_width(sheet: &umya_spreadsheet::Worksheet) -> Option<f64> {
    let width_chars: f64 = *sheet
        .get_sheet_format_properties()
        .get_default_column_width();
    (width_chars > 0.0).then_some(width_chars)
}

/// The sheet's `sheetFormatPr@baseColWidth`, only when the file declares
/// one. umya reports 0 for an absent attribute, a base width Excel never
/// writes.
pub(super) fn declared_base_column_width(sheet: &umya_spreadsheet::Worksheet) -> Option<u32> {
    let base_width_chars: u32 = *sheet.get_sheet_format_properties().get_base_column_width();
    (base_width_chars > 0).then_some(base_width_chars)
}

/// Fallback when `xl/styles.xml` is unreadable: infer the column unit from
/// the dominant cell font. umya resolves each cell's effective style while
/// reading, so the dominant family is a stable approximation. The Normal
/// size is unknown too, so Excel's default of 11pt is assumed.
pub(super) fn sheet_column_unit_pt(sheet: &umya_spreadsheet::Worksheet) -> f64 {
    let mut family_counts: HashMap<String, usize> = HashMap::new();
    for cell in sheet.get_cell_collection() {
        let Some(font) = cell.get_style().get_font() else {
            continue;
        };
        let family = font.get_name().trim();
        if !family.is_empty() {
            *family_counts
                .entry(family.to_ascii_lowercase())
                .or_default() += 1;
        }
    }

    let dominant_family: Option<String> = family_counts
        .into_iter()
        .max_by(|(family_a, count_a), (family_b, count_b)| {
            count_a.cmp(count_b).then_with(|| family_b.cmp(family_a))
        })
        .map(|(family, _)| family);

    match dominant_family {
        Some(family) => column_unit_pt(&family, 11.0),
        // No fonts at all: keep the legacy 7px × 0.75 = 5.25pt UNIT (issue
        // #716). Only the unit survives from the old model — the widths built
        // on it still change under #621: default columns move from 44.2575pt
        // (8.43 × 5.25) to 8 × 5.25 + 5 = 47pt, and declared widths now
        // quantize to integer points. Those surrounding changes are
        // extrapolated from the probed model, not measured: the #621 probes
        // never covered a workbook without a readable stylesheet.
        None => 5.25,
    }
}

/// Parse an Excel column letter string (e.g., "A", "B", "AA") into a 1-indexed column number.
pub(super) fn parse_column_letters(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut col: u32 = 0;
    for c in s.chars() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        col = col * 26 + (c as u32 - b'A' as u32 + 1);
    }
    Some(col)
}

/// Parse a cell reference like "$A$1", "A1", "$B$10" into (col, row), both 1-indexed.
pub(crate) fn parse_cell_ref(s: &str) -> Option<(u32, u32)> {
    // Strip dollar signs
    let s = s.replace('$', "");
    // Split into letter part and number part
    let split_pos = s.find(|c: char| c.is_ascii_digit())?;
    let col_str = &s[..split_pos];
    let row_str = &s[split_pos..];
    let col = parse_column_letters(col_str)?;
    let row: u32 = row_str.parse().ok()?;
    Some((col, row))
}

/// Parse a print area address string (e.g., "Sheet1!$A$1:$C$10") into a CellRange.
pub(super) fn parse_print_area_range(address: &str) -> Option<CellRange> {
    // Strip optional sheet prefix (everything up to and including '!')
    let range_part = if let Some(pos) = address.rfind('!') {
        &address[pos + 1..]
    } else {
        address
    };

    let (start_str, end_str) = range_part.split_once(':')?;
    let (start_col, start_row) = parse_cell_ref(start_str)?;
    let (end_col, end_row) = parse_cell_ref(end_str)?;
    Some(CellRange {
        start_col,
        start_row,
        end_col,
        end_row,
    })
}

/// Look up the print area for a given sheet from its defined names.
pub(super) fn find_print_area(sheet: &umya_spreadsheet::Worksheet) -> Option<CellRange> {
    for dn in sheet.get_defined_names() {
        if dn.get_name() == "_xlnm.Print_Area" {
            let addr = dn.get_address();
            if let Some(range) = parse_print_area_range(&addr) {
                return Some(range);
            }
        }
    }
    None
}

/// Print-title ranges from `_xlnm.Print_Titles`: rows and/or columns that
/// Excel repeats on every printed page (1-indexed, inclusive).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PrintTitles {
    pub(super) rows: Option<(u32, u32)>,
    pub(super) cols: Option<(u32, u32)>,
}

/// Look up the sheet's print titles. The defined name holds one or two
/// comma-separated parts like `Sheet4!$A:$B,Sheet4!$2:$3`. Sheet-scoped
/// names (localSheetId) land on the worksheet; names the reader could not
/// scope stay at the workbook level, so both are consulted.
pub(super) fn find_print_titles(
    book: &umya_spreadsheet::Spreadsheet,
    sheet: &umya_spreadsheet::Worksheet,
) -> PrintTitles {
    let mut titles = PrintTitles::default();
    for dn in sheet.get_defined_names() {
        if dn.get_name() == "_xlnm.Print_Titles" {
            parse_print_title_address(&dn.get_address(), &mut titles);
        }
    }
    if titles.rows.is_none() && titles.cols.is_none() {
        let plain_prefix: String = format!("{}!", sheet.get_name());
        let quoted_prefix: String = format!("'{}'!", sheet.get_name());
        for dn in book.get_defined_names() {
            let address: String = dn.get_address();
            if dn.get_name() == "_xlnm.Print_Titles"
                && (address.contains(&plain_prefix) || address.contains(&quoted_prefix))
            {
                parse_print_title_address(&address, &mut titles);
            }
        }
    }
    titles
}

fn parse_print_title_address(address: &str, titles: &mut PrintTitles) {
    for part in address.split(',') {
        let range_part: String = part
            .rsplit('!')
            .next()
            .unwrap_or(part)
            .replace('$', "")
            .trim()
            .to_string();
        let Some((start_str, end_str)) = range_part.split_once(':') else {
            continue;
        };
        if let (Ok(row_start), Ok(row_end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
            titles.rows = Some((row_start.min(row_end), row_start.max(row_end)));
        } else if let (Some(col_start), Some(col_end)) = (
            parse_column_letters(start_str),
            parse_column_letters(end_str),
        ) {
            titles.cols = Some((col_start.min(col_end), col_start.max(col_end)));
        }
    }
}

/// Collect sorted manual row page break positions from a sheet.
pub(super) fn collect_row_breaks(sheet: &umya_spreadsheet::Worksheet) -> Vec<u32> {
    let mut breaks: Vec<u32> = sheet
        .get_row_breaks()
        .get_break_list()
        .iter()
        .filter(|b| *b.get_manual_page_break())
        .map(|b| *b.get_id())
        .collect();
    breaks.sort_unstable();
    breaks.dedup();
    breaks
}

/// Build a lookup of merge info from the sheet's merged cell ranges.
///
/// Returns two structures:
/// - `top_left_map`: top-left coordinate → MergeInfo for each merge
/// - `skip_set`: set of coordinates that are inside a merge but NOT the top-left
pub(super) fn build_merge_maps(
    sheet: &umya_spreadsheet::Worksheet,
) -> (HashMap<CellPos, MergeInfo>, HashSet<CellPos>) {
    let mut top_left_map: HashMap<CellPos, MergeInfo> = HashMap::new();
    let mut skip_set: HashSet<CellPos> = HashSet::new();

    for range in sheet.get_merge_cells() {
        let start_col = range
            .get_coordinate_start_col()
            .map(|c| *c.get_num())
            .unwrap_or(1);
        let start_row = range
            .get_coordinate_start_row()
            .map(|r| *r.get_num())
            .unwrap_or(1);
        let end_col = range
            .get_coordinate_end_col()
            .map(|c| *c.get_num())
            .unwrap_or(start_col);
        let end_row = range
            .get_coordinate_end_row()
            .map(|r| *r.get_num())
            .unwrap_or(start_row);

        let col_span = end_col.saturating_sub(start_col) + 1;
        let row_span = end_row.saturating_sub(start_row) + 1;

        top_left_map.insert((start_col, start_row), MergeInfo { col_span, row_span });

        // Mark all other cells in the range as skip
        for r in start_row..=end_row {
            for c in start_col..=end_col {
                if r != start_row || c != start_col {
                    skip_set.insert((c, r));
                }
            }
        }
    }

    (top_left_map, skip_set)
}

/// Shared context for processing a single XLSX sheet.
pub(super) struct SheetContext {
    pub(super) col_start: u32,
    pub(super) col_end: u32,
    pub(super) num_cols: usize,
    pub(super) column_widths: Vec<f64>,
    /// Printed width of a column with no `<col>` entry, honouring a declared
    /// `defaultColWidth` (issue #621).
    pub(super) default_column_width_pt: f64,
    pub(super) merge_tops: HashMap<(u32, u32), MergeInfo>,
    pub(super) merge_skips: HashSet<(u32, u32)>,
    pub(super) cond_fmt_overrides: HashMap<(u32, u32), crate::parser::cond_fmt::CondFmtOverride>,
    /// The workbook Normal font, which every cell without its own font
    /// inherits (issue #462). `None` when `styles.xml` is unreadable.
    pub(super) normal_font: Option<NormalFont>,
    /// Banded-row shading declared by the sheet's tables (issue #532).
    pub(super) row_stripes: Vec<crate::parser::xlsx::tables::RowStripes>,
    /// The workbook's colour scheme, which `<color theme="N"/>` indexes into
    /// (issue #853). Cloned rather than borrowed so the context stays free of
    /// the workbook's lifetime; it is twelve colours and a font scheme.
    pub(super) theme: Option<umya_spreadsheet::structs::drawing::Theme>,
}

/// First strong bidi direction of a character: Some(true) for right-to-left
/// scripts (Hebrew, Arabic and its supplements), Some(false) for Latin-like
/// letters, None for neutral characters (digits, punctuation, spaces).
fn strong_direction(c: char) -> Option<bool> {
    match c as u32 {
        // Hebrew, Arabic, Syriac, Thaana, and Arabic presentation forms.
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF => Some(true),
        _ if c.is_alphabetic() => Some(false),
        _ => None,
    }
}

/// Map ASCII digits (and separators) to Arabic-Indic digits, as Excel does
/// for number formats carrying a native-digit locale prefix like
/// `[$-3000401]`.
fn to_arabic_indic_digits(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32(0x0660 + (c as u32 - '0' as u32)).unwrap_or(c),
            '.' => '\u{066B}',
            ',' => '\u{066C}',
            _ => c,
        })
        .collect()
}

/// Excel number formats may carry a locale prefix `[$-XXXXXXXX]` whose high
/// byte selects digit shaping (>= 2 substitutes national digits). Arabic
/// primary language (low byte 0x01) then prints Arabic-Indic digits.
fn uses_native_arabic_digits(format_code: &str) -> bool {
    let Some(rest) = format_code.strip_prefix("[$-") else {
        return false;
    };
    let Some(end) = rest.find(']') else {
        return false;
    };
    let Ok(locale) = u64::from_str_radix(&rest[..end], 16) else {
        return false;
    };
    let digit_substitution: u64 = locale >> 24;
    let language_id: u64 = locale & 0xFF;
    digit_substitution >= 2 && language_id == 0x01
}

/// Rough single-line text width estimate in points: ASCII glyphs average
/// about half the font size in Calibri-class fonts, CJK glyphs are full-width.
fn estimate_text_width_pt(runs: &[Run]) -> f64 {
    runs.iter()
        .map(|run| {
            let font_size: f64 = run.style.font_size.unwrap_or(11.0);
            run.text
                .chars()
                .map(|c| {
                    if c.is_ascii() {
                        0.55 * font_size
                    } else {
                        1.05 * font_size
                    }
                })
                .sum::<f64>()
        })
        .sum()
}

/// The width an unwrapped cell's single line may paint across, or `None` when
/// the text fits its own column and needs no special handling.
///
/// `wrapText="false"` means exactly that in Excel: the text never moves to a
/// second line. What varies is only how far it may paint before being clipped —
/// a general/left cell paints on across consecutive empty neighbours to its
/// right, and a cell with nowhere to go is clipped at its own edge. Probed
/// against Excel 16.0: a centred cell whose text runs well past its column,
/// with occupied cells on both sides, prints one clipped line; it does not
/// wrap.
///
/// Restricting this to left alignment made every overflowing centred or
/// right-aligned cell fall through to wrapping, which grew the row and, once
/// rows take the height Excel recorded, overflowed it (issue #615).
#[allow(clippy::too_many_arguments)]
fn compute_spill_width(
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
    col_idx: u32,
    row_idx: u32,
    runs: &[Run],
    cell_alignment: Option<crate::ir::Alignment>,
    col_span: u32,
    umya_cell: Option<&umya_spreadsheet::Cell>,
) -> Option<f64> {
    if runs.is_empty() {
        return None;
    }
    // Explicit wrapText wraps inside the cell instead.
    let has_wrap_text: bool = umya_cell
        .and_then(|cell| cell.get_style().get_alignment().cloned())
        .map(|alignment| *alignment.get_wrap_text())
        .unwrap_or(false);
    if has_wrap_text {
        return None;
    }
    // Embedded line breaks always wrap.
    if runs.iter().any(|run| run.text.contains('\n')) {
        return None;
    }

    // A merged cell never paints past the merge edge: Excel keeps unwrapped
    // text on one line and clips it at the merged width. Apply this even when
    // the text fits — column pagination may clamp the merge to fewer columns
    // on a page, and the line is still not wrapped there.
    //
    // Two caveats this width alone does not carry. The line is clipped at the
    // page-column edge and its remainder is redrawn on the next page-column;
    // we blank that continuation instead (#631). And the renderer lays this
    // width out as a wrapping box rather than a one-line clip, so the fragment
    // left visible is the tail of the text, not its head (#811).
    if col_span > 1 {
        let merged_width: f64 = (col_idx..col_idx + col_span)
            .map(|c| {
                ctx.column_widths
                    .get((c - ctx.col_start) as usize)
                    .copied()
                    .unwrap_or(0.0)
            })
            .sum();
        return Some(merged_width);
    }

    let own_width: f64 = *ctx.column_widths.get((col_idx - ctx.col_start) as usize)?;
    // Leave room for the horizontal cell inset. Taken from the constant rather
    // than written out, so the threshold cannot drift from the padding the
    // cell is actually laid out with (it did when the sides moved to 3pt for
    // issue #657).
    let horizontal_inset: f64 = XLSX_CELL_PADDING.left + XLSX_CELL_PADDING.right;
    if estimate_text_width_pt(runs) <= own_width - horizontal_inset {
        return None;
    }

    // Only a general/left cell paints on into what lies to its right. A centred
    // or right-aligned one is clipped at its own edge — but still on one line.
    let spills_right: bool = matches!(cell_alignment, None | Some(crate::ir::Alignment::Left));
    if !spills_right {
        return Some(own_width);
    }

    let mut total_width: f64 = own_width;
    let mut has_empty_neighbor = false;
    let mut blocked = false;
    for neighbor_col in (col_idx + 1)..=ctx.col_end {
        // Merged regions block the spill like occupied cells do.
        if ctx.merge_skips.contains(&(neighbor_col, row_idx))
            || ctx.merge_tops.contains_key(&(neighbor_col, row_idx))
        {
            blocked = true;
            break;
        }
        let neighbor_is_empty: bool = sheet
            .get_cell((neighbor_col, row_idx))
            .map(|cell| cell.get_formatted_value().is_empty())
            .unwrap_or(true);
        if !neighbor_is_empty {
            blocked = true;
            break;
        }
        total_width += *ctx
            .column_widths
            .get((neighbor_col - ctx.col_start) as usize)
            .unwrap_or(&0.0);
        has_empty_neighbor = true;
    }

    // Every used cell to the right is empty: Excel keeps painting across
    // the virtual empty cells beyond the used range toward the page edge.
    // Give the text the width it needs; the page boundary clips the rest.
    if !blocked {
        let needed_width: f64 = estimate_text_width_pt(runs) + 4.0;
        if needed_width > total_width {
            total_width = needed_width;
            has_empty_neighbor = true;
        }
    }

    // Nowhere to spill: the line is clipped at the cell's own edge rather than
    // wrapped onto a second line, which is what Excel prints.
    if !has_empty_neighbor {
        return Some(own_width);
    }
    Some(total_width)
}

/// Excel's fallback row height when the sheet declares none (Calibri 11).
const EXCEL_DEFAULT_ROW_HEIGHT_PT: f64 = 15.0;

/// Convert an OOXML row height to the whole-point track emitted by native
/// Excel's macOS PDF path. Excel exposes the stored value in points in the
/// worksheet UI, but its PDF grid is vertically compacted and snapped to
/// whole PDF points. Across the ten XLSX audit workbooks, the two repeated
/// fixed heights map consistently: 15pt -> 14pt and 25.5pt -> 23pt.
///
/// "Consistently" is measured, not assumed: reading the golden exports'
/// horizontal rules with `mutool draw -F trace` gives 23.00pt for every
/// `ht=25.5 customHeight="true"` header in all ten, and 14.00pt for every
/// `ht=15 customHeight="true"` row. Issue #658 reports two of them at
/// 24.00pt ("50 px @150 DPI"); that reads the band's *outer* extent — a 23pt
/// track plus the 1pt rule bounding each end is 24pt, or 50px at 150 DPI —
/// where this maps rule centre to rule centre.
///
/// Only fixed tracks go through here. A `customHeight="false"` row is
/// auto-sized, and Excel prints it at the taller of this track and the height
/// its own font needs: the same `ht=15` row measures 14.00pt in Arial 10 and
/// 15.00pt in Malgun Gothic 10 in the golden exports. That font term is not
/// applied yet (issue #709), so Korean auto rows print 1.00pt short.
///
/// Keep this conversion in the XLSX parser rather than the generic table
/// renderer so DOCX/PPTX table heights retain their native semantics.
pub(super) fn native_excel_pdf_row_height(height: f64) -> f64 {
    (height * 0.92).round().max(1.0)
}

/// Cell insets for spreadsheet tables. Excel's native single-line track is
/// asymmetric around bottom-aligned text: 1pt above and 1.5pt below. Typst's
/// default 5pt vertical inset overflowed auto-height rows (issue #396), while
/// a 1pt bottom inset left them about 0.5pt short (issue #411).
///
/// The 3pt sides are Excel's documented 4px inset at 96 DPI, and they were
/// checked against its own exports: with 2pt, the two purely left-aligned text
/// columns across the business mocks landed exactly 1.00pt left of Excel, and
/// 3pt puts them exactly on it (issue #657).
///
/// Both sides moved together, not just the left. Excel's right inset measures
/// ~2.4pt in those exports, inside the same quantisation band as 3pt, and
/// leaving the right at 2pt would make the pair asymmetric — which shifts
/// every *centred* run by half the difference. Measured over all ten business
/// mocks, 3/3 improves every workbook and regresses none, taking the mean
/// absolute x error over 428 text runs from 1.454pt to 1.013pt; 3/2 instead
/// pushed centred and right-aligned columns further out.
/// Horizontal space an icon-set icon takes before its cell's value.
///
/// Excel reserves the icon's advance and then aligns the value in what is left
/// to its right; the icon itself is drawn out of layout here, so without this
/// a centred value centres in the whole cell and lands left of Excel's (#652).
///
/// Fitted, not derived. `10_kpi_tracker_en` is the only workbook in the corpus
/// with icon sets *and* a ground-truth Excel export to measure against: every
/// value in its icon column sat 4.79-5.01pt left of Excel's, and this reserve
/// closes that to within 0.4pt.
///
/// Its icons are `3Arrows`. Other tracked fixtures carry `3TrafficLights1`,
/// `3Flags`, `3Symbols`, `4Rating`, `5ArrowsGray` and more, none of which has
/// a ground truth here, so none was used to derive or check this. A set whose
/// icons are a different width will want a different advance — the honest
/// shape of this is per-icon-set, once there is something to measure it on.
const ICON_SET_VALUE_RESERVE_PT: f64 = 9.6;

pub(super) const XLSX_CELL_PADDING: crate::ir::Insets = crate::ir::Insets {
    top: 1.0,
    right: 3.0,
    bottom: 1.5,
    left: 3.0,
};

/// Whether this cell's wrapped text needs more than the single line its row's
/// mapped track allows.
///
/// `wrapText` only says the cell *may* wrap. What decides is whether the text
/// fits the width it has — its own column, or the whole merge when it spans
/// several — after the horizontal inset the cell is laid out with. An explicit
/// line break always needs a second line.
fn cell_wraps_past_one_line(
    ctx: &SheetContext,
    col_idx: u32,
    col_span: u32,
    runs: &[Run],
    umya_cell: Option<&umya_spreadsheet::Cell>,
) -> bool {
    if runs.is_empty() {
        return false;
    }
    let has_wrap_text: bool = umya_cell
        .and_then(|cell| cell.get_style().get_alignment().cloned())
        .map(|alignment| *alignment.get_wrap_text())
        .unwrap_or(false);
    if !has_wrap_text {
        return false;
    }
    if runs.iter().any(|run| run.text.contains('\n')) {
        return true;
    }
    let available_width: f64 = (col_idx..col_idx + col_span)
        .map(|col| {
            ctx.column_widths
                .get((col - ctx.col_start) as usize)
                .copied()
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        - XLSX_CELL_PADDING.left
        - XLSX_CELL_PADDING.right;
    estimate_text_width_pt(runs) > available_width
}

/// The height a row prints at. A recorded `ht` is the current worksheet
/// height even when `customHeight` is false; rows without one use the sheet's
/// defaultRowHeight. Fixed tracks are calibrated to native Excel's PDF grid.
/// Exception: auto-sized rows (customHeight=false) whose wrapped text needs a
/// second line stay content-driven — our text metrics differ slightly from
/// Excel's and a fixed height could clip a wrapped line.
///
/// The exception is deliberately about the *text*, not about the `wrapText`
/// flag. Keying it on the flag made it fire on rows that never wrap: the ten
/// business mocks set `wrapText` on every data cell, so every ht=15 auto row
/// was sized by its own content box instead of by Excel's track — 15.00pt
/// against Excel's 14.00pt on the six Latin workbooks (issue #710), and
/// 22.32pt against 15.00pt on the Korean ones, where the East Asian line
/// factor compounds it (issue #709).
fn printed_row_height(
    sheet: &umya_spreadsheet::Worksheet,
    row_idx: u32,
    row_wraps_past_one_line: bool,
) -> Option<f64> {
    let row_dimension = sheet.get_row_dimension(&row_idx);
    let is_custom_height: bool = row_dimension
        .map(|row| *row.get_custom_height())
        .unwrap_or(false);
    if !is_custom_height && row_wraps_past_one_line {
        return None;
    }
    let declared_height: Option<f64> = row_dimension
        .map(|row| *row.get_height())
        .filter(|height| *height > 0.0);
    declared_height
        .or_else(|| {
            let sheet_default: f64 = *sheet.get_sheet_format_properties().get_default_row_height();
            if sheet_default > 0.0 {
                Some(sheet_default)
            } else {
                Some(EXCEL_DEFAULT_ROW_HEIGHT_PT)
            }
        })
        .map(native_excel_pdf_row_height)
}

/// The outline a merged range prints: each side taken from the members that
/// sit on that edge, rather than from the top-left member alone.
///
/// Excel writes a range's border format onto its constituent cells, so a rule
/// under a two-row header lands on the *bottom* row's cells and a rule down the
/// right-hand side lands on the right column's — neither of which the top-left
/// member records. Collapsing the range to that one cell dropped both
/// (issue #939).
///
/// One IR border holds a single side each, so the first member along an edge
/// that declares that side wins. Excel lets the members disagree and paints
/// each segment from its own cell; a range whose edge is formatted as a unit —
/// which is what applying a border to a merged range produces — has them all
/// agreeing anyway.
fn merged_range_border(
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
    col: u32,
    row: u32,
    info: &MergeInfo,
) -> Option<CellBorder> {
    let last_col: u32 = col + info.col_span.saturating_sub(1);
    let last_row: u32 = row + info.row_span.saturating_sub(1);
    let side_of = |member_col: u32, member_row: u32| -> Option<CellBorder> {
        sheet
            .get_cell((member_col, member_row))
            .and_then(|cell| extract_cell_borders(cell, ctx.theme.as_ref()))
    };
    let first_along = |cells: &mut dyn Iterator<Item = (u32, u32)>,
                       pick: fn(CellBorder) -> Option<BorderSide>|
     -> Option<BorderSide> {
        cells
            .filter_map(|(c, r)| side_of(c, r).and_then(pick))
            .next()
    };

    let border = CellBorder {
        top: first_along(&mut (col..=last_col).map(|c| (c, row)), |b| b.top),
        bottom: first_along(&mut (col..=last_col).map(|c| (c, last_row)), |b| b.bottom),
        left: first_along(&mut (row..=last_row).map(|r| (col, r)), |b| b.left),
        right: first_along(&mut (row..=last_row).map(|r| (last_col, r)), |b| b.right),
    };
    let CellBorder {
        top,
        bottom,
        left,
        right,
    } = &border;
    (top.is_some() || bottom.is_some() || left.is_some() || right.is_some()).then_some(border)
}

/// Build TableRows for a range of rows in a sheet.
pub(super) fn build_rows_for_range(
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
    row_start: u32,
    row_end: u32,
) -> Vec<TableRow> {
    let num_rows = (row_end - row_start + 1) as usize;
    let mut rows = Vec::with_capacity(num_rows);
    for row_idx in row_start..=row_end {
        let mut cells = Vec::with_capacity(ctx.num_cols);
        let mut row_wraps_past_one_line = false;
        for col_idx in ctx.col_start..=ctx.col_end {
            // Skip cells that are part of a merge but not the top-left
            if ctx.merge_skips.contains(&(col_idx, row_idx)) {
                continue;
            }

            // umya-spreadsheet tuple is (column, row), both 1-indexed
            let umya_cell = sheet.get_cell((col_idx, row_idx));
            let mut value = umya_cell
                .map(|cell| cell.get_formatted_value())
                .unwrap_or_default();
            if let Some(cell) = umya_cell
                && let Some(number_format) = cell.get_style().get_number_format()
                && uses_native_arabic_digits(number_format.get_format_code())
            {
                value = to_arabic_indic_digits(&value);
            }

            // Extract formatting from the cell
            let mut text_style = umya_cell
                .map(|cell| {
                    extract_cell_text_style(cell, ctx.normal_font.as_ref(), ctx.theme.as_ref())
                })
                .unwrap_or_default();
            let (cell_alignment, cell_vertical_align) = umya_cell
                .map(extract_cell_alignment)
                .unwrap_or((None, None));
            let mut background =
                umya_cell.and_then(|cell| extract_cell_background(cell, ctx.theme.as_ref()));
            // A merged range is one IR cell, but Excel composes its outline
            // from the members on each edge, so reading only the top-left one
            // loses every side the range declares elsewhere (issue #939).
            let border = match ctx.merge_tops.get(&(col_idx, row_idx)) {
                Some(info) => merged_range_border(sheet, ctx, col_idx, row_idx, info),
                None => umya_cell.and_then(|cell| extract_cell_borders(cell, ctx.theme.as_ref())),
            };

            // Apply conditional formatting overrides
            let mut data_bar = None;
            let mut icon_text = None;
            let mut icon_color = None;
            if let Some(ovr) = ctx.cond_fmt_overrides.get(&(col_idx, row_idx)) {
                if ovr.background.is_some() {
                    background = ovr.background;
                }
                if ovr.font_color.is_some() {
                    text_style.color = ovr.font_color;
                }
                if let Some(bold) = ovr.bold {
                    text_style.bold = Some(bold);
                }
                data_bar = ovr.data_bar.clone();
                icon_text = ovr.icon_text.clone();
                icon_color = ovr.icon_color;
            }

            // Rich-text shared strings carry per-run formatting (bold labels,
            // per-run fonts/colors) that the cell's single xf style loses —
            // emit one IR run per rich run instead of flattening.
            let rich_text: Option<umya_spreadsheet::RichText> =
                umya_cell.and_then(|cell| cell.get_cell_value().get_raw_value().get_rich_text());
            let runs: Vec<Run> = if let Some(rich_text) = rich_text {
                rich_text
                    .get_rich_text_elements()
                    .iter()
                    .filter(|element| !element.get_text().is_empty())
                    .map(|element| Run {
                        text: element.get_text().to_string(),
                        style: element
                            .get_run_properties()
                            .map(|font| apply_rich_run_font(&text_style, font, ctx.theme.as_ref()))
                            .unwrap_or_else(|| text_style.clone()),
                        href: None,
                        footnote: None,
                    })
                    .collect()
            } else if value.is_empty() {
                Vec::new()
            } else {
                vec![Run {
                    text: value,
                    style: text_style,
                    href: None,
                    footnote: None,
                }]
            };

            // Excel's "general" horizontal alignment follows the text
            // direction: cells whose text starts with a right-to-left script
            // print right-aligned.
            let cell_alignment: Option<crate::ir::Alignment> = cell_alignment.or_else(|| {
                runs.iter()
                    .flat_map(|run| run.text.chars())
                    .find_map(strong_direction)
                    .filter(|is_rtl| *is_rtl)
                    .map(|_| crate::ir::Alignment::Right)
            });
            let paragraph_alignment = cell_alignment.or_else(|| {
                umya_cell
                    .and_then(|cell| cell.get_value_number())
                    .map(|_| crate::ir::Alignment::Right)
            });

            let (col_span, row_span) = if let Some(info) = ctx.merge_tops.get(&(col_idx, row_idx)) {
                (info.col_span, info.row_span)
            } else {
                (1, 1)
            };

            let spill_width: Option<f64> = compute_spill_width(
                sheet,
                ctx,
                col_idx,
                row_idx,
                &runs,
                paragraph_alignment,
                col_span,
                umya_cell,
            );

            row_wraps_past_one_line |=
                cell_wraps_past_one_line(ctx, col_idx, col_span, &runs, umya_cell);

            let content = if runs.is_empty() {
                Vec::new()
            } else {
                vec![Block::Paragraph(Paragraph {
                    style: ParagraphStyle {
                        alignment: paragraph_alignment,
                        ..ParagraphStyle::default()
                    },
                    runs,
                })]
            };

            // An explicit cell fill wins; the table's banding only shows
            // through where the cell declares none (issue #532).
            let background: Option<Color> = background.or_else(|| {
                ctx.row_stripes
                    .iter()
                    .find_map(|stripes| stripes.fill_at(col_idx, row_idx))
            });

            cells.push(TableCell {
                content,
                col_span,
                row_span,
                border,
                background,
                data_bar,
                // An icon is drawn out of layout at the cell's left edge, so
                // it consumes no width and a centred value centres in the
                // whole cell. Excel reserves the icon's advance first and
                // aligns the value in what remains to its right, which is
                // where the extra left inset comes from (issue #652).
                padding: icon_text.as_ref().map(|_| Insets {
                    left: XLSX_CELL_PADDING.left + ICON_SET_VALUE_RESERVE_PT,
                    ..XLSX_CELL_PADDING
                }),
                icon_text,
                icon_color,
                spill_width,
                vertical_align: cell_vertical_align,
            });
        }

        let height: Option<f64> = printed_row_height(sheet, row_idx, row_wraps_past_one_line);

        rows.push(TableRow { cells, height });
    }
    rows
}

/// The point metric every column width is scaled by. Excel derives it from
/// the workbook Normal font; cell fonts do not participate (issue #366).
/// When `xl/styles.xml` was unreadable, fall back to the dominant cell font
/// — which on a sheet with no cells lands on the legacy 5.25pt default.
/// Shared by populated and drawing-only sheets so both scale from the same
/// digit metric (issue #620); drawing-only sheets still price every column at
/// the default width because their context carries no `<cols>` overrides.
pub(super) fn resolve_column_unit_pt(
    sheet: &umya_spreadsheet::Worksheet,
    normal_font: Option<&NormalFont>,
) -> f64 {
    normal_font
        .map(|font| column_unit_pt(&font.family, font.size_pt))
        .unwrap_or_else(|| sheet_column_unit_pt(sheet))
}

/// Prepare the shared context for processing a sheet (dimensions, merges, styles, etc.).
/// Returns (SheetContext, row_start, row_end) or None if the sheet is empty.
pub(super) fn prepare_sheet_context(
    sheet: &umya_spreadsheet::Worksheet,
    normal_font: Option<&NormalFont>,
    raw_cond_fmt_hints: Option<&super::cond_fmt_raw::RawCondFmtHints>,
    defined_names: &HashMap<String, String>,
    row_stripes: Vec<crate::parser::xlsx::tables::RowStripes>,
    theme: Option<&umya_spreadsheet::structs::drawing::Theme>,
) -> Option<(SheetContext, u32, u32)> {
    let (mut max_col, mut max_row) = sheet.get_highest_column_and_row();
    if max_col == 0 || max_row == 0 {
        return None;
    }

    // Expand grid to include the extent of all merged ranges
    for range in sheet.get_merge_cells() {
        if let Some(c) = range.get_coordinate_end_col() {
            max_col = max_col.max(*c.get_num());
        }
        if let Some(r) = range.get_coordinate_end_row() {
            max_row = max_row.max(*r.get_num());
        }
    }

    // Check for print area — limit to that range if defined
    let print_area = find_print_area(sheet);
    let (col_start, col_end, row_start, row_end) = if let Some(pa) = print_area {
        (pa.start_col, pa.end_col, pa.start_row, pa.end_row)
    } else {
        (1, max_col, 1, max_row)
    };

    let unit_pt: f64 = resolve_column_unit_pt(sheet, normal_font);
    let default_width_pt: f64 = default_column_width_pt(
        declared_default_column_width(sheet),
        declared_base_column_width(sheet),
        unit_pt,
    );
    let column_widths: Vec<f64> = (col_start..=col_end)
        .map(|col| {
            sheet
                .get_column_dimension_by_number(&col)
                .map(|c| column_width_to_pt(*c.get_width(), unit_pt))
                .unwrap_or(default_width_pt)
        })
        .collect();

    let (merge_tops, merge_skips) = build_merge_maps(sheet);
    let cond_fmt_overrides =
        build_cond_fmt_overrides(sheet, raw_cond_fmt_hints, defined_names, theme);
    let num_cols = (col_end - col_start + 1) as usize;

    Some((
        SheetContext {
            col_start,
            col_end,
            num_cols,
            column_widths,
            default_column_width_pt: default_width_pt,
            merge_tops,
            merge_skips,
            cond_fmt_overrides,
            normal_font: normal_font.cloned(),
            row_stripes,
            theme: theme.cloned(),
        },
        row_start,
        row_end,
    ))
}

//! Column-wise pagination for sheets wider than the printable page.
//!
//! Excel prints columns that overflow the page width on subsequent pages
//! (default order: down, then over). office2pdf previously clipped them at
//! the right page edge, silently losing content.

use crate::ir::{Block, HFInline, HeaderFooter, SheetPage, Table, TableCell, TableRow};

/// Upper bound on overflow pages per sheet chunk. Pathological sheets (used
/// ranges thousands of columns wide) would otherwise explode into thousands
/// of pages and blow the Typst compiler's stack; columns beyond the cap stay
/// on the last page (clipped, the pre-pagination behavior).
const MAX_COLUMN_GROUPS: usize = 12;

/// Split a sheet page into column groups that each fit the printable width.
/// Returns the page unchanged when everything fits. `title_columns` is the
/// 0-based inclusive-exclusive range of print-title columns (from
/// `_xlnm.Print_Titles`) repeated at the left of every overflow page.
pub(super) fn split_sheet_page_by_width(
    page: SheetPage,
    title_columns: Option<(usize, usize)>,
    fit_to_width: Option<u32>,
    header_footer_scales_with_doc: bool,
) -> Vec<SheetPage> {
    let page: SheetPage = fit_page_to_width(page, fit_to_width, header_footer_scales_with_doc);
    let printable_width: f64 = page.size.width - page.margins.left - page.margins.right;
    let total_width: f64 = page.table.column_widths.iter().sum();
    if total_width <= printable_width || page.table.column_widths.len() <= 1 {
        return vec![page];
    }

    let title_columns: Option<(usize, usize)> = title_columns
        .map(|(start, end)| (start, end.min(page.table.column_widths.len())))
        .filter(|(start, end)| start < end);
    // Reserve the repeated title width so overflow groups still fit the
    // page. The first group holds the title columns physically (they never
    // get prepended to it), so it packs against the full printable width —
    // reserving there too underpacked page 1 by the title width (issue #623
    // adversarial review, finding 3).
    let title_width: f64 = title_columns
        .map(|(start, end)| page.table.column_widths[start..end].iter().sum())
        .unwrap_or(0.0);
    let widest_column: f64 = page.table.column_widths.iter().cloned().fold(0.0, f64::max);
    let first_group_packing_width: f64 = printable_width.max(widest_column);
    let overflow_packing_width: f64 = (printable_width - title_width).max(widest_column);

    let mut groups: Vec<(usize, usize)> = column_groups(
        &page.table.column_widths,
        first_group_packing_width,
        overflow_packing_width,
    );
    if groups.len() <= 1 {
        return vec![page];
    }
    if groups.len() > MAX_COLUMN_GROUPS {
        let column_count = page.table.column_widths.len();
        groups.truncate(MAX_COLUMN_GROUPS);
        if let Some(last) = groups.last_mut() {
            last.1 = column_count;
        }
    }

    let title_table: Option<Table> =
        title_columns.map(|(start, end)| slice_table_columns(&page.table, start, end));

    let mut result: Vec<SheetPage> = Vec::with_capacity(groups.len());
    for (index, &(start, end)) in groups.iter().enumerate() {
        let mut table: Table = slice_table_columns(&page.table, start, end);
        // Excel repeats title columns on pages that no longer show them.
        if let (Some(title_table), Some((title_start, _))) = (title_table.as_ref(), title_columns)
            && start > title_start
        {
            table = prepend_title_columns(title_table, table);
        }
        result.push(SheetPage {
            name: page.name.clone(),
            size: page.size,
            margins: page.margins,
            table,
            header: page.header.clone(),
            footer: page.footer.clone(),
            // Charts and images anchor to rows of the first column group only.
            charts: if index == 0 {
                page.charts.clone()
            } else {
                Vec::new()
            },
            images: if index == 0 {
                page.images.clone()
            } else {
                Vec::new()
            },
            text_boxes: if index == 0 {
                page.text_boxes.clone()
            } else {
                Vec::new()
            },
        });
    }
    result
}

/// Concatenate the repeated title columns before a column group's table.
/// Shrink a sheet until its columns fit the pages `fitToWidth` allows.
///
/// A sheet with `<pageSetUpPr fitToPage="1"/>` and `fitToWidth="1"` asks Excel
/// to scale it onto one page wide rather than to spill the overflow onto a
/// second strip. Reading neither attribute printed the repository workbook on
/// 53 pages where Excel prints 23 (issue #530).
///
/// Excel scales the whole sheet, not the columns alone, so the row heights and
/// the type scale with the widths — the audited sheet's 10pt body text prints
/// at 7.50pt, the same 0.75 the columns take.
///
/// Excel never scales *up* to fill a page, so a sheet that already fits is
/// left alone.
fn fit_page_to_width(
    page: SheetPage,
    fit_to_width: Option<u32>,
    header_footer_scales_with_doc: bool,
) -> SheetPage {
    let Some(pages_wide) = fit_to_width.filter(|pages| *pages > 0) else {
        return page;
    };
    let printable_width: f64 = page.size.width - page.margins.left - page.margins.right;
    let total_width: f64 = page.table.column_widths.iter().sum();
    if printable_width <= 0.0 || total_width <= 0.0 {
        return page;
    }
    // Excel's auto-fit scale is a whole percent, truncated rather than rounded
    // so the content is guaranteed to fit. Keeping the raw ratio leaves every
    // derived type size a fraction of a point off the printed sheet — the
    // audited sheet came out at 7.55pt against Excel's 7.50pt.
    let exact_scale: f64 = (printable_width * f64::from(pages_wide)) / total_width;
    let scale: f64 = (exact_scale * 100.0).floor() / 100.0;
    if scale >= 1.0 {
        return page;
    }
    scale_sheet_page(page, scale, header_footer_scales_with_doc)
}

/// Multiply a sheet's widths, heights, type sizes, and cell padding by
/// `scale`.
///
/// Padding has to scale with the rest: it is a fixed per-row overhead, so
/// leaving it at full size while the rows shrink costs a constant slice of
/// every row and accumulates into whole extra pages over a long sheet.
///
/// The header and footer scale too, unless the sheet opts out.
/// `headerFooter/@scaleWithDoc` defaults to 1 (ECMA-376 §18.3.1.46), so Excel
/// shrinks them with the sheet; leaving them at full size printed the Gantt
/// template's 8pt `&8` run beside 5.85pt body text (issue #940).
fn scale_sheet_page(
    mut page: SheetPage,
    scale: f64,
    header_footer_scales_with_doc: bool,
) -> SheetPage {
    if header_footer_scales_with_doc {
        for header_footer in [page.header.as_mut(), page.footer.as_mut()]
            .into_iter()
            .flatten()
        {
            scale_header_footer_font_sizes(header_footer, scale);
        }
    }
    for width in &mut page.table.column_widths {
        *width *= scale;
    }
    for row in &mut page.table.rows {
        if let Some(height) = row.height.as_mut() {
            *height *= scale;
        }
        for cell in &mut row.cells {
            if let Some(padding) = cell.padding.as_mut() {
                padding.top *= scale;
                padding.right *= scale;
                padding.bottom *= scale;
                padding.left *= scale;
            }
            for block in &mut cell.content {
                scale_block_font_sizes(block, scale);
            }
        }
    }
    page
}

/// Scale every run of a header or footer.
///
/// A run that states no size takes the renderer's default rather than being
/// left alone: it is the size the run actually prints at, and skipping it left
/// the Gantt template's leading `_x000D_` at 11pt while everything around it
/// shrank (issue #940).
fn scale_header_footer_font_sizes(header_footer: &mut HeaderFooter, scale: f64) {
    for paragraph in &mut header_footer.paragraphs {
        for element in &mut paragraph.elements {
            if let HFInline::Run(run) = element {
                let size_pt: f64 = run
                    .style
                    .font_size
                    .unwrap_or(crate::defaults::TYPST_DEFAULT_FONT_SIZE_PT);
                run.style.font_size = Some(size_pt * scale);
            }
        }
    }
}

fn scale_block_font_sizes(block: &mut Block, scale: f64) {
    match block {
        Block::Paragraph(paragraph) => {
            for run in &mut paragraph.runs {
                if let Some(size) = run.style.font_size.as_mut() {
                    *size *= scale;
                }
            }
        }
        Block::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    for nested in &mut cell.content {
                        scale_block_font_sizes(nested, scale);
                    }
                }
            }
        }
        _ => {}
    }
}

fn prepend_title_columns(title_table: &Table, group_table: Table) -> Table {
    let mut column_widths: Vec<f64> = title_table.column_widths.clone();
    column_widths.extend(group_table.column_widths.iter().copied());

    let rows: Vec<TableRow> = title_table
        .rows
        .iter()
        .zip(group_table.rows)
        .map(|(title_row, group_row)| {
            let mut cells: Vec<TableCell> = title_row.cells.clone();
            cells.extend(group_row.cells);
            TableRow {
                minimum_height: None,
                cells,
                height: group_row.height,
            }
        })
        .collect();

    Table {
        rows,
        column_widths,
        ..group_table
    }
}

/// Greedily pack columns left-to-right into groups whose summed width fits
/// their capacity; every group holds at least one column. The first group
/// packs against `first_group_width` (the full printable width — it shows
/// the title columns in place); later groups pack against
/// `overflow_group_width`, which reserves room for the prepended titles.
fn column_groups(
    column_widths: &[f64],
    first_group_width: f64,
    overflow_group_width: f64,
) -> Vec<(usize, usize)> {
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut start: usize = 0;
    let mut acc: f64 = 0.0;
    for (index, width) in column_widths.iter().enumerate() {
        let capacity: f64 = if groups.is_empty() {
            first_group_width
        } else {
            overflow_group_width
        };
        if index > start && acc + width > capacity {
            groups.push((start, index));
            start = index;
            acc = 0.0;
        }
        acc += width;
    }
    groups.push((start, column_widths.len()));
    groups
}

/// Build a table containing only columns `[start, end)`, truncating cell
/// spans at the group boundary. A merged cell that starts before the group
/// keeps its geometry (background/border) but blanks its content.
///
/// That blanking is a stopgap, not a match for how a spreadsheet application
/// prints the continuation. A LibreOffice render of
/// `tests/fixtures/xlsx/merged_row_overflows_page_column.xlsx` redraws the
/// merge's line on the following page-column at a negative x so its tail lands
/// there, rather than leaving the cell empty. Reproducing that is #631; no
/// native Excel export has been measured yet, so the exact geometry is
/// corroborated rather than settled.
fn slice_table_columns(table: &Table, start: usize, end: usize) -> Table {
    let column_count: usize = table.column_widths.len();
    // Tracks rows still covered by a row-spanning cell, per column.
    let mut rowspan_remaining: Vec<usize> = vec![0; column_count];

    let mut rows: Vec<TableRow> = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        let mut column_cursor: usize = 0;
        let mut cells: Vec<TableCell> = Vec::new();

        for cell in &row.cells {
            while column_cursor < column_count && rowspan_remaining[column_cursor] > 0 {
                rowspan_remaining[column_cursor] -= 1;
                column_cursor += 1;
            }
            if column_cursor >= column_count {
                break;
            }

            let span: usize = cell.col_span.max(1) as usize;
            let cell_start: usize = column_cursor;
            let cell_end: usize = (column_cursor + span).min(column_count);

            if cell.row_span > 1 {
                for occupied in rowspan_remaining.iter_mut().take(cell_end).skip(cell_start) {
                    *occupied = (cell.row_span - 1) as usize;
                }
            }

            let overlap_start: usize = cell_start.max(start);
            let overlap_end: usize = cell_end.min(end);
            if overlap_start < overlap_end {
                let mut sliced: TableCell = cell.clone();
                sliced.col_span = (overlap_end - overlap_start) as u32;
                if cell_start < start {
                    // Continuation of a merge that began on an earlier page.
                    sliced.content = Vec::new();
                    sliced.spill_width = None;
                } else if let Some(spill) = sliced.spill_width {
                    // The spill width was measured against the whole sheet, so
                    // it can reach far past the columns this page actually
                    // carries — on a sheet wide enough to split, past the paper
                    // edge, losing the ink entirely (#631). Clamp it to what
                    // remains of the group from this cell's left edge.
                    let available: f64 = table.column_widths[overlap_start..end].iter().sum();
                    sliced.spill_width = Some(spill.min(available));
                }
                cells.push(sliced);
            }

            column_cursor = cell_end;
        }

        // Columns occupied only by rowspans still need their counters advanced.
        while column_cursor < column_count {
            if rowspan_remaining[column_cursor] > 0 {
                rowspan_remaining[column_cursor] -= 1;
            }
            column_cursor += 1;
        }

        rows.push(TableRow {
            minimum_height: None,
            cells,
            height: row.height,
        });
    }

    Table {
        rows,
        column_widths: table.column_widths[start..end].to_vec(),
        header_row_count: table.header_row_count,
        non_repeating_header_row_count: table.non_repeating_header_row_count,
        alignment: table.alignment,
        default_cell_padding: table.default_cell_padding,
        use_content_driven_row_heights: table.use_content_driven_row_heights,
        default_vertical_align: table.default_vertical_align,
        seats_bottom_aligned_text_on_descender: table.seats_bottom_aligned_text_on_descender,
        paints_borders_inside_boundary: table.paints_borders_inside_boundary,
        prints_gridlines: table.prints_gridlines,
        prints_headings: table.prints_headings,
    }
}

#[cfg(test)]
#[path = "xlsx_pagination_tests.rs"]
mod tests;

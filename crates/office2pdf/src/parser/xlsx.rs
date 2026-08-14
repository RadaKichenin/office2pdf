use std::io::Cursor;

use crate::config::ConvertOptions;
use crate::error::{ConvertError, ConvertWarning};
use crate::ir::{
    Document, ImageData, Margins, Metadata, Page, PageSize, SheetPage, StyleSheet, Table,
    TableBorderPaintModel, TableRow,
};
use crate::parser::Parser;

#[path = "xlsx_cond_fmt_raw.rs"]
pub(crate) mod cond_fmt_raw;

#[path = "xlsx_fit_to_page.rs"]
mod fit_to_page;
#[path = "xlsx_print_headings.rs"]
mod print_headings;
#[path = "xlsx_print_options.rs"]
mod print_options;
#[path = "xlsx_tables.rs"]
mod tables;
#[path = "xlsx_cells.rs"]
mod xlsx_cells;
#[path = "xlsx_drawing.rs"]
mod xlsx_drawing;
#[path = "xlsx_hf.rs"]
mod xlsx_hf;
#[path = "xlsx_pagination.rs"]
mod xlsx_pagination;
#[path = "xlsx_style.rs"]
pub(crate) mod xlsx_style;

use self::xlsx_cells::*;
use self::xlsx_drawing::*;
use self::xlsx_hf::*;

// Re-export cell address types for cond_fmt module.
pub(crate) use self::xlsx_cells::{CellPos, CellRange, parse_cell_ref};

/// Parser for XLSX (Office Open XML Excel) spreadsheets.
/// Print margins for a sheet: the worksheet's explicit `<pageMargins>` when
/// present, otherwise Excel's defaults (0.7" left/right, 0.75" top/bottom).
/// umya leaves absent margin attributes at 0.0, which is not a value Excel
/// ever writes, so ≤0 means "not specified".
fn sheet_print_margins(sheet: &umya_spreadsheet::Worksheet) -> Margins {
    let page_margins = sheet.get_page_margins();
    let inches_to_pt = |inches: f64, default_pt: f64| -> f64 {
        if inches > 0.0 {
            inches * 72.0
        } else {
            default_pt
        }
    };
    Margins {
        top: inches_to_pt(*page_margins.get_top(), 54.0),
        bottom: inches_to_pt(*page_margins.get_bottom(), 54.0),
        left: inches_to_pt(*page_margins.get_left(), 50.4),
        right: inches_to_pt(*page_margins.get_right(), 50.4),
    }
}

/// Map an OOXML worksheet paper-size code to portrait dimensions in points.
///
/// Code 0 is not a paper size — it is the zero umya leaves in an unset
/// `UInt32Value`, so it means `paperSize` was never written, either because
/// `<pageSetup>` omits the attribute or because the element is absent
/// entirely. ECMA-376 defaults `paperSize` to 1, US Letter, and Excel prints
/// such a sheet on Letter; A4 left it 16.7pt narrow and 49.9pt tall, which
/// repaginated the whole sheet (issue #717).
///
/// An unrecognised *positive* code is a different case: the file names a paper
/// this table does not model, and nothing in the schema says what to
/// substitute, so those keep the renderer's A4 default.
fn worksheet_paper_size(code: u32) -> PageSize {
    let (width, height) = match code {
        0 => (612.0, 792.0),        // `paperSize` omitted — schema default
        1 | 2 => (612.0, 792.0),    // Letter / Letter Small
        3 => (792.0, 1224.0),       // Tabloid
        4 => (1224.0, 792.0),       // Ledger
        5 => (612.0, 1008.0),       // Legal
        6 => (396.0, 612.0),        // Statement
        7 => (522.0, 756.0),        // Executive
        8 => (841.89, 1190.55),     // A3
        9 | 10 => (595.28, 841.89), // A4 / A4 Small
        11 => (419.53, 595.28),     // A5
        12 => (728.50, 1031.81),    // B4 (JIS)
        13 => (515.91, 728.50),     // B5 (JIS)
        _ => return PageSize::default(),
    };
    PageSize { width, height }
}

/// The number of pages wide a sheet asks to be scaled onto, if it asks.
///
/// `fitToWidth` counts for nothing unless `<pageSetUpPr fitToPage="1"/>` is
/// also set — Excel writes the attribute into sheets that print at 100% too
/// (issue #530). `fit_to_page::sheets_fit_to_width` has already applied that
/// gate and ECMA-376's default of one page wide.
///
/// `fitToHeight="0"` leaves the row direction free, which is the shape the
/// audited workbooks use; only the width bound is modelled here. A zero here
/// is Excel's unconstrained width, so it scales nothing.
fn sheet_fit_to_width(
    sheet_name: &str,
    fitting_sheets: &std::collections::HashMap<String, fit_to_page::SheetFitToPage>,
) -> Option<u32> {
    fitting_sheets
        .get(sheet_name)
        .map(|fit| fit.pages_wide)
        .filter(|pages| *pages > 0)
}

/// Whether the sheet's header and footer shrink with its fit-to-page scale.
///
/// `headerFooter/@scaleWithDoc` defaults to 1, so a sheet that states nothing —
/// including one with no `<headerFooter>` at all — scales (issue #940).
fn sheet_header_footer_scales_with_doc(
    sheet_name: &str,
    fitting_sheets: &std::collections::HashMap<String, fit_to_page::SheetFitToPage>,
) -> bool {
    fitting_sheets
        .get(sheet_name)
        .is_none_or(|fit| fit.header_footer_scales_with_doc)
}

/// The first sheet the caller asked for, honouring `sheet_names`.
fn first_selected_sheet<'a>(
    book: &'a umya_spreadsheet::Spreadsheet,
    options: &ConvertOptions,
) -> Option<&'a umya_spreadsheet::Worksheet> {
    book.get_sheet_collection().iter().find(|sheet| {
        options
            .sheet_names
            .as_ref()
            .is_none_or(|names| names.iter().any(|name| name == sheet.get_name()))
    })
}

/// The single page a workbook prints when none of its sheets has a used range.
///
/// The sheet loop skips a sheet with no used cells and no drawings, so such a
/// workbook reached codegen with no pages at all and the Typst default — a
/// blank A4 — stood in for it. That default answers to nothing in the file, so
/// a sheet declaring `<pageSetup paperSize="1"/>` printed on A4 (issue #632).
/// A blank page comes out either way; this one is the size the file asks for.
///
/// The page stays blank. The sheet's header and footer are deliberately not
/// carried onto it: the ground truth for an empty sheet is a blank page, and
/// Excel itself refuses to print one at all ("nothing found to print"), so
/// there is no observed behaviour that puts running text on a page with no
/// cells behind it. Rendering the paper the file asks for is what the evidence
/// supports; inventing content for it is not.
fn empty_workbook_page(
    book: &umya_spreadsheet::Spreadsheet,
    options: &ConvertOptions,
) -> Option<SheetPage> {
    let sheet = first_selected_sheet(book, options)?;
    Some(SheetPage {
        name: sheet.get_name().to_string(),
        size: sheet_page_size(sheet),
        margins: sheet_print_margins(sheet),
        table: Table::default(),
        header: None,
        footer: None,
        charts: Vec::new(),
        images: Vec::new(),
        text_boxes: Vec::new(),
    })
}

/// Preserve a worksheet's paper size and landscape orientation in the IR.
fn sheet_page_size(sheet: &umya_spreadsheet::Worksheet) -> PageSize {
    let page_setup = sheet.get_page_setup();
    let size = worksheet_paper_size(*page_setup.get_paper_size());
    if matches!(
        page_setup.get_orientation(),
        umya_spreadsheet::structs::OrientationValues::Landscape
    ) {
        PageSize {
            width: size.height,
            height: size.width,
        }
    } else {
        size
    }
}

/// Convert absolute print-title columns to 0-based indices within the
/// rendered column range, half-open. None when the titles fall outside it.
fn title_column_indices(print_titles: PrintTitles, ctx: &SheetContext) -> Option<(usize, usize)> {
    let (col_start, col_end) = print_titles.cols?;
    if col_end < ctx.col_start || col_start > ctx.col_end {
        return None;
    }
    let start_idx = col_start.max(ctx.col_start) - ctx.col_start;
    let end_idx = col_end.min(ctx.col_end) - ctx.col_start + 1;
    Some((start_idx as usize, end_idx as usize))
}

/// The default height of a row that records no height of its own.
///
/// Excel does not honour the `sheetFormatPr defaultRowHeight` hint for such
/// rows unless the sheet marks it `customHeight`: it recomputes the default
/// from the workbook's Normal font (issue #715). On the `WithDrawing.xlsx`
/// native export (declared default 15, no `customHeight`) every
/// dimension-less row lays out at 17pt — an 8-row picture anchor measures
/// 148pt against the 132pt the declared hint gives — which is the same
/// recompute the printed grid was later measured to apply, so both paths
/// read it from one place (issue #1047).
fn default_row_height_pt(
    sheet: &umya_spreadsheet::Worksheet,
    normal_font: Option<&xlsx_cells::NormalFont>,
) -> f64 {
    xlsx_cells::recomputed_default_row_height_pt(sheet, normal_font)
        .unwrap_or_else(|| xlsx_cells::declared_default_row_height_pt(sheet))
}

/// Convert a raw drawing anchor into a render-ready image: 1-indexed anchor
/// row plus a size in points resolved against the sheet's column widths and
/// row heights (twoCellAnchor) or the declared extent (oneCellAnchor).
fn anchored_image(
    anchor: xlsx_drawing::RawImageAnchor,
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
) -> crate::ir::SheetImage {
    const EMU_PER_PT: f64 = 12_700.0;

    let column_width_at = |col_zero_based: u32| -> f64 {
        let col: u32 = col_zero_based + 1;
        if col >= ctx.col_start && col <= ctx.col_end {
            ctx.column_widths
                .get((col - ctx.col_start) as usize)
                .copied()
                .unwrap_or(0.0)
        } else {
            ctx.default_column_width_pt
        }
    };
    // Excel resolves a drawing's anchor against the worksheet's own row
    // heights, not against the vertically compacted track its PDF grid
    // prints. Measured on the native export: a two-cell anchor spanning six
    // 18pt rows is 108pt tall, while the printed grid rounds those rows to
    // 17pt each. Running the anchor through the grid conversion left every
    // shape 6pt short (issue #460).
    let row_height_at = |row_zero_based: u32| -> f64 {
        sheet
            .get_row_dimension(&(row_zero_based + 1))
            .map(|row| *row.get_height())
            .filter(|height| *height > 0.0)
            .unwrap_or_else(|| default_row_height_pt(sheet, ctx.normal_font.as_ref()))
    };

    let (width, height): (f64, f64) =
        if let Some((to_col, to_col_off, to_row, to_row_off)) = anchor.to {
            let width: f64 = (anchor.from_col..to_col).map(column_width_at).sum::<f64>()
                - anchor.from_col_off_emu as f64 / EMU_PER_PT
                + to_col_off as f64 / EMU_PER_PT;
            let height: f64 = (anchor.from_row..to_row).map(row_height_at).sum::<f64>()
                - anchor.from_row_off_emu as f64 / EMU_PER_PT
                + to_row_off as f64 / EMU_PER_PT;
            (width.max(1.0), height.max(1.0))
        } else if let Some((cx, cy)) = anchor.ext_emu {
            (
                (cx as f64 / EMU_PER_PT).max(1.0),
                (cy as f64 / EMU_PER_PT).max(1.0),
            )
        } else {
            (100.0, 100.0)
        };

    let x_offset_pt: f64 = (0..anchor.from_col).map(column_width_at).sum::<f64>()
        + anchor.from_col_off_emu as f64 / EMU_PER_PT;
    // Excel places a drawing at absolute worksheet coordinates, so the
    // vertical origin is the summed height of every row above the anchor
    // row plus its `xdr:rowOff` - the same geometry the width and height
    // already use (issue #474).
    let y_offset_pt: f64 = (0..anchor.from_row).map(row_height_at).sum::<f64>()
        + anchor.from_row_off_emu as f64 / EMU_PER_PT;

    let image = ImageData {
        rotation_deg: None,
        flip_h: false,
        flip_v: false,
        data: anchor.data,
        format: anchor.format,
        width: Some(width),
        height: Some(height),
        crop: None,
        stroke: None,
        alignment: None,
        clip_shape: None,
        shadow: None,
        paragraph_spacing: None,
    };
    crate::ir::SheetImage {
        anchor_row: anchor.from_row + 1,
        x_offset_pt,
        y_offset_pt,
        image,
        clip_width_pt: None,
    }
}

/// Context stand-in for sheets with no used cells, so drawing anchors can
/// still resolve against the sheet's column widths and row heights.
///
/// Such a sheet may still declare `<cols>`, and those widths are read here the
/// way `prepare_sheet_context` reads them for a populated sheet. Only a sheet
/// declaring none falls back to the default width for every column.
///
/// The column metric must come from the workbook Normal font exactly as it
/// does for populated sheets: hardcoding a 7px digit metric laid every
/// drawing-only sheet out on 44.2575pt columns while the workbook's own
/// Calibri-11 metric prices default columns at 53pt, shrinking anchors and
/// distorting picture aspect ratios (issue #620). Without a readable Normal
/// font the shared fallback inspects cell fonts, finds none on an empty
/// sheet, and keeps the legacy 5.25pt unit.
fn empty_sheet_context(
    sheet: &umya_spreadsheet::Worksheet,
    normal_font: Option<&NormalFont>,
    theme: Option<&umya_spreadsheet::structs::drawing::Theme>,
) -> SheetContext {
    let unit_pt: f64 = resolve_column_unit_pt(sheet, normal_font);
    let default_width_pt: f64 = default_column_width_pt(
        declared_default_column_width(sheet),
        declared_base_column_width(sheet),
        unit_pt,
    );

    // A sheet with no used cells can still declare `<cols>`, and a drawing
    // anchored to those columns is placed against their widths. Leaving the
    // window empty priced every column at the default: on a probe declaring
    // width=20, an anchored picture came out 141pt wide at x=144.40 where a
    // reference render puts it 340pt wide at x=280.63 (issue #714).
    let declared: Vec<(u32, f64)> = sheet
        .get_column_dimensions()
        .iter()
        .map(|column| (*column.get_col_num(), *column.get_width()))
        .collect();
    let (col_start, col_end) = match (
        declared.iter().map(|(col, _)| *col).min(),
        declared.iter().map(|(col, _)| *col).max(),
    ) {
        (Some(first), Some(last)) => (first, last),
        // No `<cols>` either: keep the empty window, so every column falls
        // through to the default width as before.
        _ => (1, 0),
    };
    let column_widths: Vec<f64> = (col_start..=col_end)
        .map(|col| {
            sheet
                .get_column_dimension_by_number(&col)
                .map(|column| column_width_to_pt(*column.get_width(), unit_pt))
                .unwrap_or(default_width_pt)
        })
        .collect();

    SheetContext {
        col_start,
        col_end,
        num_cols: column_widths.len(),
        column_widths,
        default_column_width_pt: default_width_pt,
        merge_tops: std::collections::HashMap::new(),
        merge_skips: std::collections::HashSet::new(),
        cond_fmt_overrides: std::collections::HashMap::new(),
        normal_font: normal_font.cloned(),
        row_stripes: Vec::new(),
        theme: theme.cloned(),
    }
}

/// Convert a raw text-box anchor into a render-ready box, sized like images.
fn anchored_text_box(
    anchor: xlsx_drawing::RawTextBoxAnchor,
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
) -> crate::ir::SheetTextBox {
    let placed = anchored_image(
        xlsx_drawing::RawImageAnchor {
            from_row: anchor.geometry.from_row,
            from_col: anchor.geometry.from_col,
            from_col_off_emu: anchor.geometry.from_col_off_emu,
            from_row_off_emu: anchor.geometry.from_row_off_emu,
            to: anchor.geometry.to,
            ext_emu: anchor.geometry.ext_emu,
            data: Vec::new(),
            format: crate::ir::ImageFormat::Png,
        },
        sheet,
        ctx,
    );
    crate::ir::SheetTextBox {
        anchor_row: placed.anchor_row,
        x_offset_pt: placed.x_offset_pt,
        y_offset_pt: placed.y_offset_pt,
        width: placed.image.width.unwrap_or(100.0),
        height: placed.image.height.unwrap_or(50.0),
        paragraphs: anchor.paragraphs,
        fill: anchor.fill,
        border: anchor.border,
        vertical_center: anchor.vertical_center,
    }
}

/// Convert a raw chart anchor into a render-ready sheet chart: the anchor's
/// absolute placement resolved exactly as a picture's is, or no placement at
/// all for a chart no drawing references (issue #982).
fn anchored_chart(
    anchor: xlsx_drawing::RawChartAnchor,
    sheet: &umya_spreadsheet::Worksheet,
    ctx: &SheetContext,
) -> crate::ir::SheetChart {
    let Some(geometry) = anchor.geometry else {
        return crate::ir::SheetChart {
            anchor_row: u32::MAX,
            placement: None,
            chart: anchor.chart,
        };
    };
    let placed = anchored_image(
        xlsx_drawing::RawImageAnchor {
            from_row: geometry.from_row,
            from_col: geometry.from_col,
            from_col_off_emu: geometry.from_col_off_emu,
            from_row_off_emu: geometry.from_row_off_emu,
            to: geometry.to,
            ext_emu: geometry.ext_emu,
            data: Vec::new(),
            format: crate::ir::ImageFormat::Png,
        },
        sheet,
        ctx,
    );
    crate::ir::SheetChart {
        anchor_row: placed.anchor_row,
        placement: Some(crate::ir::SheetChartPlacement {
            x_offset_pt: placed.x_offset_pt,
            y_offset_pt: placed.y_offset_pt,
            width: placed.image.width.unwrap_or(100.0),
            height: placed.image.height.unwrap_or(50.0),
        }),
        chart: anchor.chart,
    }
}

pub struct XlsxParser;

impl XlsxParser {
    /// Parse XLSX in streaming mode, returning one `Document` per chunk of rows.
    ///
    /// Each chunk contains a single `SheetPage` with at most `chunk_size` rows.
    /// This allows the caller to compile each chunk independently, bounding peak
    /// memory during Typst compilation.
    pub fn parse_streaming(
        &self,
        data: &[u8],
        options: &ConvertOptions,
        chunk_size: usize,
    ) -> Result<(Vec<Document>, Vec<ConvertWarning>), ConvertError> {
        let cursor = Cursor::new(data);
        let book = umya_spreadsheet::reader::xlsx::read_reader(cursor, true).map_err(|e| {
            crate::parser::parse_err(format!("Failed to parse XLSX (umya-spreadsheet): {e}"))
        })?;

        let metadata = extract_xlsx_metadata(&book);
        let cond_fmt_hints = cond_fmt_raw::extract_cond_fmt_hints(data);
        // A `cfRule type="expression"` names the workbook's defined names
        // rather than repeating their formulas (issue #852).
        let defined_names = cond_fmt_raw::extract_defined_names(data);
        let fitting_sheets = fit_to_page::sheets_fit_to_width(data);
        let print_options_by_sheet = print_options::sheets_print_options(data);
        let mut row_stripes = tables::extract_row_stripes(data);
        let normal_font = extract_normal_font(data);

        let mut chart_map = extract_charts_with_anchors(data);
        let mut image_map = extract_images_with_anchors(data);
        let mut text_box_map = extract_text_boxes_with_anchors(data);

        let mut chunks = Vec::new();
        let mut warnings = Vec::new();

        for sheet in book.get_sheet_collection() {
            // Filter by sheet name if specified
            if let Some(ref names) = options.sheet_names
                && !names.iter().any(|n| n == sheet.get_name())
            {
                continue;
            }

            let Some((ctx, row_start, row_end)) = prepare_sheet_context(
                sheet,
                normal_font.as_ref(),
                cond_fmt_hints.get(sheet.get_name()),
                &defined_names,
                row_stripes.remove(sheet.get_name()).unwrap_or_default(),
                Some(book.get_theme()),
            ) else {
                // A sheet without used cells can still carry drawings; give
                // its images a page instead of dropping them.
                let sheet_name = sheet.get_name().to_string();
                let raw_images = image_map.remove(&sheet_name);
                let raw_text_boxes = text_box_map.remove(&sheet_name);
                let raw_charts = chart_map.remove(&sheet_name);
                if raw_images.is_some() || raw_text_boxes.is_some() || raw_charts.is_some() {
                    let stub_ctx =
                        empty_sheet_context(sheet, normal_font.as_ref(), Some(book.get_theme()));
                    let images: Vec<crate::ir::SheetImage> = raw_images
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_image(anchor, sheet, &stub_ctx))
                        .collect();
                    let text_boxes: Vec<crate::ir::SheetTextBox> = raw_text_boxes
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_text_box(anchor, sheet, &stub_ctx))
                        .collect();
                    let charts: Vec<crate::ir::SheetChart> = raw_charts
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_chart(anchor, sheet, &stub_ctx))
                        .collect();
                    if !images.is_empty() || !text_boxes.is_empty() || !charts.is_empty() {
                        chunks.push(Document {
                            metadata: metadata.clone(),
                            // Drawings past the printable width split into
                            // page-columns as Excel prints them (issue #713).
                            pages: xlsx_pagination::split_drawing_only_page(SheetPage {
                                name: sheet_name,
                                size: sheet_page_size(sheet),
                                margins: sheet_print_margins(sheet),
                                table: Table::default(),
                                header: None,
                                footer: None,
                                charts,
                                images,
                                text_boxes,
                            })
                            .into_iter()
                            .map(Page::Sheet)
                            .collect(),
                            styles: StyleSheet::default(),
                        });
                    }
                }
                continue;
            };

            let sheet_name = sheet.get_name().to_string();
            let sheet_print_options: print_options::SheetPrintOptions = print_options_by_sheet
                .get(&sheet_name)
                .copied()
                .unwrap_or_default();

            // Extract sheet header/footer
            let hf = sheet.get_header_footer();
            let sheet_header = parse_hf_format_string(
                hf.get_odd_header().get_value(),
                &sheet_name,
                normal_font.as_ref(),
                &mut warnings,
            );
            let sheet_footer = parse_hf_format_string(
                hf.get_odd_footer().get_value(),
                &sheet_name,
                normal_font.as_ref(),
                &mut warnings,
            );

            // Pull charts for this sheet
            let mut sheet_charts: Vec<crate::ir::SheetChart> = chart_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_chart(anchor, sheet, &ctx))
                .collect();
            for sheet_chart in &sheet_charts {
                let title = sheet_chart
                    .chart
                    .title
                    .as_deref()
                    .unwrap_or("untitled")
                    .to_string();
                warnings.push(ConvertWarning::FallbackUsed {
                    format: "XLSX".to_string(),
                    from: format!("chart ({title})"),
                    to: "data table".to_string(),
                });
            }
            sheet_charts.sort_by_key(|sheet_chart| sheet_chart.anchor_row);
            let mut sheet_images: Vec<crate::ir::SheetImage> = image_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_image(anchor, sheet, &ctx))
                .collect();
            sheet_images.sort_by_key(|sheet_image| sheet_image.anchor_row);
            let mut sheet_text_boxes: Vec<crate::ir::SheetTextBox> = text_box_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_text_box(anchor, sheet, &ctx))
                .collect();
            sheet_text_boxes.sort_by_key(|text_box| text_box.anchor_row);

            let print_titles = find_print_titles(&book, sheet);
            let title_columns: Option<(usize, usize)> =
                print_headings::heading_adjusted_title_columns(
                    title_column_indices(print_titles, &ctx),
                    sheet_print_options.prints_headings,
                );
            let fit_to_width: Option<u32> = sheet_fit_to_width(&sheet_name, &fitting_sheets);
            let header_footer_scales_with_doc: bool =
                sheet_header_footer_scales_with_doc(&sheet_name, &fitting_sheets);

            // Process rows in chunks
            let mut chunk_start = row_start;
            let mut first_chunk = true;
            while chunk_start <= row_end {
                let chunk_end = (chunk_start + chunk_size as u32 - 1).min(row_end);

                let mut rows = build_rows_for_range(sheet, &ctx, chunk_start, chunk_end);
                // Worksheet row number of each built row, for the printed
                // heading gutter (issue #623).
                let mut sheet_row_numbers: Option<Vec<u32>> = sheet_print_options
                    .prints_headings
                    .then(|| (chunk_start..=chunk_end).collect());
                let mut header_row_count: usize = 0;
                // Rows above the print-title range print once; only the title
                // rows themselves repeat.
                let mut non_repeating_header_row_count: usize = 0;
                if let Some((title_start, title_end)) = print_titles.rows
                    && title_end < chunk_start
                {
                    // Later chunks don't contain the title rows — prepend them.
                    let mut title_rows = build_rows_for_range(sheet, &ctx, title_start, title_end);
                    header_row_count = title_rows.len();
                    title_rows.append(&mut rows);
                    rows = title_rows;
                    if let Some(numbers) = sheet_row_numbers.as_mut() {
                        numbers.splice(0..0, title_start..=title_end);
                    }
                } else if let Some((title_start, title_end)) = print_titles.rows
                    && title_end >= chunk_start
                    && title_end <= chunk_end
                {
                    non_repeating_header_row_count =
                        title_start.saturating_sub(chunk_start) as usize;
                    header_row_count =
                        (title_end + 1).saturating_sub(title_start.max(chunk_start)) as usize;
                }

                let mut sheet_page = SheetPage {
                    name: sheet_name.clone(),
                    size: sheet_page_size(sheet),
                    margins: sheet_print_margins(sheet),
                    table: Table {
                        rows,
                        column_widths: ctx.column_widths.clone(),
                        header_row_count,
                        non_repeating_header_row_count,
                        alignment: None,
                        default_cell_padding: Some(xlsx_cells::XLSX_CELL_PADDING),
                        use_content_driven_row_heights: false,
                        default_vertical_align: Some(crate::ir::CellVerticalAlign::Bottom),
                        seats_bottom_aligned_text_on_descender: true,
                        border_paint_model: TableBorderPaintModel::ExcelBoundaryBands,
                        prints_gridlines: sheet_print_options.prints_gridlines,
                        prints_headings: false,
                    },
                    header: sheet_header.clone(),
                    footer: sheet_footer.clone(),
                    charts: if first_chunk {
                        std::mem::take(&mut sheet_charts)
                    } else {
                        vec![]
                    },
                    images: if first_chunk {
                        std::mem::take(&mut sheet_images)
                    } else {
                        vec![]
                    },
                    text_boxes: if first_chunk {
                        first_chunk = false;
                        std::mem::take(&mut sheet_text_boxes)
                    } else {
                        vec![]
                    },
                };
                if let Some(numbers) = sheet_row_numbers.as_deref() {
                    print_headings::augment_page_with_print_headings(
                        &mut sheet_page,
                        numbers,
                        ctx.col_start,
                        normal_font.as_ref(),
                    );
                }
                let doc = Document {
                    metadata: metadata.clone(),
                    pages: xlsx_pagination::split_sheet_page_by_width(
                        sheet_page,
                        title_columns,
                        fit_to_width,
                        header_footer_scales_with_doc,
                    )
                    .into_iter()
                    .map(Page::Sheet)
                    .collect(),
                    styles: StyleSheet::default(),
                };

                chunks.push(doc);
                chunk_start = chunk_end + 1;
            }
        }

        if chunks.is_empty()
            && let Some(page) = empty_workbook_page(&book, options)
        {
            chunks.push(Document {
                metadata,
                pages: vec![Page::Sheet(page)],
                styles: StyleSheet::default(),
            });
        }

        Ok((chunks, warnings))
    }
}

impl Parser for XlsxParser {
    fn parse(
        &self,
        data: &[u8],
        options: &ConvertOptions,
    ) -> Result<(Document, Vec<ConvertWarning>), ConvertError> {
        let cursor = Cursor::new(data);
        let book = umya_spreadsheet::reader::xlsx::read_reader(cursor, true).map_err(|e| {
            crate::parser::parse_err(format!("Failed to parse XLSX (umya-spreadsheet): {e}"))
        })?;

        // Extract metadata from umya-spreadsheet properties
        let metadata = extract_xlsx_metadata(&book);
        let cond_fmt_hints = cond_fmt_raw::extract_cond_fmt_hints(data);
        // A `cfRule type="expression"` names the workbook's defined names
        // rather than repeating their formulas (issue #852).
        let defined_names = cond_fmt_raw::extract_defined_names(data);
        let fitting_sheets = fit_to_page::sheets_fit_to_width(data);
        let print_options_by_sheet = print_options::sheets_print_options(data);
        let mut row_stripes = tables::extract_row_stripes(data);
        let normal_font = extract_normal_font(data);

        // Extract charts with anchor positions per sheet
        let mut chart_map = extract_charts_with_anchors(data);
        let mut image_map = extract_images_with_anchors(data);
        let mut text_box_map = extract_text_boxes_with_anchors(data);

        let sheet_count = book.get_sheet_collection().len();
        let mut pages = Vec::with_capacity(sheet_count);
        let mut warnings = Vec::new();

        for sheet in book.get_sheet_collection() {
            // Filter by sheet name if specified
            if let Some(ref names) = options.sheet_names
                && !names.iter().any(|n| n == sheet.get_name())
            {
                continue;
            }

            let Some((ctx, row_start, row_end)) = prepare_sheet_context(
                sheet,
                normal_font.as_ref(),
                cond_fmt_hints.get(sheet.get_name()),
                &defined_names,
                row_stripes.remove(sheet.get_name()).unwrap_or_default(),
                Some(book.get_theme()),
            ) else {
                // A sheet without used cells can still carry drawings; give
                // its images a page instead of dropping them.
                let sheet_name = sheet.get_name().to_string();
                let raw_images = image_map.remove(&sheet_name);
                let raw_text_boxes = text_box_map.remove(&sheet_name);
                let raw_charts = chart_map.remove(&sheet_name);
                if raw_images.is_some() || raw_text_boxes.is_some() || raw_charts.is_some() {
                    let stub_ctx =
                        empty_sheet_context(sheet, normal_font.as_ref(), Some(book.get_theme()));
                    let images: Vec<crate::ir::SheetImage> = raw_images
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_image(anchor, sheet, &stub_ctx))
                        .collect();
                    let text_boxes: Vec<crate::ir::SheetTextBox> = raw_text_boxes
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_text_box(anchor, sheet, &stub_ctx))
                        .collect();
                    let charts: Vec<crate::ir::SheetChart> = raw_charts
                        .unwrap_or_default()
                        .into_iter()
                        .map(|anchor| anchored_chart(anchor, sheet, &stub_ctx))
                        .collect();
                    if !images.is_empty() || !text_boxes.is_empty() || !charts.is_empty() {
                        // Drawings past the printable width split into
                        // page-columns as Excel prints them (issue #713).
                        pages.extend(
                            xlsx_pagination::split_drawing_only_page(SheetPage {
                                name: sheet_name,
                                size: sheet_page_size(sheet),
                                margins: sheet_print_margins(sheet),
                                table: Table::default(),
                                header: None,
                                footer: None,
                                charts,
                                images,
                                text_boxes,
                            })
                            .into_iter()
                            .map(Page::Sheet),
                        );
                    }
                }
                continue;
            };

            let rows = build_rows_for_range(sheet, &ctx, row_start, row_end);

            let sheet_name = sheet.get_name().to_string();
            let sheet_print_options: print_options::SheetPrintOptions = print_options_by_sheet
                .get(&sheet_name)
                .copied()
                .unwrap_or_default();

            let print_titles = find_print_titles(&book, sheet);
            let title_columns: Option<(usize, usize)> =
                print_headings::heading_adjusted_title_columns(
                    title_column_indices(print_titles, &ctx),
                    sheet_print_options.prints_headings,
                );
            let fit_to_width: Option<u32> = sheet_fit_to_width(sheet.get_name(), &fitting_sheets);
            let header_footer_scales_with_doc: bool =
                sheet_header_footer_scales_with_doc(sheet.get_name(), &fitting_sheets);
            // Only the rows named by `_xlnm.Print_Titles` repeat on later
            // pages. Rows above them still lead the table, but print once, so
            // they go into a non-repeating header block.
            let (non_repeating_header_row_count, header_row_count): (usize, usize) = print_titles
                .rows
                .filter(|(_, title_end)| *title_end >= row_start)
                .map(|(title_start, title_end)| {
                    let lead: usize = title_start.saturating_sub(row_start) as usize;
                    let repeat: usize = (title_end.min(row_end) + 1)
                        .saturating_sub(title_start.max(row_start))
                        as usize;
                    (lead, repeat)
                })
                .unwrap_or((0, 0));

            // Collect row page breaks and split rows into page segments
            let row_breaks = collect_row_breaks(sheet);

            // Extract sheet header/footer
            let hf = sheet.get_header_footer();
            let sheet_header = parse_hf_format_string(
                hf.get_odd_header().get_value(),
                &sheet_name,
                normal_font.as_ref(),
                &mut warnings,
            );
            let sheet_footer = parse_hf_format_string(
                hf.get_odd_footer().get_value(),
                &sheet_name,
                normal_font.as_ref(),
                &mut warnings,
            );

            // Pull charts for this sheet (if any)
            let mut sheet_charts: Vec<crate::ir::SheetChart> = chart_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_chart(anchor, sheet, &ctx))
                .collect();
            for sheet_chart in &sheet_charts {
                let title = sheet_chart
                    .chart
                    .title
                    .as_deref()
                    .unwrap_or("untitled")
                    .to_string();
                warnings.push(ConvertWarning::FallbackUsed {
                    format: "XLSX".to_string(),
                    from: format!("chart ({title})"),
                    to: "data table".to_string(),
                });
            }
            // Sort by anchor row
            sheet_charts.sort_by_key(|sheet_chart| sheet_chart.anchor_row);
            let mut sheet_images: Vec<crate::ir::SheetImage> = image_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_image(anchor, sheet, &ctx))
                .collect();
            sheet_images.sort_by_key(|sheet_image| sheet_image.anchor_row);
            let mut sheet_text_boxes: Vec<crate::ir::SheetTextBox> = text_box_map
                .remove(&sheet_name)
                .unwrap_or_default()
                .into_iter()
                .map(|anchor| anchored_text_box(anchor, sheet, &ctx))
                .collect();
            sheet_text_boxes.sort_by_key(|text_box| text_box.anchor_row);

            if row_breaks.is_empty() {
                // No page breaks — single page
                let sheet_row_numbers: Option<Vec<u32>> = sheet_print_options
                    .prints_headings
                    .then(|| (row_start..=row_end).collect());
                let mut sheet_page = SheetPage {
                    name: sheet_name,
                    size: sheet_page_size(sheet),
                    margins: sheet_print_margins(sheet),
                    table: Table {
                        rows,
                        column_widths: ctx.column_widths.clone(),
                        header_row_count,
                        non_repeating_header_row_count,
                        alignment: None,
                        default_cell_padding: Some(xlsx_cells::XLSX_CELL_PADDING),
                        use_content_driven_row_heights: false,
                        default_vertical_align: Some(crate::ir::CellVerticalAlign::Bottom),
                        seats_bottom_aligned_text_on_descender: true,
                        border_paint_model: TableBorderPaintModel::ExcelBoundaryBands,
                        prints_gridlines: sheet_print_options.prints_gridlines,
                        prints_headings: false,
                    },
                    header: sheet_header.clone(),
                    footer: sheet_footer.clone(),
                    charts: sheet_charts,
                    images: sheet_images,
                    text_boxes: sheet_text_boxes,
                };
                if let Some(numbers) = sheet_row_numbers.as_deref() {
                    print_headings::augment_page_with_print_headings(
                        &mut sheet_page,
                        numbers,
                        ctx.col_start,
                        normal_font.as_ref(),
                    );
                }
                pages.extend(
                    xlsx_pagination::split_sheet_page_by_width(
                        sheet_page,
                        title_columns,
                        fit_to_width,
                        header_footer_scales_with_doc,
                    )
                    .into_iter()
                    .map(Page::Sheet),
                );
            } else {
                // Split rows at break points
                // Breaks are 1-indexed row numbers; break after that row
                let mut segments: Vec<(u32, Vec<TableRow>)> = Vec::new();
                let mut current_segment: Vec<TableRow> = Vec::new();
                let mut current_segment_start: u32 = row_start;
                let mut break_idx = 0;

                for (i, row) in rows.into_iter().enumerate() {
                    let actual_row = row_start + i as u32; // 1-indexed row number
                    if current_segment.is_empty() {
                        current_segment_start = actual_row;
                    }
                    current_segment.push(row);

                    // Check if this row is a break point
                    if break_idx < row_breaks.len() && actual_row == row_breaks[break_idx] {
                        segments
                            .push((current_segment_start, std::mem::take(&mut current_segment)));
                        break_idx += 1;
                    }
                }
                // Push remaining rows as the last segment
                if !current_segment.is_empty() {
                    segments.push((current_segment_start, current_segment));
                }

                // For page-break segments, attach all charts to the first segment
                let mut first_segment = true;
                for (segment_start_row, mut segment) in segments {
                    let mut segment_header_rows: usize = 0;
                    let mut segment_lead_rows: usize = 0;
                    // Title rows a later segment repeats, with their original
                    // worksheet numbers for the heading gutter.
                    let mut prepended_title_range: Option<(u32, u32)> = None;
                    if first_segment {
                        segment_lead_rows = non_repeating_header_row_count.min(segment.len());
                        segment_header_rows =
                            header_row_count.min(segment.len() - segment_lead_rows);
                    } else if let Some((title_start, title_end)) = print_titles.rows
                        && title_end >= row_start
                    {
                        // Later segments don't contain the title rows — prepend.
                        let clamped_title_start: u32 = title_start.max(row_start);
                        let mut title_rows =
                            build_rows_for_range(sheet, &ctx, clamped_title_start, title_end);
                        segment_header_rows = title_rows.len();
                        prepended_title_range = Some((clamped_title_start, title_end));
                        title_rows.append(&mut segment);
                        segment = title_rows;
                    }
                    let sheet_row_numbers: Option<Vec<u32>> =
                        sheet_print_options.prints_headings.then(|| {
                            let prepended_rows: usize = prepended_title_range
                                .map(|(start, end)| (end - start + 1) as usize)
                                .unwrap_or(0);
                            let data_rows: u32 = (segment.len() - prepended_rows) as u32;
                            prepended_title_range
                                .map(|(start, end)| (start..=end).collect::<Vec<u32>>())
                                .unwrap_or_default()
                                .into_iter()
                                .chain(segment_start_row..segment_start_row + data_rows)
                                .collect()
                        });
                    let mut sheet_page = SheetPage {
                        name: sheet_name.clone(),
                        size: sheet_page_size(sheet),
                        margins: sheet_print_margins(sheet),
                        table: Table {
                            rows: segment,
                            column_widths: ctx.column_widths.clone(),
                            header_row_count: segment_header_rows,
                            non_repeating_header_row_count: segment_lead_rows,
                            alignment: None,
                            default_cell_padding: Some(xlsx_cells::XLSX_CELL_PADDING),
                            use_content_driven_row_heights: false,
                            default_vertical_align: Some(crate::ir::CellVerticalAlign::Bottom),
                            seats_bottom_aligned_text_on_descender: true,
                            border_paint_model: TableBorderPaintModel::ExcelBoundaryBands,
                            prints_gridlines: sheet_print_options.prints_gridlines,
                            prints_headings: false,
                        },
                        header: sheet_header.clone(),
                        footer: sheet_footer.clone(),
                        charts: if first_segment {
                            std::mem::take(&mut sheet_charts)
                        } else {
                            vec![]
                        },
                        images: if first_segment {
                            std::mem::take(&mut sheet_images)
                        } else {
                            vec![]
                        },
                        text_boxes: if first_segment {
                            first_segment = false;
                            std::mem::take(&mut sheet_text_boxes)
                        } else {
                            vec![]
                        },
                    };
                    if let Some(numbers) = sheet_row_numbers.as_deref() {
                        print_headings::augment_page_with_print_headings(
                            &mut sheet_page,
                            numbers,
                            ctx.col_start,
                            normal_font.as_ref(),
                        );
                    }
                    pages.extend(
                        xlsx_pagination::split_sheet_page_by_width(
                            sheet_page,
                            title_columns,
                            fit_to_width,
                            header_footer_scales_with_doc,
                        )
                        .into_iter()
                        .map(Page::Sheet),
                    );
                }
            }
        }

        if pages.is_empty()
            && let Some(page) = empty_workbook_page(&book, options)
        {
            pages.push(Page::Sheet(page));
        }

        Ok((
            Document {
                metadata,
                pages,
                styles: StyleSheet::default(),
            },
            warnings,
        ))
    }
}

/// Extract metadata from umya-spreadsheet Properties.
/// Empty strings are converted to None.
fn extract_xlsx_metadata(book: &umya_spreadsheet::Spreadsheet) -> Metadata {
    let props = book.get_properties();
    let non_empty = |s: &str| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    };
    Metadata {
        title: non_empty(props.get_title()),
        author: non_empty(props.get_creator()),
        subject: non_empty(props.get_subject()),
        description: non_empty(props.get_description()),
        created: non_empty(props.get_created()),
        modified: non_empty(props.get_modified()),
    }
}

#[cfg(test)]
#[path = "xlsx_tests.rs"]
mod tests;

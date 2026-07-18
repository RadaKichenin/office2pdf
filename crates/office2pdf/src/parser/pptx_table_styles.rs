use super::*;

// ── Table style data structures ─────────────────────────────────────────

/// Borders a table style region draws. Outer sides apply to boundary cells,
/// insideH/insideV to shared interior edges.
#[derive(Debug, Clone, Default)]
pub(super) struct RegionBorders {
    pub(super) left: Option<BorderSide>,
    pub(super) right: Option<BorderSide>,
    pub(super) top: Option<BorderSide>,
    pub(super) bottom: Option<BorderSide>,
    pub(super) inside_h: Option<BorderSide>,
    pub(super) inside_v: Option<BorderSide>,
}

/// Styling for a table cell region (e.g., firstRow, band1H, wholeTbl).
#[derive(Debug, Clone, Default)]
pub(super) struct TableCellRegionStyle {
    pub(super) fill: Option<Color>,
    pub(super) text_color: Option<Color>,
    pub(super) text_bold: Option<bool>,
    pub(super) borders: RegionBorders,
}

/// Parsed definition of a single `<a:tblStyle>` element.
#[derive(Debug, Clone, Default)]
pub(super) struct PptxTableStyleDef {
    pub(super) whole_table: Option<TableCellRegionStyle>,
    pub(super) band1_h: Option<TableCellRegionStyle>,
    pub(super) band2_h: Option<TableCellRegionStyle>,
    pub(super) first_row: Option<TableCellRegionStyle>,
    pub(super) last_row: Option<TableCellRegionStyle>,
    pub(super) first_col: Option<TableCellRegionStyle>,
    pub(super) last_col: Option<TableCellRegionStyle>,
}

/// Map from style ID (GUID string) to parsed table style definition.
pub(super) type TableStyleMap = HashMap<String, PptxTableStyleDef>;

/// Attributes from `<a:tblPr>` that control which style regions are active.
#[derive(Debug, Clone, Default)]
pub(super) struct PptxTableProps {
    pub(super) style_id: Option<String>,
    pub(super) first_row: bool,
    pub(super) last_row: bool,
    pub(super) first_col: bool,
    pub(super) last_col: bool,
    pub(super) band_row: bool,
    pub(super) band_col: bool,
}

// ── Parsing ─────────────────────────────────────────────────────────────

/// Parse `ppt/tableStyles.xml` into a map of table style definitions.
pub(super) fn parse_table_styles_xml(
    xml: &str,
    theme: &ThemeData,
    color_map: &ColorMapData,
) -> TableStyleMap {
    let mut styles: TableStyleMap = HashMap::new();
    let mut reader = Reader::from_str(xml);

    let mut current_style_id: Option<String> = None;
    let mut current_def = PptxTableStyleDef::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"tblStyle" => {
                    current_style_id = get_attr_str(e, b"styleId");
                    current_def = PptxTableStyleDef::default();
                }
                b"wholeTbl" if current_style_id.is_some() => {
                    current_def.whole_table = Some(parse_region_style(
                        &mut reader,
                        b"wholeTbl",
                        theme,
                        color_map,
                    ));
                }
                b"band1H" if current_style_id.is_some() => {
                    current_def.band1_h =
                        Some(parse_region_style(&mut reader, b"band1H", theme, color_map));
                }
                b"band2H" if current_style_id.is_some() => {
                    current_def.band2_h =
                        Some(parse_region_style(&mut reader, b"band2H", theme, color_map));
                }
                b"firstRow" if current_style_id.is_some() => {
                    current_def.first_row = Some(parse_region_style(
                        &mut reader,
                        b"firstRow",
                        theme,
                        color_map,
                    ));
                }
                b"lastRow" if current_style_id.is_some() => {
                    current_def.last_row = Some(parse_region_style(
                        &mut reader,
                        b"lastRow",
                        theme,
                        color_map,
                    ));
                }
                b"firstCol" if current_style_id.is_some() => {
                    current_def.first_col = Some(parse_region_style(
                        &mut reader,
                        b"firstCol",
                        theme,
                        color_map,
                    ));
                }
                b"lastCol" if current_style_id.is_some() => {
                    current_def.last_col = Some(parse_region_style(
                        &mut reader,
                        b"lastCol",
                        theme,
                        color_map,
                    ));
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"tblStyle" => {
                if let Some(id) = current_style_id.take() {
                    styles.insert(id, std::mem::take(&mut current_def));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    styles
}

/// Parse a region element (e.g., `<a:firstRow>`) and extract fill, text color, and bold.
fn parse_region_style(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    theme: &ThemeData,
    color_map: &ColorMapData,
) -> TableCellRegionStyle {
    let mut style = TableCellRegionStyle::default();
    let mut in_tc_style = false;
    let mut in_tc_tx_style = false;
    let mut in_fill = false;
    let mut in_solid_fill = false;
    let mut in_font_ref = false;
    let mut in_tc_bdr = false;
    // Which tcBdr side element we're inside, with the pending line width.
    let mut current_border_side: Option<(Vec<u8>, f64)> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"tcStyle" => in_tc_style = true,
                b"tcTxStyle" => {
                    in_tc_tx_style = true;
                    if let Some(bold) = get_attr_str(e, b"b") {
                        style.text_bold = Some(bold == "on");
                    }
                }
                b"tcBdr" if in_tc_style => in_tc_bdr = true,
                side @ (b"left" | b"right" | b"top" | b"bottom" | b"insideH" | b"insideV")
                    if in_tc_bdr =>
                {
                    current_border_side = Some((side.to_vec(), 1.0));
                }
                b"ln" if current_border_side.is_some() => {
                    if let Some((_, width)) = current_border_side.as_mut()
                        && let Some(w) = get_attr_str(e, b"w").and_then(|v| v.parse::<f64>().ok())
                    {
                        *width = w / 12700.0;
                    }
                }
                b"fill" if in_tc_style && !in_tc_bdr => in_fill = true,
                b"solidFill" if in_fill || (in_tc_style && !in_tc_bdr) => in_solid_fill = true,
                b"fontRef" if in_tc_tx_style => in_font_ref = true,
                b"srgbClr" | b"schemeClr" | b"sysClr" if current_border_side.is_some() => {
                    let parsed: ParsedColor = parse_color_from_start(reader, e, theme, color_map);
                    if let (Some((side, width)), Some(color)) =
                        (current_border_side.as_ref(), parsed.color)
                    {
                        set_region_border(&mut style.borders, side, *width, color);
                    }
                }
                b"srgbClr" | b"schemeClr" | b"sysClr" if in_solid_fill => {
                    let parsed: ParsedColor = parse_color_from_start(reader, e, theme, color_map);
                    style.fill = parsed.color;
                }
                b"srgbClr" | b"schemeClr" | b"sysClr" if in_font_ref => {
                    let parsed: ParsedColor = parse_color_from_start(reader, e, theme, color_map);
                    style.text_color = parsed.color;
                }
                _ => {}
            },
            Ok(Event::Empty(ref e)) => match e.local_name().as_ref() {
                b"srgbClr" | b"schemeClr" | b"sysClr" if current_border_side.is_some() => {
                    let parsed: ParsedColor = parse_color_from_empty(e, theme, color_map);
                    if let (Some((side, width)), Some(color)) =
                        (current_border_side.as_ref(), parsed.color)
                    {
                        set_region_border(&mut style.borders, side, *width, color);
                    }
                }
                b"srgbClr" | b"schemeClr" | b"sysClr" if in_solid_fill => {
                    let parsed: ParsedColor = parse_color_from_empty(e, theme, color_map);
                    style.fill = parsed.color;
                }
                b"srgbClr" | b"schemeClr" | b"sysClr" if in_font_ref => {
                    let parsed: ParsedColor = parse_color_from_empty(e, theme, color_map);
                    style.text_color = parsed.color;
                }
                b"tcTxStyle" => {
                    if let Some(bold) = get_attr_str(e, b"b") {
                        style.text_bold = Some(bold == "on");
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == end_tag {
                    break;
                }
                match local.as_ref() {
                    b"tcStyle" => in_tc_style = false,
                    b"tcTxStyle" => in_tc_tx_style = false,
                    b"tcBdr" => in_tc_bdr = false,
                    b"left" | b"right" | b"top" | b"bottom" | b"insideH" | b"insideV"
                        if in_tc_bdr =>
                    {
                        current_border_side = None;
                    }
                    b"fill" => in_fill = false,
                    b"solidFill" => in_solid_fill = false,
                    b"fontRef" => in_font_ref = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    style
}

/// Record a parsed tcBdr side into the region's borders.
fn set_region_border(borders: &mut RegionBorders, side: &[u8], width: f64, color: Color) {
    let border = Some(BorderSide {
        width,
        color,
        style: BorderLineStyle::Solid,
    });
    match side {
        b"left" => borders.left = border,
        b"right" => borders.right = border,
        b"top" => borders.top = border,
        b"bottom" => borders.bottom = border,
        b"insideH" => borders.inside_h = border,
        b"insideV" => borders.inside_v = border,
        _ => {}
    }
}

// ── Style application ───────────────────────────────────────────────────

/// Apply table style colors/formatting to cells that don't have explicit overrides.
///
/// Priority (highest wins): cell-level explicit → firstRow/lastRow/firstCol/lastCol → band → wholeTbl
pub(super) fn apply_table_style(table: &mut Table, props: &PptxTableProps, styles: &TableStyleMap) {
    let style_id: &str = match props.style_id.as_deref() {
        Some(id) => id,
        None => return,
    };
    let style_def: &PptxTableStyleDef = match styles.get(style_id) {
        Some(def) => def,
        None => return,
    };

    let total_rows: usize = table.rows.len();
    let total_cols: usize = table.column_widths.len();
    let header_rows: usize = if props.first_row { 1 } else { 0 };
    let footer_rows: usize = if props.last_row { 1 } else { 0 };

    for (row_idx, row) in table.rows.iter_mut().enumerate() {
        let is_first_row: bool = props.first_row && row_idx < header_rows;
        let is_last_row: bool =
            props.last_row && total_rows > header_rows && row_idx == total_rows - 1;

        // Data row index for banding (excludes first/last special rows)
        let data_row_idx: Option<usize> = if !is_first_row && !is_last_row {
            Some(row_idx.saturating_sub(header_rows))
        } else {
            None
        };

        for (col_idx, cell) in row.cells.iter_mut().enumerate() {
            let is_first_col: bool = props.first_col && col_idx == 0;
            let is_last_col: bool = props.last_col && total_cols > 0 && col_idx == total_cols - 1;

            // Determine which region style applies (highest priority first)
            let region_style: Option<&TableCellRegionStyle> = if is_first_row {
                style_def.first_row.as_ref()
            } else if is_last_row {
                style_def.last_row.as_ref()
            } else if is_first_col {
                style_def.first_col.as_ref()
            } else if is_last_col {
                style_def.last_col.as_ref()
            } else if props.band_row
                && let Some(data_idx) = data_row_idx
            {
                if data_idx % 2 == 0 {
                    style_def.band1_h.as_ref()
                } else {
                    style_def.band2_h.as_ref()
                }
            } else {
                style_def.whole_table.as_ref()
            };

            match region_style {
                Some(region) => apply_region_to_cell(cell, region),
                None => {
                    // Fall back to wholeTbl if the region is active but has
                    // no definition (e.g. band2H in built-in styles).
                    if let Some(whole) = style_def.whole_table.as_ref() {
                        apply_region_to_cell(cell, whole);
                    }
                }
            }

            apply_style_borders(
                cell,
                style_def,
                is_first_row,
                is_last_row,
                row_idx == 0,
                row_idx + 1 == total_rows,
                col_idx == 0,
                total_cols > 0 && col_idx + 1 == total_cols,
            );
        }
    }

    // Suppress footer_rows warning
    let _ = footer_rows;
}

/// Resolve the borders a cell gets from the style: wholeTbl draws the grid
/// (outer sides on boundary cells, insideH/V on interior edges), and the
/// firstRow/lastRow separators override the adjacent edge.
#[allow(clippy::too_many_arguments)]
fn apply_style_borders(
    cell: &mut TableCell,
    style_def: &PptxTableStyleDef,
    is_first_row: bool,
    is_last_row: bool,
    at_top: bool,
    at_bottom: bool,
    at_left: bool,
    at_right: bool,
) {
    // Explicit tcBorders on the cell win over the style grid.
    if cell.border.is_some() {
        return;
    }

    let mut top: Option<BorderSide> = None;
    let mut bottom: Option<BorderSide> = None;
    let mut left: Option<BorderSide> = None;
    let mut right: Option<BorderSide> = None;

    if let Some(whole) = style_def.whole_table.as_ref() {
        let grid = &whole.borders;
        top = if at_top {
            grid.top.clone()
        } else {
            grid.inside_h.clone()
        };
        bottom = if at_bottom {
            grid.bottom.clone()
        } else {
            grid.inside_h.clone()
        };
        left = if at_left {
            grid.left.clone()
        } else {
            grid.inside_v.clone()
        };
        right = if at_right {
            grid.right.clone()
        } else {
            grid.inside_v.clone()
        };
    }

    if is_first_row
        && let Some(first_row) = style_def.first_row.as_ref()
        && first_row.borders.bottom.is_some()
    {
        bottom = first_row.borders.bottom.clone();
    }
    if is_last_row
        && let Some(last_row) = style_def.last_row.as_ref()
        && last_row.borders.top.is_some()
    {
        top = last_row.borders.top.clone();
    }

    if top.is_some() || bottom.is_some() || left.is_some() || right.is_some() {
        cell.border = Some(CellBorder {
            top,
            bottom,
            left,
            right,
        });
    }
}

/// Apply a region style to a cell, respecting explicit cell-level overrides.
fn apply_region_to_cell(cell: &mut TableCell, region: &TableCellRegionStyle) {
    // Only apply fill if cell doesn't have an explicit background
    if cell.background.is_none() {
        cell.background = region.fill;
    }

    // Apply text color and bold to all runs that don't have explicit overrides
    if region.text_color.is_some() || region.text_bold.is_some() {
        for block in &mut cell.content {
            if let Block::Paragraph(paragraph) = block {
                for run in &mut paragraph.runs {
                    if region.text_color.is_some() && run.style.color.is_none() {
                        run.style.color = region.text_color;
                    }
                    if let Some(bold) = region.text_bold
                        && run.style.bold.is_none()
                    {
                        run.style.bold = Some(bold);
                    }
                }
            }
        }
    }
}

// ── Built-in table styles ───────────────────────────────────────────────

/// PowerPoint's built-in table styles are referenced by GUID but carry no
/// definition in `ppt/tableStyles.xml`. Generate the ones we support from
/// theme colors, mirroring LibreOffice's oox predefined-table-styles
/// construction. Styles already defined in the file win.
pub(super) fn add_builtin_table_styles(
    styles: &mut TableStyleMap,
    theme: &ThemeData,
    color_map: &ColorMapData,
) {
    // Medium Style 2 family: (styleId GUID, accent scheme color).
    const MEDIUM_STYLE_2_IDS: [(&str, &str); 7] = [
        ("{073A0DAA-6AF3-43AB-8588-CEC1D06C72B9}", "dk1"),
        ("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", "accent1"),
        ("{21E4AEA4-8DFA-4A89-87EB-49C32662AFE0}", "accent2"),
        ("{F5AB1C69-6EDB-4FF4-983F-18BD219EF322}", "accent3"),
        ("{00A15C55-8517-42AA-B614-E9B94910E393}", "accent4"),
        ("{7DF18680-E054-41AD-8BC1-D1AEF772440D}", "accent5"),
        ("{93296810-A885-4BE3-A3E7-6D5BEEA58F35}", "accent6"),
    ];

    let Some(lt1) = resolve_scheme_color(theme, color_map, "lt1") else {
        return;
    };
    let Some(dk1) = resolve_scheme_color(theme, color_map, "dk1") else {
        return;
    };

    for (style_id, accent_name) in MEDIUM_STYLE_2_IDS {
        if styles.contains_key(style_id) {
            continue;
        }
        let Some(accent) = resolve_scheme_color(theme, color_map, accent_name) else {
            continue;
        };
        styles.insert(style_id.to_string(), medium_style_2_def(accent, lt1, dk1));
    }
}

/// DrawingML tint: blend toward white keeping `factor` of the base color.
fn tint_color(color: Color, factor: f64) -> Color {
    let tint = |channel: u8| -> u8 {
        (255.0 - (255.0 - channel as f64) * factor)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::new(tint(color.r), tint(color.g), tint(color.b))
}

fn solid_border(color: Color) -> Option<BorderSide> {
    Some(BorderSide {
        width: 1.0,
        color,
        style: BorderLineStyle::Solid,
    })
}

/// Medium Style 2: solid accent header/footer/edge columns with lt1 text,
/// accent-tinted body (10%) and bands (20%), lt1 grid borders. Tint factors
/// were measured from PowerPoint's own PDF output (203/231 grays for the
/// dk1-based style) — LibreOffice's 20%/40% approximation renders too dark.
fn medium_style_2_def(accent: Color, lt1: Color, dk1: Color) -> PptxTableStyleDef {
    let solid_region = |fill: Color| TableCellRegionStyle {
        fill: Some(fill),
        text_color: Some(lt1),
        text_bold: None,
        borders: RegionBorders::default(),
    };

    PptxTableStyleDef {
        whole_table: Some(TableCellRegionStyle {
            fill: Some(tint_color(accent, 0.1)),
            text_color: Some(dk1),
            text_bold: None,
            borders: RegionBorders {
                left: solid_border(lt1),
                right: solid_border(lt1),
                top: solid_border(lt1),
                bottom: solid_border(lt1),
                inside_h: solid_border(lt1),
                inside_v: solid_border(lt1),
            },
        }),
        band1_h: Some(TableCellRegionStyle {
            fill: Some(tint_color(accent, 0.2)),
            ..TableCellRegionStyle::default()
        }),
        band2_h: None,
        first_row: Some(TableCellRegionStyle {
            borders: RegionBorders {
                bottom: solid_border(lt1),
                ..RegionBorders::default()
            },
            ..solid_region(accent)
        }),
        last_row: Some(TableCellRegionStyle {
            borders: RegionBorders {
                top: solid_border(lt1),
                ..RegionBorders::default()
            },
            ..solid_region(accent)
        }),
        first_col: Some(solid_region(accent)),
        last_col: Some(solid_region(accent)),
    }
}

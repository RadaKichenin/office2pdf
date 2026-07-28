use super::*;

/// How a chart is drawn. Selecting the variant once lets the atomicity decision
/// and the emitter agree on which geometry applies.
enum ChartVariant {
    /// Axis-scaled bar/column plot with gridlines, tick labels, and a legend.
    AxisPlot,
    /// Polyline plot over a value axis, for line and area charts.
    LinePlot,
    /// Bordered box holding a title, a type label, and a data table.
    BorderedTable,
}

fn chart_variant(chart: &Chart) -> ChartVariant {
    if matches!(chart.chart_type, ChartType::Bar | ChartType::Column)
        && !chart.series.is_empty()
        && !chart.categories.is_empty()
    {
        return ChartVariant::AxisPlot;
    }
    if matches!(chart.chart_type, ChartType::Line | ChartType::Area)
        && !chart.series.is_empty()
        && chart.categories.len() >= 2
    {
        return ChartVariant::LinePlot;
    }
    ChartVariant::BorderedTable
}

/// Height budget a chart must stay within to be kept atomic, in points.
///
/// Comfortably under the ~700pt text column an A4 page with default margins
/// offers. An unbreakable block taller than the column does not move to the
/// next page — it runs off the page edge and the overflow is never drawn — so
/// a chart that cannot fit anywhere is left breakable instead.
const MAX_ATOMIC_CHART_HEIGHT_PT: f64 = 620.0;

/// Vertical space one row of the bordered-table fallback occupies, in points.
const BORDERED_TABLE_ROW_PT: f64 = 16.0;

/// Title, type label, header row, and box insets above the fallback's data
/// rows, in points.
const BORDERED_TABLE_CHROME_PT: f64 = 90.0;

/// Report whether a chart is short enough that keeping it whole is safe.
fn chart_fits_on_one_page(chart: &Chart) -> bool {
    let height: f64 = match chart_variant(chart) {
        // The plot box plus the title block above it.
        ChartVariant::AxisPlot => chart_axis_extent(chart).1 + 24.0,
        // The polyline plot's box is a fixed size regardless of point count.
        ChartVariant::LinePlot => return true,
        ChartVariant::BorderedTable => {
            BORDERED_TABLE_CHROME_PT + chart.categories.len() as f64 * BORDERED_TABLE_ROW_PT
        }
    };
    height <= MAX_ATOMIC_CHART_HEIGHT_PT
}

/// Generate Typst markup for a chart.
///
/// Bar and column charts render as an axis-scaled plot, line and area charts as
/// a polyline plot over the same axis, and everything else falls back to a
/// bordered box holding the title, a type label, and a data table.
///
/// Excel and PowerPoint treat a chart as one floating graphic that never splits
/// at a page boundary: it moves to the next page whole. Typst blocks are
/// breakable by default, and every variant emits its title as a block separate
/// from its plot, so the whole chart is wrapped once here rather than each
/// sub-renderer repeating the flag. Charts too tall to fit on any page stay
/// breakable — see [`MAX_ATOMIC_CHART_HEIGHT_PT`].
pub(super) fn generate_chart(out: &mut String, chart: &Chart) {
    generate_chart_in(out, chart, None);
}

/// Render a chart into a frame of a known size.
///
/// PowerPoint lays a chart out at its `<p:graphicFrame>` extent, so a chart
/// authored to fill the left half of a slide is that size. Rendering at an
/// intrinsic size instead left it at 44% of the frame height with a band of
/// empty slide underneath (issue #548). Flowed charts have no frame and keep
/// the intrinsic size.
pub(super) fn generate_chart_in(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    // A framed chart is already bounded by its frame, so the page-break guard
    // only concerns the flowed case.
    let atomic: bool = frame.is_none() && chart_fits_on_one_page(chart);
    if atomic {
        out.push_str("#block(breakable: false)[\n");
    }
    generate_chart_body(out, chart, frame);
    if atomic {
        out.push_str("]\n");
    }
}

/// Emit the chart's own markup, without the atomicity wrapper.
fn generate_chart_body(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    match chart_variant(chart) {
        ChartVariant::AxisPlot => return generate_chart_axis(out, chart, frame),
        ChartVariant::LinePlot => return generate_chart_line_plot(out, chart, frame),
        ChartVariant::BorderedTable => {}
    }

    // A framed chart's box is its frame; `width: 100%` would otherwise take
    // the whole page and run under whatever sits beside it on the slide.
    match frame {
        Some((width, _)) => {
            let _ = writeln!(
                out,
                "#block(stroke: 1pt + rgb(100, 100, 100), radius: 4pt, inset: 10pt, width: {}pt)[",
                format_f64(width)
            );
        }
        None => {
            let _ = writeln!(
                out,
                "#block(stroke: 1pt + rgb(100, 100, 100), radius: 4pt, inset: 10pt, width: 100%)["
            );
        }
    }

    let type_label: &str = match &chart.chart_type {
        ChartType::Bar => "Bar Chart",
        ChartType::Column => "Column Chart",
        ChartType::Line => "Line Chart",
        ChartType::Pie => "Pie Chart",
        ChartType::Area => "Area Chart",
        ChartType::Scatter => "Scatter Chart",
        ChartType::Other(label) => label.as_str(),
    };

    if let Some(title) = chart.title.as_ref() {
        let escaped: String = escape_typst(title);
        let _ = writeln!(
            out,
            "#align(center)[#text(size: 14pt, weight: \"bold\")[{escaped}]]\n"
        );
    }
    let _ = writeln!(
        out,
        "#align(center)[#text(fill: rgb(100, 100, 100))[_{type_label}_]]\n"
    );

    if chart.series.is_empty() {
        out.push_str("]\n");
        return;
    }

    match &chart.chart_type {
        ChartType::Bar | ChartType::Column => generate_chart_bar(out, chart),
        ChartType::Pie => generate_chart_pie(out, chart),
        ChartType::Line => generate_chart_line(out, chart),
        _ => generate_chart_table(out, chart),
    }

    out.push_str("]\n");
}

/// Series palette matching Office's default accent colors.
const CHART_SERIES_COLORS: [&str; 6] = [
    "rgb(68, 114, 196)",
    "rgb(237, 125, 49)",
    "rgb(165, 165, 165)",
    "rgb(255, 192, 0)",
    "rgb(91, 155, 213)",
    "rgb(112, 173, 71)",
];

/// The Typst colour for one plotted point.
///
/// A point's own `<c:dPt>` fill outranks its series' `<c:spPr>` fill, and the
/// built-in palette is a fallback for charts that declare neither — not a
/// replacement for what the file states (issue #535).
fn series_color(
    series: &crate::ir::ChartSeries,
    series_index: usize,
    point_index: usize,
) -> String {
    match series.fill_for_point(point_index) {
        Some(color) => rgb(&color),
        None => CHART_SERIES_COLORS[series_index % CHART_SERIES_COLORS.len()].to_string(),
    }
}

/// As [`series_color`], but falling back to the category palette the pie and
/// bar fallbacks use.
fn category_color(series: &crate::ir::ChartSeries, point_index: usize, palette: &[&str]) -> String {
    match series.fill_for_point(point_index) {
        Some(color) => rgb(&color),
        None => palette[point_index % palette.len()].to_string(),
    }
}

/// Category palette used by the bar-plot and pie-table fallbacks.
///
/// Intentionally distinct from [`CHART_SERIES_COLORS`]; unifying them would
/// change rendered output and needs visual verification.
const CHART_CATEGORY_COLORS: [&str; 6] = [
    "rgb(66, 133, 244)",
    "rgb(219, 68, 55)",
    "rgb(244, 180, 0)",
    "rgb(15, 157, 88)",
    "rgb(171, 71, 188)",
    "rgb(0, 172, 193)",
];

/// Format a chart value without floating-point noise (e.g. 8.2000001 → 8.2).
pub(super) fn chart_value_label(value: f64) -> String {
    if value.fract().abs() < 1e-9 {
        return format!("{}", value.round() as i64);
    }
    // Round to at most 4 significant fractional digits, then trim zeros.
    let rounded: f64 = (value * 10_000.0).round() / 10_000.0;
    let mut text: String = format!("{rounded}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// Choose a "nice" axis maximum and tick step covering `[0, max]`
/// (e.g. max 8.2 → (10, 2), giving ticks 0,2,4,6,8,10).
fn nice_axis(max_value: f64) -> (f64, f64) {
    if max_value <= 0.0 {
        return (1.0, 1.0);
    }
    // Excel rounds the *step*, then takes the smallest whole number of steps
    // that covers the data — it does not round the maximum itself. Rounding
    // the maximum put 23,334 against a 50,000 axis, so every column drew at
    // half the height Excel gives it (#553).
    let target_step: f64 = max_value / MAJOR_UNITS;
    let magnitude: f64 = 10f64.powf(target_step.log10().floor());
    let normalized: f64 = target_step / magnitude;
    let nice_norm: f64 = *NICE_STEPS
        .iter()
        .find(|candidate| **candidate >= normalized - 1e-9)
        .unwrap_or(&10.0);
    let step: f64 = nice_norm * magnitude;
    let nice_max: f64 = (max_value / step).ceil() * step;
    (nice_max, step)
}

/// Major units Excel aims for on an auto-scaled value axis.
const MAJOR_UNITS: f64 = 5.0;

/// Step sizes Excel will choose, as multiples of a power of ten.
const NICE_STEPS: [f64; 5] = [1.0, 2.0, 2.5, 5.0, 10.0];

const PLOT_MAIN: f64 = 300.0; // value-axis length in points
const ROW: f64 = 34.0; // per-category thickness
const LABEL_W: f64 = 62.0; // category label gutter
const TICK_GAP: f64 = 22.0; // value tick label gutter
const GAP: f64 = 6.0;
const LEGEND_ROW_H: f64 = 14.0; // per-entry height when the legend stacks
const LEGEND_ENTRY_W: f64 = 78.0; // per-entry width when the legend runs across

/// Space a legend reserves around the plot, and the direction its entries run.
///
/// A legend on an edge runs along that edge, so a bottom or top one lays its
/// entries out left to right and leaves the plot the full frame width — which
/// is the difference `<c:legendPos val="b"/>` asks for (#546).
struct LegendBox {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    horizontal: bool,
}

impl LegendBox {
    /// Reserve space at `position`, given one stacked entry's height and one
    /// across-the-edge entry's width. A vertical legend is one column wide and
    /// a horizontal one is one row tall whatever the entry count, so the count
    /// only matters when laying the entries out.
    fn new(position: LegendPosition, row_h: f64, entry_w: f64) -> Self {
        let horizontal: bool = position.is_horizontal();
        let side_w: f64 = entry_w + GAP;
        let edge_h: f64 = row_h + GAP;
        let mut placement = LegendBox {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            horizontal,
        };
        match position {
            LegendPosition::Left => placement.left = side_w,
            LegendPosition::Right | LegendPosition::TopRight => placement.right = side_w,
            LegendPosition::Top => placement.top = edge_h,
            LegendPosition::Bottom => placement.bottom = edge_h,
        }
        placement
    }

    /// Top-left of the `index`-th entry.
    ///
    /// `content` is the plot *plus* its axis-label gutters, not the bare
    /// plotting rectangle: a legend under a column chart has to clear the
    /// category labels, or the two land in the same band.
    fn entry_origin(
        &self,
        position: LegendPosition,
        index: usize,
        entries: usize,
        content: (f64, f64, f64, f64),
        row_h: f64,
        entry_w: f64,
    ) -> (f64, f64) {
        let (content_x, content_y, content_w, content_h) = content;
        if self.horizontal {
            // Centre the row of entries under (or over) the content.
            let row_w: f64 = entries as f64 * entry_w;
            let start_x: f64 = content_x + (content_w - row_w).max(0.0) / 2.0;
            let y: f64 = match position {
                LegendPosition::Top => (content_y - self.top).max(0.0),
                _ => content_y + content_h + GAP,
            };
            (start_x + index as f64 * entry_w, y)
        } else {
            let stack_h: f64 = entries as f64 * row_h;
            let x: f64 = match position {
                LegendPosition::Left => (content_x - self.left).max(0.0),
                _ => content_x + content_w + GAP,
            };
            let y: f64 = match position {
                // PowerPoint pins a top-right legend to the top edge rather
                // than centring it.
                LegendPosition::TopRight => content_y,
                _ => content_y + (content_h - stack_h).max(0.0) / 2.0,
            };
            (x, y + index as f64 * row_h)
        }
    }
}

/// The text one data label prints, or `None` when the series prints none.
///
/// Office joins the enabled parts with the series' separator in the order
/// series, category, value, percent. Excel prints the audited workbook's pie
/// labels as `커밋 픽스처 수; DOCX; 115; 33%`, which fixes that order.
fn data_label_text(
    chart: &Chart,
    series: &crate::ir::ChartSeries,
    category_index: usize,
) -> Option<String> {
    let labels = &series.data_labels;
    if labels.is_empty() {
        return None;
    }
    let value: f64 = series.values.get(category_index).copied()?;
    let mut parts: Vec<String> = Vec::new();
    if labels.show_series
        && let Some(name) = series.name.as_deref()
    {
        parts.push(name.to_string());
    }
    if labels.show_category
        && let Some(category) = chart.categories.get(category_index)
    {
        parts.push(category.clone());
    }
    if labels.show_value {
        parts.push(chart_value_label(value));
    }
    if labels.show_percent {
        let total: f64 = category_total(&chart.series, category_index);
        let percent: f64 = if total == 0.0 {
            0.0
        } else {
            value / total * 100.0
        };
        parts.push(format!("{}%", chart_value_label(percent.round())));
    }
    (!parts.is_empty()).then(|| parts.join(&labels.separator))
}

/// Sum of every series' value in one category — the length of its stacked bar.
fn category_total(series: &[crate::ir::ChartSeries], category_index: usize) -> f64 {
    series
        .iter()
        .filter_map(|s| s.values.get(category_index))
        .sum()
}

/// Outer size of the axis plot box, in points.
///
/// A bar chart grows along the category axis, so its height rises with the
/// category count; a column chart's height is fixed. Shared with
/// [`chart_fits_on_one_page`] so the atomicity decision uses the same geometry
/// the box is actually drawn with.
fn chart_axis_extent(chart: &Chart) -> (f64, f64) {
    let (plot_w, plot_h) = axis_plot_size(chart, None);
    let legend: LegendBox = axis_legend_box(chart);
    let (label_gutter_w, label_gutter_h) = axis_label_gutters(chart);
    (
        label_gutter_w + plot_w + legend.left + legend.right,
        plot_h + label_gutter_h + legend.top + legend.bottom,
    )
}

/// Gutters the category labels and the value tick labels take inside the box,
/// alongside whatever the legend and the axis titles reserve.
fn axis_label_gutters(chart: &Chart) -> (f64, f64) {
    let (title_left, title_bottom) = axis_title_gutters(chart);
    if matches!(chart.chart_type, ChartType::Bar) {
        (LABEL_W + GAP + title_left, TICK_GAP + title_bottom)
    } else {
        (TICK_GAP + GAP + title_left, ROW + title_bottom)
    }
}

/// Space the axis titles reserve, as `(left, bottom)` in points.
///
/// The value-axis title runs a quarter turn anticlockwise down the left edge,
/// so it costs width; the category-axis title sits flat under the tick labels
/// and costs height (issue #552).
fn axis_title_gutters(chart: &Chart) -> (f64, f64) {
    (
        if chart.value_axis_title.is_some() {
            AXIS_TITLE_H
        } else {
            0.0
        },
        if chart.category_axis_title.is_some() {
            AXIS_TITLE_H
        } else {
            0.0
        },
    )
}

/// Thickness of an axis-title band: a 9pt line plus breathing room.
const AXIS_TITLE_H: f64 = 15.0;

/// Height of one data-label line, for centring it on its segment.
const LABEL_LINE_H: f64 = 10.0;

/// Size of the plotting rectangle itself.
///
/// Given a frame, the plot takes whatever is left of it after the label gutters
/// and the legend, so the chart fills its `<p:graphicFrame>` the way PowerPoint
/// lays it out. Without one it keeps the intrinsic size: `PLOT_MAIN` along the
/// value axis, one `ROW` per category across it.
fn axis_plot_size(chart: &Chart, frame: Option<(f64, f64)>) -> (f64, f64) {
    let plot_cross: f64 = chart.categories.len() as f64 * ROW;
    let (intrinsic_w, intrinsic_h) = if matches!(chart.chart_type, ChartType::Bar) {
        (PLOT_MAIN, plot_cross)
    } else {
        (plot_cross, PLOT_MAIN)
    };
    let Some((frame_w, frame_h)) = frame else {
        return (intrinsic_w, intrinsic_h);
    };
    let legend: LegendBox = axis_legend_box(chart);
    let (gutter_w, gutter_h) = axis_label_gutters(chart);
    // A frame too small for the chrome would give a negative plot, so the
    // intrinsic size is the floor rather than a source of inverted geometry.
    (
        (frame_w - gutter_w - legend.left - legend.right).max(MIN_PLOT_PT),
        (frame_h - gutter_h - legend.top - legend.bottom).max(MIN_PLOT_PT),
    )
}

/// Smallest plotting rectangle worth drawing, in points.
const MIN_PLOT_PT: f64 = 24.0;

/// Height the chart-area title block takes above the plot box: an 11pt line
/// plus the 4pt gap under it. A framed chart spends this out of its frame
/// rather than on top of it, or the plot runs past the frame's bottom edge.
const AREA_TITLE_H: f64 = 19.0;

/// Space the axis plot's legend reserves.
fn axis_legend_box(chart: &Chart) -> LegendBox {
    LegendBox::new(chart.legend_position, LEGEND_ROW_H, LEGEND_ENTRY_W)
}

/// Render a bar (horizontal) or column (vertical) chart as an axis-scaled
/// plot with gridlines, tick labels, and a legend.
fn generate_chart_axis(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    let horizontal: bool = matches!(chart.chart_type, ChartType::Bar);
    let categories: usize = chart.categories.len();
    let series: &[crate::ir::ChartSeries] = &chart.series;
    let series_count: usize = series.len().max(1);
    let stacked: bool = matches!(
        chart.grouping,
        ChartGrouping::Stacked | ChartGrouping::PercentStacked
    );

    // A stacked bar is read against its category's total, so the axis must
    // cover the tallest stack rather than the largest single segment.
    let (nice_max, step) = match chart.grouping {
        // Every stack fills the plot, so the axis is the percentage scale
        // itself and needs no rounding.
        ChartGrouping::PercentStacked => (100.0, 20.0),
        ChartGrouping::Stacked => nice_axis(
            (0..categories)
                .map(|index| category_total(series, index))
                .fold(0.0_f64, f64::max),
        ),
        ChartGrouping::Clustered => nice_axis(
            series
                .iter()
                .flat_map(|s| s.values.iter())
                .copied()
                .fold(0.0_f64, f64::max),
        ),
    };

    // Chart-area title: the explicit chart title, else the single series
    // name (which is what the audited fixture carries).
    let area_title: Option<&str> = chart.title.as_deref().or_else(|| {
        if series.len() == 1 {
            series[0].name.as_deref()
        } else {
            None
        }
    });
    if let Some(title) = area_title {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: 11pt, weight: \"bold\")[{}]]",
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }

    // The title is emitted above the box, so a framed chart's box gets what is
    // left of the frame beneath it.
    let title_h: f64 = if area_title.is_some() {
        AREA_TITLE_H
    } else {
        0.0
    };
    let frame: Option<(f64, f64)> =
        frame.map(|(width, height)| (width, (height - title_h).max(MIN_PLOT_PT)));
    let (total_w, total_h) = match frame {
        Some(extent) => extent,
        None => chart_axis_extent(chart),
    };

    let _ = writeln!(
        out,
        "#box(width: {}pt, height: {}pt)[",
        format_f64(total_w),
        format_f64(total_h)
    );

    // Plot-area origin (top-left of the plotting rectangle), shifted by
    // whatever the legend reserves on the left or above.
    let legend: LegendBox = axis_legend_box(chart);
    let (plot_w, plot_h) = axis_plot_size(chart, frame);
    let (gutter_w, _) = axis_label_gutters(chart);
    let (plot_x, plot_y) = (legend.left + gutter_w, legend.top);
    // Pitch of one category along the category axis. `ROW` is the intrinsic
    // value; a framed chart divides the axis it actually got, so widening the
    // frame widens the bars rather than leaving them stranded at one end.
    let row: f64 = if categories == 0 {
        ROW
    } else if horizontal {
        plot_h / categories as f64
    } else {
        plot_w / categories as f64
    };

    // Gridlines + value tick labels.
    let mut tick: f64 = 0.0;
    while tick <= nice_max + step * 1e-6 {
        let frac: f64 = tick / nice_max;
        if horizontal {
            let x: f64 = plot_x + frac * plot_w;
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: 0.6pt + rgb(200, 200, 200)))",
                format_f64(x),
                format_f64(plot_y),
                format_f64(plot_h)
            );
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: 24pt)[#align(center)[#text(size: 8pt)[{}]]])",
                format_f64(x - 12.0),
                format_f64(plot_y + plot_h + 4.0),
                chart_value_label(tick)
            );
        } else {
            let y: f64 = plot_y + (1.0 - frac) * plot_h;
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: 0.6pt + rgb(200, 200, 200)))",
                format_f64(plot_x),
                format_f64(y),
                format_f64(plot_w)
            );
            let _ = writeln!(
                out,
                "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: 10pt)[#align(right + horizon)[#text(size: 8pt)[{}]]])",
                format_f64(y - 5.0),
                format_f64(TICK_GAP),
                chart_value_label(tick)
            );
        }
        tick += step;
    }

    // Bars, grouped per category when multiple series are present.
    for (cat_index, category) in chart.categories.iter().enumerate() {
        let group_start: f64 = cat_index as f64 * row;
        // A stacked category is one bar, so every segment takes the full
        // thickness a clustered chart would split between the series.
        let sub: f64 = if stacked {
            row * 0.7
        } else {
            (row * 0.7) / series_count as f64
        };
        // Fraction of the axis already consumed by the segments below.
        let mut stack_base: f64 = 0.0;
        let category_total: f64 = category_total(series, cat_index);
        for (s_index, s) in series.iter().enumerate() {
            let value: f64 = s.values.get(cat_index).copied().unwrap_or(0.0);
            // Percent stacking rescales each stack to fill the axis, so an
            // XLSX column totalling 6 reads the same height as a DOCX one
            // totalling 9.
            let value: f64 = match chart.grouping {
                ChartGrouping::PercentStacked if category_total > 0.0 => {
                    value / category_total * 100.0
                }
                ChartGrouping::PercentStacked => 0.0,
                _ => value,
            };
            let frac: f64 = (value / nice_max).clamp(0.0, 1.0);
            let color: String = series_color(s, s_index, cat_index);
            let offset: f64 = if stacked {
                row * 0.15
            } else {
                row * 0.15 + s_index as f64 * sub
            };
            if horizontal {
                // Bar charts stack categories bottom-up.
                let row_top: f64 = plot_h - (cat_index as f64 + 1.0) * row;
                let bar_w: f64 = frac * plot_w;
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, rect(width: {}pt, height: {}pt, fill: {}, stroke: none))",
                    format_f64(plot_x + stack_base * plot_w),
                    format_f64(row_top + offset),
                    format_f64(bar_w.max(0.0)),
                    format_f64(sub),
                    color
                );
            } else {
                let bar_h: f64 = frac * plot_h;
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, rect(width: {}pt, height: {}pt, fill: {}, stroke: none))",
                    format_f64(plot_x + group_start + offset),
                    format_f64(plot_y + plot_h - bar_h - stack_base * plot_h),
                    format_f64(sub),
                    format_f64(bar_h.max(0.0)),
                    color
                );
            }
            if let Some(label) = data_label_text(chart, s, cat_index) {
                // Centred on the segment, as `<c:dLblPos val="ctr"/>` asks and
                // as a stacked bar needs — an outside label would sit on the
                // segment above it.
                let (label_x, label_y, label_w) = if horizontal {
                    let row_top: f64 = plot_h - (cat_index as f64 + 1.0) * row;
                    (
                        plot_x + stack_base * plot_w,
                        row_top + offset + sub / 2.0 - LABEL_LINE_H / 2.0,
                        frac * plot_w,
                    )
                } else {
                    (
                        plot_x + group_start + offset,
                        plot_y + plot_h
                            - stack_base * plot_h
                            - frac * plot_h / 2.0
                            - LABEL_LINE_H / 2.0,
                        sub,
                    )
                };
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#text(size: 8pt, weight: \"bold\", fill: white)[{}]]])",
                    format_f64(label_x),
                    format_f64(label_y),
                    format_f64(label_w.max(0.0)),
                    format_f64(LABEL_LINE_H),
                    escape_typst(&label)
                );
            }
            if stacked {
                stack_base += frac;
            }
        }
        // Category label.
        if horizontal {
            let row_top: f64 = plot_h - (cat_index as f64 + 1.0) * row;
            let _ = writeln!(
                out,
                "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(right + horizon)[#text(size: 9pt)[{}]]])",
                format_f64(row_top),
                format_f64(LABEL_W),
                format_f64(row),
                escape_typst(category)
            );
        } else {
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#text(size: 9pt)[{}]]])",
                format_f64(plot_x + group_start),
                format_f64(plot_y + plot_h + 2.0),
                format_f64(row),
                format_f64(ROW),
                escape_typst(category)
            );
        }
    }

    // Axis baseline (value = 0 line).
    if horizontal {
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: 0.8pt + rgb(120, 120, 120)))",
            format_f64(plot_x),
            format_f64(plot_y),
            format_f64(plot_h)
        );
    } else {
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: 0.8pt + rgb(120, 120, 120)))",
            format_f64(plot_x),
            format_f64(plot_y + plot_h),
            format_f64(plot_w)
        );
    }

    // Axis titles, in the bands `axis_title_gutters` reserved for them.
    if let Some(title) = chart.value_axis_title.as_deref() {
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#rotate(-90deg, reflow: false)[#text(size: 9pt, weight: \"bold\")[{}]]]])",
            format_f64(legend.left),
            format_f64(plot_y),
            format_f64(AXIS_TITLE_H),
            format_f64(plot_h),
            escape_typst(title)
        );
    }
    if let Some(title) = chart.category_axis_title.as_deref() {
        let (_, gutter_h) = axis_label_gutters(chart);
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#text(size: 9pt, weight: \"bold\")[{}]]])",
            format_f64(plot_x),
            format_f64(plot_y + plot_h + 2.0 + gutter_h - AXIS_TITLE_H),
            format_f64(plot_w),
            format_f64(AXIS_TITLE_H),
            escape_typst(title)
        );
    }

    // Legend on the edge `<c:legendPos>` asks for.
    for (s_index, s) in series.iter().enumerate() {
        let color: String = series_color(s, s_index, 0);
        let default_name: String = format!("Series {}", s_index + 1);
        let name: &str = s.name.as_deref().unwrap_or(&default_name);
        // The content the legend sits beside spans the plot and both label
        // gutters, so a bottom legend clears the category labels.
        let (gutter_w, gutter_h) = if horizontal {
            (LABEL_W + GAP, TICK_GAP)
        } else {
            (TICK_GAP + GAP, ROW)
        };
        let (entry_x, entry_y) = legend.entry_origin(
            chart.legend_position,
            s_index,
            series_count,
            (
                plot_x - gutter_w,
                plot_y,
                gutter_w + plot_w,
                plot_h + gutter_h,
            ),
            LEGEND_ROW_H,
            LEGEND_ENTRY_W,
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box[#box(width: 9pt, height: 9pt, fill: {}) #text(size: 9pt)[{}]])",
            format_f64(entry_x),
            format_f64(entry_y),
            color,
            escape_typst(name)
        );
    }

    out.push_str("]\n");
}

fn generate_chart_bar(out: &mut String, chart: &Chart) {
    let max_value: f64 = chart
        .series
        .iter()
        .flat_map(|series| series.values.iter())
        .copied()
        .fold(0.0_f64, f64::max);
    let max_value: f64 = if max_value == 0.0 { 1.0 } else { max_value };

    let colors: &[&str] = &CHART_CATEGORY_COLORS[..4];

    for (row_index, category) in chart.categories.iter().enumerate() {
        let escaped_category: String = escape_typst(category);
        let _ = writeln!(out, "#text(weight: \"bold\")[{escaped_category}]");
        for (series_index, series) in chart.series.iter().enumerate() {
            let value: f64 = series.values.get(row_index).copied().unwrap_or(0.0);
            let percent: u32 = (value / max_value * 100.0).round().min(100.0) as u32;
            // The fallback here indexes by series, not by point, because each
            // row of this table is one category across all series.
            let color: String = match series.fill_for_point(row_index) {
                Some(declared) => rgb(&declared),
                None => colors[series_index % colors.len()].to_string(),
            };
            let _ = writeln!(
                out,
                "#box(width: {percent}%, height: 14pt, fill: {color}, radius: 2pt)[#text(size: 8pt, fill: white)[ {}]]",
                format_f64(value)
            );
        }
        let _ = writeln!(out);
    }

    if chart.series.len() > 1 {
        let _ = writeln!(out);
        for (index, series) in chart.series.iter().enumerate() {
            let default_name: String = format!("Series {}", index + 1);
            let name: &str = series.name.as_deref().unwrap_or(&default_name);
            let color: &str = colors[index % colors.len()];
            let _ = writeln!(
                out,
                "#box(width: 10pt, height: 10pt, fill: {color}) #text(size: 9pt)[{name}] "
            );
        }
    }
}

/// Render a line/area chart as a polyline plot over a value axis, matching
/// the native Excel/PowerPoint composition (gridlines, tick labels, category
/// axis, markers, legend).
fn generate_chart_line_plot(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    const PLOT_W: f64 = 320.0;
    const PLOT_H: f64 = 210.0;
    const VALUE_GAP: f64 = 24.0; // value tick label gutter (left)
    const CAT_GAP: f64 = 18.0; // category label gutter (bottom)
    const LEGEND_W: f64 = 88.0;
    const LINE_LEGEND_ROW_H: f64 = 16.0;
    const INSET: f64 = 10.0; // keep first/last points off the axes
    const GAP: f64 = 6.0;

    let categories: usize = chart.categories.len();
    let series: &[crate::ir::ChartSeries] = &chart.series;

    let max_value: f64 = series
        .iter()
        .flat_map(|s| s.values.iter())
        .copied()
        .fold(0.0_f64, f64::max);
    let (nice_max, step) = nice_axis(max_value);

    if let Some(title) = chart.title.as_deref() {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: 11pt, weight: \"bold\")[{}]]",
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }
    // As in `generate_chart_axis`: the title sits above the box, so a framed
    // chart spends its height out of the frame rather than on top of it.
    let frame: Option<(f64, f64)> = frame.map(|(width, height)| {
        let title_h: f64 = if chart.title.is_some() {
            AREA_TITLE_H
        } else {
            0.0
        };
        (width, (height - title_h).max(MIN_PLOT_PT))
    });

    let legend: LegendBox = LegendBox::new(chart.legend_position, LINE_LEGEND_ROW_H, LEGEND_W);
    // A framed chart fills its `<p:graphicFrame>`; a flowed one keeps the
    // intrinsic plot size (issue #548).
    let (plot_w, plot_h) = match frame {
        Some((frame_w, frame_h)) => (
            (frame_w - (VALUE_GAP + GAP) - legend.left - legend.right).max(MIN_PLOT_PT),
            (frame_h - CAT_GAP - legend.top - legend.bottom).max(MIN_PLOT_PT),
        ),
        None => (PLOT_W, PLOT_H),
    };
    let plot_x: f64 = legend.left + VALUE_GAP + GAP;
    let plot_y: f64 = legend.top;
    let (total_w, total_h) = match frame {
        Some(extent) => extent,
        None => (
            legend.left + VALUE_GAP + GAP + PLOT_W + legend.right,
            legend.top + PLOT_H + CAT_GAP + legend.bottom,
        ),
    };
    let _ = writeln!(
        out,
        "#box(width: {}pt, height: {}pt)[",
        format_f64(total_w),
        format_f64(total_h)
    );

    // Horizontal gridlines + value tick labels.
    let mut tick: f64 = 0.0;
    while tick <= nice_max + step * 1e-6 {
        let y: f64 = plot_y + (1.0 - tick / nice_max) * plot_h;
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: 0.6pt + rgb(200, 200, 200)))",
            format_f64(plot_x),
            format_f64(y),
            format_f64(plot_w)
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: 10pt)[#align(right + horizon)[#text(size: 8pt)[{}]]])",
            format_f64(y - 5.0),
            format_f64(VALUE_GAP),
            chart_value_label(tick)
        );
        tick += step;
    }

    let point_x = |index: usize| -> f64 {
        if categories <= 1 {
            plot_x + plot_w / 2.0
        } else {
            plot_x + INSET + (index as f64 / (categories as f64 - 1.0)) * (plot_w - 2.0 * INSET)
        }
    };
    let point_y =
        |value: f64| -> f64 { plot_y + (1.0 - (value / nice_max).clamp(0.0, 1.0)) * plot_h };

    // Category axis labels.
    for (index, category) in chart.categories.iter().enumerate() {
        let x: f64 = point_x(index);
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box(width: 24pt)[#align(center)[#text(size: 8pt)[{}]]])",
            format_f64(x - 12.0),
            format_f64(plot_y + plot_h + 3.0),
            escape_typst(category)
        );
    }

    // Series polylines + markers.
    for (s_index, s) in series.iter().enumerate() {
        let color: String = series_color(s, s_index, 0);
        let points: Vec<(f64, f64)> = s
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| (point_x(index), point_y(*value)))
            .collect();
        if points.len() >= 2 {
            let coords: String = points
                .iter()
                .map(|(x, y)| format!("({}pt, {}pt)", format_f64(*x), format_f64(*y)))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "#place(top + left, path(stroke: 2pt + {color}, {coords}))"
            );
        }
        // Point markers.
        for (x, y) in &points {
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, rect(width: 5pt, height: 5pt, fill: {color}, stroke: none))",
                format_f64(x - 2.5),
                format_f64(y - 2.5)
            );
        }
    }

    // Value/category axis lines.
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: 0.8pt + rgb(120, 120, 120)))",
        format_f64(plot_x),
        format_f64(plot_y),
        format_f64(plot_h)
    );
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: 0.8pt + rgb(120, 120, 120)))",
        format_f64(plot_x),
        format_f64(plot_y + plot_h),
        format_f64(plot_w)
    );

    // Legend on the edge `<c:legendPos>` asks for.
    for (s_index, s) in series.iter().enumerate() {
        let color: String = series_color(s, s_index, 0);
        let default_name: String = format!("Series {}", s_index + 1);
        let name: &str = s.name.as_deref().unwrap_or(&default_name);
        let (entry_x, entry_y) = legend.entry_origin(
            chart.legend_position,
            s_index,
            series.len().max(1),
            (
                plot_x - (VALUE_GAP + GAP),
                plot_y,
                VALUE_GAP + GAP + plot_w,
                plot_h + CAT_GAP,
            ),
            LINE_LEGEND_ROW_H,
            LEGEND_W,
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box[#box(width: 12pt, height: 3pt, fill: {color}, baseline: -3pt) #text(size: 9pt)[{}]])",
            format_f64(entry_x),
            format_f64(entry_y),
            escape_typst(name)
        );
    }

    out.push_str("]\n");
}

fn generate_chart_pie(out: &mut String, chart: &Chart) {
    let Some(series) = chart.series.first() else {
        return;
    };

    let total: f64 = series.values.iter().sum();
    let total: f64 = if total == 0.0 { 1.0 } else { total };

    let colors: &[&str] = &CHART_CATEGORY_COLORS;

    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: 3,");
    let _ = writeln!(out, "  [*Slice*], [*Value*], [*%*],");

    for (index, category) in chart.categories.iter().enumerate() {
        let value: f64 = series.values.get(index).copied().unwrap_or(0.0);
        let percent: f64 = value / total * 100.0;
        let escaped_category: String = escape_typst(category);
        // Each pie slice is one data point of the single series, so a
        // `<c:dPt>` fill names the wedge's colour directly.
        let color: String = category_color(series, index, colors);
        let _ = writeln!(
            out,
            "  [#box(width: 8pt, height: 8pt, fill: {color}) {escaped_category}], [{}], [{:.1}%],",
            format_f64(value),
            percent
        );
    }

    let _ = writeln!(out, ")\n");
}

fn generate_chart_line(out: &mut String, chart: &Chart) {
    let column_count: usize = 1 + chart.series.len();
    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: {column_count},");

    out.push_str("  [*Category*], ");
    for (index, series) in chart.series.iter().enumerate() {
        let default_name: String = format!("Series {}", index + 1);
        let name: &str = series.name.as_deref().unwrap_or(&default_name);
        let _ = write!(out, "[*{name}*]");
        if index + 1 < chart.series.len() {
            out.push_str(", ");
        }
    }
    out.push_str(",\n");

    for (row_index, category) in chart.categories.iter().enumerate() {
        let escaped_category: String = escape_typst(category);
        let _ = write!(out, "  [{escaped_category}], ");
        for (series_index, series) in chart.series.iter().enumerate() {
            let value: f64 = series.values.get(row_index).copied().unwrap_or(0.0);
            let trend: &str = if row_index > 0 {
                let previous: f64 = series.values.get(row_index - 1).copied().unwrap_or(0.0);
                if value > previous {
                    " ↑"
                } else if value < previous {
                    " ↓"
                } else {
                    " →"
                }
            } else {
                ""
            };
            let _ = write!(out, "[{}{}]", format_f64(value), trend);
            if series_index + 1 < chart.series.len() {
                out.push_str(", ");
            }
        }
        out.push_str(",\n");
    }

    let _ = writeln!(out, ")\n");
}

fn generate_chart_table(out: &mut String, chart: &Chart) {
    let column_count: usize = 1 + chart.series.len();
    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: {column_count},");

    out.push_str("  [*Category*], ");
    for (index, series) in chart.series.iter().enumerate() {
        let default_name: String = format!("Series {}", index + 1);
        let name: &str = series.name.as_deref().unwrap_or(&default_name);
        let _ = write!(out, "[*{name}*]");
        if index + 1 < chart.series.len() {
            out.push_str(", ");
        }
    }
    out.push_str(",\n");

    for (row_index, category) in chart.categories.iter().enumerate() {
        let escaped_category: String = escape_typst(category);
        let _ = write!(out, "  [{escaped_category}], ");
        for (index, series) in chart.series.iter().enumerate() {
            let value: f64 = series.values.get(row_index).copied().unwrap_or(0.0);
            let _ = write!(out, "[{}]", format_f64(value));
            if index + 1 < chart.series.len() {
                out.push_str(", ");
            }
        }
        out.push_str(",\n");
    }

    let _ = writeln!(out, ")\n");
}

/// Generate Typst markup for a SmartArt diagram.
///
/// Renders SmartArt as a visually distinct bordered box with:
/// - Hierarchy items (varying depths): indented tree with depth-based padding
/// - Flat items (all same depth): numbered steps with arrows
pub(super) fn generate_smartart(out: &mut String, smartart: &SmartArt, width: f64, height: f64) {
    let _ = writeln!(
        out,
        "#block(width: {}pt, height: {}pt, stroke: 1pt + rgb(70, 130, 180), radius: 4pt, inset: 10pt, fill: rgb(245, 248, 255))[",
        format_f64(width),
        format_f64(height),
    );
    let _ = writeln!(
        out,
        "#align(center)[#text(size: 11pt, weight: \"bold\", fill: rgb(70, 130, 180))[SmartArt Diagram]]\n"
    );

    if smartart.items.is_empty() {
        out.push_str("]\n");
        return;
    }

    let has_hierarchy: bool = smartart.items.iter().any(|node| node.depth > 0);

    if has_hierarchy {
        generate_smartart_hierarchy(out, smartart);
    } else {
        generate_smartart_steps(out, smartart);
    }

    out.push_str("]\n");
}

fn generate_smartart_hierarchy(out: &mut String, smartart: &SmartArt) {
    for node in &smartart.items {
        let escaped: String = escape_typst(&node.text);
        if node.depth == 0 {
            let _ = writeln!(out, "#text(weight: \"bold\")[{escaped}]");
        } else {
            let indent: f64 = node.depth as f64 * 16.0;
            let branch: &str = if node.depth == 1 { "├" } else { "└" };
            let _ = writeln!(
                out,
                "#pad(left: {}pt)[{branch} {escaped}]",
                format_f64(indent),
            );
        }
    }
}

fn generate_smartart_steps(out: &mut String, smartart: &SmartArt) {
    for (index, node) in smartart.items.iter().enumerate() {
        let escaped: String = escape_typst(&node.text);
        let step_number: usize = index + 1;
        let _ = writeln!(
            out,
            "#box(stroke: 0.5pt + rgb(70, 130, 180), radius: 3pt, inset: 6pt)[#text(weight: \"bold\")[{}. ] {escaped}]",
            step_number,
        );
        if index + 1 < smartart.items.len() {
            let _ = writeln!(out, "#align(center)[#text(size: 14pt)[↓]]");
        }
    }
}

#[cfg(test)]
mod chart_value_label_tests {
    use super::{chart_value_label, nice_axis};

    #[test]
    fn formats_without_float_noise() {
        assert_eq!(chart_value_label(8.200000000000001), "8.2");
        assert_eq!(chart_value_label(3.0), "3");
        assert_eq!(chart_value_label(0.0), "0");
        assert_eq!(chart_value_label(1234.5), "1234.5");
        assert_eq!(chart_value_label(0.333333333), "0.3333");
    }

    #[test]
    fn nice_axis_rounds_up() {
        assert_eq!(nice_axis(8.2), (10.0, 2.0));
        assert_eq!(nice_axis(3.2), (4.0, 1.0));
        assert_eq!(nice_axis(45.0), (50.0, 10.0));
        assert_eq!(nice_axis(0.0), (1.0, 1.0));
    }

    #[test]
    fn nice_axis_matches_excels_auto_scale() {
        // Measured from native Excel and PowerPoint exports of the audited
        // fixtures. Rounding the maximum itself to 1/2/5x10^n put 23,334
        // against a 50,000 axis, drawing every column at half height (#553).
        assert_eq!(nice_axis(23334.0), (25000.0, 5000.0), "01_대시보드 LOC");
        assert_eq!(nice_axis(9.0), (10.0, 2.0), "slide 17 stack totals");
        assert_eq!(nice_axis(78.0), (80.0, 20.0), "release intervals");
    }

    #[test]
    fn nice_axis_leaves_no_more_than_one_step_of_headroom() {
        // The property that makes a chart readable: the tallest bar always
        // reaches within one major unit of the top.
        for value in [
            1.0,
            2.0,
            3.7,
            9.0,
            23.0,
            45.0,
            78.0,
            99.0,
            100.0,
            250.0,
            4999.0,
            23334.0,
            1_000_001.0,
        ] {
            let (max, step) = nice_axis(value);

            assert!(max >= value, "axis {max} must cover {value}");
            assert!(
                max - value < step,
                "axis {max} leaves {} of headroom over {value}, more than one {step} step",
                max - value
            );
            assert!(
                step > 0.0 && (max / step - (max / step).round()).abs() < 1e-9,
                "{max} must divide into whole {step} steps"
            );
        }
    }
}

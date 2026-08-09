use super::*;
use crate::ir::{ChartAreaOutline, DataLabelPosition};
use crate::render::font_subst;

/// How a chart is drawn. Selecting the variant once lets the atomicity decision
/// and the emitter agree on which geometry applies.
enum ChartVariant {
    /// Axis-scaled bar/column plot with gridlines, tick labels, and a legend.
    AxisPlot,
    /// Polyline plot over a value axis, for line and area charts.
    LinePlot,
    /// Circular plot whose wedges are each point's share of the total.
    PiePlot,
    /// One spoke per category radiating from a centre, each series a closed
    /// polygon through its value on every spoke.
    RadarPlot,
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
    // A radar needs a closed ring of spokes, so two categories cannot make one.
    if is_radar(chart)
        && !chart.series.is_empty()
        && chart.categories.len() >= 3
        && chart
            .series
            .iter()
            .any(|series| series.values.iter().any(|value| *value > 0.0))
    {
        return ChartVariant::RadarPlot;
    }
    if matches!(chart.chart_type, ChartType::Pie | ChartType::Doughnut)
        && chart
            .series
            .first()
            .is_some_and(|series| series.values.iter().any(|value| *value > 0.0))
    {
        return ChartVariant::PiePlot;
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
        // The polyline, pie and radar plots are a fixed size regardless of how
        // many points they carry.
        ChartVariant::LinePlot | ChartVariant::PiePlot | ChartVariant::RadarPlot => return true,
        ChartVariant::BorderedTable => {
            BORDERED_TABLE_CHROME_PT + chart.categories.len() as f64 * BORDERED_TABLE_ROW_PT
        }
    };
    height <= MAX_ATOMIC_CHART_HEIGHT_PT
}

/// Generate Typst markup for a chart.
///
/// Bar and column charts render as an axis-scaled plot; line and area charts
/// as a polyline plot over the same axis; pie and doughnut charts as a wedge
/// plot; and a radar carrying at least three categories and one positive value
/// as a spoke-and-polygon plot. What is left — bubble, stock, surface, and a
/// radar too small or too flat to draw — falls back to a bordered box holding
/// the title, a type label, and a data table.
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
    // `c:chartSpace/c:txPr` sets the face for every string the chart draws, and
    // no sub-renderer below names a font of its own, so one scoped `set` reaches
    // the title, tick labels, legend and data labels alike. Without it they all
    // fell through to the engine's default serif, a face that appears nowhere
    // else in the document (issue #668).
    let font_scope: Option<String> = chart_text_font_scope(chart);
    if let Some(ref scope) = font_scope {
        out.push_str("#[\n");
        out.push_str(scope);
    }
    generate_chart_body(out, chart, frame);
    if font_scope.is_some() {
        out.push_str("]\n");
    }
    if atomic {
        out.push_str("]\n");
    }
}

/// The `#set text(font: …)` a chart's declared face calls for, or `None` when
/// it names none and the theme supplied nothing either.
///
/// The fallback chain is built from the chart's own strings, because they carry
/// the scripts: a Korean category label needs the East Asian chain that a Latin
/// family alone would not reach.
fn chart_text_font_scope(chart: &Chart) -> Option<String> {
    let family: &str = chart.text_font_family.as_deref()?;
    let sample: String = chart.text_sample();
    Some(format!(
        "#set text(font: {})\n",
        font_subst::font_for_mixed_script_text(family, &sample)
    ))
}

/// Emit the chart's own markup, without the atomicity wrapper.
fn generate_chart_body(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    match chart_variant(chart) {
        ChartVariant::AxisPlot => return generate_chart_axis(out, chart, frame),
        ChartVariant::LinePlot => return generate_chart_line_plot(out, chart, frame),
        ChartVariant::PiePlot => return generate_chart_pie_plot(out, chart, frame),
        ChartVariant::RadarPlot => return generate_chart_radar_plot(out, chart, frame),
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
        ChartType::Doughnut => "Doughnut Chart",
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

/// Fallback series palette — the Office 2013+ default accents.
///
/// Reached only when the file's own theme supplies no usable accent list;
/// see [`automatic_color`]. A file built on another theme that lands here
/// is recoloured, which is what issue #670 was.
const CHART_SERIES_COLORS: [&str; 6] = [
    "rgb(68, 114, 196)",
    "rgb(237, 125, 49)",
    "rgb(165, 165, 165)",
    "rgb(255, 192, 0)",
    "rgb(91, 155, 213)",
    "rgb(112, 173, 71)",
];

/// Side of an automatic series marker, in points.
///
/// Left at 5pt rather than changed. #635 reports the marker is about twice
/// Excel's, but the only reference available here is a LibreOffice render of
/// `WithChart.xlsx`, whose markers measure 5.0 x 5.0pt — the same as ours. That
/// is not a measurement of Excel, so it does not disprove the report; it means
/// there was nothing to size against, and guessing would be as likely to move
/// away from Excel as toward it.
const SERIES_MARKER_SIZE_PT: f64 = 5.0;

/// Weight a line series' polyline is stroked at.
///
/// Shared with the legend key, which Excel draws as a sample of the line
/// itself: a key drawn at some other weight stops standing for its series
/// (#801).
pub(super) const SERIES_LINE_PT: f64 = 2.0;

/// `baseline:` offset that sets the legend key against its label.
///
/// The native export puts the key line 2.64pt above its label's baseline, and
/// -0.5 reproduces that. The previous flat bar sat 4.08pt high, three pixels
/// out at the 150 DPI this is measured at.
///
/// Calibrated rather than derived: raising the box also grows the line's ascent
/// and carries the baseline with it, so the offset is not a plain translation
/// of the key. Measured on this fixture, the key rises 2.16pt at an offset of
/// zero and 4.32pt at -2.
pub(super) const LEGEND_KEY_BASELINE_PT: f64 = -0.5;

/// Length of a line series' legend key.
///
/// Measured on the native Excel export of `WithChart.xlsx` at 150 DPI: the two
/// keys run 20.16pt and 20.64pt, either side of a 20pt nominal.
pub(super) const LEGEND_KEY_LEN_PT: f64 = 20.0;

/// Explicit space between a legend key and its label.
///
/// Zero removes Typst's implicit document-sized word space while leaving the
/// label glyph's own side bearing intact. The remaining bearing differs until
/// chart text resolves its declared theme face (#668), so a compensating
/// negative gap would overfit the current fallback (#804).
const LEGEND_KEY_LABEL_GAP_PT: f64 = 0.0;

/// Marker shape for the `index`-th series, when the file asks for a default
/// marker rather than naming a `c:symbol`.
///
/// The sequence exists so adjacent series stay apart in monochrome; drawing one
/// square for every series defeats it (issue #635).
///
/// The first two entries are confirmed against the native Excel export of
/// `WithChart.xlsx`: at 150 DPI its first series carries a diamond and its
/// second a square, both in the plot and on the legend key. LibreOffice cycles
/// the same two the other way round on that file, so its render is not evidence
/// about the order — only the native one is.
///
/// Entries beyond the second remain the order #635 states Excel uses, with
/// nothing here checking them; that workbook has only two series.
fn write_series_marker(out: &mut String, series_index: usize, x: f64, y: f64, color: &str) {
    out.push_str(&series_marker_markup(series_index, x, y, color));
}

/// The `#place`d markup for one series marker centred on (`x`, `y`).
///
/// Returned rather than written so the legend key can embed the same marker the
/// plot draws, instead of restating the shape cycle (#801).
fn series_marker_markup(series_index: usize, x: f64, y: f64, color: &str) -> String {
    let size: f64 = SERIES_MARKER_SIZE_PT;
    let half: f64 = size / 2.0;
    let left: String = format_f64(x - half);
    let top: String = format_f64(y - half);
    let full: String = format_f64(size);
    let mid: String = format_f64(half);

    let shape: String = match series_index % 4 {
        // Diamond.
        0 => format!(
            "polygon(fill: {color}, stroke: none, ({mid}pt, 0pt), ({full}pt, {mid}pt), ({mid}pt, {full}pt), (0pt, {mid}pt))"
        ),
        // Square.
        1 => format!("rect(width: {full}pt, height: {full}pt, fill: {color}, stroke: none)"),
        // Triangle.
        2 => format!(
            "polygon(fill: {color}, stroke: none, ({mid}pt, 0pt), ({full}pt, {full}pt), (0pt, {full}pt))"
        ),
        // Cross, as a filled X.
        _ => {
            let thin: String = format_f64(size / 3.0);
            let thick: String = format_f64(size * 2.0 / 3.0);
            format!(
                "polygon(fill: {color}, stroke: none, ({thin}pt, 0pt), ({thick}pt, 0pt), ({thick}pt, {thin}pt), ({full}pt, {thin}pt), ({full}pt, {thick}pt), ({thick}pt, {thick}pt), ({thick}pt, {full}pt), ({thin}pt, {full}pt), ({thin}pt, {thick}pt), (0pt, {thick}pt), (0pt, {thin}pt), ({thin}pt, {thin}pt))"
            )
        }
    };
    format!("#place(top + left, dx: {left}pt, dy: {top}pt, {shape})\n")
}

/// The automatic colour for the `index`-th slot, from the file's own theme.
///
/// A chart that states no fill takes `accent1`..`accent6` of the theme its
/// package declares. Only when the package supplies no usable accent list does
/// the built-in palette stand in — that palette is the Office 2013+ one, so
/// using it on a file built from another theme recolours the chart (#670).
fn automatic_color(theme_accents: &[Color], index: usize, fallback: &[&str]) -> String {
    if theme_accents.is_empty() {
        return fallback[index % fallback.len()].to_string();
    }
    rgb(&theme_accents[index % theme_accents.len()])
}

/// The Typst colour for one plotted point.
///
/// A point's own `<c:dPt>` fill outranks its series' `<c:spPr>` fill, and an
/// automatic colour is the fallback for charts that declare neither — not a
/// replacement for what the file states (issue #535).
fn series_color(
    series: &crate::ir::ChartSeries,
    series_index: usize,
    point_index: usize,
    theme_accents: &[Color],
) -> String {
    match series.fill_for_point(point_index) {
        Some(color) => rgb(&color),
        None => automatic_color(theme_accents, series_index, &CHART_SERIES_COLORS),
    }
}

/// As [`series_color`], but for the plots that colour by data point rather
/// than by series, so the accent advances with the point.
fn category_color(
    series: &crate::ir::ChartSeries,
    point_index: usize,
    palette: &[&str],
    theme_accents: &[Color],
) -> String {
    match series.fill_for_point(point_index) {
        Some(color) => rgb(&color),
        None => automatic_color(theme_accents, point_index, palette),
    }
}

/// Category palette used by the bar-plot and pie-table fallbacks.
///
/// Like [`CHART_SERIES_COLORS`], this now sits behind the file's own theme
/// accents and is reached only when those are absent.
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

/// A chart value rendered through the number format its data declares, or
/// through [`chart_value_label`] when it declares none.
///
/// A chart stores a ratio as a fraction and says `0.00%` beside it in
/// `<c:numCache><c:formatCode>`, so a value axis and its data labels printed
/// `0.2` and `0.024` where the source, and every other renderer, show `20%`
/// and `2.4%` (issue #865). The formatter is the one the XLSX path already
/// uses, so a code means the same thing in both.
pub(super) fn chart_value_label_formatted(value: f64, number_format: Option<&str>) -> String {
    match number_format {
        Some(format_code) => umya_spreadsheet::helper::number_format::to_formatted_string(
            value.to_string(),
            format_code,
        ),
        None => chart_value_label(value),
    }
}

/// The number format a chart's value axis and data labels take: the first one
/// any series declares. A chart's series share one value axis, and Office
/// writes the same code into each series' cache.
pub(super) fn chart_value_number_format(chart: &Chart) -> Option<&str> {
    chart
        .value_axis_number_format
        .as_deref()
        .or_else(|| chart.series.iter().find_map(|s| s.number_format.as_deref()))
}

/// The number format one series' data labels take: the label's own, else the
/// series' cache format, which is the source cell's.
pub(super) fn series_label_number_format(series: &crate::ir::ChartSeries) -> Option<&str> {
    series
        .data_labels
        .number_format
        .as_deref()
        .or(series.number_format.as_deref())
}

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

/// Apply `<c:majorUnit>` to an axis the auto-scale has already sized.
///
/// The stated unit sets the interval; the maximum is then the fewest whole
/// units that still cover the data, so a declared 0.2 on a 0.689 maximum gives
/// ticks at 0, 0.2, 0.4, 0.6, 0.8 rather than the automatic tenths (#882).
fn axis_with_stated_unit(axis: (f64, f64), stated: Option<f64>) -> (f64, f64) {
    let (nice_max, step) = axis;
    let Some(unit) = stated.filter(|unit| unit.is_finite() && *unit > 0.0) else {
        return (nice_max, step);
    };
    let covered: f64 = (nice_max / unit - 1e-9).ceil().max(1.0) * unit;
    (covered, unit)
}

/// Choose Excel's automatic axis maximum and major unit covering `[0, max]`
/// (e.g. max 8.2 → (10, 1), giving ticks 0,1,…,10).
fn nice_axis(max_value: f64) -> (f64, f64) {
    if max_value <= 0.0 {
        return (1.0, 1.0);
    }
    // Excel clears the data by a twentieth of the range *before* rounding, so
    // the tallest bar stops short of the top. Rounding the bare maximum put a
    // 17 maximum under an axis of 18 only by accident of the step, and a
    // maximum of 100 flush against a 100 axis (#634).
    let cleared: f64 = max_value + max_value / AXIS_HEADROOM_DIVISOR;
    let magnitude: f64 = 10f64.powf(cleared.log10().floor());
    let mantissa: f64 = cleared / magnitude;
    let step: f64 = MAJOR_UNIT_FRACTIONS
        .iter()
        .find(|(upper, _)| mantissa < *upper)
        .map_or(1.0, |(_, fraction)| *fraction)
        * magnitude;
    // The step, not the maximum, carries the rounding: the axis is the fewest
    // whole steps that cover the cleared data. Rounding the maximum to the
    // ladder itself put 23,334 against 50,000 (#553).
    let nice_max: f64 = (cleared / step - 1e-9).ceil() * step;
    (nice_max, step)
}

/// Share of the plotted range an auto-scaled axis clears the data by before it
/// rounds up to a whole number of major units.
///
/// Excel's documented rule puts the axis maximum at the first major unit above
/// `Ymax + (Ymax - Ymin)/20`, so the tallest bar never touches the top. Only
/// the divisor is documented; the major unit itself is not, hence
/// [`MAJOR_UNIT_FRACTIONS`].
const AXIS_HEADROOM_DIVISOR: f64 = 20.0;

/// Major unit an auto-scaled axis takes, as a fraction of the power of ten
/// below the cleared maximum, keyed by the exclusive upper bound of that
/// maximum's mantissa.
///
/// Excel does not document how it picks the unit. Measured across rescalings of
/// one auto-scaled chart, the unit is a step function of the mantissa: 1.78 and
/// 2.45 give 0.2 and 0.5 in Excel's own exports (issues #634 and #553), and
/// every mantissa from 1.0 to 9.9 agrees with these three bands in
/// LibreOffice's renderings of the same files.
///
/// The interval count is therefore not constant — it runs from 4 to 10 across a
/// decade, and aiming for a fixed five instead is what put a 17 maximum under a
/// 20 axis in five steps rather than Excel's 18 in nine (#634).
const MAJOR_UNIT_FRACTIONS: [(f64, f64); 3] = [(2.0, 0.2), (5.0, 0.5), (10.0, 1.0)];

/// The stroke PowerPoint draws for an automatic major gridline and for an
/// automatic axis line: 0.75pt (9525 EMU) in `#868686`.
///
/// A `c:majorGridlines` carrying no `c:spPr` leaves the renderer to supply its
/// own default. Ours was 0.6pt `#C8C8C8`, which puts roughly a quarter of the
/// ink on each line and left the grid barely visible against a white plot area.
/// The axis line ran a milder version of the same drift at 0.8pt `#787878`
/// (issue #673).
pub(super) const CHART_AUTOMATIC_LINE: &str = "0.75pt + rgb(134, 134, 134)";

/// The stroke to draw one piece of chart chrome with, or `None` when the part
/// suppressed it with `<a:ln><a:noFill/></a:ln>` and nothing should be drawn
/// at all (issue #900).
///
/// A stated line falls back to [`CHART_AUTOMATIC_LINE`] for whichever half it
/// leaves out, so a `<a:ln>` naming only a width keeps the automatic colour.
fn chart_chrome_stroke(declared: crate::ir::ChartLine) -> Option<String> {
    match declared {
        crate::ir::ChartLine::Automatic => Some(CHART_AUTOMATIC_LINE.to_string()),
        crate::ir::ChartLine::Suppressed => None,
        crate::ir::ChartLine::Explicit { width_pt, color } => Some(format!(
            "{}pt + {}",
            format_f64(width_pt.unwrap_or(CHART_AUTOMATIC_LINE_PT)),
            color.map_or_else(
                || CHART_AUTOMATIC_LINE_RGB.to_string(),
                |c| format!("rgb({}, {}, {})", c.r, c.g, c.b)
            )
        )),
    }
}

/// Outline **Excel** draws around the whole chart area — plot, axis labels and
/// legend alike — when the file states no `c:chartSpace/c:spPr/a:ln`.
///
/// Not Office's in general: PowerPoint draws none in the same case, so which
/// hosts reach this is [`automatic_chart_area_stroke`]'s decision, not this
/// constant's (issue #823).
///
/// It is the same stroke as the gridlines: the native Excel export of
/// `WithChart.xlsx` draws the border as a single grey pixel at 150 DPI. In the
/// committed `assets/bugfixes/issue-637/gt.jpg`, pixel (104, 300) — on the
/// border's left edge — samples RGB(133,133,133), against the RGB(134,134,134)
/// of [`CHART_AUTOMATIC_LINE`]; the one-level gap is the JPEG. Without the
/// outline a chart has no boundary against the sheet behind it (#637).
pub(super) const CHART_AREA_OUTLINE: &str = CHART_AUTOMATIC_LINE;

/// What "the automatic chart-area outline" is for the application whose package
/// the chart came out of.
///
/// Excel draws one and PowerPoint draws none. [`CHART_AREA_OUTLINE`] was
/// calibrated against an Excel export, and applying it everywhere put a border
/// around every chart on a slide that the deck never asks for: on
/// `bar-chart.pptx` a 480.00 x 301.00pt rectangle at 0.75pt, where a pixel scan
/// of the native export finds no straight run longer than the axis line
/// (issue #823).
///
/// Word's automatic outline is unmeasured, so it keeps Excel's — which is what
/// every chart took before this.
fn automatic_chart_area_stroke(host: crate::ir::ChartHost) -> &'static str {
    match host {
        crate::ir::ChartHost::Presentation => "none",
        crate::ir::ChartHost::Spreadsheet | crate::ir::ChartHost::WordProcessing => {
            CHART_AREA_OUTLINE
        }
    }
}

/// The Typst `stroke:` argument for a chart's own area outline.
///
/// The default is *not* unconditional: chart parts across the corpus declare
/// `<a:ln><a:noFill/></a:ln>` to suppress the outline, and others declare a line
/// of their own, so drawing [`CHART_AREA_OUTLINE`] regardless would put a border
/// on charts that ask for none and the wrong border on charts that ask for
/// theirs. See [`ChartAreaOutline`] for the fixtures covering each case (#637).
///
/// Nor is the automatic case itself one answer: `host` decides it, because
/// Excel and PowerPoint disagree about what an automatic outline is. See
/// [`automatic_chart_area_stroke`] (#823).
fn chart_area_stroke(outline: &ChartAreaOutline, host: crate::ir::ChartHost) -> String {
    match outline {
        ChartAreaOutline::Default => automatic_chart_area_stroke(host).to_string(),
        ChartAreaOutline::Suppressed => "none".to_string(),
        // A width or colour the file left out, or one the host theme cannot
        // resolve, falls back to the automatic one rather than to nothing:
        // the file did ask for *a* line.
        ChartAreaOutline::Explicit { width_pt, color } => format!(
            "{}pt + {}",
            format_f64(width_pt.unwrap_or(CHART_AUTOMATIC_LINE_PT)),
            color.map_or_else(
                || CHART_AUTOMATIC_LINE_RGB.to_string(),
                |c| format!("rgb({}, {}, {})", c.r, c.g, c.b)
            )
        ),
    }
}

/// The width and colour [`CHART_AUTOMATIC_LINE`] is built from, for an explicit
/// line that names only one of them.
const CHART_AUTOMATIC_LINE_PT: f64 = 0.75;
const CHART_AUTOMATIC_LINE_RGB: &str = "rgb(134, 134, 134)";

/// Share of the tick-label font's ascent one major tick mark is long.
///
/// Office sizes a tick against the face labelling the axis rather than at a
/// fixed length. Measured on its own exports: Calibri at 10/12/18/36pt gives
/// 3.17/3.81/5.71/11.42pt and Arial at 10/18pt gives 3.02/5.43pt, each within
/// 0.006pt of `size * usWinAscent / unitsPerEm / 3` for that face (issue #672).
const CHART_TICK_ASCENT_FRACTION: f64 = 1.0 / 3.0;

/// Ascent of Calibri over its em — the face Office labels chart axes with.
///
/// The tick labels' own faces are not resolved here, so every axis is measured
/// against the default one.
const CHART_LABEL_ASCENT_RATIO: f64 = 1950.0 / 2048.0;

/// Size every chart label prints at when the file declares no text properties.
///
/// Office's chart default is 10pt, and it is one size for the whole chart: the
/// value tick labels, the category labels and the legend entries all take it.
/// Three separate constants — 8pt for the value labels, 9pt for the bar plot's
/// categories, 8pt for the line plot's — put the labels at a size no file asked
/// for and left the two axes of one chart disagreeing with each other (#800).
///
/// Both references that can be measured against `WithChart.xlsx`, whose
/// `chart1.xml` carries no `c:txPr`, agree on 10pt: the native Excel export's
/// tick labels have a 6.24pt cap height, which is 10pt Calibri, and LibreOffice
/// writes a 10.0pt text matrix for every run on the page.
///
/// A chart declaring `c:txPr/a:defRPr@sz` overrides this — see
/// [`chart_text_pt`] — so it applies only where the file states nothing.
pub(super) const CHART_DEFAULT_TEXT_PT: f64 = 10.0;

/// The size every string the chart draws takes, from
/// `c:chartSpace/c:txPr/a:p/a:pPr/a:defRPr@sz`.
///
/// The 10pt default stands only for a chart that declares nothing; a chart
/// asking for 18pt used to render at 10, a little over half the size the file
/// requested (issue #669).
pub(super) fn chart_text_pt(chart: &Chart) -> f64 {
    chart.text_style.size_pt.unwrap_or(CHART_DEFAULT_TEXT_PT)
}

/// The size one axis' own labels take, honouring the `c:catAx`/`c:valAx`
/// `c:txPr` that overrides the chart space's.
pub(super) fn chart_axis_text_pt(chart: &Chart, axis: crate::ir::ChartTextStyle) -> f64 {
    chart
        .text_style
        .resolved_size_pt(axis)
        .unwrap_or(CHART_DEFAULT_TEXT_PT)
}

/// The `weight:` argument one axis' labels take, as a leading `, weight: …`
/// fragment or the empty string.
///
/// `a:defRPr@b` on a `c:catAx` was dropped entirely, so bold category labels
/// rendered regular while the data labels beside them — which carry their own
/// weight — kept theirs (issue #669).
pub(super) fn chart_axis_text_weight(
    chart: &Chart,
    axis: crate::ir::ChartTextStyle,
) -> &'static str {
    if chart.text_style.resolved_bold(axis).unwrap_or(false) {
        ", weight: \"bold\""
    } else {
        ""
    }
}

/// Height of the box that vertically centres one value tick label on its
/// gridline, as a multiple of the text size.
///
/// The box was a flat 10pt around 8pt text; keeping that 1.25x relationship
/// means the larger text still centres on the gridline instead of the box
/// clipping it or the label drifting off the tick.
const CHART_LABEL_BOX_RATIO: f64 = 1.25;

/// Height of the box holding one value tick label set at `text_pt`.
fn chart_label_box_h(text_pt: f64) -> f64 {
    text_pt * CHART_LABEL_BOX_RATIO
}

/// The value every major unit of an axis reaching `nice_max` in `step`s sits
/// on, from zero to the maximum inclusive.
///
/// The gridlines, the tick labels, and the tick marks all have to land on the
/// same units, and stepping a float accumulates error, so they walk one list
/// rather than each repeating the accumulation.
fn major_units(nice_max: f64, step: f64) -> Vec<f64> {
    let mut units: Vec<f64> = Vec::new();
    let mut unit: f64 = 0.0;
    // The accumulated error can leave the last unit a hair over `nice_max`,
    // which would drop the axis' top gridline and label.
    while unit <= nice_max + step * 1e-6 {
        units.push(unit);
        unit += step;
    }
    units
}

/// Length of a major tick mark on an axis labelled at `label_size_pt`.
fn chart_major_tick_length(label_size_pt: f64) -> f64 {
    label_size_pt * CHART_LABEL_ASCENT_RATIO * CHART_TICK_ASCENT_FRACTION
}

/// How far an axis' major ticks reach away from the plot and back into it, or
/// `None` when the axis asks for no ticks at all.
///
/// `in` and `out` are the same length on opposite sides of the axis line, and
/// `cross` is both at once rather than that length split between them: on
/// PowerPoint's export of `tests/fixtures/pptx/bar-chart.pptx` the axis sits at
/// y=390.10 and the ticks run 390.10..395.81 for `out`, 384.39..390.10 for `in`,
/// and 384.39..395.81 for `cross` — 5.71pt each way, so a crossing tick is twice
/// as long overall (issue #672).
fn tick_reach(mark: AxisTickMark, label_size_pt: f64) -> Option<(f64, f64)> {
    let length: f64 = chart_major_tick_length(label_size_pt);
    match mark {
        AxisTickMark::None => None,
        AxisTickMark::Inside => Some((0.0, length)),
        AxisTickMark::Outside => Some((length, 0.0)),
        AxisTickMark::Cross => Some((length, length)),
    }
}

/// Stroke the axis line down the plot's left edge.
///
/// The bar family used to stroke exactly one of its two edges: the left one
/// when the bars ran horizontally and the bottom one when they ran vertically.
/// Both of those are the category axis, so the value axis went unstroked in
/// either orientation (issue #672).
fn write_left_axis_line(out: &mut String, plot_x: f64, plot_y: f64, plot_h: f64, stroke: &str) {
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: {}))",
        format_f64(plot_x),
        format_f64(plot_y),
        format_f64(plot_h),
        stroke
    );
}

/// Stroke the axis line along the plot's bottom edge, at `axis_y`.
fn write_bottom_axis_line(out: &mut String, plot_x: f64, axis_y: f64, plot_w: f64, stroke: &str) {
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}))",
        format_f64(plot_x),
        format_f64(axis_y),
        format_f64(plot_w),
        stroke
    );
}

/// Stroke one major tick across the axis line running under the plot, at `x`:
/// `outward` reaches below the axis and `inward` back up into the plot.
fn write_tick_under_plot(
    out: &mut String,
    x: f64,
    axis_y: f64,
    (outward, inward): (f64, f64),
    stroke: &str,
) {
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: {}))",
        format_f64(x),
        format_f64(axis_y - inward),
        format_f64(outward + inward),
        stroke
    );
}

/// Stroke one major tick across the axis line running down the plot's left
/// edge, at `y`: `outward` reaches left of the axis and `inward` back into the
/// plot.
fn write_tick_left_of_plot(
    out: &mut String,
    axis_x: f64,
    y: f64,
    (outward, inward): (f64, f64),
    stroke: &str,
) {
    let _ = writeln!(
        out,
        "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}))",
        format_f64(axis_x - outward),
        format_f64(y),
        format_f64(outward + inward),
        stroke
    );
}

const PLOT_MAIN: f64 = 300.0; // value-axis length in points
pub(super) const ROW: f64 = 34.0; // per-category thickness
pub(super) const LABEL_W: f64 = 62.0; // category label gutter
pub(super) const TICK_GAP: f64 = 22.0; // value tick label gutter
const GAP: f64 = 6.0;
const LEGEND_ROW_H: f64 = 14.0; // per-entry height when the legend stacks
/// Floor for one entry's width in a legend that runs across the chart, and the
/// flat width a legend down the side reserves for its gutter.
///
/// It was the horizontal pitch itself until #827: every entry advanced by it,
/// so a name wider than 78pt ran under its neighbour. A horizontal entry now
/// takes the greater of this and what its own text measures — see
/// [`legend_entry_widths`] — which leaves a legend of short names exactly where
/// it was.
pub(super) const LEGEND_ENTRY_W: f64 = 78.0;

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
    /// Reserve nothing, for a chart that declares no legend: the plot then
    /// gets the whole frame instead of a gutter nothing is drawn in
    /// (issue #762).
    fn hidden() -> Self {
        LegendBox {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            horizontal: false,
        }
    }

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
        entry_widths: &[f64],
    ) -> (f64, f64) {
        let (content_x, content_y, content_w, content_h) = content;
        if self.horizontal {
            // Centre the row of entries under (or over) the content. Each entry
            // advances by its own width, not by a flat pitch: a name wider than
            // the pitch used to run under the entry beside it and the two
            // overprinted into unreadable text (issue #827).
            let row_w: f64 = entry_widths.iter().sum();
            let start_x: f64 = content_x + (content_w - row_w).max(0.0) / 2.0;
            let y: f64 = match position {
                LegendPosition::Top => (content_y - self.top).max(0.0),
                _ => content_y + content_h + GAP,
            };
            let offset: f64 = entry_widths.iter().take(index).sum();
            (start_x + offset, y)
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
///
/// `percent_base` is what `showPercent` measures the point against: the
/// category total for a stacked bar, where the label answers "how much of this
/// column", and the series total for a pie, where it answers "how much of the
/// whole". Measuring a pie against its category would call every slice 100%,
/// since a pie has one series.
fn data_label_text(
    chart: &Chart,
    series: &crate::ir::ChartSeries,
    category_index: usize,
    percent_base: f64,
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
        parts.push(chart_value_label_formatted(
            value,
            series_label_number_format(series),
        ));
    }
    if labels.show_percent {
        let percent: f64 = if percent_base == 0.0 {
            0.0
        } else {
            value / percent_base * 100.0
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

/// The band a value tick label needs, for text at the size the chart declares.
///
/// [`TICK_GAP`] was calibrated at [`CHART_DEFAULT_TEXT_PT`], so a chart that
/// declares nothing reserves exactly what it always did. A chart declaring 18pt
/// used to reserve the 10pt band and the plot swallowed the difference. On
/// `bar-chart.pptx`, whose GT frame is 480x320pt, that left the plot 260.88pt
/// tall against PowerPoint's 233.28; scaling the band brings it to 243.12, so
/// the height error falls from 27.60pt to 9.84pt. The remaining 9.84pt is why
/// #706 stays open.
pub(super) fn chart_tick_band_pt(chart: &Chart) -> f64 {
    TICK_GAP / CHART_DEFAULT_TEXT_PT * chart_axis_text_pt(chart, chart.value_axis_text_style)
}

/// The band one category takes across the category axis, at the declared size.
pub(super) fn chart_category_band_pt(chart: &Chart) -> f64 {
    ROW / CHART_DEFAULT_TEXT_PT * chart_axis_text_pt(chart, chart.category_axis_text_style)
}

/// Width the category labels take down the left of a bar plot.
///
/// Measured from the widest label in the face it is set in, rather than scaled
/// from [`LABEL_W`]: this is a width holding text, not a height, so it grows
/// with what the labels say as well as with their size. `bar-chart.pptx`'s
/// labels are as short as `4th Qtr`, and scaling the constant by the same 1.8
/// the band takes would have reserved far more than they need — the plot is
/// 16.32pt wider than PowerPoint's before this and 10.08pt after, so the gutter
/// had room to grow but not by the constant's full ratio.
///
/// Falls back to the flat constant where the face cannot be measured — wasm has
/// no font search — so the gutter is never narrower than it was.
pub(super) fn chart_category_gutter_pt(chart: &Chart) -> f64 {
    let size_pt: f64 = chart_axis_text_pt(chart, chart.category_axis_text_style);
    let bold: bool = chart
        .text_style
        .resolved_bold(chart.category_axis_text_style)
        .unwrap_or(false);
    let family: &str = chart
        .text_font_family
        .as_deref()
        .unwrap_or(crate::defaults::TYPST_DEFAULT_FONT_FAMILY);
    let widest_em: f64 = chart
        .categories
        .iter()
        .filter_map(|category| crate::render::pdf::text_advance_em(family, bold, category))
        .fold(0.0_f64, f64::max);
    if widest_em <= 0.0 {
        return LABEL_W;
    }
    // The tick mark reaches out of the plot into this gutter, and the label
    // stops a gap short of it.
    let tick: f64 = chart_major_tick_length(size_pt);
    (widest_em * size_pt + tick + GAP).max(LABEL_W)
}

/// Gutters the category labels and the value tick labels take inside the box,
/// alongside whatever the legend and the axis titles reserve.
fn axis_label_gutters(chart: &Chart) -> (f64, f64) {
    let (title_left, title_bottom) = axis_title_gutters(chart);
    if matches!(chart.chart_type, ChartType::Bar) {
        (
            chart_category_gutter_pt(chart) + GAP + title_left,
            chart_tick_band_pt(chart) + title_bottom,
        )
    } else {
        (
            chart_tick_band_pt(chart) + GAP + title_left,
            chart_category_band_pt(chart) + title_bottom,
        )
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

/// Height of one data-label line: the span to centre across when the label
/// sits on its segment, and the box to offset by when it sits at the
/// segment's end instead (issue #901).
const LABEL_LINE_H: f64 = 10.0;

/// Width of the box a label gets when it sits at the end of a horizontal bar
/// rather than across it. The centred case spans the bar, which is the wrong
/// span once the label is beside it (issue #901).
const LABEL_OUTSIDE_W: f64 = 40.0;

/// Clearance between a bar's end and an `outEnd` label, so the text does not
/// sit flush against it (issue #907).
///
/// Measured on `002.CONTOSO.pptx` (#841) through LibreOffice 24.2 with the
/// deck's label size rewritten, to tell an absolute gap from one that scales:
/// 8pt labels clear the bar by a mean 2.66pt, 11.97pt by 2.99pt and 18pt by
/// 2.73pt. Across a 2.25x range in size the gap moves by 0.33pt while the
/// ratio to the size swings from 0.33 to 0.15, so it is a constant. This is
/// the offset added to the placement; the glyphs already sit about 0.44pt
/// inside their line box, which brings the drawn clearance to about 2.8pt.
const LABEL_OUTSIDE_GAP: f64 = 2.4;

/// Size of the plotting rectangle itself.
///
/// Given a frame, the plot takes whatever is left of it after the label gutters
/// and the legend, so the chart fills its `<p:graphicFrame>` the way PowerPoint
/// lays it out. Without one it keeps the intrinsic size: `PLOT_MAIN` along the
/// value axis, one `ROW` per category across it.
fn axis_plot_size(chart: &Chart, frame: Option<(f64, f64)>) -> (f64, f64) {
    let plot_cross: f64 = chart.categories.len() as f64 * chart_category_band_pt(chart);
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

/// Size of the chart-area title when the chart declares no `c:txPr`.
const CHART_AREA_TITLE_PT: f64 = 11.0;

/// What Office scales the chart's text size by for the chart-area title: the
/// 18pt `bar-chart.pptx` declares comes back as a 22pt title.
const CHART_AREA_TITLE_SCALE: f64 = 1.2;

/// The chart-area title's size.
///
/// A chart declaring nothing keeps [`CHART_AREA_TITLE_PT`], which is what
/// [`AREA_TITLE_H`] was measured against; one that declares a size gets that
/// size scaled the way Office scales it (issue #669).
fn chart_area_title_pt(chart: &Chart) -> f64 {
    chart
        .text_style
        .size_pt
        .map_or(CHART_AREA_TITLE_PT, |declared| {
            // Office states sizes in hundredths of a point, so the scaled value
            // is rounded there rather than carried as `18 * 1.2` binary noise.
            (declared * CHART_AREA_TITLE_SCALE * 100.0).round() / 100.0
        })
}

/// Height the chart-area title block takes, for a title at `title_pt`.
///
/// [`AREA_TITLE_H`] is this at [`CHART_AREA_TITLE_PT`], so a chart declaring
/// nothing reserves exactly what it always did. The band is still short of what
/// Office reserves at any size — that is #706, and it is not what decides the
/// text's own size here.
fn chart_area_title_h(chart: &Chart) -> f64 {
    AREA_TITLE_H / CHART_AREA_TITLE_PT * chart_area_title_pt(chart)
}

/// Width each legend entry occupies when the legend runs across the chart.
///
/// The key, the gap to the label, and the label itself measured in the face the
/// chart sets its text in. [`LEGEND_ENTRY_W`] is the floor, so a legend of short
/// names lays out exactly where it always did and only a name too wide for the
/// old flat pitch moves (issue #827).
///
/// Falls back to the floor for any name that cannot be measured — wasm has no
/// font search — so an entry is never narrower than its text.
fn legend_entry_widths(chart: &Chart, key_len_pt: f64, names: &[String]) -> Vec<f64> {
    let size_pt: f64 = chart_text_pt(chart);
    let family: &str = chart
        .text_font_family
        .as_deref()
        .unwrap_or(crate::defaults::TYPST_DEFAULT_FONT_FAMILY);
    names
        .iter()
        .map(|name| {
            let label: f64 = crate::render::pdf::text_advance_em(family, false, name)
                .map_or(0.0, |advance| advance * size_pt);
            // A gutter after the label keeps neighbouring entries apart rather
            // than butting the next key against the last glyph.
            (key_len_pt + LEGEND_KEY_LABEL_GAP_PT + label + GAP).max(LEGEND_ENTRY_W)
        })
        .collect()
}

/// Space the axis plot's legend reserves.
fn axis_legend_box(chart: &Chart) -> LegendBox {
    if !chart.has_legend {
        return LegendBox::hidden();
    }
    LegendBox::new(chart.legend_position, LEGEND_ROW_H, LEGEND_ENTRY_W)
}

/// Where the bars of one category sit inside the band it gets, in points along
/// the category axis.
struct BandBars {
    /// Thickness of one bar.
    thickness: f64,
    /// Offset of the first series' bar from the start of the band.
    lead: f64,
    /// Distance from one series' bar to the next one's along the category axis.
    /// Zero only when the two sit exactly on top of each other.
    step: f64,
}

/// Divide a category's band between the bars sharing it, the way Office does.
///
/// `<c:gapWidth>` and `<c:overlap>` are both measured in units of ONE bar, not
/// of the band, so the band spans the cluster its series form plus a gutter of
/// `gap_width_percent`: `bars - (bars - 1) * overlap + gap` bars in all. The
/// cluster then sits centred, half the gutter on each side.
///
/// Measured against PowerPoint 16.0 rather than read off the schema: sweeping
/// `<c:gapWidth>` from 0 to 500 over `tests/fixtures/pptx/bar-chart.pptx` and
/// tracing each export put every bar edge within one 1/1200in device quantum of
/// this, with the band itself never moving, and a two-series sweep of
/// `<c:overlap>` over -27, 0 and 50 did the same for the step.
///
/// The grouping does not enter into it: a stacked chart divides its band by the
/// same law, with `bars` still the series count. Rewriting `<c:overlap>` on the
/// four-series stacked chart of `office2pdf_introduction_ko.pptx` (gapWidth 90)
/// and tracing PowerPoint 16.0's export gave, on a 167.6pt pitch, one 88.2pt
/// column at 100 (167.64/1.9) but a STAIRCASE of four 34.2pt segments stepping
/// 34.2pt at 0 (167.52/4.9) — each segment still stacked on the running total,
/// only slid sideways. Overlaps of 50 and -25 landed on 49.3/24.7pt and
/// 29.6/37.1pt, both what this predicts. Deleting the element reproduced the 0
/// case exactly, so an absent `<c:overlap>` is 0 here and not the 100 Office
/// happens to write beside its own stacked charts.
fn band_bars(band: f64, series_count: usize, layout: BarBandLayout) -> BandBars {
    let bars: f64 = series_count.max(1) as f64;
    let gap: f64 = layout.gap_width_percent / 100.0;
    let overlap: f64 = layout.overlap_percent / 100.0;
    // How many bar widths the band is worth. Over the ranges the parser holds
    // its inputs to this bottoms out at 1 — an overlap of 100% collapses the
    // cluster to a single bar and the gap only ever adds — so it can neither
    // vanish nor turn the geometry inside out.
    let bar_widths_per_band: f64 = (bars - (bars - 1.0) * overlap + gap).max(1.0);
    let thickness: f64 = band / bar_widths_per_band;
    let step: f64 = thickness * (1.0 - overlap);
    let cluster: f64 = thickness + (bars - 1.0) * step;
    BandBars {
        thickness,
        lead: (band - cluster) / 2.0,
        step,
    }
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
    let auto_axis = match chart.grouping {
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
    let (nice_max, step) = axis_with_stated_unit(auto_axis, chart.value_axis_major_unit);

    // Chart-area title: the explicit chart title, else the automatic one
    // Office derives from a single series' name — unless the chart declined
    // that with `<c:autoTitleDeleted val="1"/>` (issue #883).
    let area_title: Option<&str> = chart.title.as_deref().or_else(|| {
        if series.len() == 1 && !chart.auto_title_deleted {
            series[0].name.as_deref()
        } else {
            None
        }
    });
    if let Some(title) = area_title {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: {}pt, weight: \"bold\")[{}]]",
            format_f64(chart_area_title_pt(chart)),
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }

    // The title is emitted above the box, so a framed chart's box gets what is
    // left of the frame beneath it.
    let title_h: f64 = if area_title.is_some() {
        chart_area_title_h(chart)
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
        "#box(width: {}pt, height: {}pt, stroke: {})[",
        format_f64(total_w),
        format_f64(total_h),
        chart_area_stroke(&chart.chart_area_outline, chart.host)
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
        chart_category_band_pt(chart)
    } else if horizontal {
        plot_h / categories as f64
    } else {
        plot_w / categories as f64
    };

    // `<c:delete val="1"/>` switches an axis off: Office then draws neither its
    // line, nor its tick marks, nor its tick labels. Gridlines are a chart
    // element of their own — switching the axis off leaves them standing — so
    // only the axis' own furniture answers to this.
    //
    // TODO(gutter reflow): `axis_label_gutters` still reserves the band a
    // switched-off axis' labels would have printed in, so the plot keeps the
    // size and position it has with them drawn. Office reclaims that space.
    let value_axis_drawn: bool = !chart.value_axis_deleted;
    let category_axis_drawn: bool = !chart.category_axis_deleted;

    // Gridlines + value tick labels. The gridlines take the line
    // `<c:majorGridlines><c:spPr><a:ln>` declares, if it declares one (#900).
    let gridline_stroke = chart_chrome_stroke(chart.major_gridline_line);
    let major_units: Vec<f64> = major_units(nice_max, step);
    for tick in &major_units {
        let frac: f64 = tick / nice_max;
        if horizontal {
            let x: f64 = plot_x + frac * plot_w;
            if let Some(stroke) = gridline_stroke.as_deref() {
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, line(end: (0pt, {}pt), stroke: {}))",
                    format_f64(x),
                    format_f64(plot_y),
                    format_f64(plot_h),
                    stroke
                );
            }
            if value_axis_drawn {
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, box(width: 24pt)[#align(center)[#text(size: {}pt)[{}]]])",
                    format_f64(x - 12.0),
                    format_f64(plot_y + plot_h + 4.0),
                    format_f64(chart_axis_text_pt(chart, chart.value_axis_text_style)),
                    escape_typst(&chart_value_label_formatted(
                        *tick,
                        chart_value_number_format(chart)
                    ))
                );
            }
        } else {
            let y: f64 = plot_y + (1.0 - frac) * plot_h;
            if let Some(stroke) = gridline_stroke.as_deref() {
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}))",
                    format_f64(plot_x),
                    format_f64(y),
                    format_f64(plot_w),
                    stroke
                );
            }
            if value_axis_drawn {
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(right + horizon)[#text(size: {}pt)[{}]]])",
                    format_f64(
                        y - chart_label_box_h(chart_axis_text_pt(
                            chart,
                            chart.value_axis_text_style
                        )) / 2.0
                    ),
                    format_f64(chart_tick_band_pt(chart)),
                    format_f64(chart_label_box_h(chart_axis_text_pt(
                        chart,
                        chart.value_axis_text_style
                    ))),
                    format_f64(chart_axis_text_pt(chart, chart.value_axis_text_style)),
                    escape_typst(&chart_value_label_formatted(
                        *tick,
                        chart_value_number_format(chart)
                    ))
                );
            }
        }
    }

    // Bars, grouped per category when multiple series are present.
    let bars: BandBars = band_bars(row, series_count, chart.bar_band_layout);
    let bar_thickness: f64 = bars.thickness;
    for (cat_index, category) in chart.categories.iter().enumerate() {
        let group_start: f64 = cat_index as f64 * row;
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
            let color: String = series_color(s, s_index, cat_index, &chart.theme_accent_colors);
            let offset: f64 = bars.lead + s_index as f64 * bars.step;
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
                    format_f64(bar_thickness),
                    color
                );
            } else {
                let bar_h: f64 = frac * plot_h;
                let _ = writeln!(
                    out,
                    "#place(top + left, dx: {}pt, dy: {}pt, rect(width: {}pt, height: {}pt, fill: {}, stroke: none))",
                    format_f64(plot_x + group_start + offset),
                    format_f64(plot_y + plot_h - bar_h - stack_base * plot_h),
                    format_f64(bar_thickness),
                    format_f64(bar_h.max(0.0)),
                    color
                );
            }
            if let Some(label) = data_label_text(chart, s, cat_index, category_total) {
                // Where the label sits along the bar, from `<c:dLblPos>` or the
                // grouping's default (issue #901). A stacked segment centres
                // because an outside label would land on the segment above.
                let position = s.data_labels.position;
                let (label_x, label_y, label_w) = if horizontal {
                    let row_top: f64 = plot_h - (cat_index as f64 + 1.0) * row;
                    let bar_start: f64 = plot_x + stack_base * plot_w;
                    let bar_w: f64 = frac * plot_w;
                    let x: f64 = match position {
                        DataLabelPosition::Center => bar_start,
                        DataLabelPosition::OutsideEnd => bar_start + bar_w + LABEL_OUTSIDE_GAP,
                        DataLabelPosition::InsideEnd => bar_start + bar_w - LABEL_OUTSIDE_W,
                        DataLabelPosition::InsideBase => bar_start,
                    };
                    let w: f64 = match position {
                        DataLabelPosition::Center => bar_w,
                        _ => LABEL_OUTSIDE_W,
                    };
                    (
                        x,
                        row_top + offset + bar_thickness / 2.0 - LABEL_LINE_H / 2.0,
                        w,
                    )
                } else {
                    let bar_top: f64 = plot_y + plot_h - stack_base * plot_h - frac * plot_h;
                    let bar_bottom: f64 = plot_y + plot_h - stack_base * plot_h;
                    let y: f64 = match position {
                        DataLabelPosition::Center => {
                            (bar_top + bar_bottom) / 2.0 - LABEL_LINE_H / 2.0
                        }
                        DataLabelPosition::OutsideEnd => bar_top - LABEL_LINE_H - LABEL_OUTSIDE_GAP,
                        DataLabelPosition::InsideEnd => bar_top,
                        DataLabelPosition::InsideBase => bar_bottom - LABEL_LINE_H,
                    };
                    (plot_x + group_start + offset, y, bar_thickness)
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
        // Category label, which goes with the axis it labels.
        if !category_axis_drawn {
            continue;
        }
        if horizontal {
            let row_top: f64 = plot_h - (cat_index as f64 + 1.0) * row;
            let _ = writeln!(
                out,
                "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(right + horizon)[#text(size: {}pt{})[{}]]])",
                format_f64(row_top),
                format_f64(chart_category_gutter_pt(chart)),
                format_f64(row),
                format_f64(chart_axis_text_pt(chart, chart.category_axis_text_style)),
                chart_axis_text_weight(chart, chart.category_axis_text_style),
                escape_typst(category)
            );
        } else {
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#text(size: {}pt{})[{}]]])",
                format_f64(plot_x + group_start),
                format_f64(plot_y + plot_h + 2.0),
                format_f64(row),
                format_f64(chart_category_band_pt(chart)),
                format_f64(chart_axis_text_pt(chart, chart.category_axis_text_style)),
                chart_axis_text_weight(chart, chart.category_axis_text_style),
                escape_typst(category)
            );
        }
    }

    // The axis lines and their major tick marks, drawn after the bars so they
    // paint on top as Office paints them — an inward tick would otherwise
    // disappear under the bar it crosses.
    //
    // A bar chart's value axis runs along the bottom edge and its category axis
    // down the left one; a column chart's are the other way round.
    let (left_axis_drawn, bottom_axis_drawn) = if horizontal {
        (category_axis_drawn, value_axis_drawn)
    } else {
        (value_axis_drawn, category_axis_drawn)
    };
    // Each axis draws with the line it declares, if it declares one (#900).
    let (left_stroke, bottom_stroke) = if horizontal {
        (
            chart_chrome_stroke(chart.category_axis_line),
            chart_chrome_stroke(chart.value_axis_line),
        )
    } else {
        (
            chart_chrome_stroke(chart.value_axis_line),
            chart_chrome_stroke(chart.category_axis_line),
        )
    };
    if let (true, Some(stroke)) = (left_axis_drawn, left_stroke.as_deref()) {
        write_left_axis_line(out, plot_x, plot_y, plot_h, stroke);
    }
    if let (true, Some(stroke)) = (bottom_axis_drawn, bottom_stroke.as_deref()) {
        write_bottom_axis_line(out, plot_x, plot_y + plot_h, plot_w, stroke);
    }
    if value_axis_drawn
        && let Some(reach) = tick_reach(
            chart.value_axis_major_tick_mark,
            chart_axis_text_pt(chart, chart.value_axis_text_style),
        )
    {
        // Every value tick sits on its own gridline, both being one major unit.
        for tick in &major_units {
            let frac: f64 = tick / nice_max;
            if horizontal {
                if let Some(stroke) = bottom_stroke.as_deref() {
                    write_tick_under_plot(
                        out,
                        plot_x + frac * plot_w,
                        plot_y + plot_h,
                        reach,
                        stroke,
                    );
                }
            } else if let Some(stroke) = left_stroke.as_deref() {
                write_tick_left_of_plot(out, plot_x, plot_y + (1.0 - frac) * plot_h, reach, stroke);
            }
        }
    }
    // The category ticks land on band boundaries rather than band centres:
    // `<c:crossBetween val="between"/>` sits each band between two ticks, so
    // the bars, which fill the bands, sit between them too and three categories
    // take four ticks.
    if categories > 0
        && category_axis_drawn
        && let Some(reach) = tick_reach(
            chart.category_axis_major_tick_mark,
            chart_axis_text_pt(chart, chart.category_axis_text_style),
        )
    {
        for boundary in 0..=categories {
            let offset: f64 = boundary as f64 * row;
            if horizontal {
                if let Some(stroke) = left_stroke.as_deref() {
                    write_tick_left_of_plot(out, plot_x, plot_y + offset, reach, stroke);
                }
            } else {
                if let Some(stroke) = bottom_stroke.as_deref() {
                    write_tick_under_plot(out, plot_x + offset, plot_y + plot_h, reach, stroke);
                }
            }
        }
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

    // Legend on the edge `<c:legendPos>` asks for — none when the chart
    // declares no `<c:legend>` (issue #762). Bounded rather than returned
    // early: the markup's closing delimiter is written after this loop.
    let legend_names: Vec<String> = series
        .iter()
        .enumerate()
        .map(|(index, s)| {
            s.name
                .clone()
                .unwrap_or_else(|| format!("Series {}", index + 1))
        })
        .collect();
    let entry_widths: Vec<f64> = legend_entry_widths(chart, LEGEND_KEY_LEN_PT, &legend_names);
    let legend_entries: usize = if chart.has_legend { series.len() } else { 0 };
    for (s_index, s) in series.iter().enumerate().take(legend_entries) {
        let color: String = series_color(s, s_index, 0, &chart.theme_accent_colors);
        let default_name: String = format!("Series {}", s_index + 1);
        let name: &str = s.name.as_deref().unwrap_or(&default_name);
        // The content the legend sits beside spans the plot and both label
        // gutters, so a bottom legend clears the category labels.
        let (gutter_w, gutter_h) = if horizontal {
            (
                chart_category_gutter_pt(chart) + GAP,
                chart_tick_band_pt(chart),
            )
        } else {
            (
                chart_tick_band_pt(chart) + GAP,
                chart_category_band_pt(chart),
            )
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
            &entry_widths,
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box[#box(width: 9pt, height: 9pt, fill: {})#h({}pt)#text(size: {}pt)[{}]])",
            format_f64(entry_x),
            format_f64(entry_y),
            color,
            format_f64(LEGEND_KEY_LABEL_GAP_PT),
            format_f64(chart_text_pt(chart)),
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
                "#box(width: 10pt, height: 10pt, fill: {color}) #text(size: {}pt)[{name}] ",
                format_f64(chart_text_pt(chart))
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
    // Entry-to-entry pitch, measured between the legend key centres of the
    // native Excel export of `WithChart.xlsx` at 150 DPI: 37px = 17.76pt for
    // 10pt entries. LibreOffice puts its own entries 14.07pt apart on the same
    // file, so this one is Excel's rather than a renderer consensus (#800).
    const LINE_LEGEND_ROW_H: f64 = 17.76;
    const GAP: f64 = 6.0;

    let categories: usize = chart.categories.len();
    let series: &[crate::ir::ChartSeries] = &chart.series;

    let max_value: f64 = series
        .iter()
        .flat_map(|s| s.values.iter())
        .copied()
        .fold(0.0_f64, f64::max);
    let (nice_max, step) = axis_with_stated_unit(nice_axis(max_value), chart.value_axis_major_unit);

    if let Some(title) = chart.title.as_deref() {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: {}pt, weight: \"bold\")[{}]]",
            format_f64(chart_area_title_pt(chart)),
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }
    // As in `generate_chart_axis`: the title sits above the box, so a framed
    // chart spends its height out of the frame rather than on top of it.
    let frame: Option<(f64, f64)> = frame.map(|(width, height)| {
        let title_h: f64 = if chart.title.is_some() {
            chart_area_title_h(chart)
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
        "#box(width: {}pt, height: {}pt, stroke: {})[",
        format_f64(total_w),
        format_f64(total_h),
        chart_area_stroke(&chart.chart_area_outline, chart.host)
    );

    // `<c:delete val="1"/>` switches an axis off; see `generate_chart_axis`.
    let value_axis_drawn: bool = !chart.value_axis_deleted;
    let category_axis_drawn: bool = !chart.category_axis_deleted;

    // Horizontal gridlines + value tick labels, with the line
    // `<c:majorGridlines>` declares when it declares one (#900).
    let gridline_stroke = chart_chrome_stroke(chart.major_gridline_line);
    let major_units: Vec<f64> = major_units(nice_max, step);
    for tick in &major_units {
        let y: f64 = plot_y + (1.0 - tick / nice_max) * plot_h;
        if let Some(stroke) = gridline_stroke.as_deref() {
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}))",
                format_f64(plot_x),
                format_f64(y),
                format_f64(plot_w),
                stroke
            );
        }
        if value_axis_drawn {
            let _ = writeln!(
                out,
                "#place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(right + horizon)[#text(size: {}pt)[{}]]])",
                format_f64(
                    y - chart_label_box_h(chart_axis_text_pt(chart, chart.value_axis_text_style))
                        / 2.0
                ),
                format_f64(VALUE_GAP),
                format_f64(chart_label_box_h(chart_axis_text_pt(
                    chart,
                    chart.value_axis_text_style
                ))),
                format_f64(chart_axis_text_pt(chart, chart.value_axis_text_style)),
                escape_typst(&chart_value_label_formatted(
                    *tick,
                    chart_value_number_format(chart)
                ))
            );
        }
    }

    // The category axis is split into one band per category, and both the point
    // and its label sit at their band's centre — `<c:crossBetween val="between"/>`,
    // which is what every category axis in the fixture corpus asks for and what
    // the category tick marks below are the boundaries of. PowerPoint's own
    // export of `tests/fixtures/pptx/line-chart.pptx` spaces its four points
    // 90.91pt apart over a 363.65pt axis, the first of them half a band in
    // (issue #672).
    //
    // TODO(crossBetween): the element itself is not parsed, so an axis asking
    // for `midCat` — points on the boundaries, the series spanning the plot
    // edge to edge — is laid out as `between` as well.
    //
    // `chart_variant` only routes a chart with two categories or more here, but
    // the band width still has to be safe if that ever changes.
    let band_w: f64 = plot_w / categories.max(1) as f64;
    let point_x = |index: usize| -> f64 { plot_x + (index as f64 + 0.5) * band_w };
    let point_y =
        |value: f64| -> f64 { plot_y + (1.0 - (value / nice_max).clamp(0.0, 1.0)) * plot_h };

    // Category axis labels.
    if category_axis_drawn {
        for (index, category) in chart.categories.iter().enumerate() {
            let x: f64 = point_x(index);
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: 24pt)[#align(center)[#text(size: {}pt{})[{}]]])",
                format_f64(x - 12.0),
                format_f64(plot_y + plot_h + 3.0),
                format_f64(chart_axis_text_pt(chart, chart.category_axis_text_style)),
                chart_axis_text_weight(chart, chart.category_axis_text_style),
                escape_typst(category)
            );
        }
    }

    // Series polylines + markers.
    for (s_index, s) in series.iter().enumerate() {
        let color: String = series_color(s, s_index, 0, &chart.theme_accent_colors);
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
                "#place(top + left, path(stroke: {}pt + {color}, {coords}))",
                format_f64(SERIES_LINE_PT)
            );
        }
        // Point markers, cycling by series index.
        for (x, y) in &points {
            write_series_marker(out, s_index, *x, *y, &color);
        }
    }

    // Value/category axis lines and their major tick marks. The value axis
    // always runs down the left edge here and the category axis along the
    // bottom, whatever shape the series take.
    let value_stroke = chart_chrome_stroke(chart.value_axis_line);
    let category_stroke = chart_chrome_stroke(chart.category_axis_line);
    if let (true, Some(stroke)) = (value_axis_drawn, value_stroke.as_deref()) {
        write_left_axis_line(out, plot_x, plot_y, plot_h, stroke);
    }
    if let (true, Some(stroke)) = (category_axis_drawn, category_stroke.as_deref()) {
        write_bottom_axis_line(out, plot_x, plot_y + plot_h, plot_w, stroke);
    }
    if value_axis_drawn
        && let Some(reach) = tick_reach(
            chart.value_axis_major_tick_mark,
            chart_axis_text_pt(chart, chart.value_axis_text_style),
        )
    {
        // Every value tick sits on its own gridline, both being one major unit.
        for tick in &major_units {
            let y: f64 = plot_y + (1.0 - tick / nice_max) * plot_h;
            if let Some(stroke) = value_stroke.as_deref() {
                write_tick_left_of_plot(out, plot_x, y, reach, stroke);
            }
        }
    }
    // The boundaries of the bands `point_x` centres each category in, so every
    // category label sits midway between two ticks.
    if categories > 0
        && category_axis_drawn
        && let Some(reach) = tick_reach(
            chart.category_axis_major_tick_mark,
            chart_axis_text_pt(chart, chart.category_axis_text_style),
        )
    {
        for boundary in 0..=categories {
            if let Some(stroke) = category_stroke.as_deref() {
                write_tick_under_plot(
                    out,
                    plot_x + boundary as f64 * band_w,
                    plot_y + plot_h,
                    reach,
                    stroke,
                );
            }
        }
    }

    // Legend on the edge `<c:legendPos>` asks for — none when the chart
    // declares no `<c:legend>` (issue #762). Bounded rather than returned
    // early: the markup's closing delimiter is written after this loop.
    let legend_names: Vec<String> = series
        .iter()
        .enumerate()
        .map(|(index, s)| {
            s.name
                .clone()
                .unwrap_or_else(|| format!("Series {}", index + 1))
        })
        .collect();
    let entry_widths: Vec<f64> = legend_entry_widths(chart, LEGEND_KEY_LEN_PT, &legend_names);
    let legend_entries: usize = if chart.has_legend { series.len() } else { 0 };
    for (s_index, s) in series.iter().enumerate().take(legend_entries) {
        let color: String = series_color(s, s_index, 0, &chart.theme_accent_colors);
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
            &entry_widths,
        );
        // The key is a sample of the plotted line: the same stroke, carrying the
        // same marker the series draws on each of its points (#801).
        let key_mid: f64 = SERIES_MARKER_SIZE_PT / 2.0;
        let key: String = format!(
            "#box(width: {}pt, height: {}pt, baseline: {}pt)[\
             #place(top + left, dx: 0pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}pt + {color}))\
             {}]",
            format_f64(LEGEND_KEY_LEN_PT),
            format_f64(SERIES_MARKER_SIZE_PT),
            format_f64(LEGEND_KEY_BASELINE_PT),
            format_f64(key_mid),
            format_f64(LEGEND_KEY_LEN_PT),
            format_f64(SERIES_LINE_PT),
            series_marker_markup(s_index, LEGEND_KEY_LEN_PT / 2.0, key_mid, &color).trim_end()
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box[{key}#h({}pt)#text(size: {}pt)[{}]])",
            format_f64(entry_x),
            format_f64(entry_y),
            format_f64(LEGEND_KEY_LABEL_GAP_PT),
            format_f64(chart_text_pt(chart)),
            escape_typst(name)
        );
    }

    out.push_str("]\n");
}

/// Whether the chart part declared `<c:radarChart>`.
///
/// The parser labels it `ChartType::Other("Radar Chart")` because the family
/// has no variant of its own; matching the label keeps that decision in one
/// place (issue #679).
fn is_radar(chart: &Chart) -> bool {
    matches!(&chart.chart_type, ChartType::Other(kind) if kind == crate::ir::RADAR_CHART_LABEL)
}

/// Render a radar chart: one spoke per category radiating from a common
/// centre, each series a closed polygon through its value on every spoke.
///
/// Before this the family fell through to the bordered-table fallback, so a
/// slide whose primary content was a radar lost it entirely and showed a plain
/// table of the series values instead (issue #679).
fn generate_chart_radar_plot(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    /// Intrinsic plot size for a flowed radar, matching the pie's.
    const RADAR_DIAMETER: f64 = 200.0;
    const RADAR_LEGEND_ROW_H: f64 = 14.0;
    /// Width of the gutter the value tick labels are right-aligned in, left of
    /// the centre. Matches the line plot's own value gutter.
    const RADAR_VALUE_GAP: f64 = 24.0;
    /// Room left outside the outermost web ring for the category labels.
    ///
    /// A label sits beyond its spoke's end, so the web has to stop short of the
    /// box or the labels leave it.
    const RADAR_LABEL_MARGIN_PT: f64 = 30.0;
    /// Half-width of the box a category label is centred in.
    ///
    /// Wider than the margin the web gives up: the box is centred on the point
    /// outside the spoke, so half of it lies back over the web, and a category
    /// name only as wide as the margin would wrap. `기동 지연 최소화` still
    /// wrapped to two lines at the margin's 30pt; widening the box to 48pt
    /// stopped it. Observed on the rendered page rather than measured from the
    /// face's advances.
    const RADAR_LABEL_HALF_W_PT: f64 = 48.0;

    let category_count: usize = chart.categories.len();
    if category_count < 3 {
        return;
    }
    let max_value: f64 = chart
        .series
        .iter()
        .flat_map(|series| series.values.iter())
        .cloned()
        .fold(0.0_f64, f64::max);
    let (nice_max, step) = axis_with_stated_unit(nice_axis(max_value), chart.value_axis_major_unit);
    if nice_max <= 0.0 {
        return;
    }

    if let Some(title) = chart.title.as_deref() {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: {}pt, weight: \"bold\")[{}]]",
            format_f64(chart_area_title_pt(chart)),
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }

    let legend: LegendBox = if chart.has_legend {
        LegendBox::new(chart.legend_position, RADAR_LEGEND_ROW_H, LEGEND_ENTRY_W)
    } else {
        LegendBox::hidden()
    };
    // As elsewhere: the title is drawn above the box, so a framed chart takes
    // its height out of the frame.
    let frame: Option<(f64, f64)> = frame.map(|(width, height)| {
        let title_h: f64 = if chart.title.is_some() {
            chart_area_title_h(chart)
        } else {
            0.0
        };
        (width, (height - title_h).max(MIN_PLOT_PT))
    });
    let (total_w, total_h) = match frame {
        Some(extent) => extent,
        None => (
            legend.left + RADAR_DIAMETER + legend.right,
            legend.top + RADAR_DIAMETER + legend.bottom,
        ),
    };

    // The web stays circular, so it takes the smaller of the two axes, less the
    // room the category labels need outside it.
    let span_w: f64 = total_w - legend.left - legend.right;
    let span_h: f64 = total_h - legend.top - legend.bottom;
    let radius: f64 = (span_w.min(span_h) / 2.0 - RADAR_LABEL_MARGIN_PT).max(MIN_PLOT_PT / 2.0);
    let centre_x: f64 = legend.left + span_w / 2.0;
    let centre_y: f64 = legend.top + span_h / 2.0;

    let _ = writeln!(
        out,
        "#box(width: {}pt, height: {}pt, stroke: {})[",
        format_f64(total_w),
        format_f64(total_h),
        chart_area_stroke(&chart.chart_area_outline, chart.host)
    );

    // Office puts the first category at twelve o'clock and runs clockwise, the
    // same origin and direction the pie's first wedge takes.
    let angle = |index: usize| -> f64 {
        -std::f64::consts::FRAC_PI_2
            + (index as f64) * std::f64::consts::TAU / (category_count as f64)
    };
    let point = |index: usize, value: f64| -> (f64, f64) {
        let reach: f64 = radius * (value / nice_max).clamp(0.0, 1.0);
        let a: f64 = angle(index);
        (centre_x + reach * a.cos(), centre_y + reach * a.sin())
    };

    // The web: one closed ring per major unit, so the rings land on the same
    // values the tick labels name.
    for unit in major_units(nice_max, step) {
        if unit <= 0.0 {
            continue;
        }
        let ring: String = (0..category_count)
            .map(|index| {
                let (x, y) = point(index, unit);
                format!("({}pt, {}pt)", format_f64(x), format_f64(y))
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "#place(top + left, path(closed: true, stroke: {}, {ring}))",
            CHART_AUTOMATIC_LINE
        );
    }

    // The spokes, each running the full radius so the outermost ring's vertices
    // sit on them.
    for index in 0..category_count {
        let (x, y) = point(index, nice_max);
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, line(end: ({}pt, {}pt), stroke: {}))",
            format_f64(centre_x),
            format_f64(centre_y),
            format_f64(x - centre_x),
            format_f64(y - centre_y),
            CHART_AUTOMATIC_LINE
        );
    }

    // The value tick labels, read up the first spoke as Office reads them.
    let label_pt: f64 = chart_axis_text_pt(chart, chart.value_axis_text_style);
    if !chart.value_axis_deleted {
        for unit in major_units(nice_max, step) {
            if unit <= 0.0 {
                continue;
            }
            let (_, y) = point(0, unit);
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(right + horizon)[#text(size: {}pt)[{}]]])",
                format_f64(centre_x - RADAR_VALUE_GAP - GAP),
                format_f64(y - chart_label_box_h(label_pt) / 2.0),
                format_f64(RADAR_VALUE_GAP),
                format_f64(chart_label_box_h(label_pt)),
                format_f64(label_pt),
                chart_value_label(unit)
            );
        }
    }

    // Each series as one closed polygon through its value on every spoke.
    for (series_index, series) in chart.series.iter().enumerate() {
        let color: String = series_color(series, series_index, 0, &chart.theme_accent_colors);
        let points: Vec<(f64, f64)> = (0..category_count)
            .map(|index| point(index, series.values.get(index).copied().unwrap_or(0.0)))
            .collect();
        let coords: String = points
            .iter()
            .map(|(x, y)| format!("({}pt, {}pt)", format_f64(*x), format_f64(*y)))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "#place(top + left, path(closed: true, stroke: {}pt + {color}, {coords}))",
            format_f64(SERIES_LINE_PT)
        );
        for (x, y) in &points {
            write_series_marker(out, series_index, *x, *y, &color);
        }
    }

    // The category labels, each just outside its spoke's end.
    if !chart.category_axis_deleted {
        let category_pt: f64 = chart_axis_text_pt(chart, chart.category_axis_text_style);
        let weight: &str = chart_axis_text_weight(chart, chart.category_axis_text_style);
        for (index, category) in chart.categories.iter().enumerate() {
            let a: f64 = angle(index);
            let label_x: f64 = centre_x + (radius + GAP) * a.cos();
            let label_y: f64 = centre_y + (radius + GAP) * a.sin();
            // The box is centred on the point outside the spoke, so a label at
            // the left of the web grows leftwards and one at the right grows
            // rightwards rather than every label running off one side.
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt)[#align(center + horizon)[#text(size: {}pt{})[{}]]])",
                format_f64(label_x - RADAR_LABEL_HALF_W_PT),
                format_f64(label_y - chart_label_box_h(category_pt) / 2.0),
                format_f64(RADAR_LABEL_HALF_W_PT * 2.0),
                format_f64(chart_label_box_h(category_pt)),
                format_f64(category_pt),
                weight,
                escape_typst(category)
            );
        }
    }

    // The legend, keyed like the line plot's: a stroke sample carrying the
    // marker the series draws on each vertex.
    let legend_names: Vec<String> = chart
        .series
        .iter()
        .enumerate()
        .map(|(index, series)| {
            series
                .name
                .clone()
                .unwrap_or_else(|| format!("Series {}", index + 1))
        })
        .collect();
    let entry_widths: Vec<f64> = legend_entry_widths(chart, LEGEND_KEY_LEN_PT, &legend_names);
    if chart.has_legend {
        for (series_index, series) in chart.series.iter().enumerate() {
            let color: String = series_color(series, series_index, 0, &chart.theme_accent_colors);
            let default_name: String = format!("Series {}", series_index + 1);
            let name: &str = series.name.as_deref().unwrap_or(&default_name);
            let (entry_x, entry_y) = legend.entry_origin(
                chart.legend_position,
                series_index,
                chart.series.len().max(1),
                (legend.left, legend.top, span_w, span_h),
                RADAR_LEGEND_ROW_H,
                &entry_widths,
            );
            let key_mid: f64 = SERIES_MARKER_SIZE_PT / 2.0;
            let key: String = format!(
                "#box(width: {}pt, height: {}pt, baseline: {}pt)[\
                 #place(top + left, dx: 0pt, dy: {}pt, line(end: ({}pt, 0pt), stroke: {}pt + {color}))\
                 {}]",
                format_f64(LEGEND_KEY_LEN_PT),
                format_f64(SERIES_MARKER_SIZE_PT),
                format_f64(LEGEND_KEY_BASELINE_PT),
                format_f64(key_mid),
                format_f64(LEGEND_KEY_LEN_PT),
                format_f64(SERIES_LINE_PT),
                series_marker_markup(series_index, LEGEND_KEY_LEN_PT / 2.0, key_mid, &color)
                    .trim_end()
            );
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box[{key}#h({}pt)#text(size: {}pt)[{}]])",
                format_f64(entry_x),
                format_f64(entry_y),
                format_f64(LEGEND_KEY_LABEL_GAP_PT),
                format_f64(chart_text_pt(chart)),
                escape_typst(name)
            );
        }
    }

    out.push_str("]\n");
}

/// Render a pie chart as a circle of wedges, each sized by its share of the
/// series total, with the legend on the edge `<c:legendPos>` asks for.
fn generate_chart_pie_plot(out: &mut String, chart: &Chart, frame: Option<(f64, f64)>) {
    const PIE_DIAMETER: f64 = 200.0;
    const PIE_LEGEND_ROW_H: f64 = 14.0;

    let Some(series) = chart.series.first() else {
        return;
    };
    let total: f64 = series.values.iter().filter(|value| **value > 0.0).sum();
    if total <= 0.0 {
        return;
    }

    if let Some(title) = chart.title.as_deref() {
        let _ = writeln!(
            out,
            "#align(center)[#text(size: {}pt, weight: \"bold\")[{}]]",
            format_f64(chart_area_title_pt(chart)),
            escape_typst(title)
        );
        out.push_str("#v(4pt)\n");
    }

    let legend: LegendBox = if chart.has_legend {
        LegendBox::new(chart.legend_position, PIE_LEGEND_ROW_H, LEGEND_ENTRY_W)
    } else {
        LegendBox::hidden()
    };
    // As elsewhere: the title is drawn above the box, so a framed chart takes
    // its height out of the frame.
    let frame: Option<(f64, f64)> = frame.map(|(width, height)| {
        let title_h: f64 = if chart.title.is_some() {
            chart_area_title_h(chart)
        } else {
            0.0
        };
        (width, (height - title_h).max(MIN_PLOT_PT))
    });
    let (total_w, total_h) = match frame {
        Some(extent) => extent,
        None => (
            legend.left + PIE_DIAMETER + legend.right,
            legend.top + PIE_DIAMETER + legend.bottom,
        ),
    };
    // The pie stays circular, so it takes the smaller of the two axes.
    let diameter: f64 = (total_w - legend.left - legend.right)
        .min(total_h - legend.top - legend.bottom)
        .max(MIN_PLOT_PT);
    let radius: f64 = diameter / 2.0;
    let centre_x: f64 = legend.left + (total_w - legend.left - legend.right) / 2.0;
    let centre_y: f64 = legend.top + (total_h - legend.top - legend.bottom) / 2.0;

    let _ = writeln!(
        out,
        "#box(width: {}pt, height: {}pt, stroke: {})[",
        format_f64(total_w),
        format_f64(total_h),
        chart_area_stroke(&chart.chart_area_outline, chart.host)
    );

    // Office starts the first wedge at twelve o'clock and sweeps clockwise.
    let mut start: f64 = -std::f64::consts::FRAC_PI_2;
    for (index, value) in series.values.iter().enumerate() {
        if *value <= 0.0 {
            continue;
        }
        let sweep: f64 = value / total * std::f64::consts::TAU;
        let color: String = category_color(
            series,
            index,
            &CHART_CATEGORY_COLORS,
            &chart.theme_accent_colors,
        );
        match doughnut_inner_radius(chart, radius) {
            Some(inner) => {
                write_doughnut_segment(out, centre_x, centre_y, radius, inner, start, sweep, &color)
            }
            None => write_pie_wedge(out, centre_x, centre_y, radius, start, sweep, &color),
        }
        if let Some(label) = data_label_text(chart, series, index, total) {
            // A wedge label sits on the bisector, two thirds of the way out —
            // clear of the centre where narrow wedges converge, and inside the
            // circumference where the fill still backs it.
            let bisector: f64 = start + sweep / 2.0;
            let label_radius: f64 = radius * 2.0 / 3.0;
            // The box is centred on that point, so it is placed from its own
            // top-left corner.
            let label_w: f64 = radius;
            let _ = writeln!(
                out,
                "#place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt)[#align(center)[#text(size: 8pt, weight: \"bold\", fill: white)[{}]]])",
                format_f64(centre_x + label_radius * bisector.cos() - label_w / 2.0),
                format_f64(centre_y + label_radius * bisector.sin() - LABEL_LINE_H / 2.0),
                format_f64(label_w),
                escape_typst(&label)
            );
        }
        start += sweep;
    }

    // Legend entries, one per slice, at the position the chart asks for —
    // none when the chart declares no `<c:legend>`. A pie's own legend
    // duplicates the slice labels, so one the file never asked for is doubly
    // visible (issue #762).
    let entries: usize = chart.categories.len().max(series.values.len());
    let entry_widths: Vec<f64> = legend_entry_widths(chart, LEGEND_KEY_LEN_PT, &chart.categories);
    let legend_entries: usize = if chart.has_legend { entries } else { 0 };
    for (index, category) in chart.categories.iter().enumerate().take(legend_entries) {
        let color: String = category_color(
            series,
            index,
            &CHART_CATEGORY_COLORS,
            &chart.theme_accent_colors,
        );
        let (entry_x, entry_y) = legend.entry_origin(
            chart.legend_position,
            index,
            entries,
            (centre_x - radius, centre_y - radius, diameter, diameter),
            PIE_LEGEND_ROW_H,
            &entry_widths,
        );
        let _ = writeln!(
            out,
            "#place(top + left, dx: {}pt, dy: {}pt, box[#box(width: 9pt, height: 9pt, fill: {})#h({}pt)#text(size: {}pt)[{}]])",
            format_f64(entry_x),
            format_f64(entry_y),
            color,
            format_f64(LEGEND_KEY_LABEL_GAP_PT),
            format_f64(chart_text_pt(chart)),
            escape_typst(category)
        );
    }

    out.push_str("]\n");
}

/// Emit one filled wedge from `start` through `sweep` radians.
///
/// A cubic Bézier tracks a circular arc closely up to a quarter turn, so the
/// sweep is split into at most quarter-turn segments. Each arc vertex carries
/// handles of `4/3 * tan(step/4) * radius` along the tangent — the standard
/// construction — as Typst's `(point, control-in, control-out)` triple, both
/// controls relative to the vertex.
fn write_pie_wedge(
    out: &mut String,
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    start: f64,
    sweep: f64,
    color: &str,
) {
    let segments: usize = (sweep / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step: f64 = sweep / segments as f64;
    let handle: f64 = 4.0 / 3.0 * (step / 4.0).tan() * radius;

    let point = |angle: f64| -> (f64, f64) {
        (
            centre_x + radius * angle.cos(),
            centre_y + radius * angle.sin(),
        )
    };
    // Unit tangent in the sweep direction, which the handles run along.
    let tangent = |angle: f64| -> (f64, f64) { (-angle.sin(), angle.cos()) };

    // The wedge starts at the centre; `closed: true` draws the final radius
    // back to it, so the last vertex leaves no outgoing handle to curve it.
    let mut path = format!(
        "#place(top + left, path(fill: {color}, stroke: none, closed: true, ({}pt, {}pt)",
        format_f64(centre_x),
        format_f64(centre_y)
    );
    for segment in 0..=segments {
        let angle: f64 = start + step * segment as f64;
        let (x, y) = point(angle);
        let (tx, ty) = tangent(angle);
        // The first vertex has nothing arriving at it and the last nothing
        // leaving, so their unused handles stay zero.
        let (in_dx, in_dy) = if segment == 0 {
            (0.0, 0.0)
        } else {
            (-tx * handle, -ty * handle)
        };
        let (out_dx, out_dy) = if segment == segments {
            (0.0, 0.0)
        } else {
            (tx * handle, ty * handle)
        };
        let _ = write!(
            path,
            ", (({}pt, {}pt), ({}pt, {}pt), ({}pt, {}pt))",
            format_f64(x),
            format_f64(y),
            format_f64(in_dx),
            format_f64(in_dy),
            format_f64(out_dx),
            format_f64(out_dy)
        );
    }
    path.push_str("))");
    let _ = writeln!(out, "{path}");
}

/// The inner radius of a doughnut, or `None` for a pie.
///
/// `<c:holeSize>` gives the inner radius as a percentage of the outer. The
/// bounds here are defensive rather than quoted from the schema: at 0 the hole
/// closes and the ring becomes a pie, and at 100 there is no ring left to
/// draw, so both ends are clamped away from those degenerate results.
///
/// The 50 used when the element is absent is a placeholder, not a measured
/// default — the audited deck always writes `holeSize`, so no fixture
/// exercises it. If one ever does, check what the source application draws
/// before trusting this number.
fn doughnut_inner_radius(chart: &Chart, outer_radius: f64) -> Option<f64> {
    if !matches!(chart.chart_type, ChartType::Doughnut) {
        return None;
    }
    let percent: f64 = chart.hole_size_percent.unwrap_or(50) as f64;
    Some(outer_radius * percent.clamp(1.0, 90.0) / 100.0)
}

/// A doughnut ring segment: the outer arc swept forward, the inner arc swept
/// back, closed.
///
/// Kept apart from `write_pie_wedge` rather than folded into it: a wedge starts
/// at the centre, which has no incoming handle to curve, and merging the two
/// would bury that.
///
/// The hole is absent ink, not a background-coloured disc — a chart draws over
/// whatever the slide puts behind it, so punching with a guessed colour would
/// be wrong (issue #679).
#[allow(clippy::too_many_arguments)]
fn write_doughnut_segment(
    out: &mut String,
    centre_x: f64,
    centre_y: f64,
    outer_radius: f64,
    inner_radius: f64,
    start: f64,
    sweep: f64,
    color: &str,
) {
    let segments: usize = (sweep / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step: f64 = sweep / segments as f64;

    let mut path = format!("#place(top + left, path(fill: {color}, stroke: none, closed: true");

    let mut arc = |radius: f64, forward: bool| {
        let handle: f64 = 4.0 / 3.0 * (step / 4.0).tan() * radius;
        for index in 0..=segments {
            let position = if forward { index } else { segments - index };
            let angle: f64 = start + step * position as f64;
            let (x, y) = (
                centre_x + radius * angle.cos(),
                centre_y + radius * angle.sin(),
            );
            // The return leg reverses the sweep, so its tangent flips.
            let direction = if forward { 1.0 } else { -1.0 };
            let (tx, ty) = (-angle.sin() * direction, angle.cos() * direction);
            // The join between the arcs is a straight radial edge, so the
            // handles facing it stay zero.
            let (in_dx, in_dy) = if index == 0 {
                (0.0, 0.0)
            } else {
                (-tx * handle, -ty * handle)
            };
            let (out_dx, out_dy) = if index == segments {
                (0.0, 0.0)
            } else {
                (tx * handle, ty * handle)
            };
            let _ = write!(
                path,
                ", (({}pt, {}pt), ({}pt, {}pt), ({}pt, {}pt))",
                format_f64(x),
                format_f64(y),
                format_f64(in_dx),
                format_f64(in_dy),
                format_f64(out_dx),
                format_f64(out_dy)
            );
        }
    };
    arc(outer_radius, true);
    arc(inner_radius, false);

    path.push_str("))");
    let _ = writeln!(out, "{path}");
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
        let color: String = category_color(series, index, colors, &chart.theme_accent_colors);
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
            let _ = write!(
                out,
                "[{}]",
                escape_typst(&chart_value_label_formatted(
                    value,
                    series.number_format.as_deref()
                ))
            );
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
    use super::{AXIS_HEADROOM_DIVISOR, chart_value_label, nice_axis};

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
        // The first three are entries of MEASURED_AUTO_SCALE, restated here so
        // the everyday shape of the rule — clear the data, divide into whole
        // steps — stays readable next to the degenerate guard, which is the
        // only assertion below that no rendering pins.
        assert_eq!(nice_axis(8.2), (9.0, 1.0));
        assert_eq!(nice_axis(3.2), (3.5, 0.5));
        assert_eq!(nice_axis(45.0), (50.0, 5.0));
        assert_eq!(nice_axis(0.0), (1.0, 1.0));
    }

    /// Axis maxima read off renderings of `WithChart.xlsx` with both series
    /// scaled by one factor, one file per data maximum, the chart declaring no
    /// `c:max`/`c:min`/`c:majorUnit` so the axis is entirely auto-scaled.
    /// `scripts/measure_chart_axis.py` regenerates the whole table.
    ///
    /// Excel's own choice is known for only two of these (issue #634's export
    /// at 17, issue #553's at 23,334); the rest are LibreOffice's, which agrees
    /// with Excel on both of those and is the widest independent sample of the
    /// same rule available without Excel.
    ///
    /// Eight entries — 0.44, 1.9, 3.2, 5.5, 8.2, 12.5, 45 and 199 — were
    /// predicted from the rule fitted to the other thirty and only then
    /// rendered, so they are a held-out check rather than fitted data. They
    /// also carry the two decades the fitted set never reached, which is what
    /// stops a table this dense from being satisfied by a lookup.
    const MEASURED_AUTO_SCALE: [(f64, f64, f64); 38] = [
        (0.44, 0.5, 0.05),
        (1.9, 2.0, 0.2),
        (3.2, 3.5, 0.5),
        (5.5, 6.0, 1.0),
        (6.0, 7.0, 1.0),
        (6.3, 7.0, 1.0),
        (7.4, 8.0, 1.0),
        (8.0, 9.0, 1.0),
        (8.2, 9.0, 1.0),
        (8.6, 10.0, 1.0),
        (9.0, 10.0, 1.0),
        (9.7, 12.0, 2.0),
        (12.5, 14.0, 2.0),
        (14.0, 16.0, 2.0),
        (17.0, 18.0, 2.0),
        (19.0, 20.0, 2.0),
        (21.0, 25.0, 5.0),
        (24.0, 30.0, 5.0),
        (31.0, 35.0, 5.0),
        (45.0, 50.0, 5.0),
        (46.0, 50.0, 5.0),
        (52.0, 60.0, 10.0),
        (63.0, 70.0, 10.0),
        (74.0, 80.0, 10.0),
        (78.0, 90.0, 10.0),
        (86.0, 100.0, 10.0),
        (97.0, 120.0, 20.0),
        (140.0, 160.0, 20.0),
        (199.0, 250.0, 50.0),
        (230.0, 250.0, 50.0),
        (460.0, 500.0, 50.0),
        (520.0, 600.0, 100.0),
        (740.0, 800.0, 100.0),
        (860.0, 1000.0, 100.0),
        (970.0, 1200.0, 200.0),
        (1400.0, 1600.0, 200.0),
        (2300.0, 2500.0, 500.0),
        (23334.0, 25000.0, 5000.0),
    ];

    #[test]
    fn nice_axis_reproduces_every_measured_auto_scale() {
        let mut wrong: Vec<String> = Vec::new();
        for (data_max, want_max, want_step) in MEASURED_AUTO_SCALE {
            let got: (f64, f64) = nice_axis(data_max);
            if (got.0 - want_max).abs() > 1e-9 || (got.1 - want_step).abs() > 1e-9 {
                wrong.push(format!(
                    "data max {data_max}: got {got:?}, measured ({want_max}, {want_step})"
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "{} of {} measured axes not reproduced:\n  {}",
            wrong.len(),
            MEASURED_AUTO_SCALE.len(),
            wrong.join("\n  ")
        );
    }

    #[test]
    fn nice_axis_scales_with_the_decimal_exponent() {
        // The rule reads a mantissa and an exponent, so one measured maximum
        // implies the whole decade. A rule fitted only to the sampled decades
        // would pass the table above and fail here.
        for exponent in [-3i32, -1, 0, 2, 5, 8] {
            let factor: f64 = 10f64.powi(exponent);
            for (data_max, want_max, want_step) in MEASURED_AUTO_SCALE {
                let (got_max, got_step): (f64, f64) = nice_axis(data_max * factor);
                let scale: f64 = (want_max * factor).abs().max(1e-12);
                assert!(
                    ((got_max - want_max * factor) / scale).abs() < 1e-9,
                    "max {data_max}e{exponent}: got {got_max}, want {}",
                    want_max * factor
                );
                assert!(
                    ((got_step - want_step * factor) / scale).abs() < 1e-9,
                    "step for {data_max}e{exponent}: got {got_step}, want {}",
                    want_step * factor
                );
            }
        }
    }

    #[test]
    fn nice_axis_does_not_round_the_maximum_to_the_step_ladder() {
        // Rounding the maximum itself to 1/2/5x10^n put 23,334 against a
        // 50,000 axis, drawing every column at half the height Excel gives it
        // (#553). Its export is one of the two entries in the table above that
        // Excel itself produced, so keep the symptom pinned by name.
        let (axis_max, step): (f64, f64) = nice_axis(23334.0);
        assert_eq!((axis_max, step), (25000.0, 5000.0));
        assert!(
            23334.0 / axis_max > 0.9,
            "the tallest column reaches {:.0}% of the plot; Excel's export reaches 93%",
            23334.0 / axis_max * 100.0
        );
    }

    #[test]
    fn nice_axis_leaves_no_more_than_one_step_of_headroom() {
        // The property that makes a chart readable: the tallest bar reaches
        // within one major unit of the top, plus the twentieth of the range
        // Excel adds before it rounds — so a maximum of 100 sits under a 120
        // axis rather than touching a 100 one.
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
            let allowed: f64 = step + value / AXIS_HEADROOM_DIVISOR;
            assert!(
                max - value < allowed || (max - value - allowed).abs() < 1e-9,
                "axis {max} leaves {} of headroom over {value}, more than a {step} step \
                 plus a twentieth of the range",
                max - value
            );
            assert!(
                step > 0.0 && (max / step - (max / step).round()).abs() < 1e-9,
                "{max} must divide into whole {step} steps"
            );
        }
    }
}

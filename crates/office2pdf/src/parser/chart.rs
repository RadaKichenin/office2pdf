//! Chart XML parser for DOCX embedded charts.
//!
//! Parses chart*.xml files from DOCX ZIP archives and extracts chart type,
//! title, category labels, and series data into IR `Chart` structs.

use quick_xml::Reader;
use quick_xml::events::Event;

use super::drawingml::{self, SchemeColors};
use super::xml_util;
use crate::ir::{
    AxisTickMark, BarBandLayout, Chart, ChartAreaOutline, ChartGrouping, ChartHost, ChartLine,
    ChartSeries, ChartTextStyle, ChartType, Color, DataLabels, LegendPosition,
};

/// Mapping from XML chart element tag names to their corresponding `ChartType`.
/// Both 2-D and 3-D variants map to the same logical type.
///
/// The bar family maps to `Column` because that is the orientation ECMA-376
/// gives `ST_BarDir`'s `val` attribute by default, and the one Excel and
/// PowerPoint write for their default clustered chart. `<c:barDir val="bar"/>`
/// overrides it; see [`bar_direction_chart_type`].
const CHART_TAG_TYPES: &[(&[u8], ChartType)] = &[
    (b"barChart", ChartType::Column),
    (b"bar3DChart", ChartType::Column),
    (b"lineChart", ChartType::Line),
    (b"line3DChart", ChartType::Line),
    (b"pieChart", ChartType::Pie),
    (b"pie3DChart", ChartType::Pie),
    (b"doughnutChart", ChartType::Doughnut),
    (b"areaChart", ChartType::Area),
    (b"scatterChart", ChartType::Scatter),
];

/// Display labels for the plot-area families ECMA-376 defines that no plot
/// implementation covers. They render through the data-table fallback, which
/// prints this label as the chart's kind.
///
/// Dropping them instead made the chart vanish with no diagnostic, taking the
/// whole graphic frame with it (issue #544).
const UNPLOTTED_CHART_LABELS: &[(&[u8], &str)] = &[
    (b"radarChart", crate::ir::RADAR_CHART_LABEL),
    (b"bubbleChart", "Bubble Chart"),
    (b"stockChart", "Stock Chart"),
    (b"surfaceChart", "Surface Chart"),
    (b"surface3DChart", "Surface Chart"),
    (b"area3DChart", "Area Chart"),
];

/// `<c:ofPieChart>` covers two shapes, told apart by `<c:ofPieType>`. ECMA-376
/// defaults `ST_OfPieType` to `pie`.
fn of_pie_label(of_pie_type: Option<&str>) -> &'static str {
    match of_pie_type {
        Some("bar") => "Bar of Pie Chart",
        _ => "Pie of Pie Chart",
    }
}

/// Resolve a plot-area element to the chart type it opens.
///
/// Families with a plot implementation take it; the rest keep their data and
/// a readable label so the fallback can draw them. Anything else whose name
/// ends in `Chart` is a family this list has not caught up with, and is better
/// rendered as a table than dropped.
fn chart_type_for_tag(tag: &[u8]) -> Option<ChartType> {
    if let Some((_, chart_type)) = CHART_TAG_TYPES.iter().find(|(name, _)| *name == tag) {
        return Some(chart_type.clone());
    }
    if let Some((_, label)) = UNPLOTTED_CHART_LABELS.iter().find(|(name, _)| *name == tag) {
        return Some(ChartType::Other(label.to_string()));
    }
    // The suffix is enough on its own: the part's own `chartSpace` and `chart`
    // elements differ in case or ending, and every element that does carry it
    // is a plot-area family.
    let name: &str = std::str::from_utf8(tag).ok()?;
    name.ends_with("Chart")
        .then(|| ChartType::Other(generic_chart_label(name)))
}

/// Turn an unknown `fooBarChart` element name into `Foo Bar Chart`.
fn generic_chart_label(element: &str) -> String {
    let mut label = String::new();
    for (index, character) in element.char_indices() {
        if index > 0 && character.is_ascii_uppercase() {
            label.push(' ');
        }
        if index == 0 {
            label.extend(character.to_uppercase());
        } else {
            label.push(character);
        }
    }
    label
}

/// Resolve a `<c:legendPos>` value to the edge the legend sits on.
fn legend_position_for(value: &str) -> LegendPosition {
    match value {
        "b" => LegendPosition::Bottom,
        "l" => LegendPosition::Left,
        "t" => LegendPosition::Top,
        "tr" => LegendPosition::TopRight,
        _ => LegendPosition::Right,
    }
}

/// Resolve a `<c:grouping>` value to the way a category's series combine.
///
/// Line and area charts spell their unstacked form `standard` where the bar
/// family spells it `clustered`; both mean one mark per series.
fn chart_grouping_for(value: &str) -> ChartGrouping {
    match value {
        "stacked" => ChartGrouping::Stacked,
        "percentStacked" => ChartGrouping::PercentStacked,
        _ => ChartGrouping::Clustered,
    }
}

/// Resolve a `<c:barDir>` value to the chart orientation it selects.
///
/// `<c:barDir>` is exclusive to the bar family, so `None` — either an absent
/// element or a non-bar chart — leaves the tag's own mapping in place.
fn bar_direction_chart_type(direction: Option<&str>) -> Option<ChartType> {
    match direction? {
        "bar" => Some(ChartType::Bar),
        "col" => Some(ChartType::Column),
        _ => None,
    }
}

/// Parse a chart XML file (e.g., `word/charts/chart1.xml`) into a `Chart` IR.
pub(crate) fn parse_chart_xml(xml: &str, scheme: &SchemeColors<'_>) -> Option<Chart> {
    let mut reader = Reader::from_str(xml);
    let mut chart_type = None;
    let mut hole_size_percent: Option<u32> = None;
    let mut title = None;
    let mut categories: Vec<String> = Vec::new();
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut grouping: Option<ChartGrouping> = None;
    let mut gap_width_percent: Option<f64> = None;
    let mut overlap_percent: Option<f64> = None;
    let mut legend_position: Option<LegendPosition> = None;
    // `<c:legend>` presence, not its position: a chart that declares no legend
    // still gets a default position, so the position alone cannot say whether
    // one was asked for (issue #762).
    let mut has_legend: bool = false;
    let mut auto_title_deleted: bool = false;
    let mut category_axis: Axis = Axis::default();
    let mut value_axis: Axis = Axis::default();
    // `c:chartSpace/c:spPr` is a *sibling* of `c:chart`, and the schema puts it
    // after it. This loop is flat over every Start event, so without that
    // marker a `c:spPr` belonging to `c:plotArea` would be read as the chart
    // area's own (#637).
    let mut chart_element_ended: bool = false;
    let mut chart_area_outline: ChartAreaOutline = ChartAreaOutline::Default;
    // `c:chartSpace/c:txPr` is a sibling of `c:chart` for the same reason
    // `c:spPr` is, so it needs the same marker: an axis carries a `c:txPr` of
    // its own inside `c:plotArea`, and a flat loop would read that one (#668).
    let mut text_font_family: Option<String> = None;
    let mut text_style: ChartTextStyle = ChartTextStyle::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                let tag: &[u8] = local.as_ref();
                if tag == b"spPr" && chart_element_ended {
                    chart_area_outline = parse_chart_area_outline(&mut reader, scheme);
                } else if tag == b"txPr" && chart_element_ended {
                    let (family, style) = parse_chart_text_properties(&mut reader);
                    text_font_family = family;
                    text_style = style;
                } else if tag == b"legend" {
                    // Declared, unless the element switches itself off.
                    let (deleted, position) = parse_legend(&mut reader);
                    has_legend = !deleted;
                    legend_position = legend_position.or(position);
                } else if tag == b"title" && title.is_none() {
                    title = parse_chart_title(&mut reader);
                } else if tag == b"catAx" {
                    category_axis = parse_axis(&mut reader, b"catAx", scheme);
                } else if tag == b"valAx" {
                    value_axis = parse_axis(&mut reader, b"valAx", scheme);
                } else if let Some(ct) = chart_type_for_tag(tag) {
                    let mut plot: PlotAreaProps = PlotAreaProps::default();
                    parse_chart_series(
                        &mut reader,
                        tag,
                        &mut categories,
                        &mut series,
                        &mut plot,
                        scheme,
                    );
                    chart_type = Some(match ct {
                        // `<c:ofPieChart>` names its shape in a child element,
                        // so the label waits until the body has been read.
                        ChartType::Other(_) if tag == b"ofPieChart" => {
                            ChartType::Other(of_pie_label(plot.of_pie_type.as_deref()).to_string())
                        }
                        other => {
                            bar_direction_chart_type(plot.bar_direction.as_deref()).unwrap_or(other)
                        }
                    });
                    grouping = plot.grouping.as_deref().map(chart_grouping_for);
                    // Only the doughnut family writes it.
                    if matches!(chart_type, Some(ChartType::Doughnut)) {
                        hole_size_percent = plot.hole_size.as_deref().and_then(|v| v.parse().ok());
                    }
                    // A combo plot area holds one element per chart family and
                    // only the bar family carries these two, so the family that
                    // declared them keeps them however many follow it.
                    let (gap_width, overlap) = plot.bar_band_layout();
                    gap_width_percent = gap_width_percent.or(gap_width);
                    overlap_percent = overlap_percent.or(overlap);
                }
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"legend" => {
                has_legend = true;
            }
            // `CT_Boolean`, so always self-closing; an earlier arm already
            // owns every `Start` event in this loop.
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"autoTitleDeleted" => {
                auto_title_deleted = ct_boolean(e);
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"legendPos" => {
                legend_position = xml_util::get_attr_str(e, b"val")
                    .as_deref()
                    .map(legend_position_for);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"chart" => {
                chart_element_ended = true;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let chart_type = chart_type?;
    let default_band_layout: BarBandLayout = BarBandLayout::default();

    // Charts may omit <c:cat> entirely; Excel then labels the category axis
    // 1..N (the point count of the longest series).
    if categories.is_empty() {
        let point_count: usize = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        categories = (1..=point_count).map(|i| i.to_string()).collect();
    }

    Some(Chart {
        chart_type,
        hole_size_percent,
        title,
        categories,
        series,
        grouping: grouping.unwrap_or_default(),
        legend_position: legend_position.unwrap_or_default(),
        has_legend,
        auto_title_deleted,
        category_axis_title: category_axis.title,
        value_axis_title: value_axis.title,
        category_axis_major_tick_mark: category_axis.major_tick_mark,
        value_axis_major_tick_mark: value_axis.major_tick_mark,
        category_axis_line: category_axis.line,
        value_axis_line: value_axis.line,
        // Office hangs the gridlines off whichever axis they run across; the
        // value axis carries the horizontal set our renderer draws.
        major_gridline_line: match value_axis.gridline {
            ChartLine::Automatic => category_axis.gridline,
            stated => stated,
        },
        category_axis_deleted: category_axis.deleted,
        value_axis_deleted: value_axis.deleted,
        bar_band_layout: BarBandLayout {
            gap_width_percent: gap_width_percent.unwrap_or(default_band_layout.gap_width_percent),
            overlap_percent: overlap_percent.unwrap_or(default_band_layout.overlap_percent),
        },
        // The chart part names no theme of its own; the package that holds it
        // does. Whoever loaded this XML fills these in, since only they know
        // which theme part applies.
        theme_accent_colors: Vec::new(),
        chart_area_outline,
        // As with the theme, the chart part does not know which application's
        // package holds it; the loader sets this (issue #823).
        host: ChartHost::default(),
        // A `+mn-lt` token stays as written for the same reason: resolving it
        // needs the package theme, which only the loader has.
        text_font_family,
        text_style,
        category_axis_text_style: category_axis.text_style,
        value_axis_text_style: value_axis.text_style,
        value_axis_number_format: value_axis.number_format,
    })
}

/// Read `c:chartSpace/c:txPr` into the face and the run properties its
/// `a:defRPr` declares.
///
/// `a:latin`'s absence is the common case — the chart then inherits the theme's
/// minor font — and is reported as `None` rather than as a face, so the loader
/// can tell "said nothing" from "said this" (issue #668). The same holds for
/// each run property (issue #669).
fn parse_chart_text_properties(reader: &mut Reader<&[u8]>) -> (Option<String>, ChartTextStyle) {
    let mut typeface: Option<String> = None;
    let mut style: ChartTextStyle = ChartTextStyle::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.local_name().as_ref() {
                    b"latin" if typeface.is_none() => {
                        // An empty `typeface=""` names no face; Office writes
                        // that to mean "inherit", which is what `None` says.
                        typeface = xml_util::get_attr_str(e, b"typeface")
                            .filter(|face| !face.trim().is_empty());
                    }
                    b"defRPr" => read_def_rpr_into(e, &mut style),
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"txPr" => break,
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    (typeface, style)
}

/// Read a `c:txPr` that governs one element rather than the whole chart space,
/// keeping only its run properties.
fn parse_chart_text_style(reader: &mut Reader<&[u8]>) -> ChartTextStyle {
    parse_chart_text_properties(reader).1
}

/// Take `a:defRPr`'s run properties, leaving untouched whatever it omits.
///
/// Only the first `a:defRPr` counts: `c:txPr` holds one paragraph, and a later
/// `a:endParaRPr` describes the empty run after the text rather than the text.
fn read_def_rpr_into(element: &quick_xml::events::BytesStart<'_>, style: &mut ChartTextStyle) {
    if style.size_pt.is_none() {
        // `@sz` is in hundredths of a point.
        style.size_pt = xml_util::get_attr_str(element, b"sz")
            .and_then(|raw| raw.parse::<f64>().ok())
            .filter(|hundredths| *hundredths > 0.0)
            .map(|hundredths| hundredths / 100.0);
    }
    if style.bold.is_none() {
        style.bold =
            xml_util::get_attr_str(element, b"b").map(|raw| matches!(raw.trim(), "1" | "true"));
    }
}

/// Read `c:chartSpace/c:spPr` into what it says about the chart-area outline.
///
/// Only `a:ln` matters here. Its absence is not the same as `<a:noFill/>`:
/// absent means Office's default outline, `noFill` means none at all, and an
/// explicit line means that line (#637).
fn parse_chart_area_outline(
    reader: &mut Reader<&[u8]>,
    scheme: &SchemeColors<'_>,
) -> ChartAreaOutline {
    let mut in_line: bool = false;
    let mut saw_line: bool = false;
    let mut suppressed: bool = false;
    let mut width_pt: Option<f64> = None;
    let mut color: Option<Color> = None;
    let mut in_solid_fill: bool = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if e.local_name().as_ref() == b"ln" =>
            {
                in_line = true;
                saw_line = true;
                width_pt = width_pt.or_else(|| {
                    xml_util::get_attr_str(e, b"w")
                        .and_then(|w| w.parse::<f64>().ok())
                        .map(|emu| emu / EMU_PER_POINT)
                });
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if in_line && e.local_name().as_ref() == b"noFill" =>
            {
                suppressed = true;
            }
            Ok(Event::Start(ref e)) if in_line && e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = true;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = false;
            }
            Ok(Event::Start(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                color = color.or(drawingml::parse_color_from_start(reader, e, scheme).color);
            }
            Ok(Event::Empty(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                color = color.or(drawingml::parse_color_from_empty(e, scheme).color);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"ln" => {
                in_line = false;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"spPr" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if !saw_line {
        ChartAreaOutline::Default
    } else if suppressed {
        ChartAreaOutline::Suppressed
    } else {
        ChartAreaOutline::Explicit { width_pt, color }
    }
}

/// EMU in one point. `a:ln/@w` is in EMU.
const EMU_PER_POINT: f64 = 12700.0;

/// What one `<c:catAx>` or `<c:valAx>` element says about itself.
#[derive(Default)]
struct Axis {
    title: Option<String>,
    major_tick_mark: AxisTickMark,
    deleted: bool,
    text_style: ChartTextStyle,
    /// `<c:numFmt formatCode>` — how this axis prints its tick labels.
    number_format: Option<String>,
    /// What `<c:spPr>` says about the axis' own line.
    line: ChartLine,
    /// What `<c:majorGridlines><c:spPr>` says; the gridlines hang off the
    /// axis rather than off the plot area.
    gridline: ChartLine,
}

/// The `formatCode` a `<c:numFmt>` states, when it states one that is not
/// `General`.
///
/// `General` is Excel's "no format" and applying it would only reformat the
/// number the plain path already prints (issue #865).
fn explicit_format_code(element: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    let code = xml_util::get_attr_str(element, b"formatCode")?;
    let code = code.trim();
    (!code.is_empty() && !code.eq_ignore_ascii_case("General")).then(|| code.to_string())
}

/// Read an axis element, consuming it to `end_tag`.
///
/// Axis titles sit after the plot-area family element, so they never reach the
/// `<c:title>` branch that captures the chart's own title.
fn parse_axis(reader: &mut Reader<&[u8]>, end_tag: &[u8], scheme: &SchemeColors<'_>) -> Axis {
    let mut axis: Axis = Axis::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"title" => {
                axis.title = axis.title.or_else(|| parse_chart_title(reader));
            }
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"txPr" => {
                axis.text_style = parse_chart_text_style(reader);
            }
            // The axis' own line, and the gridlines' — both are an `<a:ln>`
            // inside a `<c:spPr>`, and both are dropped without this (#900).
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"spPr" => {
                axis.line = parse_chart_line(reader, b"spPr", scheme);
            }
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"majorGridlines" => {
                axis.gridline = parse_chart_line(reader, b"majorGridlines", scheme);
            }
            // Office writes `<c:majorTickMark val="out"/>` self-closing, so the
            // `Start` arm alone would never see it.
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.local_name().as_ref() == b"majorTickMark" =>
            {
                axis.major_tick_mark = xml_util::get_attr_str(e, b"val")
                    .as_deref()
                    .map(axis_tick_mark_for)
                    .unwrap_or_default();
            }
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.local_name().as_ref() == b"delete" =>
            {
                axis.deleted = ct_boolean(e);
            }
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.local_name().as_ref() == b"numFmt" =>
            {
                axis.number_format = explicit_format_code(e);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    axis
}

/// Consume a `<c:legend>` body, reporting whether it switches itself off and
/// which edge it names.
///
/// `<c:legend><c:delete val="1"/></c:legend>` is how Office records a legend
/// that was turned off but whose settings were kept, the same shape the axes
/// use. The position comes back from here too because this consumes the body,
/// so `<c:legendPos>` never reaches the caller's loop (issue #762).
fn parse_legend(reader: &mut Reader<&[u8]>) -> (bool, Option<LegendPosition>) {
    let mut deleted = false;
    let mut position: Option<LegendPosition> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Empty(ref e)) => match e.local_name().as_ref() {
                b"delete" => deleted = ct_boolean(e),
                b"legendPos" => {
                    position = xml_util::get_attr_str(e, b"val")
                        .as_deref()
                        .map(legend_position_for);
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"legend" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (deleted, position)
}

/// Read a `CT_Boolean` element's own state.
///
/// ECMA-376 defaults `val` to true, so a bare `<c:delete/>` or `<c:showVal/>`
/// turns the flag on.
fn ct_boolean(element: &quick_xml::events::BytesStart) -> bool {
    xml_util::get_attr_str(element, b"val")
        .map(|value| value == "1" || value == "true")
        .unwrap_or(true)
}

/// Resolve a `<c:majorTickMark>` value to the side its ticks reach from.
fn axis_tick_mark_for(value: &str) -> AxisTickMark {
    match value {
        "none" => AxisTickMark::None,
        "in" => AxisTickMark::Inside,
        "cross" => AxisTickMark::Cross,
        _ => AxisTickMark::Outside,
    }
}

/// Parse the chart title text from `<c:title>`.
fn parse_chart_title(reader: &mut Reader<&[u8]>) -> Option<String> {
    let mut text = String::new();
    let mut in_t = false;
    let mut depth = 1u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"title" {
                    depth += 1;
                } else if local.as_ref() == b"t" {
                    in_t = true;
                }
            }
            Ok(Event::Text(ref t)) if in_t => {
                if let Ok(s) = t.xml_content() {
                    text.push_str(s.as_ref());
                }
            }
            Ok(Event::End(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_t = false;
                } else if local.as_ref() == b"title" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Plot-area settings that sit beside `<c:ser>` inside a chart type element.
///
/// Office writes them all self-closing, so they arrive as `Empty` events.
#[derive(Debug, Default)]
struct PlotAreaProps {
    /// `<c:barDir>`, exclusive to the bar family.
    bar_direction: Option<String>,
    /// `<c:grouping>`.
    grouping: Option<String>,
    /// `<c:ofPieType>`, exclusive to `<c:ofPieChart>`.
    of_pie_type: Option<String>,
    /// `<c:gapWidth>`, exclusive to the bar family. Office writes it after the
    /// last `</c:ser>`, so it lands here rather than on a series.
    gap_width: Option<String>,
    /// `<c:overlap>`, exclusive to the bar family, and likewise trailing.
    overlap: Option<String>,
    /// `<c:holeSize>`, exclusive to the doughnut family (issue #679).
    hole_size: Option<String>,
}

impl PlotAreaProps {
    /// Record `e` when it is one of the settings this struct carries.
    fn absorb(&mut self, e: &quick_xml::events::BytesStart) -> bool {
        match e.local_name().as_ref() {
            b"barDir" => self.bar_direction = xml_util::get_attr_str(e, b"val"),
            b"grouping" => self.grouping = xml_util::get_attr_str(e, b"val"),
            b"ofPieType" => self.of_pie_type = xml_util::get_attr_str(e, b"val"),
            b"gapWidth" => self.gap_width = xml_util::get_attr_str(e, b"val"),
            b"overlap" => self.overlap = xml_util::get_attr_str(e, b"val"),
            b"holeSize" => self.hole_size = xml_util::get_attr_str(e, b"val"),
            _ => return false,
        }
        true
    }

    /// The band layout this element declares, as far as it declares one.
    ///
    /// Nothing is substituted for an absent or unreadable element: the caller
    /// keeps looking, and only a chart that declared nothing anywhere falls
    /// back to [`BarBandLayout::default`].
    fn bar_band_layout(&self) -> (Option<f64>, Option<f64>) {
        (
            self.gap_width
                .as_deref()
                .and_then(|value| bar_percent(value, 0.0, 500.0)),
            self.overlap
                .as_deref()
                .and_then(|value| bar_percent(value, -100.0, 100.0)),
        )
    }
}

/// Read `<c:gapWidth>`'s or `<c:overlap>`'s `val`, held to the range its type
/// allows.
///
/// `ST_GapAmount` and `ST_Overlap` are each a union of a bare integer and a
/// percentage string, so `"90"` and `"90%"` describe the same chart. Office
/// enforces the ranges itself — PowerPoint 16.0 refuses to open a file whose
/// gapWidth reads 1000 while opening 500 happily — so a value outside them
/// describes no drawable chart and the nearest bound is the closest reading of
/// what it meant.
fn bar_percent(value: &str, low: f64, high: f64) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|percent| percent.is_finite())
        .map(|percent| percent.clamp(low, high))
}

/// Parse series data from within a chart type element (e.g., `<c:barChart>`).
fn parse_chart_series(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    categories: &mut Vec<String>,
    series: &mut Vec<ChartSeries>,
    plot: &mut PlotAreaProps,
    scheme: &SchemeColors<'_>,
) {
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"ser" {
                    let (ser, cats) = parse_single_series(reader, scheme);
                    // Use categories from first series that has them
                    if categories.is_empty() && !cats.is_empty() {
                        *categories = cats;
                    }
                    series.push(ser);
                } else {
                    plot.absorb(e);
                }
            }
            Ok(Event::Empty(ref e)) => {
                plot.absorb(e);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

/// Parse a single `<c:ser>` element and return the series data + category labels.
fn parse_single_series(
    reader: &mut Reader<&[u8]>,
    scheme: &SchemeColors<'_>,
) -> (ChartSeries, Vec<String>) {
    let mut name = None;
    let mut values = Vec::new();
    let mut categories = Vec::new();
    let mut fill: Option<Color> = None;
    let mut data_labels: DataLabels = DataLabels::default();
    // Keyed by `<c:idx>` because `<c:dPt>` entries are written only for the
    // points that override the series, and not necessarily in order.
    let mut point_fills: std::collections::BTreeMap<usize, Color> =
        std::collections::BTreeMap::new();
    let mut number_format: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"tx" => name = parse_series_text(reader),
                b"cat" => categories = parse_category_data(reader),
                b"val" | b"yVal" => {
                    let (parsed, format_code) = parse_value_data(reader);
                    values = parsed;
                    number_format = format_code;
                }
                // The series' own fill. This match is flat, so it would also
                // see a `<c:spPr>` nested inside a sibling element; every
                // element that can carry one is consumed by its own branch
                // first, leaving only the series-level fill here.
                b"spPr" => fill = fill.or(parse_solid_fill(reader, b"spPr", scheme)),
                b"dPt" => {
                    if let Some((index, color)) = parse_data_point(reader, scheme) {
                        point_fills.insert(index, color);
                    }
                }
                // Consumed whole: `<c:dLbls>` carries an `<c:spPr>` of its
                // own for the label box, which would otherwise be read as the
                // series fill.
                b"dLbls" => data_labels = parse_data_labels(reader),
                b"xVal" => {
                    // For scatter charts, xVal contains category-like data
                    if categories.is_empty() {
                        categories = parse_category_data(reader);
                    } else {
                        xml_util::skip_element(reader, b"xVal");
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"ser" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let point_count: usize = point_fills.keys().last().map_or(0, |last| last + 1);
    let point_fills: Vec<Option<Color>> = (0..point_count)
        .map(|index| point_fills.get(&index).copied())
        .collect();

    (
        ChartSeries {
            name,
            values,
            fill,
            point_fills,
            data_labels,
            number_format,
        },
        categories,
    )
}

/// Read a `<c:dLbls>` element, consuming it whole.
///
/// `CT_DLbls` is `dLbl*` followed by the group-level settings, and a per-point
/// `<c:dLbl>` repeats the same `showVal`/`showCatName`/… names. This loop
/// matches on local name alone, so each `<c:dLbl>` is skipped whole; reading
/// them would let a single point's override become the series default whenever
/// the group-level settings are absent.
fn parse_data_labels(reader: &mut Reader<&[u8]>) -> DataLabels {
    let mut labels = DataLabels::default();
    let mut in_separator: bool = false;
    let mut separator = String::new();

    loop {
        let event = reader.read_event();
        match event {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.local_name().as_ref() {
                b"dLbl" => xml_util::skip_element(reader, b"dLbl"),
                b"showVal" => labels.show_value = ct_boolean(e),
                b"showCatName" => labels.show_category = ct_boolean(e),
                b"showSerName" => labels.show_series = ct_boolean(e),
                b"showPercent" => labels.show_percent = ct_boolean(e),
                b"numFmt" => labels.number_format = explicit_format_code(e),
                b"separator" => in_separator = true,
                _ => {}
            },
            Ok(Event::Text(ref text)) if in_separator => {
                if let Ok(value) = text.xml_content() {
                    separator.push_str(value.as_ref());
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"separator" => in_separator = false,
                b"dLbls" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if !separator.is_empty() {
        labels.separator = separator;
    }
    labels
}

/// Parse a `<c:dPt>` into its `(point index, fill)`.
fn parse_data_point(
    reader: &mut Reader<&[u8]>,
    scheme: &SchemeColors<'_>,
) -> Option<(usize, Color)> {
    let mut index: Option<usize> = None;
    let mut fill: Option<Color> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"spPr" => {
                fill = fill.or(parse_solid_fill(reader, b"spPr", scheme));
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if e.local_name().as_ref() == b"idx" =>
            {
                index = xml_util::get_attr_str(e, b"val").and_then(|val| val.parse().ok());
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"dPt" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    Some((index?, fill?))
}

/// Read an `<a:ln>` nested anywhere up to `end_tag` into the stroke it states.
///
/// Returns `None` when the element carries no `<a:ln>` at all, which is the
/// difference between "use the automatic line" and "use this one".
fn parse_chart_line(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    scheme: &SchemeColors<'_>,
) -> ChartLine {
    let mut saw_line: bool = false;
    let mut suppressed: bool = false;
    let mut width_pt: Option<f64> = None;
    let mut color: Option<Color> = None;
    let mut in_line: bool = false;
    let mut in_solid_fill: bool = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if e.local_name().as_ref() == b"ln" =>
            {
                in_line = true;
                saw_line = true;
                width_pt = width_pt.or_else(|| {
                    xml_util::get_attr_str(e, b"w")
                        .and_then(|w| w.parse::<f64>().ok())
                        .map(|emu| emu / EMU_PER_POINT)
                });
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if in_line && e.local_name().as_ref() == b"noFill" =>
            {
                suppressed = true;
            }
            Ok(Event::Start(ref e)) if in_line && e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = true;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = false;
            }
            Ok(Event::Start(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                color = color.or(drawingml::parse_color_from_start(reader, e, scheme).color);
            }
            Ok(Event::Empty(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                color = color.or(drawingml::parse_color_from_empty(e, scheme).color);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"ln" => {
                in_line = false;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if !saw_line {
        ChartLine::Automatic
    } else if suppressed {
        ChartLine::Suppressed
    } else {
        ChartLine::Explicit { width_pt, color }
    }
}

/// Read the colour of a solid fill, consuming up to `end_tag`.
///
/// A chart part declares no theme of its own, so `<a:schemeClr>` resolves
/// against the theme of the document the graphic frame sits in, which the
/// caller supplies (issue #876).
fn parse_solid_fill(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    scheme: &SchemeColors<'_>,
) -> Option<Color> {
    let mut in_solid_fill: bool = false;
    let mut color: Option<Color> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = true;
            }
            Ok(Event::Start(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                let parsed = drawingml::parse_color_from_start(reader, e, scheme);
                color = color.or(parsed.color);
            }
            Ok(Event::Empty(ref e))
                if in_solid_fill
                    && matches!(e.local_name().as_ref(), b"srgbClr" | b"schemeClr") =>
            {
                color = color.or(drawingml::parse_color_from_empty(e, scheme).color);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = false;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == end_tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    color
}

/// Parse series name from `<c:tx>`.
fn parse_series_text(reader: &mut Reader<&[u8]>) -> Option<String> {
    let mut text = String::new();
    let mut in_v = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"v" {
                    in_v = true;
                }
            }
            Ok(Event::Text(ref t)) if in_v => {
                if let Ok(s) = t.xml_content() {
                    text.push_str(s.as_ref());
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"v" => in_v = false,
                b"tx" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Parse category labels from `<c:cat>` (either `<c:strRef>` or `<c:strLit>`).
fn parse_category_data(reader: &mut Reader<&[u8]>) -> Vec<String> {
    let mut categories = Vec::new();
    let mut in_v = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"v" {
                    in_v = true;
                }
            }
            Ok(Event::Text(ref t)) if in_v => {
                if let Ok(s) = t.xml_content() {
                    categories.push(s.as_ref().to_string());
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"v" => in_v = false,
                b"cat" | b"xVal" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    categories
}

/// Parse numeric values from `<c:val>` or `<c:yVal>`, with the cache's
/// `<c:formatCode>` when it states one.
///
/// `General` is Excel's "no format" and is reported as `None`: applying it
/// would only reformat the number the plain path already prints.
fn parse_value_data(reader: &mut Reader<&[u8]>) -> (Vec<f64>, Option<String>) {
    let mut values = Vec::new();
    let mut format_code: Option<String> = None;
    let mut in_v = false;
    let mut in_format_code = false;
    let mut current_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"v" => {
                    in_v = true;
                    current_text.clear();
                }
                b"formatCode" => {
                    in_format_code = true;
                    current_text.clear();
                }
                _ => {}
            },
            Ok(Event::Text(ref t)) if in_v || in_format_code => {
                if let Ok(s) = t.xml_content() {
                    current_text.push_str(s.as_ref());
                }
            }
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"v" => {
                    in_v = false;
                    if let Ok(v) = current_text.trim().parse::<f64>() {
                        values.push(v);
                    }
                }
                b"formatCode" => {
                    in_format_code = false;
                    let code = current_text.trim();
                    if !code.is_empty() && !code.eq_ignore_ascii_case("General") {
                        format_code = Some(code.to_string());
                    }
                }
                b"val" | b"yVal" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    (values, format_code)
}

/// Scan document.xml for chart relationship IDs.
///
/// Returns `(body_child_index, relationship_id)` tuples for each chart reference
/// found in drawing elements.
pub(crate) fn scan_chart_references(xml: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut reader = Reader::from_str(xml);

    let mut in_body = false;
    let mut body_child_index: usize = 0;
    let mut depth_in_body: u32 = 0;
    let mut in_graphic_data = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                let name = local.as_ref();

                if name == b"body" {
                    in_body = true;
                    depth_in_body = 0;
                    body_child_index = 0;
                    continue;
                }

                if in_body {
                    depth_in_body += 1;
                }

                if name == b"graphicData" {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"uri"
                            && let Ok(val) = attr.unescape_value()
                            && val.contains("chart")
                        {
                            in_graphic_data = true;
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                let name = local.as_ref();

                if in_body {
                    depth_in_body += 1;
                    // Empty elements open and close immediately
                    depth_in_body -= 1;
                }

                if in_graphic_data && name == b"chart" {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"id"
                            && let Ok(val) = attr.unescape_value()
                        {
                            results.push((body_child_index, val.to_string()));
                        }
                    }
                }

                // Empty graphicData can't contain a chart child element, skip
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                if name.as_ref() == b"body" {
                    in_body = false;
                } else if name.as_ref() == b"graphicData" {
                    in_graphic_data = false;
                } else if in_body && depth_in_body > 0 {
                    depth_in_body -= 1;
                    if depth_in_body == 0 {
                        body_child_index += 1;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    results
}

/// Scan `word/_rels/document.xml.rels` for chart relationship targets.
///
/// Returns a map from relationship ID to chart file path (e.g., "rId4" → "word/charts/chart1.xml").
pub(crate) fn scan_chart_rels(rels_xml: &str) -> std::collections::HashMap<String, String> {
    crate::parser::xml_util::parse_relationships(rels_xml)
        .into_iter()
        .filter(|entry| {
            entry
                .rel_type
                .as_deref()
                .is_some_and(|rel_type| rel_type.contains("chart"))
        })
        .map(|entry| {
            // Target is relative to word/ directory
            let full_path = if let Some(stripped) = entry.target.strip_prefix('/') {
                stripped.to_string()
            } else {
                format!("word/{}", entry.target)
            };
            (entry.id, full_path)
        })
        .collect()
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;

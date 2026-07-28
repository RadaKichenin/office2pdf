//! Chart XML parser for DOCX embedded charts.
//!
//! Parses chart*.xml files from DOCX ZIP archives and extracts chart type,
//! title, category labels, and series data into IR `Chart` structs.

use quick_xml::Reader;
use quick_xml::events::Event;

use super::xml_util;
use crate::ir::{Chart, ChartGrouping, ChartSeries, ChartType, Color, LegendPosition};

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
    (b"radarChart", "Radar Chart"),
    (b"doughnutChart", "Doughnut Chart"),
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
pub(crate) fn parse_chart_xml(xml: &str) -> Option<Chart> {
    let mut reader = Reader::from_str(xml);
    let mut chart_type = None;
    let mut title = None;
    let mut categories: Vec<String> = Vec::new();
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut grouping: Option<ChartGrouping> = None;
    let mut legend_position: Option<LegendPosition> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = e.local_name();
                let tag: &[u8] = local.as_ref();
                if tag == b"title" && title.is_none() {
                    title = parse_chart_title(&mut reader);
                } else if let Some(ct) = chart_type_for_tag(tag) {
                    let mut plot: PlotAreaProps = PlotAreaProps::default();
                    parse_chart_series(&mut reader, tag, &mut categories, &mut series, &mut plot);
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
                }
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"legendPos" => {
                legend_position = xml_util::get_attr_str(e, b"val")
                    .as_deref()
                    .map(legend_position_for);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    let chart_type = chart_type?;

    // Charts may omit <c:cat> entirely; Excel then labels the category axis
    // 1..N (the point count of the longest series).
    if categories.is_empty() {
        let point_count: usize = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        categories = (1..=point_count).map(|i| i.to_string()).collect();
    }

    Some(Chart {
        chart_type,
        title,
        categories,
        series,
        grouping: grouping.unwrap_or_default(),
        legend_position: legend_position.unwrap_or_default(),
    })
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
/// Office writes both self-closing, so they arrive as `Empty` events.
#[derive(Debug, Default)]
struct PlotAreaProps {
    /// `<c:barDir>`, exclusive to the bar family.
    bar_direction: Option<String>,
    /// `<c:grouping>`.
    grouping: Option<String>,
    /// `<c:ofPieType>`, exclusive to `<c:ofPieChart>`.
    of_pie_type: Option<String>,
}

impl PlotAreaProps {
    /// Record `e` when it is one of the settings this struct carries.
    fn absorb(&mut self, e: &quick_xml::events::BytesStart) -> bool {
        match e.local_name().as_ref() {
            b"barDir" => self.bar_direction = xml_util::get_attr_str(e, b"val"),
            b"grouping" => self.grouping = xml_util::get_attr_str(e, b"val"),
            b"ofPieType" => self.of_pie_type = xml_util::get_attr_str(e, b"val"),
            _ => return false,
        }
        true
    }
}

/// Parse series data from within a chart type element (e.g., `<c:barChart>`).
fn parse_chart_series(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    categories: &mut Vec<String>,
    series: &mut Vec<ChartSeries>,
    plot: &mut PlotAreaProps,
) {
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"ser" {
                    let (ser, cats) = parse_single_series(reader);
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
fn parse_single_series(reader: &mut Reader<&[u8]>) -> (ChartSeries, Vec<String>) {
    let mut name = None;
    let mut values = Vec::new();
    let mut categories = Vec::new();
    let mut fill: Option<Color> = None;
    // Keyed by `<c:idx>` because `<c:dPt>` entries are written only for the
    // points that override the series, and not necessarily in order.
    let mut point_fills: std::collections::BTreeMap<usize, Color> =
        std::collections::BTreeMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"tx" => name = parse_series_text(reader),
                b"cat" => categories = parse_category_data(reader),
                b"val" | b"yVal" => values = parse_value_data(reader),
                // The series' own fill. This match is flat, so it would also
                // see a `<c:spPr>` nested inside a sibling element; every
                // element that can carry one is consumed by its own branch
                // first, leaving only the series-level fill here.
                b"spPr" => fill = fill.or(parse_solid_fill(reader, b"spPr")),
                b"dPt" => {
                    if let Some((index, color)) = parse_data_point(reader) {
                        point_fills.insert(index, color);
                    }
                }
                // `<c:dLbls>` carries an `<c:spPr>` of its own for the label
                // box, which would otherwise be read as the series fill.
                b"dLbls" => xml_util::skip_element(reader, b"dLbls"),
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
        },
        categories,
    )
}

/// Parse a `<c:dPt>` into its `(point index, fill)`.
fn parse_data_point(reader: &mut Reader<&[u8]>) -> Option<(usize, Color)> {
    let mut index: Option<usize> = None;
    let mut fill: Option<Color> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"spPr" => {
                fill = fill.or(parse_solid_fill(reader, b"spPr"));
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

/// Read the `<a:srgbClr>` of a solid fill, consuming up to `end_tag`.
///
/// Theme colours (`<a:schemeClr>`) need the chart part's own theme, which the
/// parser does not resolve, so they fall through to the palette.
fn parse_solid_fill(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Option<Color> {
    let mut in_solid_fill: bool = false;
    let mut color: Option<Color> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"solidFill" => {
                in_solid_fill = true;
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                if in_solid_fill && e.local_name().as_ref() == b"srgbClr" =>
            {
                color = color.or_else(|| {
                    xml_util::get_attr_str(e, b"val")
                        .as_deref()
                        .and_then(xml_util::parse_hex_color)
                });
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

/// Parse numeric values from `<c:val>` or `<c:yVal>`.
fn parse_value_data(reader: &mut Reader<&[u8]>) -> Vec<f64> {
    let mut values = Vec::new();
    let mut in_v = false;
    let mut current_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.local_name().as_ref() == b"v" {
                    in_v = true;
                    current_text.clear();
                }
            }
            Ok(Event::Text(ref t)) if in_v => {
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
                b"val" | b"yVal" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    values
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

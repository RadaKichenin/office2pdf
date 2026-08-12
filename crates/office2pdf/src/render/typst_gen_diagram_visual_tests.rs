use super::*;
use crate::ir::ChartAreaOutline;
use crate::ir::DataLabels;
use crate::render::typst_gen::diagrams::{
    CHART_AREA_OUTLINE, CHART_AUTOMATIC_LINE, CHART_DEFAULT_TEXT_PT, GAP, LABEL_W, LEGEND_ENTRY_W,
    LEGEND_KEY_LEN_PT, PPTX_LEGEND_KEY_EM, PPTX_LEGEND_KEY_LABEL_GAP_EM,
    PPTX_LEGEND_KEY_LABEL_GAP_PT, ROW, SERIES_LINE_PT, TICK_GAP, axis_plot_rect,
    chart_category_band_pt, chart_category_gutter_pt, chart_tick_band_pt,
};

#[test]
fn test_codegen_chart_bar_visual_bars() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Bar,
        hole_size_percent: None,
        title: Some("Sales Report".to_string()),
        categories: vec!["Q1".to_string(), "Q2".to_string()],
        series: vec![ChartSeries {
            name: Some("Revenue".to_string()),
            values: vec![100.0, 250.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("Sales Report"),
        "Expected chart title, got:\n{}",
        output.source
    );
    // Axis-scaled bar chart: series-name area title, rect bars, tick labels,
    // and gridlines (no raw "Bar Chart" placeholder or bordered box).
    assert!(
        output.source.contains("Revenue"),
        "Expected series-name area title, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("rect(width:"),
        "Expected axis-scaled bar rects, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("line(end:"),
        "Expected axis gridlines, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Q1"),
        "Expected category label, got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_chart_axis_ticks_and_no_raw_floats() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Bar,
        hole_size_percent: None,
        title: Some("My Bar Chart".to_string()),
        categories: vec!["1st Qtr".to_string(), "2nd Qtr".to_string()],
        series: vec![ChartSeries {
            name: Some("Sales".to_string()),
            values: vec![8.200000000000001, 3.2],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();
    // Bars carry no in-plot value labels (like PowerPoint), so the raw float
    // never reaches the output.
    assert!(
        !output.source.contains("8.200000000000001"),
        "raw float must not leak; got:\n{}",
        output.source
    );
    // Nice axis for max 8.2 → ticks 0,1,…,9.
    for tick in ["[0]", "[1]", "[9]"] {
        assert!(
            output.source.contains(tick),
            "expected axis tick {tick}; got:\n{}",
            output.source
        );
    }
}

#[test]
fn test_codegen_chart_pie_draws_a_pie() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Pie,
        hole_size_percent: None,
        title: Some("Market Share".to_string()),
        categories: vec!["A".to_string(), "B".to_string()],
        series: vec![ChartSeries {
            name: None,
            values: vec![60.0, 40.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();

    // A pie is a pie, not the `Slice | Value | %` table it used to be (#533).
    assert!(
        output.source.contains("Market Share"),
        "Expected chart title, got:\n{}",
        output.source
    );
    assert_eq!(
        output.source.matches("path(fill:").count(),
        2,
        "one wedge per slice, got:\n{}",
        output.source
    );
    for category in ["A", "B"] {
        assert!(
            output.source.contains(category),
            "Expected {category} in the legend, got:\n{}",
            output.source
        );
    }
}

#[test]
fn test_codegen_chart_line_trend_indicators() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Line,
        hole_size_percent: None,
        title: Some("Trends".to_string()),
        categories: vec!["Jan".to_string(), "Feb".to_string(), "Mar".to_string()],
        series: vec![ChartSeries {
            name: Some("Sales".to_string()),
            values: vec![10.0, 20.0, 15.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();
    // Multi-point line charts now render as an axis-scaled polyline plot
    // (not a trend-indicator table).
    assert!(
        output.source.contains("Trends"),
        "Expected chart title, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("path(stroke:"),
        "Expected polyline path for the line chart, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Sales"),
        "Expected series name in legend, got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_chart_empty_series() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Line,
        hole_size_percent: None,
        title: Some("Empty".to_string()),
        categories: vec![],
        series: vec![],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("Line Chart"),
        "Expected line chart label, got:\n{}",
        output.source
    );
}

/// Render `doc` and return the visible text of each PDF page separately.
fn page_texts(doc: &Document) -> Vec<String> {
    let pdf: Vec<u8> = crate::render_document(doc).unwrap();
    pdf_extract::extract_text_from_mem_by_pages(&pdf).unwrap()
}

/// Fill most of a page so the chart that follows cannot fit in what is left.
fn page_filler(lines: usize) -> Vec<Block> {
    (1..=lines)
        .map(|line| {
            make_paragraph(&format!(
                "Line {line} of the quarterly commentary preceding the chart."
            ))
        })
        .collect()
}

/// Report the index of the single page carrying `marker`, or panic with the
/// page breakdown when it is missing or duplicated.
fn page_holding(pages: &[String], marker: &str) -> usize {
    let hits: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter(|(_, text)| text.contains(marker))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected {marker:?} on exactly one page, found it on {hits:?}; pages:\n{pages:#?}"
    );
    hits[0]
}

#[test]
fn an_axis_chart_that_does_not_fit_moves_to_the_next_page_whole() {
    let mut content: Vec<Block> = page_filler(30);
    content.push(Block::Chart(Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: Some("Quarterly Units Shipped".to_string()),
        categories: vec![
            "Northlake".to_string(),
            "Eastport".to_string(),
            "Southgate".to_string(),
        ],
        series: vec![ChartSeries {
            name: Some("Units".to_string()),
            values: vec![23334.0, 8331.0, 2727.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }));
    let doc = make_doc(vec![make_flow_page(content)]);

    let pages = page_texts(&doc);

    // Excel treats a chart as one floating graphic: the title and the plot it
    // labels never land on opposite sides of a page break.
    assert_eq!(
        page_holding(&pages, "Quarterly Units Shipped"),
        page_holding(&pages, "Southgate"),
        "chart title and category labels split across pages; pages:\n{pages:#?}"
    );
}

#[test]
fn a_bordered_chart_box_that_does_not_fit_moves_to_the_next_page_whole() {
    // The pie fallback draws a bordered box; a breakable one closes with a
    // bottom border at the page end and re-opens with a fresh top border, so
    // one chart reads as two.
    let mut content: Vec<Block> = page_filler(30);
    content.push(Block::Chart(Chart {
        chart_type: ChartType::Pie,
        hole_size_percent: None,
        title: Some("Fixture Documents by Format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![ChartSeries {
            name: None,
            values: vec![115.0, 92.0, 138.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }));
    let doc = make_doc(vec![make_flow_page(content)]);

    let pages = page_texts(&doc);

    assert_eq!(
        page_holding(&pages, "Fixture Documents by Format"),
        page_holding(&pages, "XLSX"),
        "chart box split across pages; pages:\n{pages:#?}"
    );
}

fn sa_node(text: &str, depth: usize) -> SmartArtNode {
    SmartArtNode {
        text: text.to_string(),
        depth,
    }
}

#[test]
fn test_smartart_codegen_flat_numbered_steps() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![FixedElement {
            x: 72.0,
            y: 100.0,
            width: 400.0,
            height: 300.0,
            kind: FixedElementKind::SmartArt(SmartArt {
                items: vec![
                    sa_node("Step 1", 0),
                    sa_node("Step 2", 0),
                    sa_node("Step 3", 0),
                ],
            }),
        }],
    )]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("stroke:"),
        "Expected bordered box, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("SmartArt Diagram"),
        "Expected SmartArt header, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Step 1"),
        "Expected Step 1, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Step 2"),
        "Expected Step 2, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Step 3"),
        "Expected Step 3, got:\n{}",
        output.source
    );
}

#[test]
fn test_smartart_codegen_hierarchy_indented_tree() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![FixedElement {
            x: 72.0,
            y: 100.0,
            width: 400.0,
            height: 300.0,
            kind: FixedElementKind::SmartArt(SmartArt {
                items: vec![
                    sa_node("CEO", 0),
                    sa_node("VP Engineering", 1),
                    sa_node("VP Sales", 1),
                    sa_node("Dev Lead", 2),
                ],
            }),
        }],
    )]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("CEO"),
        "Expected CEO, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("pad"),
        "Expected indented items for hierarchy, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("VP Engineering"),
        "Expected VP Engineering, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("Dev Lead"),
        "Expected Dev Lead, got:\n{}",
        output.source
    );
}

#[test]
fn test_smartart_codegen_empty_items() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![FixedElement {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
            kind: FixedElementKind::SmartArt(SmartArt { items: vec![] }),
        }],
    )]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("SmartArt Diagram"),
        "Expected SmartArt header even for empty SmartArt"
    );
}

#[test]
fn test_smartart_codegen_special_chars() {
    let doc = make_doc(vec![make_fixed_page(
        720.0,
        540.0,
        vec![FixedElement {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
            kind: FixedElementKind::SmartArt(SmartArt {
                items: vec![sa_node("Item #1", 0), sa_node("Price $10", 0)],
            }),
        }],
    )]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains(r"\#"),
        "Expected escaped #, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains(r"\$"),
        "Expected escaped $, got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_chart_line_plot() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Line,
        hole_size_percent: None,
        title: None,
        categories: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        series: vec![
            ChartSeries {
                name: Some("A".to_string()),
                values: vec![1.0, 2.0, 3.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("B".to_string()),
                values: vec![10.0, 9.0, 14.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
        ],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("path(stroke:"),
        "line chart must draw polyline paths; got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("line(end:"),
        "line chart must draw axis gridlines; got:\n{}",
        output.source
    );
    // Both series names appear in the legend.
    assert!(output.source.contains("[A]") && output.source.contains("[B]"));
    // Category labels 1..3 present.
    assert!(output.source.contains("[1]") && output.source.contains("[3]"));
}

#[test]
fn a_chart_too_tall_for_a_page_still_breaks_rather_than_overflowing() {
    // Keeping an over-tall chart atomic does not move it to the next page —
    // Typst runs it off the page edge and the overflow is never drawn. Such a
    // chart stays breakable so every row survives.
    let categories: Vec<String> = (1..=60).map(|i| format!("Category{i:03}")).collect();
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Scatter,
        hole_size_percent: None,
        title: Some("Sixty Sample Sites".to_string()),
        categories: categories.clone(),
        series: vec![ChartSeries {
            name: Some("Reading".to_string()),
            values: (1..=60).map(|value| value as f64).collect(),
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let pages = page_texts(&doc);

    assert!(
        pages.len() > 1,
        "a 60-row chart cannot fit one page; it must break instead of overflowing: {pages:#?}"
    );
    for category in [&categories[0], &categories[59]] {
        assert!(
            pages.iter().any(|page| page.contains(category)),
            "{category} was dropped; pages:\n{pages:#?}"
        );
    }
}

// ----- Stacked grouping (issue #545) -----

/// The introduction deck's slide 17 chart: three formats, four support areas
/// stacked per format. Stack totals are 9, 9, and 6.
///
/// The band layout is the one that slide's `<c:barChart>` declares, so the
/// geometry these tests read is the geometry the fixture asks for.
fn stacked_support_chart(grouping: ChartGrouping) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: Some("Supported elements by format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![
            ChartSeries {
                name: Some("Text".to_string()),
                values: vec![4.0, 2.0, 2.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("Tables".to_string()),
                values: vec![1.0, 1.0, 1.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("Graphics".to_string()),
                values: vec![2.0, 4.0, 0.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("Structure".to_string()),
                values: vec![2.0, 2.0, 3.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
        ],
        grouping,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout {
            gap_width_percent: 90.0,
            overlap_percent: 100.0,
        },
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

fn chart_source(chart: Chart) -> String {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(chart)])]);
    generate_typst(&doc).unwrap().source
}

fn framed_chart_source(chart: &Chart, width: f64, height: f64) -> String {
    let mut source = String::new();
    generate_chart_in(&mut source, chart, Some((width, height)));
    source
}

/// The axis tick labels the generator emitted, in the order written.
fn emitted_axis_ticks(source: &str) -> Vec<f64> {
    emitted_axis_ticks_at_size(source, CHART_DEFAULT_TEXT_PT)
}

fn emitted_axis_ticks_at_size(source: &str, size_pt: f64) -> Vec<f64> {
    let marker: String = format!("text(size: {}pt)[", format_f64(size_pt));
    source
        .lines()
        .filter(|line| line.contains("#place") && line.contains(&marker))
        .filter_map(|line| {
            let after = line.rsplit_once(marker.as_str())?.1;
            after.split_once(']')?.0.parse::<f64>().ok()
        })
        .collect()
}

#[test]
fn a_stacked_column_scales_its_axis_to_the_stack_total() {
    // Rendering a stacked chart clustered does not merely look different, it
    // reports different numbers: the axis topped out at the largest single
    // segment (4) instead of the largest stack (9), so no bar could be read as
    // its category's total (#545).
    let ticks = emitted_axis_ticks(&chart_source(stacked_support_chart(ChartGrouping::Stacked)));

    let axis_max: f64 = ticks.iter().copied().fold(0.0, f64::max);
    assert!(
        axis_max >= 9.0,
        "the axis must reach the tallest stack of 9, got {axis_max} from {ticks:?}"
    );
}

#[test]
fn a_clustered_column_still_scales_to_the_largest_segment() {
    // Control: the same data clustered keeps today's axis, so the stacked
    // branch cannot be a blanket change to axis scaling.
    let ticks = emitted_axis_ticks(&chart_source(stacked_support_chart(
        ChartGrouping::Clustered,
    )));

    let axis_max: f64 = ticks.iter().copied().fold(0.0, f64::max);
    assert!(
        (4.0..9.0).contains(&axis_max),
        "a clustered axis covers the largest segment of 4, got {axis_max} from {ticks:?}"
    );
}

#[test]
fn a_stacked_column_draws_one_bar_per_category() {
    // Four series over three categories: clustered draws 12 rects, stacked
    // draws 12 segments too, but they share three x positions instead of
    // spreading across twelve. It is the deck's `<c:overlap val="100"/>` that
    // puts them on one x — grouping alone does not, as
    // `a_stacked_category_divides_its_band_by_the_same_law_a_clustered_one_does`
    // pins against PowerPoint.
    let source = chart_source(stacked_support_chart(ChartGrouping::Stacked));
    let x_positions: std::collections::BTreeSet<String> = source
        .lines()
        .filter(|line| line.contains("rect(width:"))
        .filter_map(|line| {
            let after = line.split_once("dx: ")?.1;
            Some(after.split_once("pt")?.0.to_string())
        })
        .collect();

    assert_eq!(
        x_positions.len(),
        3,
        "a stacked column puts every series at its category's x, got {x_positions:?}"
    );
}

#[test]
fn a_percent_stacked_column_normalises_every_stack() {
    // XLSX totals 6 against DOCX's 9, but both fill the plot completely.
    let source = chart_source(stacked_support_chart(ChartGrouping::PercentStacked));
    let ticks = emitted_axis_ticks(&source);

    let axis_max: f64 = ticks.iter().copied().fold(0.0, f64::max);
    assert!(
        (axis_max - 100.0).abs() < f64::EPSILON,
        "a percent-stacked axis runs to 100, got {axis_max} from {ticks:?}"
    );
}

// ----- Legend position (issue #546) -----

fn legend_chart(position: LegendPosition) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: Some("Supported elements by format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![
            ChartSeries {
                name: Some("Text".to_string()),
                values: vec![4.0, 2.0, 2.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("Tables".to_string()),
                values: vec![1.0, 1.0, 1.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
        ],
        grouping: ChartGrouping::Stacked,
        legend_position: position,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

/// The `(x, y)` of every legend entry the generator placed, in emit order.
fn emitted_legend_entries(source: &str) -> Vec<(f64, f64)> {
    source
        .lines()
        .filter(|line| line.contains("box[#box(width: 9pt, height: 9pt"))
        .filter_map(|line| {
            let x = line
                .split_once("dx: ")?
                .1
                .split_once("pt")?
                .0
                .parse()
                .ok()?;
            let y = line
                .split_once("dy: ")?
                .1
                .split_once("pt")?
                .0
                .parse()
                .ok()?;
            Some((x, y))
        })
        .collect()
}

#[test]
fn a_bottom_legend_lays_its_entries_out_side_by_side() {
    // A legend runs along the edge it sits on, so `val="b"` must spread the
    // entries left to right instead of stacking them (#546).
    let entries = emitted_legend_entries(&chart_source(legend_chart(LegendPosition::Bottom)));

    assert_eq!(
        entries.len(),
        2,
        "expected one entry per series: {entries:?}"
    );
    assert!(
        entries[1].0 > entries[0].0,
        "entries must advance across the page: {entries:?}"
    );
    assert!(
        (entries[0].1 - entries[1].1).abs() < 0.01,
        "entries must share one row: {entries:?}"
    );
}

#[test]
fn a_right_legend_still_stacks_its_entries() {
    // Control: the default position keeps the vertical stack, so the bottom
    // branch cannot be a blanket change to legend layout.
    let entries = emitted_legend_entries(&chart_source(legend_chart(LegendPosition::Right)));

    assert_eq!(entries.len(), 2);
    assert!(
        (entries[0].0 - entries[1].0).abs() < 0.01,
        "entries must share one column: {entries:?}"
    );
    assert!(
        entries[1].1 > entries[0].1,
        "entries must advance down the page: {entries:?}"
    );
}

#[test]
fn a_bottom_legend_leaves_the_plot_the_full_frame_width() {
    // The right-hand legend stole about 84pt of plot width from a chart that
    // asked for the legend underneath.
    let bottom = chart_source(legend_chart(LegendPosition::Bottom));
    let right = chart_source(legend_chart(LegendPosition::Right));

    let width_of = |source: &str| -> f64 {
        source
            .lines()
            .find(|line| line.starts_with("#box(width:"))
            .and_then(|line| {
                line.split_once("width: ")?
                    .1
                    .split_once("pt")?
                    .0
                    .parse()
                    .ok()
            })
            .expect("a plot box is emitted")
    };

    assert!(
        width_of(&bottom) < width_of(&right),
        "a bottom legend must not reserve a column beside the plot: {} vs {}",
        width_of(&bottom),
        width_of(&right)
    );
}

#[test]
fn a_left_legend_shifts_the_plot_clear_of_it() {
    // The plot must move right by what the legend reserves, or the two overlap.
    let left = chart_source(legend_chart(LegendPosition::Left));
    let entries = emitted_legend_entries(&left);
    let first_bar_x: f64 = left
        .lines()
        .find(|line| line.contains("rect(width:"))
        .and_then(|line| line.split_once("dx: ")?.1.split_once("pt")?.0.parse().ok())
        .expect("a bar is emitted");

    assert!(
        entries[0].0 < first_bar_x,
        "a left legend sits clear of the plot: legend at {}, first bar at {first_bar_x}",
        entries[0].0
    );
}

// ----- Declared series and point fills (issue #535) -----

#[test]
fn a_declared_series_fill_reaches_the_bars() {
    // The palette's first entry is rgb(68, 114, 196); the file says 4F81BD.
    let chart = Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: Some("Production LOC by layer".to_string()),
        categories: vec!["parser".to_string(), "render".to_string()],
        series: vec![ChartSeries {
            name: Some("LOC".to_string()),
            values: vec![23334.0, 8331.0],
            fill: Some(Color::new(0x4f, 0x81, 0xbd)),
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };

    let source = chart_source(chart);

    assert!(
        source.contains("rgb(79, 129, 189)"),
        "the declared 4F81BD must reach the bars, got:\n{source}"
    );
    assert!(
        !source.contains("rgb(68, 114, 196)"),
        "the palette must not override a declared fill, got:\n{source}"
    );
}

#[test]
fn a_series_without_a_fill_still_takes_the_palette() {
    // Control: the palette remains the fallback, so this is not a blanket
    // change to how charts are coloured.
    let chart = Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: None,
        categories: vec!["parser".to_string(), "render".to_string()],
        series: vec![ChartSeries {
            name: Some("LOC".to_string()),
            values: vec![23334.0, 8331.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };

    let source = chart_source(chart);

    assert!(
        source.contains("rgb(68, 114, 196)"),
        "an undeclared series keeps the palette, got:\n{source}"
    );
}

#[test]
fn per_point_fills_colour_each_bar_separately() {
    let chart = Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: None,
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![ChartSeries {
            name: Some("Fixtures".to_string()),
            values: vec![115.0, 92.0, 138.0],
            fill: Some(Color::new(0x11, 0x11, 0x11)),
            point_fills: vec![
                Some(Color::new(0x4f, 0x81, 0xbd)),
                Some(Color::new(0xc0, 0x50, 0x4d)),
                Some(Color::new(0x9b, 0xbb, 0x59)),
            ],
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };

    let source = chart_source(chart);

    for expected in ["rgb(79, 129, 189)", "rgb(192, 80, 77)", "rgb(155, 187, 89)"] {
        assert!(
            source.contains(expected),
            "each point paints its own fill; {expected} missing from:\n{source}"
        );
    }
}

// ----- Axis titles (issue #552) -----

fn axis_titled_chart(category: Option<&str>, value: Option<&str>) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: Some("Production LOC by layer".to_string()),
        categories: vec!["parser".to_string(), "render".to_string()],
        series: vec![ChartSeries {
            name: Some("LOC".to_string()),
            values: vec![23334.0, 8331.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: category.map(str::to_string),
        value_axis_title: value.map(str::to_string),
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

#[test]
fn axis_titles_are_drawn() {
    let source = chart_source(axis_titled_chart(Some("계층"), Some("LOC")));

    assert!(
        source.contains("계층"),
        "the category axis title must be drawn: {source}"
    );
    assert!(
        source.contains("rotate(-90deg"),
        "the value axis title runs down the left edge: {source}"
    );
}

#[test]
fn an_untitled_axis_reserves_no_band() {
    // Control: a chart with no axis titles keeps its old geometry, so the
    // gutters are spent only when there is something to put in them.
    let titled = chart_source(axis_titled_chart(Some("계층"), Some("LOC")));
    let untitled = chart_source(axis_titled_chart(None, None));

    let box_width = |source: &str| -> f64 {
        source
            .lines()
            .find(|line| line.starts_with("#box(width:"))
            .and_then(|line| {
                line.split_once("width: ")?
                    .1
                    .split_once("pt")?
                    .0
                    .parse()
                    .ok()
            })
            .expect("a plot box is emitted")
    };

    assert!(
        box_width(&titled) > box_width(&untitled),
        "the value axis title widens the box: {} vs {}",
        box_width(&titled),
        box_width(&untitled)
    );
    assert!(
        !untitled.contains("rotate(-90deg"),
        "nothing is rotated when no axis is titled: {untitled}"
    );
}

#[test]
fn each_axis_title_is_independent() {
    let value_only = chart_source(axis_titled_chart(None, Some("LOC")));

    assert!(value_only.contains("rotate(-90deg"));
    assert!(
        !value_only.contains("계층"),
        "an untitled category axis draws nothing: {value_only}"
    );
}

// ----- Data labels (issue #547) -----

fn labelled_chart(labels: DataLabels) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: None,
        categories: vec!["DOCX".to_string(), "PPTX".to_string()],
        series: vec![ChartSeries {
            name: Some("Text".to_string()),
            values: vec![4.0, 2.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: labels,
            number_format: None,
        }],
        grouping: ChartGrouping::Stacked,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

#[test]
fn show_val_prints_one_label_per_point() {
    let source = chart_source(labelled_chart(DataLabels {
        show_value: true,
        ..DataLabels::default()
    }));
    let labels = source.matches("weight: \"bold\", fill: white").count();

    assert_eq!(labels, 2, "one label per plotted point, got:\n{source}");
}

#[test]
fn a_series_without_dlbls_draws_no_labels() {
    // Control: the label pass is driven by the file, not switched on for all.
    let source = chart_source(labelled_chart(DataLabels::default()));

    assert!(
        !source.contains("weight: \"bold\", fill: white"),
        "no labels without dLbls, got:\n{source}"
    );
}

#[test]
fn the_enabled_parts_are_joined_by_the_separator() {
    let source = chart_source(labelled_chart(DataLabels {
        show_value: true,
        show_category: true,
        show_series: true,
        separator: "; ".to_string(),
        position: crate::ir::DataLabelPosition::Center,
        position_stated: false,
        ..DataLabels::default()
    }));

    assert!(
        source.contains("Text; DOCX; 4"),
        "series, category, then value, joined by the separator, got:\n{source}"
    );
}

#[test]
fn percent_labels_are_a_share_of_the_category() {
    let source = chart_source(labelled_chart(DataLabels {
        show_percent: true,
        ..DataLabels::default()
    }));

    // A lone series takes the whole category.
    assert!(
        source.contains("100%"),
        "the only series in a category is all of it, got:\n{source}"
    );
}

// ----- Pie geometry (issue #533) -----

fn pie_chart(values: Vec<f64>) -> Chart {
    Chart {
        chart_type: ChartType::Pie,
        hole_size_percent: None,
        title: Some("Fixture documents by format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![ChartSeries {
            name: None,
            values,
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

#[test]
fn a_pie_chart_draws_wedges_not_a_table() {
    let source = chart_source(pie_chart(vec![115.0, 92.0, 138.0]));

    assert_eq!(
        source.matches("path(fill:").count(),
        3,
        "one wedge per slice, got:\n{source}"
    );
    assert!(
        !source.contains("Pie Chart"),
        "the type-label fallback is gone, got:\n{source}"
    );
}

#[test]
fn a_pie_skips_slices_with_no_value() {
    // A zero slice has no wedge to draw, but keeps its legend entry.
    let source = chart_source(pie_chart(vec![115.0, 0.0, 138.0]));

    assert_eq!(source.matches("path(fill:").count(), 2);
    assert!(source.contains("PPTX"), "the legend still lists it");
}

#[test]
fn an_empty_pie_falls_back_to_the_table() {
    // Control: with nothing to apportion there is no pie, so the data table
    // still carries the categories.
    let source = chart_source(pie_chart(vec![0.0, 0.0, 0.0]));

    assert!(
        !source.contains("path(fill:"),
        "no wedges without values, got:\n{source}"
    );
    assert!(source.contains("Pie Chart"), "the fallback still runs");
}

#[test]
fn the_first_wedge_starts_at_twelve_oclock() {
    // Office sweeps clockwise from the top; the first arc vertex is therefore
    // directly above the centre.
    let source = chart_source(pie_chart(vec![115.0, 92.0, 138.0]));
    let first_path: &str = source
        .lines()
        .find(|line| line.contains("path(fill:"))
        .expect("a wedge is drawn");

    // `closed: true, (cx, cy), ((cx, cy - r), …` — the centre, then the top.
    let after_centre: &str = first_path.split_once("closed: true, (").unwrap().1;
    let (centre, rest) = after_centre.split_once("), ((").unwrap();
    let centre: Vec<f64> = centre
        .split(", ")
        .map(|value| value.trim_end_matches("pt").parse().unwrap())
        .collect();
    let start: Vec<f64> = rest
        .split_once(')')
        .unwrap()
        .0
        .split(", ")
        .map(|value| value.trim_end_matches("pt").parse().unwrap())
        .collect();

    assert!(
        (start[0] - centre[0]).abs() < 0.01,
        "the first vertex sits directly above the centre: {start:?} vs {centre:?}"
    );
    assert!(
        start[1] < centre[1],
        "and above it, not below: {start:?} vs {centre:?}"
    );
}

#[test]
fn wedge_colours_follow_the_declared_data_point_fills() {
    let mut chart = pie_chart(vec![115.0, 92.0, 138.0]);
    chart.series[0].point_fills = vec![
        Some(Color::new(0x4f, 0x81, 0xbd)),
        Some(Color::new(0xc0, 0x50, 0x4d)),
        Some(Color::new(0x9b, 0xbb, 0x59)),
    ];

    let source = chart_source(chart);

    for expected in ["rgb(79, 129, 189)", "rgb(192, 80, 77)", "rgb(155, 187, 89)"] {
        assert!(
            source.contains(expected),
            "wedge colour {expected} missing from:\n{source}"
        );
    }
}

// ----- Pie data labels (issue #570) -----

#[test]
fn a_pie_draws_a_label_on_each_wedge() {
    let mut chart = pie_chart(vec![115.0, 92.0, 138.0]);
    chart.series[0].data_labels = DataLabels {
        show_value: true,
        show_category: true,
        show_percent: true,
        separator: "; ".to_string(),
        position: crate::ir::DataLabelPosition::Center,
        position_stated: false,
        ..DataLabels::default()
    };

    let source = chart_source(chart);

    assert_eq!(
        source.matches("weight: \"bold\", fill: white").count(),
        3,
        "one label per wedge, got:\n{source}"
    );
    assert!(
        source.contains("DOCX; 115; 33%"),
        "category, value and share, joined by the separator, got:\n{source}"
    );
}

#[test]
fn a_pie_without_dlbls_draws_no_wedge_labels() {
    // Control: the labels are driven by the file, as on the axis plot.
    let source = chart_source(pie_chart(vec![115.0, 92.0, 138.0]));

    assert!(
        !source.contains("weight: \"bold\", fill: white"),
        "no labels without dLbls, got:\n{source}"
    );
}

#[test]
fn a_zero_slice_carries_no_label() {
    let mut chart = pie_chart(vec![115.0, 0.0, 138.0]);
    chart.series[0].data_labels = DataLabels {
        show_value: true,
        ..DataLabels::default()
    };

    let source = chart_source(chart);

    assert_eq!(
        source.matches("weight: \"bold\", fill: white").count(),
        2,
        "a slice with no wedge has nothing to label, got:\n{source}"
    );
}

/// An automatic major gridline is PowerPoint's 0.75pt `#868686`, not a lighter
/// hairline.
///
/// `c:majorGridlines` with no `c:spPr` leaves both sides drawing their own
/// default. Ours was 0.6pt `#C8C8C8`, which puts roughly a quarter of the ink
/// on each line and leaves the grid barely visible against a white plot area
/// (issue #673).
#[test]
fn test_chart_default_gridline_matches_powerpoint() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Bar,
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string(), "Q2".to_string()],
        series: vec![ChartSeries {
            name: Some("Revenue".to_string()),
            values: vec![100.0, 250.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    })])]);

    let source = generate_typst(&doc).unwrap().source;
    assert!(
        source.contains("stroke: 0.75pt + rgb(134, 134, 134)"),
        "gridlines should be PowerPoint's 0.75pt #868686, got:\n{source}"
    );
    assert!(
        !source.contains("rgb(200, 200, 200)"),
        "the old #C8C8C8 gridline default must be gone, got:\n{source}"
    );
    assert!(
        !source.contains("rgb(120, 120, 120)"),
        "the old #787878 axis-line default must be gone, got:\n{source}"
    );
}

// ----- Axis lines and major tick marks (issue #672) -----

/// One `line(...)` the generator placed: its top-left corner and the offset of
/// its far end, in points.
#[derive(Debug, Clone, Copy)]
struct PlacedLine {
    dx: f64,
    dy: f64,
    end_x: f64,
    end_y: f64,
}

/// Read the point measurement a slice starts with: `"12.5pt, …"` becomes 12.5.
fn leading_pt(text: &str) -> Option<f64> {
    text.split_once("pt")?.0.trim().parse::<f64>().ok()
}

/// Every plot segment the source places, in the order written.
///
/// Only the chart's chrome counts: gridlines, axis lines and tick marks all
/// take `CHART_AUTOMATIC_LINE`. A line series' legend key is also a `line`, but
/// it is drawn in the series colour at the series weight and is not plot
/// geometry, so counting it made every tick census see one tick too many
/// (#801).
///
/// The chart-area outline carries the same stroke (#637) and so passes the
/// substring filter, but it is a `box`, not a `line`, and the `line(end: (`
/// parse below drops it. Both conditions are load-bearing — neither alone
/// selects the plot segments.
fn emitted_lines(source: &str) -> Vec<PlacedLine> {
    source
        .lines()
        .filter(|line| line.contains(CHART_AUTOMATIC_LINE))
        .filter_map(|line| {
            let (placement, end) = line.split_once("line(end: (")?;
            Some(PlacedLine {
                dx: leading_pt(placement.split_once("dx: ")?.1)?,
                dy: leading_pt(placement.split_once("dy: ")?.1)?,
                end_x: leading_pt(end)?,
                end_y: leading_pt(end.split_once(", ")?.1)?,
            })
        })
        .collect()
}

/// Whether two point measurements are the same length.
fn same_length(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-6
}

/// The plotting rectangle, as `(x, y, width, height)`, read off the segments
/// the chart drew rather than off the generator's layout constants: the
/// gridlines and both axis lines each run a whole side of the plot, so the
/// longest horizontal and vertical segments give its extents and the shorter
/// tick marks fall out.
fn plot_rect(lines: &[PlacedLine]) -> (f64, f64, f64, f64) {
    let width: f64 = lines.iter().map(|line| line.end_x).fold(0.0, f64::max);
    let height: f64 = lines.iter().map(|line| line.end_y).fold(0.0, f64::max);
    let x: f64 = lines
        .iter()
        .filter(|line| same_length(line.end_x, width))
        .map(|line| line.dx)
        .fold(f64::INFINITY, f64::min);
    let y: f64 = lines
        .iter()
        .filter(|line| same_length(line.end_y, height))
        .map(|line| line.dy)
        .fold(f64::INFINITY, f64::min);
    (x, y, width, height)
}

/// The tick marks crossing the axis line under the plot and the one down its
/// left edge: every segment too short to be a gridline or an axis line.
///
/// Which axis owns which edge depends on the orientation, so the split is by
/// edge. A column chart's bottom edge is its category axis; a bar chart's is
/// its value axis.
fn tick_marks_by_edge(
    lines: &[PlacedLine],
    plot: (f64, f64, f64, f64),
) -> (Vec<PlacedLine>, Vec<PlacedLine>) {
    let (_, _, width, height) = plot;
    let under: Vec<PlacedLine> = lines
        .iter()
        .filter(|line| same_length(line.end_x, 0.0) && line.end_y < height)
        .copied()
        .collect();
    let beside: Vec<PlacedLine> = lines
        .iter()
        .filter(|line| same_length(line.end_y, 0.0) && line.end_x < width)
        .copied()
        .collect();
    (under, beside)
}

/// The categories `tick_mark_chart` plots, so a test can look their labels up.
const TICK_MARK_CATEGORIES: [&str; 3] = ["Mon", "Tue", "Wed"];

/// A three-category chart carrying the tick marks each axis asks for.
fn tick_mark_chart(
    chart_type: ChartType,
    category_axis_major_tick_mark: AxisTickMark,
    value_axis_major_tick_mark: AxisTickMark,
) -> Chart {
    Chart {
        chart_type,
        hole_size_percent: None,
        title: Some("Weekly Throughput".to_string()),
        categories: TICK_MARK_CATEGORIES.map(str::to_string).to_vec(),
        series: vec![ChartSeries {
            name: Some("Builds".to_string()),
            values: vec![4.0, 8.0, 6.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark,
        value_axis_major_tick_mark,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

/// One placed `box(...)`: its top-left corner and the extent it was given, in
/// points. A box the generator left unsized vertically reports zero height.
#[derive(Debug, Clone, Copy)]
struct PlacedBox {
    dx: f64,
    dy: f64,
    width: f64,
    height: f64,
}

/// Where the generator placed the box printing `text`.
fn placed_box_holding(source: &str, text: &str) -> PlacedBox {
    let needle: String = format!("[{text}]");
    let line: &str = source
        .lines()
        .find(|line| line.contains("box(width: ") && line.contains(&needle))
        .unwrap_or_else(|| panic!("nothing prints {text} in:\n{source}"));
    let read = |prefix: &str| -> Option<f64> { leading_pt(line.split_once(prefix)?.1) };
    PlacedBox {
        dx: read("dx: ").expect("a placed box carries a dx"),
        dy: read("dy: ").expect("a placed box carries a dy"),
        width: read("box(width: ").expect("a placed box carries a width"),
        height: read("height: ").unwrap_or(0.0),
    }
}

/// The offsets in ascending order, with the coincident ones folded together —
/// the zero gridline and the axis line beside it are drawn as two segments over
/// the same offset.
fn sorted_unique(offsets: impl IntoIterator<Item = f64>) -> Vec<f64> {
    let mut sorted: Vec<f64> = offsets.into_iter().collect();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup_by(|left, right| same_length(*left, *right));
    sorted
}

/// Where along the value axis each gridline runs, one per major unit.
///
/// A column or line chart draws them horizontally across the whole plot, a bar
/// chart vertically down its whole height.
fn gridline_offsets(
    lines: &[PlacedLine],
    plot: (f64, f64, f64, f64),
    value_axis_runs_under_the_plot: bool,
) -> Vec<f64> {
    let (_, _, width, height) = plot;
    sorted_unique(lines.iter().filter_map(|line| {
        if value_axis_runs_under_the_plot {
            (same_length(line.end_x, 0.0) && same_length(line.end_y, height)).then_some(line.dx)
        } else {
            (same_length(line.end_y, 0.0) && same_length(line.end_x, width)).then_some(line.dy)
        }
    }))
}

/// Where along its axis each tick sits. A tick runs across its axis, so the
/// coordinate that stays put over the tick's own length is the one that places
/// it along the axis — whichever side of the line the tick reaches from.
fn tick_offsets(ticks: &[PlacedLine], along_the_bottom_edge: bool) -> Vec<f64> {
    sorted_unique(ticks.iter().map(|tick| {
        if along_the_bottom_edge {
            tick.dx
        } else {
            tick.dy
        }
    }))
}

/// A value tick marks the same major unit its gridline does, so the two land on
/// the same offset along the axis. Counting ticks alone would pass an
/// implementation that drew the right number of them in the wrong places.
fn assert_value_ticks_sit_on_their_gridlines(source: &str, value_axis_runs_under_the_plot: bool) {
    let lines: Vec<PlacedLine> = emitted_lines(source);
    let plot: (f64, f64, f64, f64) = plot_rect(&lines);
    let (under, beside) = tick_marks_by_edge(&lines, plot);
    let value_ticks: &[PlacedLine] = if value_axis_runs_under_the_plot {
        &under
    } else {
        &beside
    };

    let gridlines: Vec<f64> = gridline_offsets(&lines, plot, value_axis_runs_under_the_plot);
    let ticks: Vec<f64> = tick_offsets(value_ticks, value_axis_runs_under_the_plot);

    assert!(!gridlines.is_empty(), "no gridlines drawn in:\n{source}");
    assert_eq!(
        ticks.len(),
        gridlines.len(),
        "one value tick per gridline; ticks at {ticks:?} against gridlines at {gridlines:?}\n{source}"
    );
    for (tick, gridline) in ticks.iter().zip(&gridlines) {
        assert!(
            same_length(*tick, *gridline),
            "a value tick at {tick} misses its gridline at {gridline}; ticks {ticks:?} against gridlines {gridlines:?}\n{source}"
        );
    }
}

/// The category ticks are the boundaries of the bands the labels sit in the
/// middle of: evenly spaced along the axis, with every label's centre exactly
/// midway between two neighbouring ticks.
///
/// This is what `<c:crossBetween val="between"/>` means, and it is what pins the
/// ticks to the layout: ticks placed by a rule of their own can still come out
/// evenly spaced and correctly counted while sitting nowhere near a label.
fn assert_category_ticks_bound_the_labels(
    source: &str,
    categories: &[&str],
    value_axis_runs_under_the_plot: bool,
) {
    let lines: Vec<PlacedLine> = emitted_lines(source);
    let plot: (f64, f64, f64, f64) = plot_rect(&lines);
    let (under, beside) = tick_marks_by_edge(&lines, plot);
    // A bar chart's categories run down the left edge; every other orientation
    // lays them along the bottom.
    let category_axis_is_the_bottom_edge: bool = !value_axis_runs_under_the_plot;
    let category_ticks: &[PlacedLine] = if category_axis_is_the_bottom_edge {
        &under
    } else {
        &beside
    };

    let boundaries: Vec<f64> = tick_offsets(category_ticks, category_axis_is_the_bottom_edge);
    assert_eq!(
        boundaries.len(),
        categories.len() + 1,
        "one tick per band boundary, so one more than the categories; got {boundaries:?}\n{source}"
    );
    let pitch: f64 = boundaries[1] - boundaries[0];
    for pair in boundaries.windows(2) {
        assert!(
            same_length(pair[1] - pair[0], pitch),
            "the bands the ticks bound must all be the same width; got {boundaries:?}\n{source}"
        );
    }

    let band_centres: Vec<f64> = boundaries
        .windows(2)
        .map(|pair| (pair[0] + pair[1]) / 2.0)
        .collect();
    let label_centres: Vec<f64> = sorted_unique(categories.iter().map(|category| {
        let label: PlacedBox = placed_box_holding(source, category);
        if category_axis_is_the_bottom_edge {
            label.dx + label.width / 2.0
        } else {
            label.dy + label.height / 2.0
        }
    }));
    assert_eq!(label_centres.len(), band_centres.len());
    for (label, band) in label_centres.iter().zip(&band_centres) {
        assert!(
            same_length(*label, *band),
            "a category label centred on {label} is not in the middle of a band; labels {label_centres:?} against bands bounded by {boundaries:?}\n{source}"
        );
    }
}

/// Both sides of the plot carry an axis line. The value axis was never stroked
/// for a bar or a column chart, whichever edge it owned (issue #672).
fn assert_both_axis_lines(source: &str) {
    let lines: Vec<PlacedLine> = emitted_lines(source);
    let (plot_x, plot_y, plot_w, plot_h) = plot_rect(&lines);

    assert!(
        lines.iter().any(|line| same_length(line.dx, plot_x)
            && same_length(line.dy, plot_y)
            && same_length(line.end_x, 0.0)
            && same_length(line.end_y, plot_h)),
        "no axis line down the plot's left edge at x={plot_x}, y={plot_y}..{}; got:\n{source}",
        plot_y + plot_h
    );
    assert!(
        lines.iter().any(|line| same_length(line.dx, plot_x)
            && same_length(line.dy, plot_y + plot_h)
            && same_length(line.end_x, plot_w)
            && same_length(line.end_y, 0.0)),
        "no axis line along the plot's bottom edge at y={}, x={plot_x}..{}; got:\n{source}",
        plot_y + plot_h,
        plot_x + plot_w
    );
}

#[test]
fn a_column_chart_strokes_both_of_its_axis_lines() {
    assert_both_axis_lines(&chart_source(tick_mark_chart(
        ChartType::Column,
        AxisTickMark::Outside,
        AxisTickMark::Outside,
    )));
}

#[test]
fn a_horizontal_bar_chart_strokes_both_of_its_axis_lines() {
    // Triangulation: the orientation swaps which axis owns which edge, so one
    // hardcoded edge cannot satisfy both charts.
    assert_both_axis_lines(&chart_source(tick_mark_chart(
        ChartType::Bar,
        AxisTickMark::Outside,
        AxisTickMark::Outside,
    )));
}

#[test]
fn a_line_chart_strokes_both_of_its_axis_lines() {
    assert_both_axis_lines(&chart_source(tick_mark_chart(
        ChartType::Line,
        AxisTickMark::Outside,
        AxisTickMark::Outside,
    )));
}

/// Each axis ticks every major unit, and the category axis ticks every band
/// boundary — `<c:crossBetween val="between"/>` gives three categories four of
/// them, as Excel and PowerPoint both draw.
fn assert_tick_counts(source: &str, value_axis_runs_under_the_plot: bool) {
    let lines: Vec<PlacedLine> = emitted_lines(source);
    let plot: (f64, f64, f64, f64) = plot_rect(&lines);
    let (under, beside) = tick_marks_by_edge(&lines, plot);

    let category_boundaries: usize = TICK_MARK_CATEGORIES.len() + 1;
    let major_units: usize = emitted_axis_ticks(source).len();
    assert_eq!(major_units, 10, "values 4/8/6 scale to ticks 0..9 by 1");
    let (expected_under, expected_beside) = if value_axis_runs_under_the_plot {
        (major_units, category_boundaries)
    } else {
        (category_boundaries, major_units)
    };

    assert_eq!(
        under.len(),
        expected_under,
        "tick marks under the plot: {under:#?}\n{source}"
    );
    assert_eq!(
        beside.len(),
        expected_beside,
        "tick marks left of the plot: {beside:#?}\n{source}"
    );
}

#[test]
fn a_column_chart_ticks_every_major_unit_and_every_category_boundary() {
    // A column chart's value axis runs down the left edge, so its major-unit
    // ticks are the ones beside the plot.
    assert_tick_counts(
        &chart_source(tick_mark_chart(
            ChartType::Column,
            AxisTickMark::Outside,
            AxisTickMark::Outside,
        )),
        false,
    );
}

#[test]
fn a_horizontal_bar_chart_ticks_the_edges_the_other_way_round() {
    assert_tick_counts(
        &chart_source(tick_mark_chart(
            ChartType::Bar,
            AxisTickMark::Outside,
            AxisTickMark::Outside,
        )),
        true,
    );
}

#[test]
fn a_line_chart_ticks_both_of_its_axes() {
    assert_tick_counts(
        &chart_source(tick_mark_chart(
            ChartType::Line,
            AxisTickMark::Outside,
            AxisTickMark::Outside,
        )),
        false,
    );
}

/// A chart's ticks land on the geometry the same chart drew, on both axes.
fn assert_ticks_match_the_plot(chart_type: ChartType, value_axis_runs_under_the_plot: bool) {
    let source: String = chart_source(tick_mark_chart(
        chart_type,
        AxisTickMark::Outside,
        AxisTickMark::Outside,
    ));
    assert_value_ticks_sit_on_their_gridlines(&source, value_axis_runs_under_the_plot);
    assert_category_ticks_bound_the_labels(
        &source,
        &TICK_MARK_CATEGORIES,
        value_axis_runs_under_the_plot,
    );
}

#[test]
fn a_column_chart_puts_every_tick_on_the_geometry_it_marks() {
    assert_ticks_match_the_plot(ChartType::Column, false);
}

#[test]
fn a_horizontal_bar_chart_puts_every_tick_on_the_geometry_it_marks() {
    assert_ticks_match_the_plot(ChartType::Bar, true);
}

#[test]
fn a_line_chart_puts_every_tick_on_the_geometry_it_marks() {
    // The line plot lays its categories out in bands of its own, so its ticks
    // have to be read off that layout rather than borrowed from the bar family.
    assert_ticks_match_the_plot(ChartType::Line, false);
}

#[test]
fn an_axis_asking_for_no_tick_marks_gets_none() {
    // Triangulation against drawing ticks unconditionally, and against reading
    // one axis' setting for both: only the category axis goes quiet here.
    let source: String = chart_source(tick_mark_chart(
        ChartType::Column,
        AxisTickMark::None,
        AxisTickMark::Outside,
    ));
    let lines: Vec<PlacedLine> = emitted_lines(&source);
    let plot: (f64, f64, f64, f64) = plot_rect(&lines);
    let (under, beside) = tick_marks_by_edge(&lines, plot);

    assert!(
        under.is_empty(),
        "a category axis asking for no tick marks must draw none, got {under:#?}\n{source}"
    );
    assert!(
        !beside.is_empty(),
        "the value axis still asked for tick marks, got:\n{source}"
    );
}

#[test]
fn inward_tick_marks_reach_into_the_plot_and_crossing_ones_both_ways() {
    // `in` and `out` mirror each other about the axis line and `cross` is
    // both, so the mode has to steer the geometry rather than only decide
    // whether a segment is drawn at all.
    let left_edge_ticks = |mark: AxisTickMark| -> (f64, Vec<PlacedLine>) {
        let source: String =
            chart_source(tick_mark_chart(ChartType::Column, AxisTickMark::None, mark));
        let lines: Vec<PlacedLine> = emitted_lines(&source);
        let plot: (f64, f64, f64, f64) = plot_rect(&lines);
        (plot.0, tick_marks_by_edge(&lines, plot).1)
    };

    let (axis_x, outward) = left_edge_ticks(AxisTickMark::Outside);
    let (_, inward) = left_edge_ticks(AxisTickMark::Inside);
    let (_, crossing) = left_edge_ticks(AxisTickMark::Cross);

    assert!(!outward.is_empty() && !inward.is_empty() && !crossing.is_empty());
    assert!(
        outward
            .iter()
            .all(|tick| tick.dx < axis_x && same_length(tick.dx + tick.end_x, axis_x)),
        "an outward tick ends on the axis line at x={axis_x}, got {outward:#?}"
    );
    assert!(
        inward
            .iter()
            .all(|tick| same_length(tick.dx, axis_x) && tick.end_x > 0.0),
        "an inward tick starts on the axis line at x={axis_x}, got {inward:#?}"
    );
    assert!(
        crossing
            .iter()
            .all(|tick| tick.dx < axis_x && tick.dx + tick.end_x > axis_x),
        "a crossing tick straddles the axis line at x={axis_x}, got {crossing:#?}"
    );
    assert_eq!(
        outward.len(),
        crossing.len(),
        "every mode ticks the same major units"
    );
    assert!(
        crossing[0].end_x > outward[0].end_x,
        "a crossing tick is longer than a one-sided one: {crossing:#?} vs {outward:#?}"
    );
}

/// A column chart with one of its axes switched off by `<c:delete val="1"/>`,
/// both still asking for outward ticks — which is what Office leaves behind
/// when a user unticks an axis rather than setting its tick marks to `none`.
fn chart_with_deleted_axis(category_deleted: bool, value_deleted: bool) -> Chart {
    let mut chart: Chart = tick_mark_chart(
        ChartType::Column,
        AxisTickMark::Outside,
        AxisTickMark::Outside,
    );
    chart.category_axis_deleted = category_deleted;
    chart.value_axis_deleted = value_deleted;
    chart
}

#[test]
fn a_deleted_value_axis_draws_no_line_no_ticks_and_no_labels() {
    let drawn: String = chart_source(chart_with_deleted_axis(false, false));
    let hidden: String = chart_source(chart_with_deleted_axis(false, true));
    // The gutters do not move when an axis goes, so the plot the deleted chart
    // draws into is the one the drawn chart reports.
    let plot: (f64, f64, f64, f64) = plot_rect(&emitted_lines(&drawn));
    let (plot_x, plot_y, _, plot_h) = plot;
    let lines: Vec<PlacedLine> = emitted_lines(&hidden);
    let (under, beside) = tick_marks_by_edge(&lines, plot);

    assert!(
        !lines.iter().any(|line| same_length(line.dx, plot_x)
            && same_length(line.dy, plot_y)
            && same_length(line.end_y, plot_h)),
        "a deleted value axis must not stroke the left edge it owns; got:\n{hidden}"
    );
    assert!(
        beside.is_empty(),
        "a deleted value axis must not tick, whatever `<c:majorTickMark>` still says; got {beside:#?}\n{hidden}"
    );
    assert!(
        emitted_axis_ticks(&hidden).is_empty(),
        "a deleted value axis must not label its units; got:\n{hidden}"
    );
    // Gridlines are a chart element of their own — deleting the axis leaves
    // them standing — and the category axis is untouched.
    assert_eq!(
        gridline_offsets(&lines, plot, false),
        gridline_offsets(&emitted_lines(&drawn), plot, false),
        "the gridlines belong to the chart, not to the axis switched off"
    );
    assert_eq!(
        under.len(),
        TICK_MARK_CATEGORIES.len() + 1,
        "the category axis still ticks every band boundary; got {under:#?}\n{hidden}"
    );
}

#[test]
fn a_deleted_category_axis_takes_only_its_own_furniture_with_it() {
    // Triangulation against one flag standing for both axes, and against the
    // deletion reaching further than the axis it names.
    let drawn: String = chart_source(chart_with_deleted_axis(false, false));
    let hidden: String = chart_source(chart_with_deleted_axis(true, false));
    let plot: (f64, f64, f64, f64) = plot_rect(&emitted_lines(&drawn));
    let (plot_x, plot_y, plot_w, plot_h) = plot;
    let lines: Vec<PlacedLine> = emitted_lines(&hidden);
    let (under, beside) = tick_marks_by_edge(&lines, plot);

    // The zero gridline runs along the bottom edge too, so the axis line there
    // is one of two coincident segments rather than the only one.
    let bottom_edge_strokes = |source: &str| -> usize {
        emitted_lines(source)
            .iter()
            .filter(|line| same_length(line.dy, plot_y + plot_h) && same_length(line.end_x, plot_w))
            .count()
    };
    assert_eq!(
        bottom_edge_strokes(&hidden),
        bottom_edge_strokes(&drawn) - 1,
        "a deleted category axis must stop stroking the bottom edge it owns; got:\n{hidden}"
    );
    assert!(
        under.is_empty(),
        "a deleted category axis must not tick; got {under:#?}\n{hidden}"
    );
    for category in TICK_MARK_CATEGORIES {
        assert!(
            !hidden.contains(&format!("[{category}]")),
            "a deleted category axis must not label its bands, found {category} in:\n{hidden}"
        );
    }
    assert!(
        lines.iter().any(|line| same_length(line.dx, plot_x)
            && same_length(line.dy, plot_y)
            && same_length(line.end_y, plot_h)),
        "the value axis is still drawn; got:\n{hidden}"
    );
    assert_eq!(
        beside.len(),
        emitted_axis_ticks(&hidden).len(),
        "the value axis still ticks every unit it labels; got {beside:#?}\n{hidden}"
    );
}

// ----- Bar thickness from c:gapWidth and c:overlap (issue #671) -----

/// One `rect(...)` the generator placed: its top-left corner and the extent it
/// was given, in points.
#[derive(Debug, Clone, Copy)]
struct PlacedRect {
    dx: f64,
    dy: f64,
    width: f64,
    height: f64,
}

/// Every rectangle the source places, in the order written. A bar or column
/// chart draws nothing else as a rectangle, so these are exactly its bars.
fn emitted_rects(source: &str) -> Vec<PlacedRect> {
    source
        .lines()
        .filter_map(|line| {
            let (placement, extent) = line.split_once("rect(width: ")?;
            Some(PlacedRect {
                dx: leading_pt(placement.split_once("dx: ")?.1)?,
                dy: leading_pt(placement.split_once("dy: ")?.1)?,
                width: leading_pt(extent)?,
                height: leading_pt(extent.split_once("height: ")?.1)?,
            })
        })
        .collect()
}

/// Each bar as `(start, thickness)` along the category axis — the horizontal
/// axis for a column chart, the vertical one for a horizontal bar chart.
///
/// The generator writes one category at a time, every series within it, so the
/// first `series_count` entries share the first band.
fn bars_across_the_categories(source: &str, horizontal: bool) -> Vec<(f64, f64)> {
    emitted_rects(source)
        .into_iter()
        .map(|rect| {
            if horizontal {
                (rect.dy, rect.height)
            } else {
                (rect.dx, rect.width)
            }
        })
        .collect()
}

/// Where the first category's band starts along the category axis, read off the
/// plotting rectangle the gridlines and axis lines describe.
///
/// A column chart lays its categories out left to right from the plot's left
/// edge; a horizontal bar chart stacks them bottom-up, so its first band is the
/// last one down the plot.
fn first_band_start(source: &str, horizontal: bool, categories: usize) -> f64 {
    let (plot_x, plot_y, _, plot_h) = plot_rect(&emitted_lines(source));
    if horizontal {
        plot_y + plot_h - plot_h / categories as f64
    } else {
        plot_x
    }
}

/// The three categories every band-layout test plots, with a value per series.
const BAND_SERIES_VALUES: [[f64; 3]; 4] = [
    [4.0, 2.0, 2.0],
    [1.0, 3.0, 1.0],
    [2.0, 4.0, 3.0],
    [2.0, 2.0, 3.0],
];

/// A chart of `series_count` series over three categories, declaring `layout`.
fn band_layout_chart(
    chart_type: ChartType,
    grouping: ChartGrouping,
    series_count: usize,
    layout: BarBandLayout,
) -> Chart {
    Chart {
        chart_type,
        hole_size_percent: None,
        title: Some("Weekly Throughput".to_string()),
        categories: vec!["Mon".to_string(), "Tue".to_string(), "Wed".to_string()],
        series: BAND_SERIES_VALUES
            .iter()
            .take(series_count)
            .enumerate()
            .map(|(index, values)| ChartSeries {
                name: Some(format!("Line {index}")),
                values: values.to_vec(),
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            })
            .collect(),
        grouping,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: layout,
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

/// A single-series chart's band pitch and bar thickness along the category
/// axis, in points.
fn pitch_and_thickness(source: &str, horizontal: bool) -> (f64, f64) {
    let bars: Vec<(f64, f64)> = bars_across_the_categories(source, horizontal);
    assert_eq!(bars.len(), 3, "one bar per category expected, got {bars:?}");
    let pitch: f64 = (bars[1].0 - bars[0].0).abs();
    assert!(
        same_length(pitch, (bars[2].0 - bars[1].0).abs()),
        "the categories must keep an even pitch, got {bars:?}"
    );
    (pitch, bars[0].1)
}

#[test]
fn a_single_series_bar_leaves_the_gutter_its_gap_width_asks_for() {
    // `<c:gapWidth>` measures the gutter between neighbouring categories in
    // units of ONE bar, so the band holds the bar plus that fraction of it.
    // Rewriting the element in `tests/fixtures/pptx/bar-chart.pptx` and tracing
    // PowerPoint 16.0's own export put every bar within one 1/1200in device
    // quantum of band / (1 + gapWidth/100), over the whole 0..500 range, while
    // the band itself never moved.
    for gap_width_percent in [0.0, 20.0, 50.0, 100.0, 150.0, 300.0, 500.0] {
        let source: String = chart_source(band_layout_chart(
            ChartType::Column,
            ChartGrouping::Clustered,
            1,
            BarBandLayout {
                gap_width_percent,
                overlap_percent: 0.0,
            },
        ));

        let (pitch, thickness) = pitch_and_thickness(&source, false);
        let expected: f64 = pitch / (1.0 + gap_width_percent / 100.0);
        assert!(
            same_length(thickness, expected),
            "gapWidth {gap_width_percent} wants a {expected}pt bar in a {pitch}pt band, got {thickness}pt"
        );
    }
}

#[test]
fn a_horizontal_bar_chart_sizes_its_bars_the_same_way() {
    // The gap is a property of the category axis, not of the page, so turning
    // the chart on its side must not change the ratio.
    for gap_width_percent in [0.0, 90.0, 219.0, 500.0] {
        let source: String = chart_source(band_layout_chart(
            ChartType::Bar,
            ChartGrouping::Clustered,
            1,
            BarBandLayout {
                gap_width_percent,
                overlap_percent: 0.0,
            },
        ));

        let (pitch, thickness) = pitch_and_thickness(&source, true);
        let expected: f64 = pitch / (1.0 + gap_width_percent / 100.0);
        assert!(
            same_length(thickness, expected),
            "gapWidth {gap_width_percent} wants a {expected}pt bar in a {pitch}pt band, got {thickness}pt"
        );
    }
}

#[test]
fn a_chart_declaring_no_gap_width_draws_the_office_default() {
    // Excel 16.0 renders `tests/fixtures/xlsx/chart_sheet.xlsx`, which declares
    // neither element, at gapWidth 150 — so an absent declaration has to reach
    // the bars as 150, leaving each bar 1/2.5 of its band.
    let source: String = chart_source(band_layout_chart(
        ChartType::Column,
        ChartGrouping::Clustered,
        1,
        BarBandLayout::default(),
    ));

    let (pitch, thickness) = pitch_and_thickness(&source, false);
    assert!(
        same_length(thickness, pitch / 2.5),
        "the default gap leaves a {}pt bar in a {pitch}pt band, got {thickness}pt",
        pitch / 2.5
    );
}

#[test]
fn every_bar_sits_centred_in_the_band_its_category_owns() {
    // PowerPoint splits the gutter evenly on both sides of the bar rather than
    // pushing it against one edge: on `tests/fixtures/pptx/bar-chart.pptx` the
    // traced bar centres sat within 0.02pt of their band centres.
    for (chart_type, horizontal) in [(ChartType::Column, false), (ChartType::Bar, true)] {
        let source: String = chart_source(band_layout_chart(
            chart_type.clone(),
            ChartGrouping::Clustered,
            1,
            BarBandLayout {
                gap_width_percent: 100.0,
                overlap_percent: 0.0,
            },
        ));

        let (pitch, thickness) = pitch_and_thickness(&source, horizontal);
        let bars: Vec<(f64, f64)> = bars_across_the_categories(&source, horizontal);
        let lead: f64 = bars[0].0 - first_band_start(&source, horizontal, 3);
        assert!(
            same_length(lead, (pitch - thickness) / 2.0),
            "{chart_type:?} must centre its bar: a {thickness}pt bar in a {pitch}pt band wants a {}pt lead, got {lead}pt",
            (pitch - thickness) / 2.0
        );
    }
}

#[test]
fn clustered_series_slide_over_each_other_by_the_declared_overlap() {
    // `<c:overlap>` moves each series' bar a fraction of a bar over the one
    // before it, so N series need N - (N-1)*overlap bars of room plus the gap.
    // Excel 16.0 draws `tests/fixtures/xlsx/any_sheets.xlsx` (219 / -27, two
    // series) as 52.5pt bars stepping 66.7pt in a 234pt band: 234/4.46 and
    // 52.47*1.27. Sweeping the sign of the overlap across five shapes leaves no
    // single ratio that could pass.
    for (gap_width_percent, overlap_percent, series_count) in [
        (219.0, -27.0, 2),
        (150.0, 0.0, 2),
        (100.0, 50.0, 2),
        (219.0, -27.0, 3),
        (90.0, 100.0, 4),
    ] {
        let source: String = chart_source(band_layout_chart(
            ChartType::Column,
            ChartGrouping::Clustered,
            series_count,
            BarBandLayout {
                gap_width_percent,
                overlap_percent,
            },
        ));

        let bars: Vec<(f64, f64)> = bars_across_the_categories(&source, false);
        assert_eq!(
            bars.len(),
            3 * series_count,
            "one bar per series per category"
        );
        let pitch: f64 = bars[series_count].0 - bars[0].0;
        let bars_wide: f64 = series_count as f64;
        let expected: f64 = pitch
            / (bars_wide - (bars_wide - 1.0) * overlap_percent / 100.0 + gap_width_percent / 100.0);
        assert!(
            same_length(bars[0].1, expected),
            "{series_count} series at {gap_width_percent}/{overlap_percent} want a {expected}pt bar in a {pitch}pt band, got {}pt",
            bars[0].1
        );

        let step: f64 = bars[1].0 - bars[0].0;
        let expected_step: f64 = expected * (1.0 - overlap_percent / 100.0);
        assert!(
            same_length(step, expected_step),
            "an overlap of {overlap_percent} steps {expected_step}pt from one series to the next, got {step}pt"
        );

        let cluster: f64 = expected + (bars_wide - 1.0) * expected_step;
        let lead: f64 = bars[0].0 - first_band_start(&source, false, 3);
        assert!(
            same_length(lead, (pitch - cluster) / 2.0),
            "the {cluster}pt cluster sits centred in its {pitch}pt band, got a lead of {lead}pt"
        );
    }
}

#[test]
fn a_stacked_category_divides_its_band_by_the_same_law_a_clustered_one_does() {
    // Stacking does not fuse the segments into one bar: `<c:overlap>` still says
    // how far each slides over the one before it. Rewriting the element on the
    // introduction deck's four-series stacked chart (gapWidth 90) and tracing
    // PowerPoint 16.0's export gave, on a 167.6pt pitch, one 88.2pt column at
    // overlap 100 (167.64/1.9) but four 34.2pt segments stepping 34.2pt at
    // overlap 0 (167.52/4.9) — a staircase, each segment still stacked on the
    // running total. Overlap 50 gave 49.3pt stepping 24.7pt and -25 gave 29.6pt
    // stepping 37.1pt. Deleting `<c:overlap>` drew the overlap-0 geometry
    // exactly, so an absent element is 0, not the 100 Office writes beside its
    // own stacked charts.
    for grouping in [ChartGrouping::Stacked, ChartGrouping::PercentStacked] {
        for overlap_percent in [100.0, 50.0, 0.0, -25.0] {
            for gap_width_percent in [90.0, 300.0] {
                let source: String = chart_source(band_layout_chart(
                    ChartType::Column,
                    grouping,
                    4,
                    BarBandLayout {
                        gap_width_percent,
                        overlap_percent,
                    },
                ));

                let bars: Vec<(f64, f64)> = bars_across_the_categories(&source, false);
                assert_eq!(bars.len(), 12, "four segments over three categories");
                let pitch: f64 = bars[4].0 - bars[0].0;
                let overlap: f64 = overlap_percent / 100.0;
                let expected: f64 = pitch / (4.0 - 3.0 * overlap + gap_width_percent / 100.0);
                let expected_step: f64 = expected * (1.0 - overlap);
                for (index, segment) in bars[..4].iter().enumerate() {
                    assert!(
                        same_length(segment.1, expected)
                            && same_length(segment.0, bars[0].0 + index as f64 * expected_step),
                        "{grouping:?} at {gap_width_percent}/{overlap_percent} wants {expected}pt segments stepping {expected_step}pt, got {bars:?}"
                    );
                }

                let cluster: f64 = expected + 3.0 * expected_step;
                let lead: f64 = bars[0].0 - first_band_start(&source, false, 3);
                assert!(
                    same_length(lead, (pitch - cluster) / 2.0),
                    "the {cluster}pt stack sits centred in its {pitch}pt band, got a lead of {lead}pt"
                );
            }
        }
    }
}

/// The Office 2007 accents both audited fixtures declare (issue #670).
fn office_2007_accents() -> Vec<crate::ir::Color> {
    vec![
        crate::ir::Color::new(0x4F, 0x81, 0xBD),
        crate::ir::Color::new(0xC0, 0x50, 0x4D),
        crate::ir::Color::new(0x9B, 0xBB, 0x59),
        crate::ir::Color::new(0x80, 0x64, 0xA2),
        crate::ir::Color::new(0x4B, 0xAC, 0xC6),
        crate::ir::Color::new(0xF7, 0x96, 0x46),
    ]
}

fn two_series_bar_chart(theme_accent_colors: Vec<crate::ir::Color>) -> Chart {
    Chart {
        chart_type: ChartType::Bar,
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string()],
        series: vec![
            ChartSeries {
                name: Some("Revenue".to_string()),
                values: vec![100.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
            ChartSeries {
                name: Some("Cost".to_string()),
                values: vec![60.0],
                fill: None,
                point_fills: Vec::new(),
                data_labels: DataLabels::default(),
                number_format: None,
            },
        ],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors,
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    }
}

#[test]
fn test_automatic_series_colors_come_from_the_file_theme() {
    let source = chart_source(two_series_bar_chart(office_2007_accents()));

    assert!(
        source.contains("rgb(79, 129, 189)"),
        "series 1 must take the theme's accent1, got:\n{source}"
    );
    assert!(
        source.contains("rgb(192, 80, 77)"),
        "series 2 must take the theme's accent2, got:\n{source}"
    );
    assert!(
        !source.contains("rgb(68, 114, 196)"),
        "the built-in 2013+ accent1 must not appear when the file names its own, got:\n{source}"
    );
}

#[test]
fn test_automatic_series_colors_keep_the_builtin_palette_without_a_theme() {
    // Triangulation: a file that supplies no accents still renders, on the
    // built-in palette rather than on nothing.
    let source = chart_source(two_series_bar_chart(Vec::new()));

    assert!(
        source.contains("rgb(68, 114, 196)"),
        "the built-in palette stands in when the package names no accents, got:\n{source}"
    );
}

#[test]
fn test_explicit_series_fill_still_outranks_the_theme() {
    // Triangulation, and the guarantee #535 established: a fill the file
    // states wins over any automatic colour.
    let mut chart = two_series_bar_chart(office_2007_accents());
    chart.series[0].fill = Some(crate::ir::Color::new(0x11, 0x22, 0x33));
    let source = chart_source(chart);

    assert!(
        source.contains("rgb(17, 34, 51)"),
        "the declared fill must survive, got:\n{source}"
    );
    assert!(
        source.contains("rgb(192, 80, 77)"),
        "the series that declares none still takes accent2, got:\n{source}"
    );
}

#[test]
fn test_line_series_markers_cycle_by_series_index() {
    // `c:marker val="1"` with no `c:symbol` means "the default marker for this
    // series index", and the point of the sequence is that adjacent series stay
    // apart in monochrome. Drawing one square for every series defeats that
    // (issue #635).
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Line;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![100.0, 120.0];
    chart.series[1].values = vec![60.0, 80.0];
    let source = chart_source(chart);

    assert!(
        source.contains("polygon("),
        "a cycled marker set needs shapes beyond `rect`, got:\n{source}"
    );
    // Series 1 and series 2 must not draw the same marker.
    let squares = source.matches("rect(width: 5pt, height: 5pt").count();
    let polygons = source.matches("polygon(").count();
    assert!(
        squares > 0 && polygons > 0,
        "the two series must draw different marker shapes, got {squares} squares \
         and {polygons} polygons in:\n{source}"
    );
}

/// Every size a `#text(size: Npt)[label]` was emitted at, for one label.
fn emitted_text_sizes(source: &str, label: &str) -> Vec<f64> {
    let suffix: String = format!(")[{label}]");
    let mut sizes: Vec<f64> = Vec::new();
    for (index, _) in source.match_indices(&suffix) {
        let Some(open) = source[..index].rfind("#text(size: ") else {
            continue;
        };
        let value: &str = &source[open + "#text(size: ".len()..index];
        if let Ok(size) = value.trim_end_matches("pt").parse::<f64>() {
            sizes.push(size);
        }
    }
    sizes
}

#[test]
fn chart_labels_take_the_default_chart_text_size() {
    // A chart that declares no `c:txPr` anywhere still has a text size: Excel's
    // 10pt chart default. The sizes were per-element constants instead — 8pt for
    // the axis and category labels, 9pt for the legend — so the labels rendered
    // at a size the file never asks for, and did not even agree with each other
    // (issue #800).
    //
    // Both renderers that can be checked against `WithChart.xlsx` put every run
    // at 10pt: the native Excel export measures a 6.24pt cap height (10pt
    // Calibri) and LibreOffice writes a literal 10.0pt text matrix for all 18
    // runs on the page.
    for chart_type in [ChartType::Bar, ChartType::Line] {
        let mut chart = two_series_bar_chart(Vec::new());
        let kind: String = format!("{chart_type:?}");
        chart.chart_type = chart_type;
        chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
        chart.series[0].values = vec![4.0, 8.0];
        chart.series[1].values = vec![6.0, 2.0];
        let source = chart_source(chart);

        for label in ["0", "Q1", "Q2", "Revenue", "Cost"] {
            let sizes = emitted_text_sizes(&source, label);
            assert!(
                !sizes.is_empty(),
                "{kind}: no #text(size:) wrapped the label {label}; got:\n{source}"
            );
            for size in &sizes {
                assert_eq!(
                    *size, CHART_DEFAULT_TEXT_PT,
                    "{kind}: label {label} drew at {size}pt, not the \
                     {CHART_DEFAULT_TEXT_PT}pt chart default; got:\n{source}"
                );
            }
        }
    }
}

#[test]
fn legend_keys_use_an_explicit_chart_owned_label_gap() {
    // A plain markup space inherits the document's body font and size. That
    // leaked an 11pt word-space run between each 10pt chart key and label,
    // widening the legend independently of the chart's own text (#804).
    for chart_type in [ChartType::Bar, ChartType::Line, ChartType::Pie] {
        let mut chart = two_series_bar_chart(Vec::new());
        let kind = format!("{chart_type:?}");
        chart.chart_type = chart_type;
        chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
        chart.series[0].values = vec![4.0, 8.0];
        chart.series[1].values = vec![6.0, 2.0];
        let source = chart_source(chart);

        assert!(
            source.contains("#h(0pt)#text(size: 10pt)"),
            "{kind}: the key-to-label gap must be explicit chart layout; got:\n{source}"
        );
    }
}

#[test]
fn a_line_legend_key_draws_the_series_line_and_its_marker() {
    // Excel's legend key for a line series is a sample of what the reader sees
    // in the plot: the series line at its own weight with the series' marker
    // centred on it. A filled 12x3pt bar carries neither, so the key could not
    // be matched to its line (issue #801).
    //
    // Measured on the native export of `WithChart.xlsx`: the key line is
    // 20.16pt and 20.64pt long for the two series, and each carries a ~5pt
    // marker centred on it — a diamond for the first series, a square for the
    // second.
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Line;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string(), "Q3".to_string()];
    chart.series[0].values = vec![4.0, 8.0, 6.0];
    chart.series[1].values = vec![6.0, 2.0, 5.0];
    let source = chart_source(chart);

    assert!(
        !source.contains("height: 3pt, fill:"),
        "the legend key must not be a flat filled bar; got:\n{source}"
    );
    // One marker per data point, plus one on each series' legend key.
    let points_per_series: usize = 3;
    for (shape, label) in [("polygon(", "diamond"), ("rect(width: 5pt", "square")] {
        assert_eq!(
            source.matches(shape).count(),
            points_per_series + 1,
            "the {label} series must draw a marker on its legend key as well as \
             on each of its {points_per_series} points; got:\n{source}"
        );
    }
    assert!(
        source.contains(&format!(
            "line(end: ({}pt, 0pt), stroke: {}pt",
            format_f64(LEGEND_KEY_LEN_PT),
            format_f64(SERIES_LINE_PT)
        )),
        "the legend key must draw the series line at its own weight; got:\n{source}"
    );
}
#[test]
fn every_chart_family_draws_the_default_chart_area_outline() {
    // A `c:chartSpace` with no `c:spPr/a:ln` still takes Office's default chart-area
    // outline — a thin rectangle enclosing the plot, the axis labels and the legend — so a
    // chart drawn without one has no boundary against the sheet behind it (#637).
    //
    // Measured on the native Excel export of `WithChart.xlsx` at 150 DPI: the border is a
    // single pixel of RGB(133,133,133), indistinguishable from the same page's gridlines,
    // which is `CHART_AUTOMATIC_LINE` — 0.75pt of #868686.
    for chart_type in [ChartType::Bar, ChartType::Line, ChartType::Pie] {
        let kind: String = format!("{chart_type:?}");
        let mut chart = two_series_bar_chart(Vec::new());
        chart.chart_type = chart_type;
        chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
        chart.series[0].values = vec![4.0, 8.0];
        chart.series[1].values = vec![6.0, 2.0];
        let source = chart_source(chart);

        let outline: String = format!("stroke: {CHART_AREA_OUTLINE})[");
        assert!(
            source.contains(&outline),
            "{kind}: the chart area must carry the default outline; got:\n{source}"
        );
        // Exactly one box takes it — the outermost. A stroke on a nested box would draw a
        // second rectangle inside the chart.
        assert_eq!(
            source.matches(&outline).count(),
            1,
            "{kind}: only the chart-area box may carry the outline; got:\n{source}"
        );
    }
}

#[test]
fn a_chart_that_asks_for_no_outline_gets_none() {
    // `<a:ln><a:noFill/></a:ln>` on `c:chartSpace/c:spPr` is the file saying it
    // wants no chart-area border. Drawing the default anyway puts a grey box
    // around every chart part that deliberately has none — `123233_charts.xlsx`
    // and `oxp_CU018-Chart-Cached-Data-41.pptx` among them (#637).
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Line;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![6.0, 2.0];
    chart.chart_area_outline = ChartAreaOutline::Suppressed;
    let source = chart_source(chart);

    assert!(
        source.contains("stroke: none)["),
        "a suppressed outline must draw nothing; got:\n{source}"
    );
    assert!(
        !source.contains(&format!("stroke: {CHART_AREA_OUTLINE})[")),
        "the default outline must not override an explicit noFill; got:\n{source}"
    );
}

#[test]
fn a_chart_outline_keeps_its_own_width_and_colour() {
    // Chart parts declare lines of their own that the automatic grey is not:
    // `xlsx/office2pdf_repository_workbook.xlsx` a 9360 EMU #d9d9d9 one, and
    // `pptx/chart-picture-bg.pptx` a 28575 EMU accent one (#637).
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Line;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![6.0, 2.0];
    chart.chart_area_outline = ChartAreaOutline::Explicit {
        width_pt: Some(0.7370079),
        color: Some(crate::ir::Color::new(0xd9, 0xd9, 0xd9)),
    };
    let source = chart_source(chart);

    assert!(
        source.contains("rgb(217, 217, 217)"),
        "the declared colour must reach the outline; got:\n{source}"
    );
    assert!(
        !source.contains(&format!("stroke: {CHART_AREA_OUTLINE})[")),
        "a declared line must not be replaced by the automatic one; got:\n{source}"
    );
}

// ----- The chart's declared text face (issue #668) -----

#[test]
fn chart_text_is_set_in_the_face_the_chart_declares() {
    // Every chart string used to fall through to the engine's default serif,
    // a face that appears nowhere else in the document. No sub-renderer names
    // a font, so one scoped `set` has to cover them all.
    let mut chart = two_series_bar_chart(Vec::new());
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![2.0, 6.0];
    chart.title = Some("Sales".to_string());
    chart.text_font_family = Some("Calibri".to_string());

    let source: String = chart_source(chart);
    assert!(
        source.contains("#set text(font: "),
        "the chart must set its declared face, got:\n{source}"
    );
    assert!(
        source.contains("Calibri"),
        "the declared face must reach the emitted font list, got:\n{source}"
    );
}

#[test]
fn a_chart_naming_no_face_sets_none() {
    // A chart whose package has no theme keeps the renderer's existing
    // behaviour rather than naming a face nothing resolves.
    let mut chart = two_series_bar_chart(Vec::new());
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![2.0, 6.0];
    chart.text_font_family = None;

    assert!(!chart_source(chart).contains("#set text(font: "));
}

#[test]
fn a_chart_with_korean_labels_keeps_an_east_asian_fallback() {
    // The declared face is Latin; the categories are not. A chain built from
    // the family alone would leave the Hangul to the engine's own pick.
    let mut chart = two_series_bar_chart(Vec::new());
    chart.categories = vec!["매출".to_string(), "비용".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![2.0, 6.0];
    chart.text_font_family = Some("Calibri".to_string());

    let source: String = chart_source(chart);
    let set_line: &str = source
        .lines()
        .find(|line| line.starts_with("#set text(font: "))
        .expect("the chart sets a face");
    assert!(
        set_line.contains(','),
        "a Korean chart needs a fallback chain, not a bare family: {set_line}"
    );
}

// ----- Run properties declared in c:txPr (issue #669) -----

fn sized_bar_chart(size_pt: f64) -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![2.0, 6.0];
    chart.title = Some("Sales".to_string());
    chart.text_style = crate::ir::ChartTextStyle {
        size_pt: Some(size_pt),
        bold: None,
        color: None,
    };
    chart
}

#[test]
fn chart_labels_take_the_size_the_chart_declares() {
    // `bar-chart.pptx` asks for 18pt and rendered at 10 — a little over half
    // the size the file requested.
    let source: String = chart_source(sized_bar_chart(18.0));
    for label in ["Q1", "Q2"] {
        assert_eq!(
            emitted_text_sizes(&source, label),
            vec![18.0],
            "category label {label} must take the declared size, got:\n{source}"
        );
    }
    assert!(
        !source.contains("#text(size: 10pt)"),
        "nothing may fall back to the chart default once a size is declared:\n{source}"
    );
}

#[test]
fn a_chart_title_takes_office_s_scaled_size() {
    // Office renders the 18pt `bar-chart.pptx` declares as a 22pt title.
    let source: String = chart_source(sized_bar_chart(18.0));
    // `emitted_text_sizes` cannot read a `#text` carrying a weight, and the
    // title always carries one.
    assert!(
        source.contains("#text(size: 21.6pt, weight: \"bold\")[Sales]"),
        "the title must scale by 1.2, got:\n{source}"
    );
}

#[test]
fn a_chart_declaring_no_size_keeps_the_eleven_point_title() {
    // The default title size is what `AREA_TITLE_H` was measured against, so a
    // chart that declares nothing must not move.
    let mut chart = sized_bar_chart(18.0);
    chart.text_style = crate::ir::ChartTextStyle::default();
    assert!(chart_source(chart).contains("#text(size: 11pt, weight: \"bold\")[Sales]"));
}

#[test]
fn category_labels_take_the_axis_weight() {
    // `a:defRPr b="1"` on `c:catAx` was dropped, so bold category labels
    // rendered regular while the data labels beside them kept their own bold.
    let mut chart = sized_bar_chart(11.0);
    chart.category_axis_text_style = crate::ir::ChartTextStyle {
        size_pt: None,
        bold: Some(true),
        color: None,
    };
    let source: String = chart_source(chart);
    assert!(
        source.contains("#text(size: 11pt, weight: \"bold\")[Q1]"),
        "the category label must carry the axis' weight, got:\n{source}"
    );
}

#[test]
fn an_axis_size_overrides_the_chart_space_size_for_that_axis_only() {
    let mut chart = sized_bar_chart(18.0);
    chart.category_axis_text_style = crate::ir::ChartTextStyle {
        size_pt: Some(9.0),
        bold: None,
        color: None,
    };
    let source: String = chart_source(chart);
    assert_eq!(emitted_text_sizes(&source, "Q1"), vec![9.0]);
    // The title still follows the chart space.
    assert!(source.contains("#text(size: 21.6pt, weight: \"bold\")[Sales]"));
}

// ----- Radar charts (issue #679) -----

fn radar_chart() -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Other(crate::ir::RADAR_CHART_LABEL.to_string());
    chart.categories = vec![
        "Deploy".to_string(),
        "Startup".to_string(),
        "Deps".to_string(),
        "Portable".to_string(),
        "Coverage".to_string(),
    ];
    chart.series[0].values = vec![5.0, 5.0, 5.0, 5.0, 3.0];
    chart.series[1].values = vec![2.0, 2.0, 1.0, 3.0, 5.0];
    chart.title = Some("Qualitative".to_string());
    chart
}

#[test]
fn a_radar_chart_draws_a_plot_rather_than_a_data_table() {
    // #544 replaced the silently dropped chart with a bordered rectangle
    // holding an italic caption and a table of the series values, so a slide
    // whose primary content was a radar still lost it.
    let source: String = chart_source(radar_chart());
    assert!(
        !source.contains("Radar Chart"),
        "the type-label caption belongs to the table fallback, got:\n{source}"
    );
    assert!(
        source.contains("path(closed: true"),
        "a radar is drawn as closed rings and polygons, got:\n{source}"
    );
}

#[test]
fn a_radar_draws_one_closed_polygon_per_series() {
    // Two series over five categories: five web rings plus two series rings.
    let source: String = chart_source(radar_chart());
    let closed: usize = source.matches("path(closed: true").count();
    assert!(
        closed > 2,
        "expected a ring per major unit plus one polygon per series, got {closed} in:\n{source}"
    );
    // Each series polygon carries the series stroke width; the web does not.
    let series_rings: usize = source
        .matches(&format!(
            "path(closed: true, stroke: {}pt + ",
            format_f64(SERIES_LINE_PT)
        ))
        .count();
    assert_eq!(
        series_rings, 2,
        "one closed polygon per series, got:\n{source}"
    );
}

#[test]
fn a_radar_labels_every_category_and_keeps_its_title() {
    let source: String = chart_source(radar_chart());
    for category in ["Deploy", "Startup", "Deps", "Portable", "Coverage"] {
        assert!(
            source.contains(&format!("[{category}]")),
            "category {category} must be labelled, got:\n{source}"
        );
    }
    assert!(source.contains("[Qualitative]"), "got:\n{source}");
}

#[test]
fn a_radar_with_too_few_categories_keeps_the_table_fallback() {
    // Two spokes cannot close a ring, so the table still says more than a
    // degenerate plot would.
    let mut chart = radar_chart();
    chart.categories = vec!["Deploy".to_string(), "Startup".to_string()];
    chart.series[0].values = vec![5.0, 5.0];
    chart.series[1].values = vec![2.0, 2.0];
    assert!(chart_source(chart).contains("Radar Chart"));
}

#[test]
fn a_radar_with_no_positive_value_keeps_the_table_fallback() {
    let mut chart = radar_chart();
    chart.series[0].values = vec![0.0; 5];
    chart.series[1].values = vec![0.0; 5];
    assert!(chart_source(chart).contains("Radar Chart"));
}

// ----- Plot chrome sized from the text it holds (issue #706) -----

fn bar_chart_at(size_pt: Option<f64>, categories: &[&str]) -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Bar;
    chart.categories = categories.iter().map(|c| (*c).to_string()).collect();
    chart.series[0].values = vec![4.0; categories.len()];
    chart.series[1].values = vec![2.0; categories.len()];
    chart.text_style = crate::ir::ChartTextStyle {
        size_pt,
        bold: None,
        color: None,
    };
    chart
}

#[test]
fn a_chart_declaring_no_size_keeps_its_chrome_where_it_was() {
    // The band constants were calibrated at the 10pt chart default, so scaling
    // from them has to be the identity there or every untouched chart moves.
    let chart = bar_chart_at(None, &["Q1", "Q2"]);
    assert_eq!(chart_tick_band_pt(&chart), TICK_GAP);
    assert_eq!(chart_category_band_pt(&chart), ROW);
    assert_eq!(chart_category_gutter_pt(&chart), LABEL_W + GAP);
}

#[test]
fn a_larger_declared_size_reserves_a_taller_tick_band() {
    // Native PowerPoint reserves 39.9817pt below the plot for an 18pt chart;
    // the band includes both a fixed base and a text-scaled component, so a
    // simple 1.8x scaling is still short.
    let chart = bar_chart_at(Some(18.0), &["Q1", "Q2"]);
    assert!(
        (chart_tick_band_pt(&chart) - 39.9817).abs() < 0.02,
        "an 18pt chart reserves PowerPoint's measured band, got {}",
        chart_tick_band_pt(&chart)
    );
    assert!(chart_tick_band_pt(&chart) > TICK_GAP);
}

#[test]
fn a_framed_bar_chart_reserves_powerpoint_measured_chrome_at_multiple_sizes() {
    // Native PowerPoint 16.112 exports of `bar-chart.pptx` with only
    // `c:chartSpace/c:txPr/a:defRPr@sz` changed. Each value is the plot's
    // left/top/right/bottom edge relative to the 480 x 320pt graphic frame.
    // Two sizes keep the regression test from fitting the original 18pt GT
    // with constants that fail as soon as the chart text changes.
    let measurements = [
        (12.0, (55.3186, 37.4150, 413.0499, 291.0732)),
        (18.0, (79.7209, 46.2050, 391.0825, 279.9383)),
    ];

    for (size_pt, expected) in measurements {
        let mut chart = bar_chart_at(Some(size_pt), &["1st Qtr", "2nd Qtr", "3rd Qtr", "4th Qtr"]);
        chart.series.truncate(1);
        chart.series[0].name = Some("Sales".to_string());
        chart.has_legend = true;
        chart.legend_position = LegendPosition::Right;
        chart.text_font_family = Some("Calibri".to_string());
        let actual = axis_plot_rect(&chart, (480.0, 320.0), true);
        let errors = [
            ("left", actual.0, expected.0),
            ("top", actual.1, expected.1),
            ("right", actual.2, expected.2),
            ("bottom", actual.3, expected.3),
        ]
        .map(|(axis, actual, expected)| (axis, actual, expected, (actual - expected).abs()));
        assert!(
            errors.iter().all(|(_, _, _, error)| *error <= 0.1),
            "{size_pt}pt chart edges: {errors:?}"
        );
    }
}

#[test]
fn a_powerpoint_right_legend_places_its_scaled_entry_at_multiple_sizes() {
    // Native PowerPoint 16.112 exports of the same 480 x 320pt chart frame.
    // These are the key's left edge relative to the frame, its size, and the
    // visible key-to-label gap. Five sizes prevent a one-off translation fitted
    // only to the 18pt #841 GT.
    let measurements = [
        (10.0, 441.4465, 5.4923, 2.3710),
        (12.0, 435.6760, 6.5926, 2.9213),
        (18.0, 418.4018, 9.8887, 4.5694),
        (24.0, 401.1207, 13.1827, 6.2164),
        (36.0, 366.5520, 19.7753, 9.5126),
    ];

    for (size_pt, expected_x, expected_key_size, expected_gap) in measurements {
        let mut chart = bar_chart_at(Some(size_pt), &["1st Qtr", "2nd Qtr", "3rd Qtr", "4th Qtr"]);
        chart.series.truncate(1);
        chart.series[0].name = Some("Sales".to_string());
        chart.has_legend = true;
        chart.legend_position = LegendPosition::Right;
        chart.host = crate::ir::ChartHost::Presentation;
        chart.text_font_family = Some("Calibri".to_string());

        let source = framed_chart_source(&chart, 480.0, 320.0);
        let actual_x = legend_entry_x(&source, "Sales");
        assert!(
            (actual_x - expected_x).abs() <= 0.1,
            "{size_pt}pt PowerPoint legend key starts at {actual_x}pt, expected {expected_x}pt; got:\n{source}"
        );
        let entry = source
            .lines()
            .find(|line| line.contains("box[#box") && line.contains("[Sales]]"))
            .expect("the chart emits its Sales legend entry");
        let key_size = PPTX_LEGEND_KEY_EM * size_pt;
        let gap = PPTX_LEGEND_KEY_LABEL_GAP_PT + PPTX_LEGEND_KEY_LABEL_GAP_EM * size_pt;
        assert!((key_size - expected_key_size).abs() <= 0.002);
        assert!((gap - expected_gap).abs() <= 0.001);
        assert!(entry.contains(&format!(
            "box(width: {}pt, height: {}pt",
            format_f64(key_size),
            format_f64(key_size)
        )));
        assert!(entry.contains(&format!("#h({}pt)", format_f64(gap))));
    }
}

#[test]
fn an_unmeasurable_powerpoint_right_legend_keeps_the_plot_relative_fallback() {
    let mut chart = bar_chart_at(Some(18.0), &["Q1", "Q2"]);
    chart.series.truncate(1);
    chart.series[0].name = Some("Sales".to_string());
    chart.host = crate::ir::ChartHost::Presentation;
    chart.text_font_family = Some("Definitely Missing Chart Face 999".to_string());

    let plot_right = axis_plot_rect(&chart, (480.0, 320.0), false).2;
    let source = framed_chart_source(&chart, 480.0, 320.0);
    let actual_x = legend_entry_x(&source, "Sales");
    assert!(
        (actual_x - (plot_right + GAP)).abs() <= 0.01,
        "an unmeasurable face must preserve the plot-relative fallback, got {actual_x} after a {plot_right}pt plot; source:\n{source}"
    );
}

#[test]
fn a_powerpoint_right_legend_uses_the_native_vertical_center_at_multiple_sizes() {
    // Native PowerPoint 16.112 exports of the same 480 x 320pt chart frame.
    // Each value is the legend key's top edge inside the post-title chart body.
    // The native absolute edge is translated by the source frame and the same
    // title band used by `generate_chart_in`.
    let measurements = [
        (10.0, 133.7634),
        (12.0, 131.5833),
        (18.0, 125.0449),
        (24.0, 118.5077),
        (36.0, 105.4309),
    ];

    for (size_pt, expected_y) in measurements {
        let mut chart = bar_chart_at(Some(size_pt), &["1st Qtr", "2nd Qtr", "3rd Qtr", "4th Qtr"]);
        chart.series.truncate(1);
        chart.series[0].name = Some("Sales".to_string());
        chart.host = crate::ir::ChartHost::Presentation;
        chart.text_font_family = Some("Calibri".to_string());

        let source = framed_chart_source(&chart, 480.0, 320.0);
        let actual_y = legend_entry_y(&source, "Sales");
        assert!(
            (actual_y - expected_y).abs() <= 0.1,
            "{size_pt}pt PowerPoint legend key starts at y={actual_y}pt, expected {expected_y}pt; got:\n{source}"
        );
    }
}

#[test]
fn a_framed_column_chart_reserves_powerpoint_measured_chrome() {
    // Native PowerPoint 16.112 export of slide 14 in the #841 Contoso deck.
    // The coordinates are relative to its 401.95 x 344.25pt graphic frame.
    let mut chart = crowded_column_chart();
    chart.text_style.size_pt = Some(11.97);
    chart.category_axis_text_style.size_pt = Some(11.97);
    chart.value_axis_text_style.size_pt = Some(11.97);
    chart.text_font_family = Some("Avenir Next LT Pro".to_string());
    chart.has_legend = false;
    let actual = axis_plot_rect(&chart, (401.95, 344.25), false);
    let expected = (46.9766, 12.266, 390.9504, 193.1674);
    let errors = [
        ("left", actual.0, expected.0),
        ("top", actual.1, expected.1),
        ("right", actual.2, expected.2),
        ("bottom", actual.3, expected.3),
    ]
    .map(|(axis, actual, expected)| (axis, actual, expected, (actual - expected).abs()));
    assert!(
        errors.iter().all(|(_, _, _, error)| *error <= 0.1),
        "column chart edges: {errors:?}"
    );
}

#[test]
fn a_chart_title_occupies_the_same_fixed_band_used_by_plot_geometry() {
    let mut chart = bar_chart_at(Some(18.0), &["Q1", "Q2"]);
    chart.title = Some("Sales".to_string());
    let source = framed_chart_source(&chart, 480.0, 320.0);
    assert!(
        source.contains("#block(width: 480pt, height: 46.21pt, above: 0pt, below: 0pt)"),
        "the emitted title must occupy its measured plot band and frame width, got:\n{source}"
    );
    assert!(
        !source.contains("#block(width: 100%, height: 46.21pt"),
        "a framed title must not resolve 100% against the slide, got:\n{source}"
    );
}

#[test]
fn framed_line_radar_and_pie_charts_center_their_titles_in_the_frame() {
    let mut line = two_series_bar_chart(Vec::new());
    line.chart_type = ChartType::Line;
    line.title = Some("Line title".to_string());
    line.categories = vec!["Q1".to_string(), "Q2".to_string()];
    line.series[0].values = vec![1.0, 2.0];
    line.series[1].values = vec![2.0, 1.0];

    let mut radar = radar_chart();
    radar.title = Some("Radar title".to_string());

    let mut pie = pie_chart(vec![60.0, 40.0]);
    pie.title = Some("Pie title".to_string());

    for (name, chart) in [("line", line), ("radar", radar), ("pie", pie)] {
        let source = framed_chart_source(&chart, 321.0, 240.0);
        assert!(
            source.contains("#block(width: 321pt)[#align(center)"),
            "the {name} title must use its chart frame, got:\n{source}"
        );
    }
}

#[test]
fn a_flowed_chart_title_keeps_its_container_width() {
    let mut chart = bar_chart_at(Some(18.0), &["Q1", "Q2"]);
    chart.title = Some("Sales".to_string());
    let source = chart_source(chart);
    assert!(source.contains("#block(width: 100%, height: 46.21pt, above: 0pt, below: 0pt)"));
}

#[test]
fn the_category_gutter_never_narrows_below_the_calibrated_width() {
    // The gutter is measured from the widest label, and a face that cannot be
    // measured — wasm has no font search — must not collapse it.
    let mut chart = bar_chart_at(Some(18.0), &["Q", "R"]);
    chart.text_font_family = Some("Definitely Missing Chart Face 706".to_string());
    assert_eq!(chart_category_gutter_pt(&chart), LABEL_W + GAP);
}

#[test]
fn the_category_gutter_grows_with_the_widest_label() {
    // A width holding text has to follow what the text says, not just its size.
    // `bar-chart.pptx`'s labels are as short as `4th Qtr`, so scaling the flat
    // constant by the band's 1.8 would reserve far more than they need.
    let short: f64 = chart_category_gutter_pt(&bar_chart_at(Some(18.0), &["Q1", "Q2"]));
    let long: f64 = chart_category_gutter_pt(&bar_chart_at(
        Some(18.0),
        &["Q1", "A considerably longer category label"],
    ));
    assert!(
        long > short,
        "a longer label must widen the gutter: {long} against {short}"
    );
}

#[test]
fn horizontal_category_labels_stop_the_text_scaled_clearance_short_of_the_plot() {
    let chart = bar_chart_at(Some(18.0), &["1st Qtr", "2nd Qtr", "3rd Qtr", "4th Qtr"]);
    let source = framed_chart_source(&chart, 480.0, 320.0);
    let label_width = chart_category_gutter_pt(&chart) - 16.686;
    let label = source
        .lines()
        .find(|line| line.contains("[4th Qtr]"))
        .expect("the chart emits its category label");
    assert!(
        label.contains(&format!("box(width: {}pt", format_f64(label_width))),
        "an 18pt label must stop 16.686pt short of the plot, got:\n{label}"
    );
}

#[test]
fn horizontal_category_labels_keep_the_legacy_gap_without_a_declared_size() {
    let chart = bar_chart_at(None, &["Q1", "Q2"]);
    let source = framed_chart_source(&chart, 480.0, 320.0);
    let label = source
        .lines()
        .find(|line| line.contains("[Q2]"))
        .expect("the chart emits its category label");
    assert!(
        label.contains(&format!("box(width: {}pt", format_f64(LABEL_W))),
        "an undeclared-size label must keep the pre-#706 6pt gap, got:\n{label}"
    );
}

#[test]
fn horizontal_category_labels_keep_the_fallback_width_when_the_face_is_unmeasurable() {
    let mut chart = bar_chart_at(Some(18.0), &["Q1", "Q2"]);
    chart.text_font_family = Some("Definitely Missing Chart Face 998".to_string());
    let source = framed_chart_source(&chart, 480.0, 320.0);
    let label = source
        .lines()
        .find(|line| line.contains("[Q2]"))
        .expect("the chart emits its category label");
    assert!(
        label.contains(&format!("box(width: {}pt", format_f64(LABEL_W))),
        "an unmeasurable face must keep the calibrated 62pt fallback, got:\n{label}"
    );
}

// ----- The automatic chart-area outline is host-dependent (issue #823) -----

/// The `#box(...)` line that opens the chart area, whose `stroke:` is the
/// chart-area outline. The gridlines below it repeat the same stroke string.
fn chart_area_box_line(source: &str) -> &str {
    source
        .lines()
        .find(|line| line.starts_with("#box(width:") && line.contains("stroke:"))
        .expect("the chart opens an area box")
}

fn framed_bar_chart_on(host: crate::ir::ChartHost) -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Bar;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.series[0].values = vec![4.0, 8.0];
    chart.series[1].values = vec![2.0, 6.0];
    chart.host = host;
    chart
}

#[test]
fn a_slide_chart_draws_no_automatic_area_outline() {
    // PowerPoint draws none. Applying Excel's default everywhere put a
    // 480 x 301pt rectangle around every chart on a slide.
    let source: String = chart_source(framed_bar_chart_on(crate::ir::ChartHost::Presentation));
    // The gridlines carry the same stroke string legitimately, so the box's own
    // `stroke:` is what has to be read, not the source as a whole.
    let box_line: &str = chart_area_box_line(&source);
    assert!(
        box_line.contains("stroke: none"),
        "a slide chart must not draw Excel's automatic border, got: {box_line}"
    );
}

#[test]
fn a_workbook_chart_keeps_the_measured_excel_outline() {
    // #637 measured this against a native Excel export and it must not move.
    let source: String = chart_source(framed_bar_chart_on(crate::ir::ChartHost::Spreadsheet));
    let box_line: &str = chart_area_box_line(&source);
    assert!(
        box_line.contains(CHART_AREA_OUTLINE),
        "a workbook chart keeps Excel's automatic border, got: {box_line}"
    );
}

#[test]
fn an_explicit_outline_survives_on_every_host() {
    // Only the *automatic* default is host-dependent; a chart that states a
    // line gets it wherever it lives, and `noFill` still suppresses.
    for host in [
        crate::ir::ChartHost::Presentation,
        crate::ir::ChartHost::Spreadsheet,
        crate::ir::ChartHost::WordProcessing,
    ] {
        let mut chart = framed_bar_chart_on(host);
        chart.chart_area_outline = ChartAreaOutline::Explicit {
            width_pt: Some(2.0),
            color: Some(crate::ir::Color::new(0xd9, 0xd9, 0xd9)),
        };
        let source: String = chart_source(chart);
        assert!(
            chart_area_box_line(&source).contains("2pt + rgb(217, 217, 217)"),
            "an explicit outline must survive on {host:?}"
        );

        let mut chart = framed_bar_chart_on(host);
        chart.chart_area_outline = ChartAreaOutline::Suppressed;
        let source: String = chart_source(chart);
        assert!(
            chart_area_box_line(&source).contains("stroke: none"),
            "on {host:?}"
        );
    }
}

// ----- The automatic horizontal value-axis scale is host-dependent (#824) -----

fn auto_scaled_bar_chart_on(host: crate::ir::ChartHost) -> Chart {
    let mut chart = framed_bar_chart_on(host);
    chart.text_style.size_pt = Some(18.0);
    chart.series[0].values = vec![8.2, 3.2];
    chart.series[1].values = vec![1.4, 1.2];
    chart
}

#[test]
fn a_slide_bar_chart_uses_powerpoints_measured_auto_scale() {
    let source = chart_source(auto_scaled_bar_chart_on(crate::ir::ChartHost::Presentation));
    assert_eq!(
        emitted_axis_ticks_at_size(&source, 18.0),
        vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
    );
}

#[test]
fn a_workbook_bar_chart_keeps_excels_measured_auto_scale() {
    let source = chart_source(auto_scaled_bar_chart_on(crate::ir::ChartHost::Spreadsheet));
    assert_eq!(
        emitted_axis_ticks_at_size(&source, 18.0),
        vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
    );
}

// ----- A horizontal legend advances by each entry's width (issue #827) -----

fn legend_entry_x(source: &str, label: &str) -> f64 {
    let marker: String = format!("[{label}]])");
    let index: usize = source.find(&marker).expect("the entry is drawn");
    let line_start: usize = source[..index].rfind('\n').map_or(0, |at| at + 1);
    let line: &str = &source[line_start..index];
    line.split("dx: ")
        .nth(1)
        .and_then(|rest| rest.split("pt").next())
        .and_then(|value| value.trim().parse::<f64>().ok())
        .expect("the entry is placed")
}

fn legend_entry_y(source: &str, label: &str) -> f64 {
    let marker: String = format!("[{label}]])");
    let index: usize = source.find(&marker).expect("the entry is drawn");
    let line_start: usize = source[..index].rfind('\n').map_or(0, |at| at + 1);
    let line: &str = &source[line_start..index];
    line.split("dy: ")
        .nth(1)
        .and_then(|rest| rest.split("pt").next())
        .and_then(|value| value.trim().parse::<f64>().ok())
        .expect("the entry is placed")
}

fn bottom_legend_chart(names: &[&str]) -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Bar;
    chart.categories = vec!["Q1".to_string(), "Q2".to_string()];
    chart.has_legend = true;
    chart.legend_position = LegendPosition::Bottom;
    chart.series.truncate(names.len().min(chart.series.len()));
    for (series, name) in chart.series.iter_mut().zip(names) {
        series.name = Some((*name).to_string());
        series.values = vec![4.0, 8.0];
    }
    chart
}

#[test]
fn a_long_legend_name_pushes_the_next_entry_clear() {
    // Every entry advanced by a flat 78pt, so a name wider than that ran under
    // the entry beside it and the two overprinted.
    let short: String = chart_source(bottom_legend_chart(&["A", "B"]));
    let long: String = chart_source(bottom_legend_chart(&[
        "A considerably longer series name",
        "B",
    ]));
    let short_gap: f64 = legend_entry_x(&short, "B") - legend_entry_x(&short, "A");
    let long_gap: f64 =
        legend_entry_x(&long, "B") - legend_entry_x(&long, "A considerably longer series name");
    assert!(
        long_gap > short_gap,
        "a wide name must push its neighbour further along: {long_gap} against {short_gap}"
    );
}

#[test]
fn short_legend_names_keep_the_calibrated_pitch() {
    // The measured width is floored at the old constant, so a legend of short
    // names lays out exactly where it always did.
    let source: String = chart_source(bottom_legend_chart(&["A", "B"]));
    let gap: f64 = legend_entry_x(&source, "B") - legend_entry_x(&source, "A");
    assert!(
        (gap - LEGEND_ENTRY_W).abs() < 1e-9,
        "short names keep the {}pt pitch, got {gap}",
        format_f64(LEGEND_ENTRY_W)
    );
}

/// The data-table fallback prints each value through the format its series
/// declares, so a ratio stored as a fraction reads as the percentage the
/// source shows (issue #865).
#[test]
fn test_data_table_prints_a_series_number_format() {
    let chart = Chart {
        // A bubble chart has no plot renderer, so it takes the data-table
        // fallback this rule lives in.
        chart_type: ChartType::Other("bubbleChart".to_string()),
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string(), "Q2".to_string()],
        series: vec![ChartSeries {
            name: Some("Rate".to_string()),
            values: vec![0.024, 0.689],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: Some("0.0%".to_string()),
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };
    let source = chart_source(chart);

    assert!(source.contains("2.4%"), "expected 2.4% in: {source}");
    assert!(source.contains("68.9%"), "expected 68.9% in: {source}");
    assert!(
        !source.contains("[0.024]"),
        "the raw fraction must go: {source}"
    );
}

/// A different code is honoured, so the renderer is not special-casing
/// percentages.
#[test]
fn test_data_table_prints_a_declared_thousands_format() {
    let chart = Chart {
        // A bubble chart has no plot renderer, so it takes the data-table
        // fallback this rule lives in.
        chart_type: ChartType::Other("bubbleChart".to_string()),
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string()],
        series: vec![ChartSeries {
            name: Some("Revenue".to_string()),
            values: vec![1234567.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: Some("#,##0".to_string()),
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };
    let source = chart_source(chart);

    assert!(
        source.contains("1,234,567"),
        "expected grouping in: {source}"
    );
}

/// A series that states no format keeps the plain rendering, so nothing else
/// in the table moves.
#[test]
fn test_data_table_without_a_number_format_prints_plainly() {
    let chart = Chart {
        // A bubble chart has no plot renderer, so it takes the data-table
        // fallback this rule lives in.
        chart_type: ChartType::Other("bubbleChart".to_string()),
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string()],
        series: vec![ChartSeries {
            name: Some("Rate".to_string()),
            values: vec![0.024],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted: false,
    };
    let source = chart_source(chart);

    assert!(
        source.contains("0.024"),
        "expected the plain value in: {source}"
    );
}

/// A currency format emits `$`, which opens math mode in Typst markup. Writing
/// a formatted axis label unescaped produced 48 "unclosed delimiter" errors on
/// a budget workbook in the bulk corpus, so every formatted label is escaped.
#[test]
fn test_a_currency_axis_label_is_escaped() {
    let chart = Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string()],
        series: vec![ChartSeries {
            name: Some("Spend".to_string()),
            values: vec![1200.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: true,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: Some("\"$\"#,##0".to_string()),
        auto_title_deleted: false,
    };
    let source = chart_source(chart);

    assert!(
        source.contains("\\$"),
        "a currency tick label must be escaped: {source}"
    );
    assert!(
        !source.contains("[$"),
        "an unescaped $ opens math mode: {source}"
    );
}

/// A single-series chart that declines the automatic title must not get one
/// from its series name (issue #883).
fn single_series_chart(auto_title_deleted: bool) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        hole_size_percent: None,
        title: None,
        categories: vec!["Q1".to_string()],
        series: vec![ChartSeries {
            name: Some("Serie 1".to_string()),
            values: vec![1.0],
            fill: None,
            point_fills: Vec::new(),
            data_labels: DataLabels::default(),
            number_format: None,
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
        has_legend: false,
        category_axis_title: None,
        value_axis_title: None,
        category_axis_major_tick_mark: AxisTickMark::Outside,
        value_axis_major_tick_mark: AxisTickMark::Outside,
        category_axis_deleted: false,
        category_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_line: crate::ir::ChartLine::Automatic,
        value_axis_major_unit: None,
        major_gridline_line: crate::ir::ChartLine::Automatic,
        value_axis_deleted: false,
        bar_band_layout: BarBandLayout::default(),
        theme_accent_colors: Vec::new(),
        chart_area_outline: ChartAreaOutline::Default,
        host: crate::ir::ChartHost::default(),
        text_font_family: None,
        text_style: crate::ir::ChartTextStyle::default(),
        category_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_text_style: crate::ir::ChartTextStyle::default(),
        value_axis_number_format: None,
        auto_title_deleted,
    }
}

#[test]
fn test_auto_title_deleted_suppresses_the_series_name_title() {
    let source = chart_source(single_series_chart(true));
    assert!(
        !source.contains("Serie 1"),
        "a declined automatic title must not be drawn: {source}"
    );
}

/// Triangulation: the fallback itself stays, so a chart that does not decline
/// it still gets its automatic title.
#[test]
fn test_a_chart_that_keeps_its_automatic_title_still_gets_one() {
    let source = chart_source(single_series_chart(false));
    assert!(
        source.contains("Serie 1"),
        "the automatic title must survive: {source}"
    );
}

/// The axes and gridlines draw with the line the part declares, and fall back
/// to the automatic stroke only where it declares none (issue #900).
#[test]
fn declared_axis_and_gridline_lines_reach_the_generated_source() {
    let white = crate::ir::ChartLine::Explicit {
        width_pt: Some(1.0),
        color: Some(Color::new(0xFF, 0xFF, 0xFF)),
    };
    let mut chart = stacked_support_chart(ChartGrouping::Clustered);
    chart.category_axis_line = white;
    chart.value_axis_line = white;
    chart.major_gridline_line = white;

    let source = chart_source(chart);

    assert!(
        source.contains("stroke: 1pt + rgb(255, 255, 255)"),
        "the declared white line must reach the source:\n{source}"
    );
    // The chart-area outline is a separate declaration and this chart states
    // none, so it keeps the automatic stroke. Only the `line(...)` draws — the
    // axes, ticks and gridlines — are this issue's.
    let automatic_lines = source
        .lines()
        .filter(|line| line.contains("line(end:") && line.contains("rgb(134, 134, 134)"))
        .count();
    assert_eq!(
        automatic_lines, 0,
        "no axis, tick or gridline should still take the automatic grey:\n{source}"
    );
}

/// A suppressed line draws nothing at all — not the automatic one.
#[test]
fn a_suppressed_axis_line_is_not_drawn() {
    let mut chart = stacked_support_chart(ChartGrouping::Clustered);
    chart.value_axis_line = crate::ir::ChartLine::Suppressed;
    chart.category_axis_line = crate::ir::ChartLine::Suppressed;
    chart.major_gridline_line = crate::ir::ChartLine::Suppressed;

    let source = chart_source(chart);

    let drawn_lines = source
        .lines()
        .filter(|line| line.contains("line(end:"))
        .count();
    assert_eq!(
        drawn_lines, 0,
        "a suppressed axis and gridline draw nothing:\n{source}"
    );
}

/// A chart that declares nothing keeps the automatic stroke, so the fallback
/// is not lost along the way.
#[test]
fn an_undeclared_axis_still_draws_the_automatic_line() {
    let source = chart_source(stacked_support_chart(ChartGrouping::Clustered));

    assert!(
        source.contains("0.75pt + rgb(134, 134, 134)"),
        "the automatic stroke must survive:\n{source}"
    );
}

/// A clustered column's labels sit beyond the bar's end, and a stacked one's
/// centre on the segment (issue #901).
///
/// The label's `dy` is what moves: `outEnd` puts its box just above the bar
/// top, `ctr` halfway down the bar.
#[test]
fn a_clustered_bar_label_sits_outside_its_end_and_a_stacked_one_centres() {
    fn label_dy(grouping: ChartGrouping, position: crate::ir::DataLabelPosition) -> f64 {
        let mut chart = stacked_support_chart(grouping);
        for series in &mut chart.series {
            series.data_labels = DataLabels {
                show_value: true,
                position,
                position_stated: true,
                ..DataLabels::default()
            };
        }
        let source = chart_source(chart);
        // The first label draw after the first bar rect.
        source
            .lines()
            .filter(|line| line.contains("align(center + horizon)"))
            .find_map(|line| {
                let dy = line.split("dy: ").nth(1)?;
                dy.split("pt").next()?.parse::<f64>().ok()
            })
            .expect("a data label is drawn")
    }

    let centred = label_dy(ChartGrouping::Stacked, crate::ir::DataLabelPosition::Center);
    let outside = label_dy(
        ChartGrouping::Clustered,
        crate::ir::DataLabelPosition::OutsideEnd,
    );

    assert!(
        outside < centred,
        "an outEnd label must sit above a centred one: outEnd dy {outside}, ctr dy {centred}"
    );
}

/// An `outEnd` label clears the bar's end rather than sitting flush against it
/// (issue #907).
///
/// The reference leaves about 2.8pt whatever the label's size — 8pt labels
/// clear by a mean 2.66pt, 11.97pt by 2.99pt, 18pt by 2.73pt — so the
/// placement carries a constant offset, asserted here against the bar's own
/// `dy` in the same generated source.
#[test]
fn an_outside_end_label_clears_the_bar_by_a_constant() {
    let mut chart = stacked_support_chart(ChartGrouping::Clustered);
    for series in &mut chart.series {
        series.data_labels = DataLabels {
            show_value: true,
            position: crate::ir::DataLabelPosition::OutsideEnd,
            position_stated: true,
            ..DataLabels::default()
        };
    }
    let source = chart_source(chart);

    fn first_dy(source: &str, needle: &str) -> f64 {
        source
            .lines()
            .filter(|line| line.contains(needle))
            .find_map(|line| line.split("dy: ").nth(1)?.split("pt").next()?.parse().ok())
            .unwrap_or_else(|| panic!("no {needle} draw in:\n{source}"))
    }

    // The first bar rect and the first label box, in draw order.
    let bar_top: f64 = first_dy(&source, "rect(width:");
    let label_dy: f64 = first_dy(&source, "align(center + horizon)");

    let clearance: f64 = bar_top - label_dy;
    assert!(
        clearance > 10.0,
        "an outEnd label must clear the bar top by more than its own line box, \
         got {clearance} (bar {bar_top}, label {label_dy})"
    );
    assert!(
        (clearance - 12.4).abs() < 0.01,
        "expected the 10pt line box plus the 2.4pt gap, got {clearance}"
    );
}

/// A declared `<c:majorUnit>` sets the tick interval (issue #882).
///
/// The deck in #841 declares 0.2 on a 0.689 maximum and the reference ticks
/// 0/20/40/60/80%; we ticked every 10%, twice as often as the file asks.
#[test]
fn a_stated_major_unit_sets_the_tick_interval() {
    fn tick_labels(unit: Option<f64>) -> Vec<String> {
        let mut chart = stacked_support_chart(ChartGrouping::Clustered);
        chart.value_axis_major_unit = unit;
        let source = chart_source(chart);
        source
            .lines()
            .filter(|line| line.contains("align(right + horizon)"))
            .filter_map(|line| {
                let start = line.rfind('[')?;
                Some(
                    line[start + 1..]
                        .trim_end_matches(&[']', ')'][..])
                        .to_string(),
                )
            })
            .collect()
    }

    let automatic = tick_labels(None);
    let stated = tick_labels(Some(4.0));

    assert!(
        stated.len() < automatic.len(),
        "a 4-unit interval must give fewer ticks than the automatic one: \
         automatic {automatic:?}, stated {stated:?}"
    );
    assert!(
        stated.len() >= 2,
        "the axis still needs its ticks: {stated:?}"
    );
}

// ----- Crowded category labels slant (issue #884) -----

/// A column chart whose category labels are far longer than their bands, as
/// the deck in #841 has.
fn crowded_column_chart() -> Chart {
    let mut chart = two_series_bar_chart(Vec::new());
    chart.chart_type = ChartType::Column;
    chart.series.truncate(1);
    chart.categories = vec![
        "Fortjenestemargin".to_string(),
        "Bruttofortjeneste".to_string(),
        "Konverteringsfrekvens for kundeemne".to_string(),
        "Frekvens for kundebevaring".to_string(),
    ];
    chart.series[0].values = vec![33.9, 68.9, 2.4, 9.3];
    chart.title = None;
    chart
}

#[test]
fn crowded_category_labels_slant_by_forty_five_degrees() {
    let source: String = chart_source(crowded_column_chart());
    assert!(
        source.contains("rotate(-45deg, origin: top + right"),
        "labels longer than their band must slant, got:\n{source}"
    );
}

#[test]
fn category_labels_that_fit_their_band_stay_flat() {
    // Triangulation: the same chart type with short labels must not rotate,
    // or the rule is "always rotate" rather than "rotate when crowded".
    let mut chart = crowded_column_chart();
    chart.categories = vec![
        "Q1".to_string(),
        "Q2".to_string(),
        "Q3".to_string(),
        "Q4".to_string(),
    ];
    let source: String = chart_source(chart);
    assert!(
        !source.contains("rotate(-45deg"),
        "short labels must stay flat, got:\n{source}"
    );
}

// ----- A chart's declared text colour (issue #916) -----

#[test]
fn axis_labels_take_the_declared_chart_text_colour() {
    // The deck in #841 sets its chart text white against a dark chart area;
    // the tick labels printed black because no colour was ever parsed.
    let mut chart = sized_bar_chart(11.0);
    chart.text_style.color = Some(crate::ir::Color::new(255, 255, 255));
    let source: String = chart_source(chart);
    assert!(
        source.contains("#text(size: 11pt, fill: rgb(255, 255, 255))[Q1]"),
        "a category label must take the declared colour, got:\n{source}"
    );
    assert!(
        source.contains("fill: rgb(255, 255, 255))[0]"),
        "a value tick label must take it too, got:\n{source}"
    );
}

#[test]
fn an_axis_colour_overrides_the_chart_space_colour_for_that_axis_only() {
    let mut chart = sized_bar_chart(11.0);
    chart.text_style.color = Some(crate::ir::Color::new(255, 255, 255));
    chart.category_axis_text_style.color = Some(crate::ir::Color::new(255, 0, 0));
    let source: String = chart_source(chart);
    assert!(
        source.contains("#text(size: 11pt, fill: rgb(255, 0, 0))[Q1]"),
        "the category axis' own colour must win, got:\n{source}"
    );
    assert!(
        source.contains("fill: rgb(255, 255, 255))[0]"),
        "the value axis must keep the chart-space colour, got:\n{source}"
    );
}

#[test]
fn a_chart_declaring_no_colour_keeps_the_colours_it_had() {
    // Triangulation: the fix must not force a default onto every chart. An
    // axis label stays uncoloured and a data label stays the hardcoded white.
    let mut chart = sized_bar_chart(11.0);
    // The white only appears where a data label is drawn at all.
    chart.series[0].data_labels.show_value = true;
    let source: String = chart_source(chart);
    // Bars carry their series colour as a fill and a legend swatch sits on the
    // same line as its label, so the question is only about what is inside a
    // `#text(...)` argument list.
    let coloured_runs: Vec<&str> = source
        .match_indices("#text(")
        .filter_map(|(at, _)| {
            let args = &source[at + "#text(".len()..];
            let args = &args[..args.find(')')?];
            // `fill: white` is the data label's long-standing default and is
            // asserted separately below; only a resolved colour would be new.
            args.contains("fill: rgb(").then_some(args)
        })
        .collect();
    assert!(
        coloured_runs.is_empty(),
        "nothing declared must leave text uncoloured, got: {coloured_runs:?}"
    );
    assert!(
        source.contains("fill: white)"),
        "the data label keeps its white, got:\n{source}"
    );
}

/// A data label was written at a literal 8pt, so a chart declaring anything
/// else drew its labels at the wrong size — smaller than its own axis on the
/// deck of #841, which asks for 11.97pt everywhere (issue #970).
#[test]
fn a_data_label_is_set_at_the_size_its_dlbls_declare() {
    let mut chart = labelled_chart(DataLabels {
        show_value: true,
        text_style: crate::ir::ChartTextStyle {
            size_pt: Some(11.97),
            ..crate::ir::ChartTextStyle::default()
        },
        ..DataLabels::default()
    });
    chart.series[0].data_labels.text_style.size_pt = Some(11.97);
    let source = chart_source(chart);

    assert!(
        source.contains("#text(size: 11.97pt, weight: \"bold\""),
        "{source}"
    );
    assert!(
        !source.contains("#text(size: 8pt, weight: \"bold\""),
        "{source}"
    );
    // The label box has to grow with the text or the larger glyphs centre on
    // a box sized for 8pt: 11.97 x 1.25 is 14.9625pt.
    assert!(source.contains("height: 14.9625pt"), "{source}");
}

/// A `<c:dLbls>` stating no size takes the chart space's, and only a chart
/// stating nothing anywhere keeps the unmeasured 8pt the labels were pinned
/// at — reading a declared size must not resize charts that declare none.
#[test]
fn a_data_label_declaring_no_size_falls_back_to_the_chart_space() {
    let mut chart = labelled_chart(DataLabels {
        show_value: true,
        ..DataLabels::default()
    });
    chart.text_style.size_pt = Some(18.0);
    assert!(chart_source(chart).contains("#text(size: 18pt, weight: \"bold\""),);

    let neither = chart_source(labelled_chart(DataLabels {
        show_value: true,
        ..DataLabels::default()
    }));
    assert!(
        neither.contains("#text(size: 8pt, weight: \"bold\""),
        "{neither}"
    );
}

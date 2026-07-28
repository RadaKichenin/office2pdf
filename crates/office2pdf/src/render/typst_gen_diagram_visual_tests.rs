use super::*;

#[test]
fn test_codegen_chart_bar_visual_bars() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Bar,
        title: Some("Sales Report".to_string()),
        categories: vec!["Q1".to_string(), "Q2".to_string()],
        series: vec![ChartSeries {
            name: Some("Revenue".to_string()),
            values: vec![100.0, 250.0],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: Some("My Bar Chart".to_string()),
        categories: vec!["1st Qtr".to_string(), "2nd Qtr".to_string()],
        series: vec![ChartSeries {
            name: Some("Sales".to_string()),
            values: vec![8.200000000000001, 3.2],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
    })])]);

    let output = generate_typst(&doc).unwrap();
    // Bars carry no in-plot value labels (like PowerPoint), so the raw float
    // never reaches the output.
    assert!(
        !output.source.contains("8.200000000000001"),
        "raw float must not leak; got:\n{}",
        output.source
    );
    // Nice axis for max 8.2 → ticks 0,2,4,6,8,10.
    for tick in ["[0]", "[2]", "[10]"] {
        assert!(
            output.source.contains(tick),
            "expected axis tick {tick}; got:\n{}",
            output.source
        );
    }
}

#[test]
fn test_codegen_chart_pie_percentages() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Pie,
        title: Some("Market Share".to_string()),
        categories: vec!["A".to_string(), "B".to_string()],
        series: vec![ChartSeries {
            name: None,
            values: vec![60.0, 40.0],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
    })])]);

    let output = generate_typst(&doc).unwrap();
    assert!(
        output.source.contains("Pie Chart"),
        "Expected pie chart label, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("60") && output.source.contains("%"),
        "Expected percentage in pie chart, got:\n{}",
        output.source
    );
    assert!(
        output.source.contains("40") && output.source.contains("%"),
        "Expected percentage in pie chart, got:\n{}",
        output.source
    );
}

#[test]
fn test_codegen_chart_line_trend_indicators() {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(Chart {
        chart_type: ChartType::Line,
        title: Some("Trends".to_string()),
        categories: vec!["Jan".to_string(), "Feb".to_string(), "Mar".to_string()],
        series: vec![ChartSeries {
            name: Some("Sales".to_string()),
            values: vec![10.0, 20.0, 15.0],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: Some("Empty".to_string()),
        categories: vec![],
        series: vec![],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: Some("Quarterly Units Shipped".to_string()),
        categories: vec![
            "Northlake".to_string(),
            "Eastport".to_string(),
            "Southgate".to_string(),
        ],
        series: vec![ChartSeries {
            name: Some("Units".to_string()),
            values: vec![23334.0, 8331.0, 2727.0],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: Some("Fixture Documents by Format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![ChartSeries {
            name: None,
            values: vec![115.0, 92.0, 138.0],
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: None,
        categories: vec!["1".to_string(), "2".to_string(), "3".to_string()],
        series: vec![
            ChartSeries {
                name: Some("A".to_string()),
                values: vec![1.0, 2.0, 3.0],
            },
            ChartSeries {
                name: Some("B".to_string()),
                values: vec![10.0, 9.0, 14.0],
            },
        ],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
        title: Some("Sixty Sample Sites".to_string()),
        categories: categories.clone(),
        series: vec![ChartSeries {
            name: Some("Reading".to_string()),
            values: (1..=60).map(|value| value as f64).collect(),
        }],
        grouping: ChartGrouping::Clustered,
        legend_position: LegendPosition::Right,
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
fn stacked_support_chart(grouping: ChartGrouping) -> Chart {
    Chart {
        chart_type: ChartType::Column,
        title: Some("Supported elements by format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![
            ChartSeries {
                name: Some("Text".to_string()),
                values: vec![4.0, 2.0, 2.0],
            },
            ChartSeries {
                name: Some("Tables".to_string()),
                values: vec![1.0, 1.0, 1.0],
            },
            ChartSeries {
                name: Some("Graphics".to_string()),
                values: vec![2.0, 4.0, 0.0],
            },
            ChartSeries {
                name: Some("Structure".to_string()),
                values: vec![2.0, 2.0, 3.0],
            },
        ],
        grouping,
        legend_position: LegendPosition::Right,
    }
}

fn chart_source(chart: Chart) -> String {
    let doc = make_doc(vec![make_flow_page(vec![Block::Chart(chart)])]);
    generate_typst(&doc).unwrap().source
}

/// The axis tick labels the generator emitted, in the order written.
fn emitted_axis_ticks(source: &str) -> Vec<f64> {
    source
        .lines()
        .filter(|line| line.contains("#place") && line.contains("text(size: 8pt)"))
        .filter_map(|line| {
            let after = line.rsplit_once("text(size: 8pt)[")?.1;
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
    // spreading across twelve.
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
        title: Some("Supported elements by format".to_string()),
        categories: vec!["DOCX".to_string(), "PPTX".to_string(), "XLSX".to_string()],
        series: vec![
            ChartSeries {
                name: Some("Text".to_string()),
                values: vec![4.0, 2.0, 2.0],
            },
            ChartSeries {
                name: Some("Tables".to_string()),
                values: vec![1.0, 1.0, 1.0],
            },
        ],
        grouping: ChartGrouping::Stacked,
        legend_position: position,
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

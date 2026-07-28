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

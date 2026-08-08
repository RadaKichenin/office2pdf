use super::*;

#[test]
fn test_parse_bar_chart() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:title><c:tx><c:rich><a:p><a:r><a:t>Sales Data</a:t></a:r></a:p></c:rich></c:tx></c:title>
                <c:plotArea>
                    <c:barChart>
                        <c:barDir val="col"/>
                        <c:grouping val="clustered"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:tx><c:strRef><c:strCache><c:pt idx="0"><c:v>Revenue</c:v></c:pt></c:strCache></c:strRef></c:tx>
                            <c:cat>
                                <c:strRef><c:strCache>
                                    <c:pt idx="0"><c:v>Q1</c:v></c:pt>
                                    <c:pt idx="1"><c:v>Q2</c:v></c:pt>
                                    <c:pt idx="2"><c:v>Q3</c:v></c:pt>
                                </c:strCache></c:strRef>
                            </c:cat>
                            <c:val>
                                <c:numRef><c:numCache>
                                    <c:pt idx="0"><c:v>100</c:v></c:pt>
                                    <c:pt idx="1"><c:v>200</c:v></c:pt>
                                    <c:pt idx="2"><c:v>150</c:v></c:pt>
                                </c:numCache></c:numRef>
                            </c:val>
                        </c:ser>
                    </c:barChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();
    assert_eq!(chart.chart_type, ChartType::Column);
    assert_eq!(chart.title.as_deref(), Some("Sales Data"));
    assert_eq!(chart.categories, vec!["Q1", "Q2", "Q3"]);
    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].name.as_deref(), Some("Revenue"));
    assert_eq!(chart.series[0].values, vec![100.0, 200.0, 150.0]);
}

#[test]
fn test_parse_pie_chart() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:pieChart>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat>
                                <c:strLit>
                                    <c:pt idx="0"><c:v>Apple</c:v></c:pt>
                                    <c:pt idx="1"><c:v>Banana</c:v></c:pt>
                                    <c:pt idx="2"><c:v>Cherry</c:v></c:pt>
                                </c:strLit>
                            </c:cat>
                            <c:val>
                                <c:numLit>
                                    <c:pt idx="0"><c:v>30</c:v></c:pt>
                                    <c:pt idx="1"><c:v>45</c:v></c:pt>
                                    <c:pt idx="2"><c:v>25</c:v></c:pt>
                                </c:numLit>
                            </c:val>
                        </c:ser>
                    </c:pieChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();
    assert_eq!(chart.chart_type, ChartType::Pie);
    assert!(chart.title.is_none());
    assert_eq!(chart.categories, vec!["Apple", "Banana", "Cherry"]);
    assert_eq!(chart.series[0].values, vec![30.0, 45.0, 25.0]);
}

#[test]
fn test_parse_line_chart_multiple_series() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:title><c:tx><c:rich><a:p><a:r><a:t>Trends</a:t></a:r></a:p></c:rich></c:tx></c:title>
                <c:plotArea>
                    <c:lineChart>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:tx><c:strRef><c:strCache><c:pt idx="0"><c:v>Series A</c:v></c:pt></c:strCache></c:strRef></c:tx>
                            <c:cat>
                                <c:strRef><c:strCache>
                                    <c:pt idx="0"><c:v>Jan</c:v></c:pt>
                                    <c:pt idx="1"><c:v>Feb</c:v></c:pt>
                                </c:strCache></c:strRef>
                            </c:cat>
                            <c:val>
                                <c:numRef><c:numCache>
                                    <c:pt idx="0"><c:v>10</c:v></c:pt>
                                    <c:pt idx="1"><c:v>20</c:v></c:pt>
                                </c:numCache></c:numRef>
                            </c:val>
                        </c:ser>
                        <c:ser>
                            <c:idx val="1"/>
                            <c:tx><c:strRef><c:strCache><c:pt idx="0"><c:v>Series B</c:v></c:pt></c:strCache></c:strRef></c:tx>
                            <c:val>
                                <c:numRef><c:numCache>
                                    <c:pt idx="0"><c:v>15</c:v></c:pt>
                                    <c:pt idx="1"><c:v>25</c:v></c:pt>
                                </c:numCache></c:numRef>
                            </c:val>
                        </c:ser>
                    </c:lineChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();
    assert_eq!(chart.chart_type, ChartType::Line);
    assert_eq!(chart.title.as_deref(), Some("Trends"));
    assert_eq!(chart.categories, vec!["Jan", "Feb"]);
    assert_eq!(chart.series.len(), 2);
    assert_eq!(chart.series[0].name.as_deref(), Some("Series A"));
    assert_eq!(chart.series[0].values, vec![10.0, 20.0]);
    assert_eq!(chart.series[1].name.as_deref(), Some("Series B"));
    assert_eq!(chart.series[1].values, vec![15.0, 25.0]);
}

#[test]
fn test_parse_chart_no_title() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:barChart>
                        <c:barDir val="col"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>42</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:barChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();
    assert!(chart.title.is_none());
    assert_eq!(chart.categories, vec!["A"]);
    assert_eq!(chart.series[0].values, vec![42.0]);
}

/// Wrap a `<c:barChart>` body in the chartSpace scaffolding Excel writes.
fn bar_chart_xml(bar_chart_body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:plotArea>
                    <c:barChart>
                        {bar_chart_body}
                        <c:ser>
                            <c:idx val="0"/>
                            <c:order val="0"/>
                            <c:tx><c:strRef><c:strCache><c:pt idx="0"><c:v>프로덕션 LOC</c:v></c:pt></c:strCache></c:strRef></c:tx>
                            <c:cat>
                                <c:strRef><c:strCache>
                                    <c:pt idx="0"><c:v>parser</c:v></c:pt>
                                    <c:pt idx="1"><c:v>render</c:v></c:pt>
                                    <c:pt idx="2"><c:v>core</c:v></c:pt>
                                </c:strCache></c:strRef>
                            </c:cat>
                            <c:val>
                                <c:numRef><c:numCache>
                                    <c:pt idx="0"><c:v>23334</c:v></c:pt>
                                    <c:pt idx="1"><c:v>8331</c:v></c:pt>
                                    <c:pt idx="2"><c:v>4120</c:v></c:pt>
                                </c:numCache></c:numRef>
                            </c:val>
                        </c:ser>
                    </c:barChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#
    )
}

#[test]
fn test_bar_dir_col_is_a_column_chart() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.chart_type, ChartType::Column);
    assert_eq!(chart.categories, vec!["parser", "render", "core"]);
    assert_eq!(chart.series[0].values, vec![23334.0, 8331.0, 4120.0]);
}

#[test]
fn test_grouping_stacked_is_read() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/><c:grouping val="stacked"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.grouping, ChartGrouping::Stacked);
}

#[test]
fn test_grouping_percent_stacked_is_read() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/><c:grouping val="percentStacked"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.grouping, ChartGrouping::PercentStacked);
}

#[test]
fn test_grouping_clustered_is_read() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.grouping, ChartGrouping::Clustered);
}

#[test]
fn test_grouping_defaults_to_clustered_when_absent() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.grouping, ChartGrouping::Clustered);
}

#[test]
fn test_line_chart_standard_grouping_is_not_stacked() {
    // Line and area charts spell their unstacked form `standard`, which must
    // not fall through to a stacked reading.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:lineChart>
                        <c:grouping val="standard"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Feb</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>3</c:v></c:pt><c:pt idx="1"><c:v>5</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:lineChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();

    assert_eq!(chart.grouping, ChartGrouping::Clustered);
}

#[test]
fn test_bar_dir_bar_is_a_bar_chart() {
    let xml = bar_chart_xml(r#"<c:barDir val="bar"/><c:grouping val="clustered"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.chart_type, ChartType::Bar);
}

#[test]
fn test_bar_dir_defaults_to_column_when_absent() {
    // ECMA-376 gives ST_BarDir's `val` a default of `col`, so a chart that omits
    // the required element is read as a column chart rather than rotated.
    let xml = bar_chart_xml(r#"<c:grouping val="clustered"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.chart_type, ChartType::Column);
}

#[test]
fn test_bar_dir_does_not_leak_into_other_chart_families() {
    // <c:barDir> is exclusive to the bar family; a line chart must stay a line
    // chart even when a bar chart precedes it in the same plot area.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:lineChart>
                        <c:grouping val="standard"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Feb</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>3</c:v></c:pt><c:pt idx="1"><c:v>5</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:lineChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();

    assert_eq!(chart.chart_type, ChartType::Line);
}

#[test]
fn test_bar3d_chart_honours_bar_dir() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:bar3DChart>
                        <c:barDir val="bar"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>7</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:bar3DChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();

    assert_eq!(chart.chart_type, ChartType::Bar);
}

#[test]
fn test_scan_chart_references() {
    let xml = r#"<?xml version="1.0"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                    xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
                    xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                    xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                    xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <w:body>
                <w:p><w:r><w:t>Hello</w:t></w:r></w:p>
                <w:p>
                    <w:r>
                        <w:drawing>
                            <wp:inline>
                                <a:graphic>
                                    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
                                        <c:chart r:id="rId4"/>
                                    </a:graphicData>
                                </a:graphic>
                            </wp:inline>
                        </w:drawing>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

    let refs = scan_chart_references(xml);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].0, 1); // second body child
    assert_eq!(refs[0].1, "rId4");
}

#[test]
fn test_scan_chart_rels() {
    let rels_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
            <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart1.xml"/>
            <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart2.xml"/>
        </Relationships>"#;

    let rels = scan_chart_rels(rels_xml);
    assert_eq!(rels.len(), 2);
    assert_eq!(rels.get("rId4").unwrap(), "word/charts/chart1.xml");
    assert_eq!(rels.get("rId5").unwrap(), "word/charts/chart2.xml");
}

// ----- Legend position (issue #546) -----

/// Wrap a bar chart in a chartSpace carrying `legend_xml` beside the plot area.
fn bar_chart_with_legend(legend_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:barChart>
                        <c:barDir val="col"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>DOCX</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>9</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:barChart>
                </c:plotArea>
                {legend_xml}
            </c:chart>
        </c:chartSpace>"#
    )
}

#[test]
fn test_legend_pos_bottom_is_read() {
    let xml =
        bar_chart_with_legend(r#"<c:legend><c:legendPos val="b"/><c:overlay val="0"/></c:legend>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.legend_position, LegendPosition::Bottom);
    assert!(chart.legend_position.is_horizontal());
}

#[test]
fn test_every_legend_pos_value_is_mapped() {
    for (val, expected) in [
        ("b", LegendPosition::Bottom),
        ("l", LegendPosition::Left),
        ("r", LegendPosition::Right),
        ("t", LegendPosition::Top),
        ("tr", LegendPosition::TopRight),
    ] {
        let xml = bar_chart_with_legend(&format!(
            r#"<c:legend><c:legendPos val="{val}"/></c:legend>"#
        ));

        let chart = parse_chart_xml(&xml).unwrap();

        assert_eq!(chart.legend_position, expected, "legendPos val=\"{val}\"");
    }
}

#[test]
fn test_legend_pos_defaults_to_right_when_absent() {
    // ECMA-376 gives ST_LegendPos a default of `r`, which is also where every
    // legend was drawn before the element was read.
    let xml = bar_chart_with_legend(r#"<c:legend><c:overlay val="0"/></c:legend>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.legend_position, LegendPosition::Right);
    assert!(!chart.legend_position.is_horizontal());
}

#[test]
fn test_chart_without_a_legend_element_keeps_the_default() {
    let xml = bar_chart_with_legend("");

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.legend_position, LegendPosition::Right);
}

// ----- Series and data-point fills (issue #535) -----

/// The audited workbook's column chart: one series with an explicit fill.
#[test]
fn test_series_sppr_fill_is_read() {
    let xml = bar_chart_xml(
        r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#,
    )
    .replace(
        "<c:tx>",
        r#"<c:spPr><a:solidFill><a:srgbClr val="4f81bd"/></a:solidFill><a:ln w="9360"><a:solidFill><a:srgbClr val="f9f9f9"/></a:solidFill></a:ln></c:spPr><c:tx>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.series[0].fill, Some(Color::new(0x4f, 0x81, 0xbd)));
}

#[test]
fn test_data_point_fills_override_the_series() {
    // The audited pie's three <c:dPt> entries. The third is what exposed the
    // palette: its declared green landed as the palette's yellow.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:plotArea>
                    <c:pieChart>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:spPr><a:solidFill><a:srgbClr val="111111"/></a:solidFill></c:spPr>
                            <c:dPt><c:idx val="0"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="4f81bd"/></a:solidFill></c:spPr></c:dPt>
                            <c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="c0504d"/></a:solidFill></c:spPr></c:dPt>
                            <c:dPt><c:idx val="2"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="9bbb59"/></a:solidFill></c:spPr></c:dPt>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>DOCX</c:v></c:pt><c:pt idx="1"><c:v>PPTX</c:v></c:pt><c:pt idx="2"><c:v>XLSX</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>115</c:v></c:pt><c:pt idx="1"><c:v>92</c:v></c:pt><c:pt idx="2"><c:v>138</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:pieChart>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#;

    let chart = parse_chart_xml(xml).unwrap();
    let series = &chart.series[0];

    assert_eq!(series.fill_for_point(0), Some(Color::new(0x4f, 0x81, 0xbd)));
    assert_eq!(series.fill_for_point(1), Some(Color::new(0xc0, 0x50, 0x4d)));
    assert_eq!(series.fill_for_point(2), Some(Color::new(0x9b, 0xbb, 0x59)));
    // A point past the declared ones falls back to the series' own fill.
    assert_eq!(series.fill_for_point(3), Some(Color::new(0x11, 0x11, 0x11)));
}

#[test]
fn test_a_series_without_a_fill_leaves_the_palette_to_decide() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.series[0].fill, None);
    assert_eq!(chart.series[0].fill_for_point(0), None);
}

#[test]
fn test_a_theme_colour_fill_falls_through_to_the_palette() {
    // `<a:schemeClr>` needs the chart part's own theme, which this parser does
    // not resolve, so it must not be mistaken for an explicit colour.
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#).replace(
        "<c:tx>",
        r#"<c:spPr><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></c:spPr><c:tx>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.series[0].fill, None);
}

#[test]
fn test_a_data_label_fill_is_not_mistaken_for_the_series_fill() {
    // `<c:dLbls>` carries an `<c:spPr>` for the label box. The series-level
    // match is flat, so without consuming the element that fill would be read
    // as the series' own — and this series declares none of its own.
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#).replace(
        "<c:cat>",
        r#"<c:dLbls><c:spPr><a:solidFill><a:srgbClr val="ff0000"/></a:solidFill></c:spPr><c:showVal val="1"/></c:dLbls><c:cat>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.series[0].fill, None);
    assert_eq!(chart.series[0].values, vec![23334.0, 8331.0, 4120.0]);
}

#[test]
fn test_a_data_label_fill_does_not_shadow_a_declared_series_fill() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#)
        .replace(
            "<c:tx>",
            r#"<c:spPr><a:solidFill><a:srgbClr val="4f81bd"/></a:solidFill></c:spPr><c:tx>"#,
        )
        .replace(
            "<c:cat>",
            r#"<c:dLbls><c:spPr><a:solidFill><a:srgbClr val="ff0000"/></a:solidFill></c:spPr></c:dLbls><c:cat>"#,
        );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.series[0].fill, Some(Color::new(0x4f, 0x81, 0xbd)));
}

// ----- Unmapped chart families (issue #544) -----

/// Wrap `chart_element` — a whole `<c:fooChart>` — in a chartSpace.
fn chart_of_type(chart_element: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart>
                <c:plotArea>
                    <c:{chart_element}>
                        {body}
                        <c:ser>
                            <c:idx val="0"/>
                            <c:tx><c:strRef><c:strCache><c:pt idx="0"><c:v>Coverage</c:v></c:pt></c:strCache></c:strRef></c:tx>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>Text</c:v></c:pt><c:pt idx="1"><c:v>Table</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>9</c:v></c:pt><c:pt idx="1"><c:v>6</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:{chart_element}>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#
    )
}

#[test]
fn test_a_radar_chart_survives_with_its_data() {
    // The deck's slide 27 loses its entire left half when this returns None.
    let xml = chart_of_type("radarChart", r#"<c:radarStyle val="marker"/>"#);

    let chart = parse_chart_xml(&xml).expect("a radar chart must not be dropped");

    assert_eq!(
        chart.chart_type,
        ChartType::Other("Radar Chart".to_string())
    );
    assert_eq!(chart.categories, vec!["Text", "Table"]);
    assert_eq!(chart.series[0].values, vec![9.0, 6.0]);
}

#[test]
fn test_a_doughnut_chart_survives_with_its_data() {
    let xml = chart_of_type("doughnutChart", r#"<c:holeSize val="50"/>"#);

    let chart = parse_chart_xml(&xml).expect("a doughnut chart must not be dropped");

    // It was `Other("Doughnut Chart")` while the tabular fallback stood in for
    // it (issue #544). It is a plotted family now, so it carries its own type
    // and its hole size rather than a caption string (issue #679).
    assert_eq!(chart.chart_type, ChartType::Doughnut);
    assert_eq!(chart.hole_size_percent, Some(50));
    assert_eq!(chart.series[0].values, vec![9.0, 6.0]);
}

/// A doughnut with no `<c:holeSize>` still plots. The parser records the
/// absence as `None`; the renderer substitutes a placeholder rather than
/// treating it as zero, which would collapse the ring into a pie. That
/// placeholder is not a measured default — see `doughnut_inner_radius`.
#[test]
fn test_a_doughnut_chart_without_a_hole_size_still_plots() {
    let xml = chart_of_type("doughnutChart", "");

    let chart = parse_chart_xml(&xml).expect("a doughnut chart must not be dropped");

    assert_eq!(chart.chart_type, ChartType::Doughnut);
    assert_eq!(chart.hole_size_percent, None);
}

/// Triangulation: a pie is not a doughnut, so it carries no hole size and the
/// new field cannot be a constant.
#[test]
fn test_a_pie_chart_carries_no_hole_size() {
    let xml = chart_of_type("pieChart", "");

    let chart = parse_chart_xml(&xml).expect("a pie chart parses");

    assert_eq!(chart.chart_type, ChartType::Pie);
    assert_eq!(chart.hole_size_percent, None);
}

#[test]
fn test_every_schema_chart_family_is_recognised() {
    // ECMA-376's full CT_PlotArea group. None may return None.
    for element in [
        "areaChart",
        "area3DChart",
        "lineChart",
        "line3DChart",
        "stockChart",
        "radarChart",
        "scatterChart",
        "pieChart",
        "pie3DChart",
        "doughnutChart",
        "barChart",
        "bar3DChart",
        "ofPieChart",
        "surfaceChart",
        "surface3DChart",
        "bubbleChart",
    ] {
        let xml = chart_of_type(element, "");

        let chart =
            parse_chart_xml(&xml).unwrap_or_else(|| panic!("<c:{element}> must not be dropped"));

        assert_eq!(
            chart.series[0].values,
            vec![9.0, 6.0],
            "<c:{element}> must keep its data"
        );
    }
}

#[test]
fn test_an_unmapped_family_keeps_a_readable_label() {
    // The label is what the data-table fallback prints as the chart's kind.
    for (element, expected) in [
        ("bubbleChart", "Bubble Chart"),
        ("stockChart", "Stock Chart"),
        ("surface3DChart", "Surface Chart"),
    ] {
        let xml = chart_of_type(element, "");

        let chart = parse_chart_xml(&xml).unwrap();

        assert_eq!(
            chart.chart_type,
            ChartType::Other(expected.to_string()),
            "<c:{element}>"
        );
    }
}

#[test]
fn test_a_non_chart_element_is_still_not_a_chart() {
    // A part with no plot-area family in it yields no chart, so the suffix
    // rule cannot be opening one on `chartSpace` or `chart` themselves.
    let xml = r#"<?xml version="1.0"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart><c:plotArea><c:layout/></c:plotArea></c:chart>
        </c:chartSpace>"#;

    assert!(parse_chart_xml(xml).is_none());
}

#[test]
fn test_of_pie_names_the_shape_its_ofpietype_asks_for() {
    // One element, two shapes. Without reading `<c:ofPieType>` a bar-of-pie
    // chart would be labelled as its sibling.
    for (of_pie_type, expected) in [
        (r#"<c:ofPieType val="bar"/>"#, "Bar of Pie Chart"),
        (r#"<c:ofPieType val="pie"/>"#, "Pie of Pie Chart"),
        // ECMA-376 defaults ST_OfPieType to `pie`.
        ("", "Pie of Pie Chart"),
    ] {
        let xml = chart_of_type("ofPieChart", of_pie_type);

        let chart = parse_chart_xml(&xml).unwrap();

        assert_eq!(
            chart.chart_type,
            ChartType::Other(expected.to_string()),
            "ofPieType {of_pie_type:?}"
        );
    }
}

// ----- Axis titles (issue #552) -----

/// The audited workbook's column chart, with `cat_ax_body` and `val_ax_body`
/// spliced into the matching axis element.
fn chart_with_axes(cat_ax_body: &str, val_ax_body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:title><c:tx><c:rich><a:p><a:r><a:t>계층별 프로덕션 LOC</a:t></a:r></a:p></c:rich></c:tx></c:title>
                <c:plotArea>
                    <c:barChart>
                        <c:barDir val="col"/>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:cat><c:strLit><c:pt idx="0"><c:v>parser</c:v></c:pt></c:strLit></c:cat>
                            <c:val><c:numLit><c:pt idx="0"><c:v>23334</c:v></c:pt></c:numLit></c:val>
                        </c:ser>
                    </c:barChart>
                    <c:catAx>
                        <c:axId val="59291440"/>
                        <c:axPos val="b"/>
                        {cat_ax_body}
                        <c:tickLblPos val="nextTo"/>
                    </c:catAx>
                    <c:valAx>
                        <c:axId val="21056836"/>
                        <c:axPos val="l"/>
                        {val_ax_body}
                        <c:tickLblPos val="nextTo"/>
                    </c:valAx>
                </c:plotArea>
            </c:chart>
        </c:chartSpace>"#
    )
}

/// A `<c:title>` as Excel writes it inside an axis.
fn axis_title(text: &str, rotation: &str) -> String {
    format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr rot="{rotation}"/><a:lstStyle/><a:p><a:r><a:rPr b="1" sz="1000"/><a:t>{text}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>"#
    )
}

#[test]
fn test_axis_titles_are_read() {
    let xml = chart_with_axes(&axis_title("계층", "0"), &axis_title("LOC", "-5400000"));

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_title.as_deref(), Some("계층"));
    assert_eq!(chart.value_axis_title.as_deref(), Some("LOC"));
}

#[test]
fn test_an_axis_title_does_not_displace_the_chart_title() {
    // The chart title is the first `<c:title>` in the part; the axis ones come
    // later and must not be mistaken for it, nor it for them.
    let xml = chart_with_axes(&axis_title("계층", "0"), &axis_title("LOC", "-5400000"));

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.title.as_deref(), Some("계층별 프로덕션 LOC"));
}

#[test]
fn test_axes_without_titles_report_none() {
    let xml = chart_with_axes("", "");

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_title, None);
    assert_eq!(chart.value_axis_title, None);
    assert_eq!(chart.title.as_deref(), Some("계층별 프로덕션 LOC"));
}

#[test]
fn test_one_titled_axis_does_not_borrow_the_others_title() {
    let xml = chart_with_axes("", &axis_title("LOC", "-5400000"));

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_title, None);
    assert_eq!(chart.value_axis_title.as_deref(), Some("LOC"));
}

// ----- Major tick marks (issue #672) -----

#[test]
fn test_outward_major_tick_marks_are_read() {
    // Both fixtures in issue #672 spell it this way, and it is what Office
    // writes for a default chart.
    let xml = chart_with_axes(
        r#"<c:majorTickMark val="out"/>"#,
        r#"<c:majorTickMark val="out"/>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_major_tick_mark, AxisTickMark::Outside);
    assert_eq!(chart.value_axis_major_tick_mark, AxisTickMark::Outside);
}

#[test]
fn test_each_axis_keeps_its_own_major_tick_mark() {
    // Triangulation: the two axes are independent, and neither `none` nor `in`
    // may collapse onto the `out` the common case uses.
    let xml = chart_with_axes(
        r#"<c:majorTickMark val="none"/>"#,
        r#"<c:majorTickMark val="in"/>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_major_tick_mark, AxisTickMark::None);
    assert_eq!(chart.value_axis_major_tick_mark, AxisTickMark::Inside);
}

#[test]
fn test_crossing_major_tick_marks_are_read() {
    let xml = chart_with_axes(
        r#"<c:majorTickMark val="cross"/>"#,
        r#"<c:majorTickMark val="cross"/>"#,
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_major_tick_mark, AxisTickMark::Cross);
    assert_eq!(chart.value_axis_major_tick_mark, AxisTickMark::Cross);
}

#[test]
fn test_an_axis_without_the_element_still_ticks_outward() {
    // Excel 16.0 exports `tests/fixtures/xlsx/WithChart.xlsx` — written by
    // Apache POI without a single `<c:majorTickMark>` — with 3.17pt outward
    // ticks on both axes, so the rendered default is `out`, not the `cross`
    // ECMA-376 gives the attribute.
    let xml = chart_with_axes("", "");

    let chart = parse_chart_xml(&xml).unwrap();

    assert_eq!(chart.category_axis_major_tick_mark, AxisTickMark::Outside);
    assert_eq!(chart.value_axis_major_tick_mark, AxisTickMark::Outside);
}

// ----- Switched-off axes (issue #672) -----

/// A `<c:delete>` element in the state `on` describes.
fn axis_delete(on: bool) -> String {
    format!(r#"<c:delete val="{}"/>"#, u8::from(on))
}

#[test]
fn test_a_switched_off_axis_is_read_as_deleted() {
    // Office leaves the rest of a switched-off axis' settings in place, tick
    // marks included, so the flag is the only thing that says it is gone.
    let xml = chart_with_axes(
        &format!(r#"{}<c:majorTickMark val="out"/>"#, axis_delete(true)),
        &format!(r#"{}<c:majorTickMark val="out"/>"#, axis_delete(false)),
    );

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(chart.category_axis_deleted);
    assert!(!chart.value_axis_deleted);
    assert_eq!(chart.category_axis_major_tick_mark, AxisTickMark::Outside);
}

#[test]
fn test_the_value_axis_switches_off_independently_of_the_category_one() {
    // Triangulation: a bar chart with only the value axis switched off is the
    // common authoring pattern, so neither flag may stand for both.
    let xml = chart_with_axes(&axis_delete(false), &axis_delete(true));

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(!chart.category_axis_deleted);
    assert!(chart.value_axis_deleted);
}

#[test]
fn test_an_axis_without_the_element_stays_on() {
    let xml = chart_with_axes("", "");

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(!chart.category_axis_deleted);
    assert!(!chart.value_axis_deleted);
}

#[test]
fn test_the_element_without_its_attribute_switches_the_axis_off() {
    // `CT_Boolean/@val` defaults to true, so the attribute is optional.
    let xml = chart_with_axes("<c:delete/>", "");

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(chart.category_axis_deleted);
    assert!(!chart.value_axis_deleted);
}

// ----- Data labels (issue #547) -----

/// A bar chart whose series carries `dlbls_xml`.
fn chart_with_data_labels(dlbls_xml: &str) -> String {
    bar_chart_xml(r#"<c:barDir val="col"/>"#).replace("<c:cat>", &format!("{dlbls_xml}<c:cat>"))
}

#[test]
fn test_show_val_turns_the_value_on() {
    // Slide 17's series: `<c:showVal val="1"/>` with everything else off.
    let xml = chart_with_data_labels(
        r#"<c:dLbls><c:dLblPos val="ctr"/><c:showLegendKey val="0"/><c:showVal val="1"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/></c:dLbls>"#,
    );

    let labels = &parse_chart_xml(&xml).unwrap().series[0].data_labels;

    assert!(labels.show_value);
    assert!(!labels.show_category && !labels.show_series && !labels.show_percent);
    assert!(!labels.is_empty());
}

#[test]
fn test_a_series_without_dlbls_prints_nothing() {
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#);

    assert!(
        parse_chart_xml(&xml).unwrap().series[0]
            .data_labels
            .is_empty()
    );
}

#[test]
fn test_all_dlbls_parts_and_the_separator_are_read() {
    // The workbook's charts turn several on at once and set a separator.
    let xml = chart_with_data_labels(
        r#"<c:dLbls><c:showVal val="1"/><c:showCatName val="1"/><c:showSerName val="1"/><c:showPercent val="1"/><c:separator>; </c:separator></c:dLbls>"#,
    );

    let labels = &parse_chart_xml(&xml).unwrap().series[0].data_labels;

    assert!(labels.show_value && labels.show_category && labels.show_series && labels.show_percent);
    assert_eq!(labels.separator, "; ");
}

#[test]
fn test_a_bare_show_flag_defaults_to_on() {
    // ECMA-376 defaults CT_Boolean's `val` to true, so `<c:showVal/>` counts.
    let xml = chart_with_data_labels(r#"<c:dLbls><c:showVal/></c:dLbls>"#);

    assert!(
        parse_chart_xml(&xml).unwrap().series[0]
            .data_labels
            .show_value
    );
}

#[test]
fn test_data_labels_do_not_disturb_the_series_fill() {
    // `<c:dLbls>` is read rather than skipped now, so the guard that keeps its
    // `<c:spPr>` out of the series fill has to still hold.
    let xml = bar_chart_xml(r#"<c:barDir val="col"/>"#).replace(
        "<c:cat>",
        r#"<c:dLbls><c:spPr><a:solidFill><a:srgbClr val="ff0000"/></a:solidFill></c:spPr><c:showVal val="1"/></c:dLbls><c:cat>"#,
    );

    let series = &parse_chart_xml(&xml).unwrap().series[0];

    assert_eq!(
        series.fill, None,
        "the label box fill is not the series fill"
    );
    assert!(series.data_labels.show_value);
}

#[test]
fn test_a_per_point_label_override_is_not_the_series_default() {
    // `CT_DLbls` is `dLbl*` then the group-level settings, and `<c:dLbl>`
    // repeats the same element names. A block carrying only a per-point
    // override must leave the series printing nothing.
    let xml = chart_with_data_labels(
        r#"<c:dLbls><c:dLbl><c:idx val="0"/><c:showVal val="1"/><c:showCatName val="1"/></c:dLbl></c:dLbls>"#,
    );

    let labels = &parse_chart_xml(&xml).unwrap().series[0].data_labels;

    assert!(
        labels.is_empty(),
        "one point's override is not the series default: {labels:?}"
    );
}

#[test]
fn test_group_settings_still_win_after_per_point_overrides() {
    let xml = chart_with_data_labels(
        r#"<c:dLbls><c:dLbl><c:idx val="0"/><c:showCatName val="1"/></c:dLbl><c:showVal val="1"/></c:dLbls>"#,
    );

    let labels = &parse_chart_xml(&xml).unwrap().series[0].data_labels;

    assert!(labels.show_value, "the group-level showVal applies");
    assert!(
        !labels.show_category,
        "the point's showCatName is not the series default: {labels:?}"
    );
}

// ----- Bar band layout (issue #671) -----

/// A column chart declaring `bar_layout_xml` where Office writes it: after the
/// last `</c:ser>`, ahead of the axis ids.
fn chart_with_bar_layout(bar_layout_xml: &str) -> String {
    bar_chart_xml(r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#).replace(
        "</c:ser>",
        &format!("</c:ser>{bar_layout_xml}<c:axId val=\"59291440\"/>"),
    )
}

#[test]
fn test_gap_width_and_overlap_are_read_from_the_bar_chart() {
    // What Office writes for a modern clustered chart, and what six of this
    // repository's bar-chart fixtures carry verbatim.
    let xml = chart_with_bar_layout(r#"<c:gapWidth val="219"/><c:overlap val="-27"/>"#);

    let layout = parse_chart_xml(&xml).unwrap().bar_band_layout;

    assert_eq!(layout.gap_width_percent, 219.0);
    assert_eq!(layout.overlap_percent, -27.0);
}

#[test]
fn test_a_different_declaration_gives_a_different_layout() {
    // Triangulation against the fixture in the issue: `bar-chart.pptx` declares
    // a gap of 100 and no overlap at all.
    let xml = chart_with_bar_layout(r#"<c:gapWidth val="100"/>"#);

    let layout = parse_chart_xml(&xml).unwrap().bar_band_layout;

    assert_eq!(layout.gap_width_percent, 100.0);
    assert_eq!(layout.overlap_percent, 0.0);
}

#[test]
fn test_a_bar_chart_without_the_elements_takes_the_office_defaults() {
    // `tests/fixtures/xlsx/chart_sheet.xlsx` declares neither element, and
    // Excel 16.0 exports it at exactly 150 / 0.
    let xml = chart_with_bar_layout("");

    let layout = parse_chart_xml(&xml).unwrap().bar_band_layout;

    assert_eq!(layout.gap_width_percent, 150.0);
    assert_eq!(layout.overlap_percent, 0.0);
}

#[test]
fn test_a_percent_suffixed_amount_is_the_same_number() {
    // `ST_GapAmount` and `ST_Overlap` are unions of a bare integer and a
    // percentage string, so `"90%"` and `"90"` describe the same chart.
    let xml = chart_with_bar_layout(r#"<c:gapWidth val="90%"/><c:overlap val="100%"/>"#);

    let layout = parse_chart_xml(&xml).unwrap().bar_band_layout;

    assert_eq!(layout.gap_width_percent, 90.0);
    assert_eq!(layout.overlap_percent, 100.0);
}

#[test]
fn test_values_past_the_schema_bounds_are_pulled_back() {
    // PowerPoint 16.0 refuses to open a file whose gapWidth reads 1000 while
    // opening 500 happily, so a value outside `ST_GapAmount` describes no
    // drawable chart and the nearest bound is the honest reading of it.
    let over = chart_with_bar_layout(r#"<c:gapWidth val="1000"/><c:overlap val="400"/>"#);
    let under = chart_with_bar_layout(r#"<c:gapWidth val="-40"/><c:overlap val="-500"/>"#);

    let over_layout = parse_chart_xml(&over).unwrap().bar_band_layout;
    let under_layout = parse_chart_xml(&under).unwrap().bar_band_layout;

    assert_eq!(over_layout.gap_width_percent, 500.0);
    assert_eq!(over_layout.overlap_percent, 100.0);
    assert_eq!(under_layout.gap_width_percent, 0.0);
    assert_eq!(under_layout.overlap_percent, -100.0);
}

#[test]
fn test_an_unreadable_amount_leaves_the_default_standing() {
    let xml = chart_with_bar_layout(r#"<c:gapWidth val="wide"/>"#);

    assert_eq!(
        parse_chart_xml(&xml).unwrap().bar_band_layout,
        BarBandLayout::default()
    );
}

#[test]
fn test_a_trailing_line_chart_keeps_the_bar_layout() {
    // A combo plot area holds one element per chart family, and only the bar
    // family carries these two. The `<c:lineChart>` that follows must leave
    // what the bars asked for standing.
    let xml = chart_with_bar_layout(r#"<c:gapWidth val="90"/><c:overlap val="100"/>"#).replace(
        "</c:barChart>",
        r#"</c:barChart><c:lineChart><c:grouping val="standard"/><c:ser><c:idx val="1"/><c:val><c:numLit><c:pt idx="0"><c:v>7</c:v></c:pt></c:numLit></c:val></c:ser></c:lineChart>"#,
    );

    let layout = parse_chart_xml(&xml).unwrap().bar_band_layout;

    assert_eq!(layout.gap_width_percent, 90.0);
    assert_eq!(layout.overlap_percent, 100.0);
}

// ── `<c:legend>` presence (issue #762) ─────────────────────────────────

/// A declared legend is a legend, whatever edge it names.
#[test]
fn test_declared_legend_is_recorded() {
    let xml =
        bar_chart_with_legend(r#"<c:legend><c:legendPos val="b"/><c:overlay val="0"/></c:legend>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(chart.has_legend);
    // The position still arrives: reading the legend body must not swallow it.
    assert_eq!(chart.legend_position, LegendPosition::Bottom);
}

/// No `<c:legend>` at all. `legend_position` falls back to its default either
/// way, which is why the presence flag exists.
#[test]
fn test_absent_legend_is_not_recorded() {
    let chart = parse_chart_xml(&bar_chart_with_legend("")).unwrap();

    assert!(!chart.has_legend);
    assert_eq!(chart.legend_position, LegendPosition::Right);
}

/// `<c:delete val="1"/>` switches a declared legend off while keeping its
/// settings, the same shape the axes use.
#[test]
fn test_deleted_legend_is_not_recorded() {
    let xml =
        bar_chart_with_legend(r#"<c:legend><c:legendPos val="b"/><c:delete val="1"/></c:legend>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(!chart.has_legend);
    // Its settings survive, so switching it back on would not lose the edge.
    assert_eq!(chart.legend_position, LegendPosition::Bottom);
}

/// `<c:delete val="0"/>` is an explicit "keep it".
#[test]
fn test_undeleted_legend_is_recorded() {
    let xml =
        bar_chart_with_legend(r#"<c:legend><c:legendPos val="t"/><c:delete val="0"/></c:legend>"#);

    let chart = parse_chart_xml(&xml).unwrap();

    assert!(chart.has_legend);
    assert_eq!(chart.legend_position, LegendPosition::Top);
}

/// A minimal chart part, with `body` spliced in after `</c:chart>` — which is
/// where the schema puts `c:chartSpace/c:spPr`.
fn chart_space_with(trailing: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:plotArea>
                    <c:spPr><a:ln w="76200"><a:solidFill><a:srgbClr val="ff0000"/></a:solidFill></a:ln></c:spPr>
                    <c:lineChart>
                        <c:ser>
                            <c:idx val="0"/>
                            <c:val><c:numRef><c:numCache>
                                <c:pt idx="0"><c:v>1</c:v></c:pt>
                                <c:pt idx="1"><c:v>2</c:v></c:pt>
                            </c:numCache></c:numRef></c:val>
                        </c:ser>
                    </c:lineChart>
                </c:plotArea>
            </c:chart>
            {trailing}
        </c:chartSpace>"#
    )
}

#[test]
fn a_chart_with_no_chart_space_line_takes_the_default_outline() {
    // The plot area in the fixture above declares a fat red `a:ln` of its own.
    // The event loop is flat, so reading the first `c:spPr` it meets would pick
    // that one up and report the chart area as red (#637).
    let chart = parse_chart_xml(&chart_space_with("")).expect("chart parses");
    assert_eq!(chart.chart_area_outline, ChartAreaOutline::Default);
}

#[test]
fn a_chart_space_no_fill_suppresses_the_outline() {
    let chart = parse_chart_xml(&chart_space_with(
        "<c:spPr><a:ln><a:noFill/></a:ln></c:spPr>",
    ))
    .expect("chart parses");
    assert_eq!(chart.chart_area_outline, ChartAreaOutline::Suppressed);
}

#[test]
fn a_chart_space_line_keeps_its_width_and_colour() {
    // 9360 EMU is the width `office2pdf_repository_workbook.xlsx` declares, and
    // #d9d9d9 its colour.
    let chart = parse_chart_xml(&chart_space_with(
        r#"<c:spPr><a:ln w="9360"><a:solidFill><a:srgbClr val="d9d9d9"/></a:solidFill></a:ln></c:spPr>"#,
    ))
    .expect("chart parses");
    match chart.chart_area_outline {
        ChartAreaOutline::Explicit { width_pt, color } => {
            let width = width_pt.expect("a declared width reaches the model");
            assert!(
                (width - 9360.0 / 12700.0).abs() < 1e-9,
                "9360 EMU is {width}pt, expected {}",
                9360.0 / 12700.0
            );
            assert_eq!(color, Some(Color::new(0xd9, 0xd9, 0xd9)));
        }
        other => panic!("expected an explicit outline, got {other:?}"),
    }
}

// ----- Chart text's declared face (issue #668) -----

#[test]
fn a_chart_space_tx_pr_latin_typeface_reaches_the_model() {
    // `office2pdf_introduction_ko.pptx`'s chart1.xml names the face outright
    // rather than deferring to the theme.
    let chart = parse_chart_xml(&chart_space_with(
        r#"<c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr>
             <a:defRPr><a:latin typeface="Calibri"/></a:defRPr>
           </a:pPr><a:endParaRPr lang="en-US"/></a:p></c:txPr>"#,
    ))
    .expect("chart parses");
    assert_eq!(chart.text_font_family.as_deref(), Some("Calibri"));
}

#[test]
fn a_chart_space_tx_pr_keeps_the_unresolved_theme_token() {
    // The chart part names no theme of its own, so the parser cannot turn
    // `+mn-lt` into a face. Whoever loaded the part resolves it.
    let chart = parse_chart_xml(&chart_space_with(
        r#"<c:txPr><a:p><a:pPr><a:defRPr><a:latin typeface="+mn-lt"/></a:defRPr></a:pPr></a:p></c:txPr>"#,
    ))
    .expect("chart parses");
    assert_eq!(chart.text_font_family.as_deref(), Some("+mn-lt"));
}

#[test]
fn a_chart_with_no_tx_pr_names_no_face() {
    // `bar-chart.pptx` is this shape: the face has to come from the theme's
    // minor font, which only the loader knows.
    let chart = parse_chart_xml(&chart_space_with("")).expect("chart parses");
    assert_eq!(chart.text_font_family, None);
}

#[test]
fn a_plot_area_tx_pr_is_not_read_as_the_chart_space_one() {
    // `c:txPr` is a sibling of `c:chart` written after it, exactly like
    // `c:spPr`. A flat event loop that does not wait for `</c:chart>` would
    // pick up an axis' own `c:txPr` instead (#637 made the same mistake).
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                      xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
            <c:chart>
                <c:plotArea>
                    <c:lineChart><c:ser><c:idx val="0"/>
                        <c:val><c:numRef><c:numCache><c:pt idx="0"><c:v>1</c:v></c:pt></c:numCache></c:numRef></c:val>
                    </c:ser></c:lineChart>
                    <c:catAx>
                        <c:txPr><a:p><a:pPr><a:defRPr><a:latin typeface="Impact"/></a:defRPr></a:pPr></a:p></c:txPr>
                    </c:catAx>
                </c:plotArea>
            </c:chart>
            <c:txPr><a:p><a:pPr><a:defRPr><a:latin typeface="Calibri"/></a:defRPr></a:pPr></a:p></c:txPr>
        </c:chartSpace>"#;
    let chart = parse_chart_xml(xml).expect("chart parses");
    assert_eq!(chart.text_font_family.as_deref(), Some("Calibri"));
}

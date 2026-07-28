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

    assert_eq!(
        chart.chart_type,
        ChartType::Other("Doughnut Chart".to_string())
    );
    assert_eq!(chart.series[0].values, vec![9.0, 6.0]);
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

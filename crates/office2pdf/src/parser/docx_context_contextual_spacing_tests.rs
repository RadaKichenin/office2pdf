use super::*;

const WORDPROCESSINGML: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Paragraph styles shaped like Word's own: `ListParagraph` carries the flag,
/// and `ListNumber2` inherits it through `w:basedOn`.
fn styles_xml() -> String {
    format!(
        r#"<w:styles xmlns:w="{WORDPROCESSINGML}">
<w:docDefaults><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="ListParagraph"><w:name w:val="List Paragraph"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720"/><w:contextualSpacing/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="ListNumber2"><w:name w:val="List Number 2"/><w:basedOn w:val="ListParagraph"/></w:style>
</w:styles>"#
    )
}

fn document_xml(body: &str) -> String {
    format!(
        r#"<w:document xmlns:w="{WORDPROCESSINGML}" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    )
}

/// A paragraph with an optional `w:pStyle` followed by more `w:pPr` children.
fn paragraph(style_id: Option<&str>, properties: &str, text: &str) -> String {
    let style: String = style_id
        .map(|style_id| format!(r#"<w:pStyle w:val="{style_id}"/>"#))
        .unwrap_or_default();
    format!(r#"<w:p><w:pPr>{style}{properties}</w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#)
}

fn table(cell_paragraphs: &[String]) -> String {
    format!(
        r#"<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="5000"/></w:tblGrid><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>"#,
        cell_paragraphs.concat()
    )
}

/// (drops `w:before`, drops `w:after`) for each paragraph, in document order.
fn drops(body: &str, styles_xml: Option<&str>) -> Vec<(bool, bool)> {
    let context = ContextualSpacingContext::from_xml(Some(&document_xml(body)), styles_xml);
    context
        .rules
        .iter()
        .map(|rule| (rule.drops_before, rule.drops_after))
        .collect()
}

#[test]
fn list_paragraph_items_drop_only_the_gaps_between_them() {
    let body: String = [
        paragraph(None, "", "Quarterly results"),
        paragraph(Some("ListParagraph"), "", "Revenue grew 12%"),
        paragraph(Some("ListParagraph"), "", "Churn fell to 3%"),
        paragraph(Some("ListParagraph"), "", "Two regions opened"),
        paragraph(None, "", "Details follow."),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![
            (false, false),
            (false, true),
            (true, true),
            (true, false),
            (false, false)
        ]
    );
}

#[test]
fn flag_acts_on_its_own_paragraph_only() {
    // Probe c02: the flagged paragraph drops its `w:after` even though the
    // paragraph below states no flag. Probe c03: flagging only the lower one
    // drops only its `w:before`.
    let flagged_upper: String = [
        paragraph(None, "<w:contextualSpacing/>", "Flagged"),
        paragraph(None, "", "Plain"),
    ]
    .concat();
    assert_eq!(
        drops(&flagged_upper, Some(&styles_xml())),
        vec![(false, true), (false, false)]
    );
    let flagged_lower: String = [
        paragraph(None, "", "Plain"),
        paragraph(None, "<w:contextualSpacing/>", "Flagged"),
    ]
    .concat();
    assert_eq!(
        drops(&flagged_lower, Some(&styles_xml())),
        vec![(false, false), (true, false)]
    );
}

#[test]
fn paragraph_value_overrides_its_style() {
    // Probes c10/c11.
    let body: String = [
        paragraph(
            Some("ListParagraph"),
            r#"<w:contextualSpacing w:val="0"/>"#,
            "Opted out",
        ),
        paragraph(Some("ListParagraph"), "", "Inherits the flag"),
        paragraph(
            Some("ListParagraph"),
            r#"<w:contextualSpacing w:val="false"/>"#,
            "Opted out",
        ),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, false), (true, true), (false, false)]
    );
}

#[test]
fn based_on_child_is_a_different_style() {
    // Probe c12: `ListNumber2` inherits the flag but is not `ListParagraph`.
    let body: String = [
        paragraph(Some("ListParagraph"), "", "Parent style"),
        paragraph(Some("ListNumber2"), "", "Child style"),
        paragraph(Some("ListNumber2"), "", "Child style again"),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, false), (false, true), (true, false)]
    );
}

#[test]
fn missing_and_undefined_style_ids_mean_the_default_style() {
    // Probes c13/c14.
    let body: String = [
        paragraph(None, "<w:contextualSpacing/>", "Bare"),
        paragraph(Some("Normal"), "<w:contextualSpacing/>", "Named default"),
        paragraph(Some("NoSuchStyle"), "<w:contextualSpacing/>", "Undefined"),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, true), (true, true), (true, false)]
    );
}

#[test]
fn package_without_styles_gives_bare_paragraphs_one_style() {
    // Issue #1684's own package (probe c18). Word drops the fourth gap too:
    // the plain paragraph below the last flagged one shares its style.
    let flagged = (1..=4).map(|item| {
        paragraph(
            None,
            r#"<w:spacing w:after="160"/><w:contextualSpacing/>"#,
            &format!("contextualSpacing item {item}"),
        )
    });
    let plain = (1..=4).map(|item| {
        paragraph(
            None,
            r#"<w:spacing w:after="160"/>"#,
            &format!("plain paragraph {item}"),
        )
    });
    let body: String = flagged.chain(plain).collect();
    assert_eq!(
        drops(&body, None),
        vec![
            (false, true),
            (true, true),
            (true, true),
            (true, true),
            (false, false),
            (false, false),
            (false, false),
            (false, false)
        ]
    );
}

#[test]
fn flag_inherits_from_document_defaults() {
    let styles: String = format!(
        r#"<w:styles xmlns:w="{WORDPROCESSINGML}"><w:docDefaults><w:pPrDefault><w:pPr><w:contextualSpacing/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style></w:styles>"#
    );
    let body: String = [paragraph(None, "", "One"), paragraph(None, "", "Two")].concat();
    assert_eq!(
        drops(&body, Some(&styles)),
        vec![(false, true), (true, false)]
    );
}

#[test]
fn tracked_property_change_does_not_count() {
    let body: String = [
        paragraph(
            Some("Normal"),
            r#"<w:pPrChange w:id="1" w:author="Reviewer"><w:pPr><w:contextualSpacing/></w:pPr></w:pPrChange>"#,
            "Formerly flagged",
        ),
        paragraph(Some("Normal"), "", "Neighbour"),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, false), (false, false)]
    );
}

#[test]
fn table_cell_is_a_flow_of_its_own() {
    // Probe c22.
    let body: String = table(&[
        paragraph(Some("ListParagraph"), "", "Cell item one"),
        paragraph(Some("ListParagraph"), "", "Cell item two"),
    ]);
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, true), (true, false)]
    );
}

#[test]
fn paragraph_above_a_table_neighbours_its_first_paragraph() {
    // Probes c23/c25: both gaps across the top edge drop. Probe c27: the last
    // paragraph of a cell meets nothing below it, and a non-default style
    // below the table follows no same-style paragraph (c23).
    let body: String = [
        paragraph(Some("ListParagraph"), "", "Above the table"),
        table(&[
            paragraph(Some("ListParagraph"), "", "First in cell"),
            paragraph(Some("ListParagraph"), "", "Last in cell"),
        ]),
        paragraph(Some("ListParagraph"), "", "Below the table"),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, true), (true, true), (true, false), (false, false)]
    );
}

#[test]
fn default_style_paragraph_below_a_table_follows_the_row_end_mark() {
    // Probes c29/c30.
    let body: String = [
        table(&[paragraph(Some("ListParagraph"), "", "Cell")]),
        paragraph(None, "<w:contextualSpacing/>", "Below the table"),
    ]
    .concat();
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, false), (true, false)]
    );
}

#[test]
fn text_box_fallback_and_vml_paragraphs_are_not_counted() {
    // docx-rs converts only the `mc:Choice` text box. The fallback's copy and
    // a VML text box never reach paragraph conversion, so counting them would
    // hand every later paragraph its predecessor's rule.
    let text_box_run: String = format!(
        r#"<w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wps:txbx><w:txbxContent>{choice}</w:txbxContent></wps:txbx></w:drawing></mc:Choice><mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent>{fallback}</w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent></w:r>"#,
        choice = paragraph(Some("ListParagraph"), "", "Box text"),
        fallback = paragraph(Some("ListParagraph"), "", "Box text"),
    );
    let vml_run: String = format!(
        r#"<w:r><w:pict><v:shape><v:textbox><w:txbxContent>{}</w:txbxContent></v:textbox></v:shape></w:pict></w:r>"#,
        paragraph(Some("ListParagraph"), "", "VML box")
    );
    let anchor: String = format!(
        r#"<w:p><w:pPr><w:pStyle w:val="ListParagraph"/></w:pPr>{text_box_run}{vml_run}</w:p>"#
    );
    let body: String = [
        anchor,
        paragraph(Some("ListParagraph"), "", "After the box"),
    ]
    .concat();

    // The anchor and the paragraph after it stay neighbours in the body; the
    // text box is a flow of its own.
    assert_eq!(
        drops(&body, Some(&styles_xml())),
        vec![(false, true), (false, false), (true, false)]
    );
}

#[test]
fn dropped_after_still_offsets_the_before_below_it() {
    // Probes c16/c19/c20: dropping 10pt above a kept 15pt leaves 5pt, 4pt
    // leaves 11pt, and 20pt leaves none.
    for (dropped_after, kept_before, expected_before) in
        [(10.0, 15.0, 5.0), (4.0, 15.0, 11.0), (20.0, 15.0, 0.0)]
    {
        let body: String = [
            paragraph(Some("Normal"), "<w:contextualSpacing/>", "Flagged"),
            paragraph(Some("Normal"), "", "Keeps its before"),
        ]
        .concat();
        let context =
            ContextualSpacingContext::from_xml(Some(&document_xml(&body)), Some(&styles_xml()));
        let (mut upper_before, mut upper_after) = (None, Some(dropped_after));
        context
            .next_paragraph(Some("Normal"))
            .apply(&mut upper_before, &mut upper_after);
        let (mut lower_before, mut lower_after) = (Some(kept_before), Some(8.0));
        context
            .next_paragraph(Some("Normal"))
            .apply(&mut lower_before, &mut lower_after);

        assert_eq!((upper_before, upper_after), (None, Some(0.0)));
        assert_eq!(
            (lower_before, lower_after),
            (Some(expected_before), Some(8.0)),
            "dropped {dropped_after}pt above a {kept_before}pt before"
        );
    }
}

#[test]
fn drifted_cursor_stops_applying() {
    let body: String = [
        paragraph(Some("ListParagraph"), "", "One"),
        paragraph(Some("ListParagraph"), "", "Two"),
        paragraph(Some("ListParagraph"), "", "Three"),
    ]
    .concat();
    let context =
        ContextualSpacingContext::from_xml(Some(&document_xml(&body)), Some(&styles_xml()));
    let mut first_after: Option<f64> = Some(8.0);
    context
        .next_paragraph(Some("Heading1"))
        .apply(&mut None, &mut first_after);
    assert_eq!(first_after, Some(8.0));

    let (mut before, mut after) = (Some(6.0), Some(8.0));
    context
        .next_paragraph(Some("ListParagraph"))
        .apply(&mut before, &mut after);
    assert_eq!(
        (before, after),
        (Some(6.0), Some(8.0)),
        "a drifted cursor must not move gaps onto other paragraphs"
    );
}

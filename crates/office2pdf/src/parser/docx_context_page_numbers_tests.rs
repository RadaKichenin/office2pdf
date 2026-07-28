use super::*;

#[test]
fn scans_one_entry_per_section_in_document_order() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:body>
        <w:p><w:pPr><w:sectPr><w:pgNumType w:start="1" w:fmt="lowerRoman"/></w:sectPr></w:pPr></w:p>
        <w:p><w:pPr><w:sectPr><w:pgNumType w:start="1" w:fmt="decimal"/></w:sectPr></w:pPr></w:p>
        <w:p><w:pPr><w:sectPr><w:pgNumType/></w:sectPr></w:pPr></w:p>
        <w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr>
      </w:body>
    </w:document>"#;

    let numbering = scan_page_numbering(xml);
    assert_eq!(numbering.len(), 4, "one entry per w:sectPr");
    assert_eq!(
        numbering[0],
        Some(PageNumbering {
            start: Some(1),
            format: PageNumberFormat::LowerRoman
        })
    );
    assert_eq!(
        numbering[1],
        Some(PageNumbering {
            start: Some(1),
            format: PageNumberFormat::Decimal
        })
    );
    assert_eq!(
        numbering[2],
        Some(PageNumbering {
            start: None,
            format: PageNumberFormat::Decimal
        }),
        "a bare w:pgNumType continues the counter in decimal"
    );
    assert_eq!(
        numbering[3], None,
        "a section without the element declares nothing"
    );
}

#[test]
fn an_unknown_format_falls_back_to_decimal_rather_than_a_wrong_alphabet() {
    let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:body><w:sectPr><w:pgNumType w:fmt="ideographDigital"/></w:sectPr></w:body>
    </w:document>"#;

    assert_eq!(
        scan_page_numbering(xml)[0].as_ref().map(|n| n.format),
        Some(PageNumberFormat::Decimal)
    );
}

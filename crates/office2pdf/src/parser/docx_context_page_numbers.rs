use crate::ir::{PageNumberFormat, PageNumbering};

/// Scan every `w:sectPr/w:pgNumType` in document order.
///
/// docx-rs reads `w:start` but not `w:fmt`, and the format is half the
/// element's meaning: a front matter that restarts at `i` states both. Reading
/// them together here keeps a section's restart and its numerals from being
/// resolved out of different places.
///
/// The result is indexed the same way `scan_column_layouts` is — one entry per
/// `w:sectPr`, in the order the sections appear.
pub(in super::super) fn scan_page_numbering(xml: &str) -> Vec<Option<PageNumbering>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut numbering: Vec<Option<PageNumbering>> = Vec::new();
    let mut in_section_properties = false;
    let mut current: Option<PageNumbering> = None;

    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(ref element)) => {
                match element.local_name().as_ref() {
                    b"sectPr" => {
                        in_section_properties = true;
                        current = None;
                    }
                    b"pgNumType" if in_section_properties => {
                        current = Some(read_page_num_type(element));
                    }
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Empty(ref element)) => {
                if element.local_name().as_ref() == b"pgNumType" && in_section_properties {
                    current = Some(read_page_num_type(element));
                }
            }
            Ok(quick_xml::events::Event::End(ref element)) => {
                if element.local_name().as_ref() == b"sectPr" {
                    in_section_properties = false;
                    numbering.push(current.take());
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    numbering
}

fn read_page_num_type(element: &quick_xml::events::BytesStart) -> PageNumbering {
    let mut start: Option<u32> = None;
    let mut format = PageNumberFormat::Decimal;
    for attribute in element.attributes().flatten() {
        let Ok(value) = attribute.unescape_value() else {
            continue;
        };
        match attribute.key.local_name().as_ref() {
            b"start" => start = value.parse::<u32>().ok(),
            b"fmt" => format = parse_page_number_format(&value),
            _ => {}
        }
    }
    PageNumbering { start, format }
}

/// `w:fmt`'s numeral formats. Word defines many more — ordinals, kanji,
/// Hangul counting — but a section that restarts states one of these; the rest
/// fall back to decimal rather than to a wrong alphabet.
fn parse_page_number_format(value: &str) -> PageNumberFormat {
    match value {
        "lowerRoman" => PageNumberFormat::LowerRoman,
        "upperRoman" => PageNumberFormat::UpperRoman,
        "lowerLetter" => PageNumberFormat::LowerLetter,
        "upperLetter" => PageNumberFormat::UpperLetter,
        _ => PageNumberFormat::Decimal,
    }
}

#[cfg(test)]
#[path = "docx_context_page_numbers_tests.rs"]
mod tests;

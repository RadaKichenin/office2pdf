use super::*;

#[test]
fn each_identifier_counts_independently_from_one() {
    let fields = FieldContext::default();
    assert_eq!(fields.next_in_sequence("Table"), 1);
    assert_eq!(fields.next_in_sequence("Figure"), 1);
    assert_eq!(fields.next_in_sequence("Table"), 2);
    assert_eq!(fields.next_in_sequence("Table"), 3);
    assert_eq!(fields.next_in_sequence("Figure"), 2);
}

#[test]
fn reads_the_identifier_out_of_the_instruction() {
    assert_eq!(seq_identifier("SEQ Table"), Some("Table"));
    assert_eq!(seq_identifier("  SEQ Figure  "), Some("Figure"));
    assert_eq!(seq_identifier("SEQ Table \\* ARABIC"), Some("Table"));
}

#[test]
fn ignores_an_instruction_that_is_not_a_sequence() {
    assert_eq!(seq_identifier("PAGE"), None);
    assert_eq!(seq_identifier("TOC \\o \"1-3\""), None);
    assert_eq!(seq_identifier("SEQUENCE Table"), None);
    // A `SEQ` with no identifier counts nothing.
    assert_eq!(seq_identifier("SEQ"), None);
    assert_eq!(seq_identifier("SEQ \\* ARABIC"), None);
}

#[test]
fn reads_the_outline_depth_a_toc_collects() {
    assert_eq!(toc_heading_depth(r#"TOC \h \o "1-3""#), Some(3));
    assert_eq!(toc_heading_depth(r#"TOC \o "1-9""#), Some(9));
    assert_eq!(toc_heading_depth(r#"TOC \o "2-2""#), Some(2));
    // `\o` with no range, and a bare `TOC`, both take the whole outline.
    assert_eq!(toc_heading_depth(r"TOC \o"), Some(9));
    assert_eq!(toc_heading_depth("TOC"), Some(9));
}

#[test]
fn a_caption_list_is_not_a_heading_outline() {
    assert_eq!(toc_heading_depth(r#"TOC \a "Figure" \h"#), None);
    assert_eq!(toc_heading_depth(r#"TOC \a "Table" \h"#), None);
}

#[test]
fn ignores_an_instruction_that_is_not_a_contents_field() {
    assert_eq!(toc_heading_depth("PAGE"), None);
    assert_eq!(toc_heading_depth("SEQ Table"), None);
    assert_eq!(toc_heading_depth("TOCX"), None);
}

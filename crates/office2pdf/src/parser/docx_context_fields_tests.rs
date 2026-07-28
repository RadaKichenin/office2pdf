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

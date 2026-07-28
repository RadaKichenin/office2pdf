use std::cell::RefCell;
use std::collections::HashMap;

/// Running values for the document's `SEQ` fields, one counter per identifier.
///
/// Word numbers a caption with `SEQ Figure` or `SEQ Table`, so the number
/// lives in the field rather than in the text. Dropping the field left every
/// caption in the technical brief reading `표` where Word reads `표 26`, and
/// left the prose that cross-references those numbers pointing at nothing
/// (issue #577).
#[derive(Debug, Default)]
pub(in super::super) struct FieldContext {
    sequences: RefCell<HashMap<String, u32>>,
}

impl FieldContext {
    /// Advance `identifier`'s counter and return its new value.
    ///
    /// Word counts a `SEQ` in document order, so the paragraphs must be
    /// visited in order — which is how the body is converted.
    pub(in super::super) fn next_in_sequence(&self, identifier: &str) -> u32 {
        let mut sequences = self.sequences.borrow_mut();
        let value = sequences.entry(identifier.to_string()).or_insert(0);
        *value += 1;
        *value
    }
}

/// The identifier a `SEQ` field instruction counts, if it is one.
///
/// The instruction is `SEQ <identifier>` followed by optional switches —
/// `\* ARABIC` for the numeral format, `\n`/`\c`/`\r` for what to do with the
/// counter. Only the identifier is read here; a switch that resets or repeats
/// the counter is rare in a caption and would need the whole field grammar.
pub(in super::super) fn seq_identifier(instruction: &str) -> Option<&str> {
    let rest = instruction.trim().strip_prefix("SEQ")?;
    // `SEQ` has to be a whole word: `SEQUENCE Table` is not a sequence field.
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let identifier = rest.split_whitespace().next()?;
    (!identifier.starts_with('\\')).then_some(identifier)
}

/// The outline depth a `TOC` field collects, if it is one.
///
/// `TOC \o "1-3"` takes headings of outline level 1 through 3. The upper bound
/// is what Typst's outline depth means, and Word's own default when the range
/// is absent is the whole outline. A `\a` list collects captions rather than
/// headings, so it reads as `None` here.
pub(in super::super) fn toc_heading_depth(instruction: &str) -> Option<u8> {
    let rest = instruction.trim().strip_prefix("TOC")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    if rest.contains("\\a") {
        return None;
    }
    let Some(after) = rest.split("\\o").nth(1) else {
        return Some(FULL_OUTLINE_DEPTH);
    };
    let range = after
        .trim()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or_default();
    let upper = range.split('-').nth(1).unwrap_or(range).trim();
    Some(
        upper
            .parse::<u8>()
            .unwrap_or(FULL_OUTLINE_DEPTH)
            .clamp(1, FULL_OUTLINE_DEPTH),
    )
}

/// Word's outline runs from level 1 to level 9.
const FULL_OUTLINE_DEPTH: u8 = 9;

/// The `SEQ` identifier a `TOC \a` field lists, if it is one.
///
/// `TOC \a "Figure"` collects the paragraphs `SEQ Figure` numbers — the
/// document's figure contents — rather than its headings.
pub(in super::super) fn toc_caption_identifier(instruction: &str) -> Option<String> {
    let rest = instruction.trim().strip_prefix("TOC")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let after = rest.split("\\a").nth(1)?;
    let identifier = after
        .trim_start()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default();
    (!identifier.is_empty()).then(|| identifier.to_string())
}

#[cfg(test)]
#[path = "docx_context_fields_tests.rs"]
mod tests;

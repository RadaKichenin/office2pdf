//! Whether each paragraph's mark survives into the final document (issue #1710).
//!
//! `w:pPr/w:rPr` holds the revision state of the paragraph mark itself: a
//! `w:del` there means the mark was deleted, a `w:moveFrom` that it is the
//! origin of a tracked move. Either way Word's final view — what "No Markup"
//! shows and what accepting every revision produces — has no paragraph break
//! at that point, so the paragraph merges into the one after it and
//! contributes neither a line of its own nor a list number. The merged
//! paragraph keeps the *following* paragraph's formatting, because the mark
//! that survives is the one that carries it.
//!
//! The scan reads the raw XML rather than docx-rs for two reasons: docx-rs
//! parses `w:del` on a mark but not `w:moveFrom`, and its `w:rPr` reader
//! returns at the first `</w:rPr>`, so a `w:rPrChange` recording an earlier
//! revision state would be read as the mark's own. It counts the paragraph
//! sequence of [`super::paragraph_cursor`], so a text box's `mc:Fallback` copy
//! cannot shift the property onto a later paragraph (issue #1689).

use std::cell::RefCell;

use crate::ir::{Paragraph, Run};

use super::paragraph_cursor::{ParagraphCursor, ScannedParagraph, scan_body_paragraphs};

pub(in super::super) struct ParagraphMarkContext {
    /// Whether each paragraph's mark is absent from the final document.
    removed_marks: ParagraphCursor<bool>,
    /// The paragraph a removed mark held back, waiting for the paragraph it
    /// merges into. Only its runs travel; its own properties go with its mark.
    withheld: RefCell<Option<Paragraph>>,
}

impl ParagraphMarkContext {
    pub(in super::super) fn from_xml(xml: Option<&str>) -> Self {
        Self {
            removed_marks: ParagraphCursor::new(
                "w:pPr/w:rPr/w:del|w:moveFrom",
                xml.map(Self::scan).unwrap_or_default(),
            ),
            withheld: RefCell::new(None),
        }
    }

    /// Whether the next paragraph's mark is absent from the final document.
    /// Must be called exactly once per converted `w:p`, with that paragraph's
    /// `w:pStyle`.
    pub(in super::super) fn next_mark_is_removed(&self, style_id: Option<&str>) -> bool {
        self.removed_marks
            .next(style_id)
            .is_some_and(|(_, is_removed)| *is_removed)
    }

    /// Hold a paragraph back so the paragraph after it absorbs its runs.
    pub(in super::super) fn withhold(&self, paragraph: Paragraph) {
        *self.withheld.borrow_mut() = Some(paragraph);
    }

    /// The runs a withheld paragraph contributes to the front of the paragraph
    /// that follows it.
    pub(in super::super) fn take_merged_runs(&self) -> Vec<Run> {
        self.withheld
            .borrow_mut()
            .take()
            .map(|paragraph| paragraph.runs)
            .unwrap_or_default()
    }

    /// Whether the paragraph just converted is being held back, so its caller
    /// knows to contribute no element for it — not even an empty one, which
    /// would end the list it sits in.
    pub(in super::super) fn is_withholding(&self) -> bool {
        self.withheld.borrow().is_some()
    }

    /// Whatever is still withheld where a flow ends or a non-paragraph follows,
    /// so a removed mark with nowhere to merge cannot swallow its own text.
    /// Word never deletes a container's last paragraph mark, so this is the
    /// malformed-input path rather than the expected one.
    pub(in super::super) fn take_withheld(&self) -> Option<Paragraph> {
        self.withheld.borrow_mut().take()
    }

    fn scan(xml: &str) -> Vec<ScannedParagraph<bool>> {
        scan_body_paragraphs(xml, |_reader, element, is_removed| {
            // Every element inside the paragraph's own `w:pPr` reaches here,
            // and `w:pPrChange`/`w:rPrChange` — the only other places a `w:pPr`
            // subtree can name a revision — are walked past, so a `w:del` or
            // `w:moveFrom` seen here belongs to the mark's own `w:rPr`.
            if matches!(element.local_name().as_ref(), b"del" | b"moveFrom") {
                *is_removed = true;
            }
        })
    }
}

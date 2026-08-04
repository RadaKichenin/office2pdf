"""Tests for the text-layer codepoint census.

The cases are the two real defects that motivated it (issues #664 and #684),
plus the negatives that keep the tool from crying wolf.
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from compare_text_layer import census, compare, normalize, render_report


class CensusTest(unittest.TestCase):
    def test_counts_injected_word_joiners_as_their_own_class(self):
        # Issue #664: U+2060 renders as nothing, so no raster check sees it.
        counts = census("of⁠fice⁠2pdf")
        self.assertEqual(counts["invisible:WORD JOINER"], 2)

    def test_counts_nbsp_apart_from_ordinary_space(self):
        counts = census("a b c")
        self.assertEqual(counts["space:SPACE"], 1)
        self.assertEqual(counts["space:NO-BREAK SPACE"], 1)

    def test_counts_ligature_codepoints(self):
        # Issue #684: the ffi ligature makes "office" unsearchable.
        counts = census("oﬃce2pdf")
        self.assertEqual(counts["ligature:ffi"], 1)

    def test_ignores_layout_whitespace(self):
        # Newlines and tabs are layout, and belong to the layout differ.
        self.assertNotIn("control", census("a\nb\tc\r"))


class NormalizeTest(unittest.TestCase):
    def test_expands_ligatures_to_their_letters(self):
        self.assertEqual(normalize("oﬃce2pdf"), "office2pdf")

    def test_strips_invisible_formatters(self):
        self.assertEqual(normalize("of⁠fice"), "office")

    def test_folds_nonstandard_spaces_and_collapses_runs(self):
        self.assertEqual(normalize("a  b\n\nc"), "a b c")


class CompareTest(unittest.TestCase):
    def test_ligature_is_an_encoding_difference_not_text_loss(self):
        # The distinction the tool exists to make: the page reads the same, the
        # text layer does not.
        result = compare("office2pdf", "oﬃce2pdf")
        self.assertTrue(result["content_matches"])
        self.assertEqual(result["census_deltas"]["ligature:ffi"], 1)

    def test_injected_joiners_are_reported_while_content_still_matches(self):
        result = compare("office2pdf", "of⁠fice2pdf")
        self.assertTrue(result["content_matches"])
        self.assertEqual(result["census_deltas"]["invisible:WORD JOINER"], 1)

    def test_dropped_word_is_content_loss(self):
        result = compare("alpha beta gamma", "alpha gamma")
        self.assertFalse(result["content_matches"])

    def test_identical_text_reports_no_delta(self):
        result = compare("office2pdf", "office2pdf")
        self.assertEqual(result["census_deltas"], {})
        self.assertTrue(result["content_matches"])


class ReportTest(unittest.TestCase):
    def test_ligature_report_names_the_searchability_consequence(self):
        report = render_report(compare("office2pdf", "oﬃce2pdf"))
        self.assertIn("unsearchable", report)

    def test_clean_comparison_says_so_plainly(self):
        self.assertIn("intact", render_report(compare("abc", "abc")))

    def test_content_loss_is_not_blamed_on_a_class(self):
        # No class was injected, so the report must point at real loss rather
        # than leaving the reader hunting for a formatting cause.
        report = render_report(compare("alpha beta", "alpha"))
        self.assertIn("genuine text loss", report)


if __name__ == "__main__":
    unittest.main()

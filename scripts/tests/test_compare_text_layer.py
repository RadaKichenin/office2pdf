"""Tests for the text-layer codepoint census.

The cases are the two real defects that motivated it (issues #664 and #684),
plus the negatives that keep the tool from crying wolf.
"""

import io
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from compare_text_layer import census, compare, main, normalize, render_report


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

    def test_sequence_mismatch_requires_order_and_content_review(self):
        # Reordered table columns can preserve every word (#1561). Neither
        # that case nor an omitted word can be diagnosed from class deltas.
        for output in ("right column left column", "left column"):
            with self.subTest(output=output):
                report = render_report(compare("left column right column", output))
                self.assertIn("extraction order", report)
                self.assertIn("missing or extra text", report)
                self.assertNotIn("genuine text loss", report)

    def test_reordered_columns_and_missing_words_still_fail_the_cli(self):
        for output in ("right column left column", "left column"):
            with self.subTest(output=output), patch(
                "sys.argv", ["compare_text_layer.py", "gt.pdf", "out.pdf"]
            ), patch(
                "compare_text_layer.extract_text",
                side_effect=["left column right column", output],
            ), patch("sys.stdout", new_callable=io.StringIO):
                self.assertEqual(main(), 1)


if __name__ == "__main__":
    unittest.main()

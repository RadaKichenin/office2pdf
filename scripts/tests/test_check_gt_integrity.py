"""Tests for the GT integrity gate.

The periodicity cases are modelled on issue #616, the duplication this gate
exists to catch.
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_gt_integrity import detect_periodicity, render_report


class PeriodicityTest(unittest.TestCase):
    def test_detects_a_workbook_exported_once_per_sheet(self):
        # Issue #616: three sheets, each export carrying all three pages.
        self.assertEqual(detect_periodicity(["a", "b", "c"] * 3), 3)

    def test_detects_every_page_identical(self):
        self.assertEqual(detect_periodicity(["a"] * 6), 1)

    def test_distinct_pages_are_not_periodic(self):
        self.assertIsNone(detect_periodicity(["a", "b", "c", "d", "e"]))

    def test_a_repeated_layout_alone_is_not_periodic(self):
        # Two pages sharing a layout is ordinary; only a whole-sequence repeat
        # counts, or every deck with a section divider would fail the gate.
        self.assertIsNone(detect_periodicity(["a", "b", "a", "c", "d", "e"]))

    def test_short_documents_are_never_flagged(self):
        # A 2-page doc whose pages match is far more likely a real duplicate
        # slide than a corrupt export; too little evidence either way.
        self.assertIsNone(detect_periodicity(["a", "a"]))

    def test_partial_trailing_cycle_is_not_periodic(self):
        # A sequence that starts to repeat but does not complete the cycle is
        # not a duplicated export.
        self.assertIsNone(detect_periodicity(["a", "b", "c", "a", "b"]))


class ReportTest(unittest.TestCase):
    def test_periodic_gt_is_called_invalid_and_says_why(self):
        report = render_report(
            {"pages": 9, "periodicity": 3, "worksheets": 3,
             "font_substitutions": [], "invalid": True}
        )
        self.assertIn("INVALID", report)
        self.assertIn("#616", report)

    def test_clean_gt_says_so(self):
        report = render_report(
            {"pages": 5, "periodicity": None, "worksheets": None,
             "font_substitutions": [], "invalid": False}
        )
        self.assertIn("No structural corruption", report)

    def test_font_substitutions_are_marked_gt_side(self):
        # The whole point: an auditor must not file these as converter defects.
        report = render_report(
            {"pages": 2, "periodicity": None, "worksheets": 1,
             "font_substitutions": ["Calibri"], "invalid": False}
        )
        self.assertIn("not", report)
        self.assertIn("converter defects", report)
        self.assertIn("Calibri", report)


class PeriodicityLimitTest(unittest.TestCase):
    def test_strict_alternation_is_flagged_and_this_is_a_known_limit(self):
        # Documented in detect_periodicity: pixel-identical alternation trips
        # the gate. It needs pages with identical ink, which differing text
        # defeats, so this is pathological rather than ordinary. Pinned so the
        # behaviour is deliberate rather than a surprise.
        self.assertEqual(detect_periodicity(["a", "b"] * 3), 2)


if __name__ == "__main__":
    unittest.main()

"""Tests for the GT integrity gate.

The periodicity cases are modelled on issue #616, the duplication this gate
exists to catch.
"""

import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import check_gt_integrity
from check_gt_integrity import (
    check,
    detect_periodicity,
    hidden_sheet_count,
    printable_sheet_count,
    read_workbook_xml,
    render_report,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]
# A real two-sheet probe workbook whose lookup sheet is `state="hidden"`,
# committed for issue #1065.
HIDDEN_SHEET_WORKBOOK = (
    PROJECT_ROOT / "tests" / "fixtures" / "xlsx" / "issue_1065_hidden_sheet_probe.xlsx"
)


def workbook_declaring(sheets: str) -> str:
    """An `xl/workbook.xml` whose `<sheets>` block is `sheets`.

    The surrounding elements are the ones Excel for Mac actually writes, so the
    sheet scan is exercised against a document that also carries `<workbookPr>`,
    `<bookViews>` and `<sheetPr>`-adjacent names it must not mistake for sheets.
    """
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
        ' xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        '<fileVersion appName="xl" lastEdited="7"/>'
        '<workbookPr defaultThemeVersion="202300"/>'
        '<bookViews><workbookView xWindow="640" yWindow="700"/></bookViews>'
        f"<sheets>{sheets}</sheets>"
        '<calcPr calcId="181029"/>'
        "</workbook>"
    )


# The workbook of issue #1211: two visible sheets and a hidden lookup sheet.
GIFT_TRACKER_SHEETS = (
    '<sheet name="Start" sheetId="1" state="visible" r:id="rId3"/>'
    '<sheet name="Gift budget and tracker" sheetId="2" state="visible" r:id="rId4"/>'
    '<sheet name="Data" sheetId="3" state="hidden" r:id="rId5"/>'
)


def write_workbook(directory: Path, name: str, sheets: str) -> Path:
    """A minimal but openable XLSX declaring `sheets`."""
    source = directory / name
    with zipfile.ZipFile(source, "w") as archive:
        archive.writestr("xl/workbook.xml", workbook_declaring(sheets))
    return source


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
            {"pages": 9, "periodicity": 3, "printable_worksheets": 3,
             "hidden_worksheets": 0, "font_substitutions": [], "invalid": True}
        )
        self.assertIn("INVALID", report)
        self.assertIn("#616", report)

    def test_clean_gt_says_so(self):
        report = render_report(
            {"pages": 5, "periodicity": None, "printable_worksheets": None,
             "hidden_worksheets": None, "font_substitutions": [], "invalid": False}
        )
        self.assertIn("No structural corruption", report)

    def test_font_substitutions_are_marked_gt_side(self):
        # The whole point: an auditor must not file these as converter defects.
        report = render_report(
            {"pages": 2, "periodicity": None, "printable_worksheets": 1,
             "hidden_worksheets": 0, "font_substitutions": ["Calibri"],
             "invalid": False}
        )
        self.assertIn("not", report)
        self.assertIn("converter defects", report)
        self.assertIn("Calibri", report)

    def test_the_page_count_line_names_the_printable_sheets(self):
        # "Worksheets declared by source" would be a lie once the hidden ones
        # are excluded, and the operator needs to see why the count dropped.
        report = render_report(
            {"pages": 2, "periodicity": None, "printable_worksheets": 2,
             "hidden_worksheets": 1, "font_substitutions": [], "invalid": False}
        )
        self.assertIn("Printable worksheets in source: 2", report)
        self.assertIn("1", report.split("Printable worksheets")[1])
        self.assertIn("hidden", report.lower())

    def test_a_workbook_with_no_hidden_sheets_says_nothing_about_them(self):
        report = render_report(
            {"pages": 3, "periodicity": None, "printable_worksheets": 3,
             "hidden_worksheets": 0, "font_substitutions": [], "invalid": False}
        )
        self.assertNotIn("hidden", report.lower())


class PeriodicityLimitTest(unittest.TestCase):
    def test_strict_alternation_is_flagged_and_this_is_a_known_limit(self):
        # Documented in detect_periodicity: pixel-identical alternation trips
        # the gate. It needs pages with identical ink, which differing text
        # defeats, so this is pathological rather than ordinary. Pinned so the
        # behaviour is deliberate rather than a surprise.
        self.assertEqual(detect_periodicity(["a", "b"] * 3), 2)


class SheetVisibilityTest(unittest.TestCase):
    """Excel never prints a hidden sheet (#1065), so the gate must not count it.

    Counting them declared a correct native export INVALID for every workbook
    carrying a hidden lookup sheet (#1211).
    """

    def test_the_gift_tracker_shape_counts_only_its_visible_sheets(self):
        # Two visible sheets plus a hidden `Data` lookup sheet: the workbook of
        # issue #1211, whose correct native export is two pages.
        self.assertEqual(printable_sheet_count(workbook_declaring(GIFT_TRACKER_SHEETS)), 2)
        self.assertEqual(hidden_sheet_count(workbook_declaring(GIFT_TRACKER_SHEETS)), 1)

    def test_very_hidden_sheets_are_not_printed_either(self):
        # `veryHidden` also keeps the sheet out of the unhide dialog; both
        # states stay off paper.
        sheets = (
            '<sheet name="Report" sheetId="1" r:id="rId1"/>'
            '<sheet name="Lookups" sheetId="2" state="veryHidden" r:id="rId2"/>'
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 1)
        self.assertEqual(hidden_sheet_count(workbook_declaring(sheets)), 1)

    def test_a_sheet_without_a_state_attribute_is_visible(self):
        # ST_SheetState defaults to `visible`; Excel omits the attribute for
        # every sheet it has never hidden.
        sheets = (
            '<sheet name="Jan" sheetId="1" r:id="rId1"/>'
            '<sheet name="Feb" sheetId="2" r:id="rId2"/>'
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 2)
        self.assertEqual(hidden_sheet_count(workbook_declaring(sheets)), 0)

    def test_an_all_visible_workbook_still_counts_every_sheet(self):
        # The truncation check has to keep its teeth: nothing here is hidden,
        # so all four sheets must be demanded of the export.
        sheets = "".join(
            f'<sheet name="Q{quarter}" sheetId="{quarter}" state="visible" r:id="rId{quarter}"/>'
            for quarter in range(1, 5)
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 4)

    def test_a_workbook_hiding_everything_reports_no_printable_sheet(self):
        sheets = (
            '<sheet name="A" sheetId="1" state="hidden" r:id="rId1"/>'
            '<sheet name="B" sheetId="2" state="veryHidden" r:id="rId2"/>'
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 0)

    def test_the_scan_ignores_elements_that_merely_start_with_sheet(self):
        # `<sheets>`, `<sheetPr>` and `<sheetView>` all share the prefix; only
        # `<sheet>` itself declares one.
        sheets = (
            '<sheet name="Only" sheetId="1" r:id="rId1"><sheetPr codeName="Sheet1"/>'
            "</sheet>"
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 1)

    def test_an_element_merely_ending_in_sheet_is_not_a_declaration(self):
        # The prefix group must require a colon, or `<worksheet>` would count.
        sheets = '<sheet name="Only" sheetId="1" r:id="rId1"/><worksheet name="x"/>'
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 1)

    def test_a_namespace_prefixed_sheet_element_still_counts(self):
        # Some producers write `xl/workbook.xml` with an explicit prefix rather
        # than a default namespace.
        sheets = (
            '<x:sheet name="Visible" sheetId="1" xmlns:x="urn:main"/>'
            '<x:sheet name="Lookup" sheetId="2" state="hidden" xmlns:x="urn:main"/>'
        )
        self.assertEqual(printable_sheet_count(workbook_declaring(sheets)), 1)
        self.assertEqual(hidden_sheet_count(workbook_declaring(sheets)), 1)

    def test_a_real_workbook_with_a_hidden_sheet_reads_one_printable_sheet(self):
        self.assertTrue(
            HIDDEN_SHEET_WORKBOOK.is_file(), f"missing fixture {HIDDEN_SHEET_WORKBOOK}"
        )
        workbook_xml = read_workbook_xml(HIDDEN_SHEET_WORKBOOK)
        self.assertIsNotNone(workbook_xml)
        self.assertEqual(printable_sheet_count(workbook_xml), 1)
        self.assertEqual(hidden_sheet_count(workbook_xml), 1)

    def test_a_non_xlsx_source_yields_no_workbook_part(self):
        self.assertIsNone(read_workbook_xml(Path("deck.pptx")))


class PageCountGateTest(unittest.TestCase):
    """The gate's verdict, with the raster step stubbed so no poppler is needed."""

    def gate(self, source: Path, pages: int) -> dict:
        with mock.patch.object(
            check_gt_integrity,
            "page_hashes",
            return_value=[f"page-{index}" for index in range(pages)],
        ):
            return check(Path("gt.pdf"), source)

    def test_a_correct_export_of_a_workbook_with_a_hidden_sheet_is_valid(self):
        # Issue #1211: three declared sheets, one hidden, and a native export of
        # exactly two pages — which is right.
        with tempfile.TemporaryDirectory() as directory:
            source = write_workbook(Path(directory), "tracker.xlsx", GIFT_TRACKER_SHEETS)
            result = self.gate(source, pages=2)
        self.assertFalse(result["invalid"])
        self.assertEqual(result["printable_worksheets"], 2)
        self.assertEqual(result["hidden_worksheets"], 1)
        self.assertNotIn("INVALID", render_report(result))

    def test_a_truncated_export_of_an_all_visible_workbook_still_fails(self):
        # The check the fix must not weaken: nothing is hidden, so two pages
        # cannot be a complete export of three sheets.
        sheets = (
            '<sheet name="Start" sheetId="1" state="visible" r:id="rId3"/>'
            '<sheet name="Gift budget and tracker" sheetId="2" state="visible" r:id="rId4"/>'
            '<sheet name="Data" sheetId="3" state="visible" r:id="rId5"/>'
        )
        with tempfile.TemporaryDirectory() as directory:
            source = write_workbook(Path(directory), "tracker.xlsx", sheets)
            result = self.gate(source, pages=2)
        self.assertTrue(result["invalid"])
        self.assertIn("INVALID", render_report(result))

    def test_a_hidden_sheet_does_not_hide_a_truncated_export(self):
        # One hidden sheet of four, so two printable sheets are still expected
        # and a one-page export is still short.
        sheets = (
            '<sheet name="Start" sheetId="1" r:id="rId1"/>'
            '<sheet name="Detail" sheetId="2" r:id="rId2"/>'
            '<sheet name="Data" sheetId="3" state="hidden" r:id="rId3"/>'
        )
        with tempfile.TemporaryDirectory() as directory:
            source = write_workbook(Path(directory), "tracker.xlsx", sheets)
            result = self.gate(source, pages=1)
        self.assertTrue(result["invalid"])

    def test_the_duplication_signature_is_measured_against_printable_sheets(self):
        # Issue #616 again, in a workbook that also hides a lookup sheet: two
        # printable sheets exported once per sheet give four distinct pages.
        with tempfile.TemporaryDirectory() as directory:
            source = write_workbook(Path(directory), "tracker.xlsx", GIFT_TRACKER_SHEETS)
            result = self.gate(source, pages=4)
        self.assertEqual(result["suspicious_multiple"], 2)


if __name__ == "__main__":
    unittest.main()

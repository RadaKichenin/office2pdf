"""Unit tests for the Excel row-height sweep (#1150).

The AppleScript reading is stubbed, so the parts that decide what Excel is
asked and how the answers are attributed run without a Mac or Excel. The
traps pinned here are the ones the first run of the sweep actually hit: a
raw-string escape that wrote `val=\\"10\\"` into the stylesheet and made Excel
reject every variant with -50, and a slug that collapsed two Korean family
names to the same stage file so one family's package silently overwrote the
other's and the report showed one column twice.
"""

from __future__ import annotations

import io
import sys
import unittest
import zipfile
from pathlib import Path
from contextlib import redirect_stderr, redirect_stdout
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import measure_excel_row_height
from probe_harness import ProbeError

STYLES_XML = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
    '<fonts count="2"><font><sz val="12"/><name val="Calibri"/></font>'
    '<font><sz val="8"/><name val="나눔명조"/><family val="3"/><charset val="129"/></font>'
    "</fonts><fills count=\"0\"/></styleSheet>"
)


class PatchedStylesTests(unittest.TestCase):
    def test_replaces_only_the_normal_font(self) -> None:
        patched = measure_excel_row_height.patched_styles(STYLES_XML, "Times New Roman", 10.0)

        self.assertIn('<font><sz val="10"/><name val="Times New Roman"/></font>', patched)
        self.assertNotIn("Calibri", patched)
        self.assertIn('<name val="나눔명조"/>', patched, "fonts[1] must survive untouched")
        self.assertIn('<fills count="0"/>', patched)

    def test_writes_plain_xml_quotes(self) -> None:
        """Excel rejects a stylesheet whose attribute quotes are backslashed."""
        patched = measure_excel_row_height.patched_styles(STYLES_XML, "Arial", 11.0)

        self.assertNotIn("\\", patched)

    def test_whole_sizes_lose_their_trailing_zero(self) -> None:
        self.assertIn(
            '<sz val="11"/>', measure_excel_row_height.patched_styles(STYLES_XML, "Arial", 11.0)
        )
        self.assertIn(
            '<sz val="10.5"/>',
            measure_excel_row_height.patched_styles(STYLES_XML, "Arial", 10.5),
        )

    def test_a_stylesheet_with_no_font_is_an_error(self) -> None:
        with self.assertRaises(ProbeError):
            measure_excel_row_height.patched_styles("<styleSheet/>", "Arial", 11.0)


class VariantFileNameTests(unittest.TestCase):
    def test_families_sharing_no_ascii_get_distinct_names(self) -> None:
        first = measure_excel_row_height.variant_file_name(0, "맑은 고딕", 12.0)
        second = measure_excel_row_height.variant_file_name(1, "나눔명조", 12.0)

        self.assertNotEqual(first, second)

    def test_sizes_within_a_family_get_distinct_names(self) -> None:
        self.assertNotEqual(
            measure_excel_row_height.variant_file_name(0, "Arial", 11.0),
            measure_excel_row_height.variant_file_name(0, "Arial", 12.0),
        )


class SweepTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.base = self.root / "base.xlsx"
        with zipfile.ZipFile(self.base, "w") as archive:
            archive.writestr("[Content_Types].xml", "<Types/>")
            archive.writestr("xl/styles.xml", STYLES_XML)

    def run_sweep(self, heights: dict[str, float], families: list[str], sizes: list[str]):
        recorded: dict[str, list[str]] = {}

        def fake_read(stage: Path, file_names: list[str]) -> dict[str, float]:
            recorded["names"] = list(file_names)
            recorded["staged"] = sorted(path.name for path in Path(stage).glob("*.xlsx"))
            return {name: heights.get(name, 16.0) for name in file_names}

        with patch.object(measure_excel_row_height, "read_heights", fake_read):
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                status = measure_excel_row_height.main(
                    [
                        "--base", str(self.base),
                        "--stage-root", str(self.root / "stage"),
                        "--report-dir", str(self.root / "reports"),
                        "--name", "sweep",
                        "--families", *families,
                        "--sizes", *sizes,
                    ]
                )
        return status, recorded

    def test_every_family_and_size_reaches_excel_as_its_own_package(self) -> None:
        status, recorded = self.run_sweep({}, ["Arial", "맑은 고딕"], ["10", "12"])

        self.assertEqual(status, 0)
        # Two families x two sizes, plus the base and its no-patch re-zip control.
        self.assertEqual(len(recorded["staged"]), 6)
        self.assertEqual(len(set(recorded["names"])), 6)

    def test_a_control_that_moves_aborts_the_sweep(self) -> None:
        """Repackaging alone must not change the reading, or the sweep measures it."""
        status, _ = self.run_sweep(
            {"base.xlsx": 16.0, "control.xlsx": 18.0}, ["Arial"], ["10"]
        )

        self.assertEqual(status, 1)


class ReportTests(unittest.TestCase):
    def test_each_family_reports_its_own_column(self) -> None:
        sizes = [10.0, 12.0]
        families = ["Arial", "맑은 고딕"]
        heights = {
            measure_excel_row_height.variant_file_name(0, "Arial", 10.0): 13.0,
            measure_excel_row_height.variant_file_name(0, "Arial", 12.0): 16.0,
            measure_excel_row_height.variant_file_name(1, "맑은 고딕", 10.0): 15.0,
            measure_excel_row_height.variant_file_name(1, "맑은 고딕", 12.0): 18.0,
        }

        report = measure_excel_row_height.render_report(
            Path("base.xlsx"), families, sizes, heights
        )

        self.assertIn("| Arial | 13 | 16 |", report)
        self.assertIn("| 맑은 고딕 | 15 | 18 |", report)


if __name__ == "__main__":
    unittest.main()

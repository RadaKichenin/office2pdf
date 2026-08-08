"""Unit tests for the three-axis render comparison.

Covers the trace page split, which decides whether the geometry axis sees any
pages at all. mutool's `<page>` opening tag has varied across releases, and a
split that misses it drops every line silently rather than failing.

Also covers the ImageMagick entry point, which decides whether the colour and
pixel axes run at all: IM7 ships one `magick` dispatcher, IM6 ships the tools
under their own names, and a host may have neither.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import compare_render


def trace_page(body: str, numbered: bool) -> str:
    attrs = 'number="1" mediabox="0 0 595.2 841.92"' if numbered else 'mediabox="0 0 595.2 841.92"'
    return f"<page {attrs}>\n{body}\n</page>"


def text_op(char: str, x: float, baseline_y: float) -> str:
    """One fill_text whose glyph sits at ``baseline_y`` in device points.

    Mirrors the scaled text space Office exports on macOS use, so the glyph
    coordinates have to be run back through the transform to be read.
    """
    scale = 0.24
    gx = x / scale
    gy = (baseline_y - 841.92) / -scale
    return (
        f'<fill_text transform="{scale} 0 0 -{scale} 0 841.92">\n'
        f'<span font="AAAAAA+ArialMT" wmode="0" trm="44 0 0 44">\n'
        f'<g unicode="{char}" glyph="2" x="{gx:.4f}" y="{gy:.4f}" adv=".5"/>\n'
        f"</span>\n</fill_text>"
    )


class TracePageSplitTest(unittest.TestCase):
    def test_splits_page_without_number_attribute(self) -> None:
        doc = trace_page(text_op("A", 72.0, 100.0), numbered=False)
        self.assertEqual(len(compare_render.TRACE_PAGE_RE.split(doc)[1:]), 1)

    def test_splits_page_with_number_attribute(self) -> None:
        doc = trace_page(text_op("A", 72.0, 100.0), numbered=True)
        self.assertEqual(len(compare_render.TRACE_PAGE_RE.split(doc)[1:]), 1)

    def test_counts_every_page_in_a_mixed_document(self) -> None:
        doc = "\n".join(
            [
                trace_page(text_op("A", 72.0, 100.0), numbered=False),
                trace_page(text_op("B", 72.0, 100.0), numbered=True),
            ]
        )
        self.assertEqual(len(compare_render.TRACE_PAGE_RE.split(doc)[1:]), 2)


def only_on_path(*names: str):
    """Patch `shutil.which` so exactly `names` resolve, mimicking a real host."""
    available = set(names)
    return mock.patch.object(
        compare_render.shutil,
        "which",
        side_effect=lambda name: f"/usr/bin/{name}" if name in available else None,
    )


class ImageMagickEntryPointTest(unittest.TestCase):
    """An IM6 host names the tools `convert`, `identify` and `compare`.

    Hardcoding IM7's `magick` aborted the run with FileNotFoundError after the
    geometry axis had already printed, so a partial report looked complete.
    """

    def test_prefers_the_im7_dispatcher_when_present(self) -> None:
        with only_on_path("magick", "convert", "identify", "compare"):
            self.assertEqual(compare_render.imagemagick_command("convert"), ["magick"])
            self.assertEqual(
                compare_render.imagemagick_command("identify"), ["magick", "identify"]
            )
            self.assertEqual(
                compare_render.imagemagick_command("compare"), ["magick", "compare"]
            )

    def test_falls_back_to_the_im6_tool_names(self) -> None:
        with only_on_path("convert", "identify", "compare"):
            self.assertEqual(compare_render.imagemagick_command("convert"), ["convert"])
            self.assertEqual(compare_render.imagemagick_command("identify"), ["identify"])
            self.assertEqual(compare_render.imagemagick_command("compare"), ["compare"])

    def test_reports_absence_instead_of_guessing(self) -> None:
        with only_on_path("pdftoppm"):
            for tool in compare_render.IMAGEMAGICK_TOOLS:
                self.assertIsNone(compare_render.imagemagick_command(tool))

    def test_availability_follows_the_whole_tool_set(self) -> None:
        with only_on_path("magick"):
            self.assertTrue(compare_render.has_imagemagick())
        with only_on_path("convert", "identify", "compare"):
            self.assertTrue(compare_render.has_imagemagick())
        with only_on_path("convert", "identify"):
            self.assertFalse(compare_render.has_imagemagick())
        with only_on_path("pdftoppm"):
            self.assertFalse(compare_render.has_imagemagick())


class DiagnoseWithoutColourTest(unittest.TestCase):
    """With no ImageMagick, two of three axes are missing, not agreeing."""

    def setUp(self) -> None:
        self.geometry = {
            "mad_y": 0.1,
            "mad_x": 0.1,
            "page_mismatch": 0.0,
            "matched": 40.0,
            "coverage": 0.9,
        }

    def render(self, histogram_result: dict[str, float] | None) -> str:
        from io import StringIO
        from contextlib import redirect_stdout

        buffer = StringIO()
        with redirect_stdout(buffer):
            compare_render.diagnose(self.geometry, histogram_result)
        return buffer.getvalue()

    def test_says_the_colour_axis_did_not_run(self) -> None:
        report = self.render(None)
        self.assertIn("colour", report.lower())
        self.assertNotIn("No axis shows a material difference", report)

    def test_still_claims_agreement_when_every_axis_ran(self) -> None:
        report = self.render({"intersection": 1.0, "shift": 0.0, "ink_delta": 0.0})
        self.assertIn("No axis shows a material difference", report)


if __name__ == "__main__":
    unittest.main()

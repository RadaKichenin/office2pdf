"""Unit tests for the three-axis render comparison.

Covers the trace page split, which decides whether the geometry axis sees any
pages at all. mutool's `<page>` opening tag has varied across releases, and a
split that misses it drops every line silently rather than failing.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

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


if __name__ == "__main__":
    unittest.main()

"""Unit tests for the trace-based layout differ.

All fixtures are synthetic ``mutool draw -F trace`` fragments that mirror the
real output format (numberless ``<page>`` opening tag, device transform on each
op, glyph coordinates in text space, sizes in trm units), so the parser is
exercised on the same shapes it will meet in production without shelling out to
mutool.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import compare_layout


def trace_document(*pages: str, numbered: bool = False) -> str:
    """Wrap page bodies in a trace document.

    Defaults to the numberless `<page mediabox="...">` mutool 1.23.x emits, so
    the shared fixtures exercise the shape the parser actually meets. Pass
    ``numbered=True`` for the attribute later builds add.
    """
    body = "\n".join(
        "<page {}mediabox=\"0 0 595.2 841.92\">\n{}\n</page>".format(
            f'number="{i + 1}" ' if numbered else "", content
        )
        for i, content in enumerate(pages)
    )
    return f'<?xml version="1.0"?>\n<document filename="x.pdf">\n{body}\n</document>'


def text_op(
    words: list[tuple[str, float]],
    baseline_y: float,
    size_units: float = 44.0,
    scale: float = 0.24,
    color: str = "0 0 0",
    font: str = "AAAAAA+ArialMT",
) -> str:
    """One fill_text whose glyphs sit at ``baseline_y`` (device pt).

    ``words`` is a list of (character, device_x) pairs; coordinates are
    converted back into text space so the parser has to apply the transform.
    """
    offset = 841.92
    glyph_lines = []
    for char, device_x in words:
        text_x = device_x / scale
        text_y = (offset - baseline_y) / scale
        glyph_lines.append(
            f'<g unicode="{char}" glyph="1" x="{text_x:.4f}" y="{text_y:.4f}" adv=".5"/>'
        )
    glyphs = "\n".join(glyph_lines)
    return (
        f'<fill_text colorspace="ICCBased(RGB,sRGB IEC61966-2.1)" color="{color}" '
        f'ri="1" bp="1" op="0" opm="0" transform="{scale} 0 0 -{scale} 0 {offset}">\n'
        f'<span font="{font}" wmode="0" bidi="0" trm="{size_units} 0 0 {size_units}">\n'
        f"{glyphs}\n</span>\n</fill_text>"
    )


def line_of(text: str, x0: float, baseline_y: float, pitch: float = 6.0) -> str:
    words = [(char, x0 + i * pitch) for i, char in enumerate(text)]
    return text_op(words, baseline_y)


def rect_op(
    x0: float,
    y0: float,
    x1: float,
    y1: float,
    kind: str = "fill_path",
    color: str = ".8 .8 .8",
    alpha: float = 1.0,
) -> str:
    extra = ' winding="nonzero"' if kind == "fill_path" else ' linewidth="1"'
    return (
        f'<{kind}{extra} colorspace="ICCBased(RGB,sRGB IEC61966-2.1)" color="{color}" '
        f'alpha="{alpha}" ri="1" bp="1" op="0" opm="0" '
        f'transform="1 0 0 -1 0 841.92">\n'
        f'<moveto x="{x0}" y="{841.92 - y0}"/>\n'
        f'<lineto x="{x1}" y="{841.92 - y0}"/>\n'
        f'<lineto x="{x1}" y="{841.92 - y1}"/>\n'
        f'<lineto x="{x0}" y="{841.92 - y1}"/>\n'
        f"<closepath/>\n</{kind}>"
    )


def image_op() -> str:
    return (
        '<fill_image alpha="1" colorspace="DeviceRGB" ri="1" bp="1" op="0" opm="0" '
        'transform="595.2 0 0 841.92 0 0" width="1280" height="720"/>'
    )


def clipped_shade_op(
    x0: float,
    y0: float,
    x1: float,
    y1: float,
    alpha: float = 1.0,
    extend: str = "1 1",
    clip_transform: str = "1 0 0 1 0 0",
) -> str:
    return "\n".join(
        [
            f'<clip_path winding="nonzero" transform="{clip_transform}">',
            f'<moveto x="{x0}" y="{y0}"/>',
            f'<lineto x="{x1}" y="{y0}"/>',
            f'<lineto x="{x1}" y="{y1}"/>',
            f'<lineto x="{x0}" y="{y1}"/>',
            "<closepath/>",
            "</clip_path>",
            f'<fill_shade alpha="{alpha}" transform="1 0 0 1 0 0" '
            f'type="linear" extend="{extend}" start="0 0" end="1 1"/>',
            "<pop_clip/>",
        ]
    )


class PageElementTest(unittest.TestCase):
    """The ``<page>`` opening tag differs across mutool releases.

    1.23.x emits ``<page mediabox="...">`` with no ``number`` attribute, so a
    parser that requires one measures nothing at all — and reports that as
    "no differences" rather than as a failure.
    """

    def test_page_without_number_attribute_is_parsed(self) -> None:
        pages = compare_layout.parse_trace(
            trace_document(text_op([("A", 72.0)], baseline_y=100.0))
        )
        self.assertEqual(len(pages), 1)
        self.assertEqual(pages[0].lines[0].text, "A")

    def test_pages_without_number_keep_document_order(self) -> None:
        pages = compare_layout.parse_trace(
            trace_document(
                text_op([("A", 72.0)], baseline_y=100.0),
                text_op([("B", 72.0)], baseline_y=100.0),
            )
        )
        self.assertEqual([p.lines[0].text for p in pages], ["A", "B"])

    def test_page_with_number_attribute_still_parses(self) -> None:
        pages = compare_layout.parse_trace(
            trace_document(text_op([("A", 72.0)], baseline_y=100.0), numbered=True)
        )
        self.assertEqual(len(pages), 1)
        self.assertEqual(pages[0].lines[0].text, "A")


class ParseTraceTest(unittest.TestCase):
    def test_glyphs_carry_device_coordinates_and_pt_sizes(self) -> None:
        doc = trace_document(text_op([("A", 72.0), ("B", 78.0)], baseline_y=100.0))
        pages = compare_layout.parse_trace(doc)
        self.assertEqual(len(pages), 1)
        line = pages[0].lines[0]
        self.assertEqual(line.text, "AB")
        self.assertAlmostEqual(line.glyphs[0].x, 72.0, places=3)
        self.assertAlmostEqual(line.glyphs[0].y, 100.0, places=3)
        # trm 44 under a 0.24 transform is a 10.56pt glyph.
        self.assertAlmostEqual(line.glyphs[0].size, 10.56, places=3)
        # adv .5 em at that size is 5.28pt.
        self.assertAlmostEqual(line.glyphs[0].advance, 5.28, places=3)

    def test_glyphs_on_one_baseline_group_into_one_line_in_x_order(self) -> None:
        page = "\n".join(
            [
                text_op([("B", 80.0)], baseline_y=100.0),
                text_op([("A", 72.0)], baseline_y=100.2),
                text_op([("C", 90.0)], baseline_y=300.0),
            ]
        )
        pages = compare_layout.parse_trace(trace_document(page))
        texts = [line.text for line in pages[0].lines]
        self.assertEqual(texts, ["AB", "C"])

    def test_rotated_fill_text_uses_the_full_affine_transform_and_stays_one_run(self) -> None:
        page = """<fill_text transform="1 2 3 4 5 6">
          <span font="AAAAAA+ArialMT" trm="10 0 0 10">
            <g unicode="A" glyph="A" x="7" y="11" adv=".5"/>
            <g unicode="B" glyph="B" x="8" y="11" adv=".5"/>
          </span>
        </fill_text>"""
        lines = compare_layout.parse_trace(trace_document(page))[0].lines
        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0].text, "AB")
        self.assertAlmostEqual(lines[0].x0, 45.0)
        self.assertAlmostEqual(lines[0].y, 64.0)

    def test_rect_bbox_uses_the_full_affine_transform(self) -> None:
        page = """<fill_path transform="1 2 3 4 5 6">
          <moveto x="7" y="11"/>
          <lineto x="8" y="12"/>
        </fill_path>"""
        rect = compare_layout.parse_trace(trace_document(page))[0].rects[0]
        self.assertEqual((rect.x0, rect.y0, rect.x1, rect.y1), (45.0, 64.0, 49.0, 70.0))

    def test_rects_capture_device_bbox_and_kind(self) -> None:
        page = "\n".join(
            [rect_op(69.36, 792.96, 525.84, 793.44), rect_op(10, 20, 30, 21, "stroke_path")]
        )
        pages = compare_layout.parse_trace(trace_document(page))
        rects = pages[0].rects
        self.assertEqual(len(rects), 2)
        fill = next(r for r in rects if r.kind == "fill")
        self.assertAlmostEqual(fill.x0, 69.36, places=2)
        self.assertAlmostEqual(fill.y1, 793.44, places=2)
        self.assertEqual({r.kind for r in rects}, {"fill", "stroke"})


class MatchAndDiffTest(unittest.TestCase):
    def diff(self, gt_page: str, out_page: str, **kwargs) -> dict:
        gt = compare_layout.parse_trace(trace_document(gt_page))[0]
        out = compare_layout.parse_trace(trace_document(out_page))[0]
        return compare_layout.diff_page(gt, out, **kwargs)

    def test_identical_pages_report_no_deviation(self) -> None:
        page = "\n".join(
            [line_of("hello", 72, 100), line_of("world", 72, 112), rect_op(70, 90, 300, 91)]
        )
        vector = self.diff(page, page)
        self.assertEqual(vector["lines"]["matched"], 2)
        self.assertEqual(vector["lines"]["missing"], 0)
        self.assertEqual(vector["lines"]["extra"], 0)
        self.assertEqual(vector["lines"]["deviant"], 0)
        self.assertAlmostEqual(vector["baseline"]["mean_abs_dy"], 0.0, places=6)
        self.assertEqual(vector["rects"]["gt_count"], vector["rects"]["out_count"])

    def test_shifted_line_reports_dy_and_deviant_count(self) -> None:
        gt = "\n".join([line_of("hello", 72, 100), line_of("world", 72, 112)])
        out = "\n".join([line_of("hello", 72, 100), line_of("world", 72, 114)])
        vector = self.diff(gt, out)
        self.assertEqual(vector["lines"]["deviant"], 1)
        self.assertAlmostEqual(vector["baseline"]["worst_dy"], 2.0, places=3)
        worst = vector["baseline"]["worst_line"]
        self.assertIn("world", worst)

    def test_sub_noise_floor_shift_is_not_deviant(self) -> None:
        gt = line_of("hello", 72, 100)
        out = line_of("hello", 72, 100.08)
        vector = self.diff(gt, out, noise_floor=0.12)
        self.assertEqual(vector["lines"]["deviant"], 0)
        # The raw statistic still carries the measurement.
        self.assertGreater(vector["baseline"]["worst_dy"], 0.0)

    def test_missing_and_extra_lines_are_counted(self) -> None:
        gt = "\n".join([line_of("alpha", 72, 100), line_of("beta", 72, 112)])
        out = line_of("alpha", 72, 100)
        vector = self.diff(gt, out)
        self.assertEqual(vector["lines"]["missing"], 1)
        self.assertEqual(vector["lines"]["extra"], 0)
        self.assertIn("beta", vector["lines"]["missing_text"][0])
        self.assertEqual(compare_layout.audit_failures([vector]), 1)

    def test_wrap_difference_is_detected_not_reported_missing(self) -> None:
        gt = "\n".join([line_of("abcdef", 72, 100), line_of("ghi", 72, 112)])
        out = "\n".join([line_of("abc", 72, 100), line_of("defghi", 72, 112)])
        vector = self.diff(gt, out)
        self.assertEqual(vector["lines"]["missing"], 0)
        self.assertEqual(vector["lines"]["extra"], 0)
        self.assertEqual(vector["wraps"]["count"], 1)
        self.assertEqual(compare_layout.audit_failures([vector]), 1)

    def test_reordered_content_is_classified_reflow_not_loss(self) -> None:
        # A table row whose cells share one baseline in GT but split across two
        # baselines in the output: same characters, different line grouping.
        gt = line_of("labelvalue", 72, 100)
        out = "\n".join([line_of("value", 120, 99), line_of("label", 72, 103)])
        vector = self.diff(gt, out)
        self.assertEqual(vector["lines"]["missing"], 0)
        self.assertEqual(vector["lines"]["extra"], 0)
        self.assertEqual(vector["reflow"]["gt_lines"], 1)
        self.assertEqual(vector["reflow"]["out_lines"], 2)
        self.assertEqual(compare_layout.audit_failures([vector]), 1)

    def test_real_text_loss_is_still_reported_missing(self) -> None:
        gt = "\n".join([line_of("kept", 72, 100), line_of("lost", 72, 112)])
        out = "\n".join([line_of("kept", 72, 100), line_of("other", 72, 112)])
        vector = self.diff(gt, out)
        self.assertEqual(vector["lines"]["missing"], 1)
        self.assertEqual(vector["lines"]["extra"], 1)
        self.assertEqual(vector["reflow"]["gt_lines"], 0)

    def test_pitch_delta_between_consecutive_matched_lines(self) -> None:
        gt = "\n".join([line_of("one", 72, 100), line_of("two", 72, 112), line_of("three", 72, 124)])
        out = "\n".join([line_of("one", 72, 100), line_of("two", 72, 113), line_of("three", 72, 126)])
        vector = self.diff(gt, out)
        self.assertAlmostEqual(vector["pitch"]["worst_delta"], 1.0, places=3)
        self.assertEqual(vector["pitch"]["pairs"], 2)

    def test_width_drift_is_relative(self) -> None:
        gt = line_of("wide", 72, 100, pitch=6.0)
        out = line_of("wide", 72, 100, pitch=6.3)
        vector = self.diff(gt, out)
        self.assertGreater(vector["width"]["worst_pct"], 3.0)

    def test_rect_census_reports_count_delta(self) -> None:
        gt = "\n".join([rect_op(70, 90, 300, 91), rect_op(70, 110, 300, 111, "stroke_path")])
        out = rect_op(70, 90, 300, 91)
        vector = self.diff(gt, out)
        self.assertEqual(vector["rects"]["gt_count"], 2)
        self.assertEqual(vector["rects"]["out_count"], 1)

    def test_repeated_labels_are_spatially_matched_and_large_shifts_are_named(self) -> None:
        gt = "\n".join(
            [line_of("Sales", 337, 133), line_of("Sales", 553, 286)]
        )
        out = "\n".join(
            [line_of("Sales", 457, 134), line_of("Sales", 526, 286)]
        )

        vector = self.diff(gt, out, large_shift=5.0)

        self.assertEqual(vector["lines"]["matched"], 2)
        self.assertEqual(vector["instances"]["large_shift_count"], 2)
        self.assertEqual(
            [item["label"] for item in vector["instances"]["large_shifts"]],
            ["Sales [1/2]", "Sales [2/2]"],
        )
        self.assertEqual(
            [round(item["dx"]) for item in vector["instances"]["large_shifts"]],
            [120, -27],
        )
        self.assertEqual(compare_layout.audit_failures([vector]), 2)

    def test_same_text_hidden_by_later_image_reports_visibility_mismatch(self) -> None:
        text = line_of("Slide9", 72, 100)
        gt = "\n".join([text, image_op()])
        out = "\n".join([image_op(), text])

        vector = self.diff(gt, out)

        self.assertEqual(vector["lines"]["matched"], 1)
        self.assertEqual(vector["visibility"]["mismatch_count"], 1)
        self.assertEqual(
            vector["visibility"]["mismatches"][0],
            {"label": "Slide9", "gt": "hidden", "out": "painted"},
        )
        self.assertEqual(compare_layout.audit_failures([vector]), 1)

    def test_same_text_hidden_by_later_clipped_shading_reports_visibility_mismatch(self) -> None:
        text = line_of("Covered", 72, 100)
        shade = clipped_shade_op(60, 80, 150, 110)

        vector = self.diff("\n".join([text, shade]), "\n".join([shade, text]))

        self.assertEqual(vector["visibility"]["mismatch_count"], 1)
        self.assertEqual(
            vector["visibility"]["mismatches"][0],
            {"label": "Covered", "gt": "hidden", "out": "painted"},
        )

    def test_translucent_clipped_shading_does_not_hide_text(self) -> None:
        text = line_of("Tinted", 72, 100)
        shade = clipped_shade_op(60, 80, 150, 110, alpha=0.5)

        vector = self.diff("\n".join([text, shade]), "\n".join([shade, text]))

        self.assertEqual(vector["visibility"]["mismatch_count"], 0)

    def test_noncovering_clipped_shading_does_not_hide_text(self) -> None:
        text = line_of("Clear", 72, 100)
        shade = clipped_shade_op(200, 80, 300, 110)

        vector = self.diff("\n".join([text, shade]), "\n".join([shade, text]))

        self.assertEqual(vector["visibility"]["mismatch_count"], 0)

    def test_clipped_shading_painted_before_text_does_not_hide_it(self) -> None:
        page = "\n".join([clipped_shade_op(60, 80, 150, 110), line_of("Above", 72, 100)])

        line = compare_layout.parse_trace(trace_document(page))[0].lines[0]

        self.assertEqual(line.visibility, "painted")

    def test_nonextended_or_rotated_clipped_shading_is_not_overclaimed(self) -> None:
        text = line_of("Conservative", 72, 100)
        nonextended = clipped_shade_op(60, 80, 180, 110, extend="0 0")
        rotated = clipped_shade_op(
            0,
            0,
            120,
            30,
            clip_transform="0.7071 0.7071 -0.7071 0.7071 75 65",
        )

        for shade in (nonextended, rotated):
            page = "\n".join([text, shade])
            line = compare_layout.parse_trace(trace_document(page))[0].lines[0]
            self.assertEqual(line.visibility, "painted")

    def test_bow_tie_clip_does_not_overclaim_shading_coverage(self) -> None:
        text = line_of("Crossed", 72, 100)
        bow_tie = """<clip_path winding="nonzero" transform="1 0 0 1 0 0">
          <moveto x="60" y="80"/><lineto x="150" y="110"/>
          <lineto x="150" y="80"/><lineto x="60" y="110"/><closepath/>
        </clip_path>
        <fill_shade alpha="1" transform="1 0 0 1 0 0"
          type="linear" extend="1 1" start="0 0" end="1 1"/>
        <pop_clip/>"""

        line = compare_layout.parse_trace(trace_document("\n".join([text, bow_tie])))[0].lines[0]

        self.assertEqual(line.visibility, "painted")

    def test_multiple_clip_subpaths_do_not_overclaim_shading_coverage(self) -> None:
        text = line_of("Subpaths", 72, 100)
        multiple = """<clip_path winding="nonzero" transform="1 0 0 1 0 0">
          <moveto x="60" y="80"/><lineto x="150" y="80"/>
          <lineto x="150" y="110"/><lineto x="60" y="110"/><closepath/>
          <moveto x="70" y="90"/><lineto x="80" y="90"/>
          <lineto x="80" y="100"/><lineto x="70" y="100"/><closepath/>
        </clip_path>
        <fill_shade alpha="1" transform="1 0 0 1 0 0"
          type="linear" extend="1 1" start="0 0" end="1 1"/>
        <pop_clip/>"""

        line = compare_layout.parse_trace(trace_document("\n".join([text, multiple])))[0].lines[0]

        self.assertEqual(line.visibility, "painted")

    def test_nested_shading_keeps_the_outer_clip_intersection(self) -> None:
        text = line_of("A", 72, 100)
        partial_outer = """<clip_path winding="nonzero" transform="1 0 0 1 0 0">
          <moveto x="60" y="80"/><lineto x="74" y="80"/>
          <lineto x="74" y="110"/><lineto x="60" y="110"/><closepath/>
        </clip_path>"""
        full_inner = clipped_shade_op(60, 80, 150, 110)
        page = "\n".join([text, partial_outer, full_inner, "<pop_clip/>"])

        line = compare_layout.parse_trace(trace_document(page))[0].lines[0]

        self.assertEqual(line.visibility, "painted")

    def test_unknown_outer_clip_prevents_a_nested_rectangle_from_overclaiming(self) -> None:
        text = line_of("A", 72, 100)
        triangular_outer = """<clip_path winding="nonzero" transform="1 0 0 1 0 0">
          <moveto x="60" y="80"/><lineto x="150" y="80"/>
          <lineto x="60" y="110"/><closepath/>
        </clip_path>"""
        full_inner = clipped_shade_op(60, 80, 150, 110)
        page = "\n".join([text, triangular_outer, full_inner, "<pop_clip/>"])

        line = compare_layout.parse_trace(trace_document(page))[0].lines[0]

        self.assertEqual(line.visibility, "painted")

    def test_same_color_text_on_flat_fill_reports_low_contrast(self) -> None:
        background = rect_op(0, 0, 595.2, 841.92, color=".8 .8 .8")
        gt = "\n".join(
            [
                background,
                line_of("Muted", 72, 100).replace(
                    'color="0 0 0"', 'color=".8 .8 .8"'
                ),
            ]
        )
        out = "\n".join([background, line_of("Muted", 72, 100)])

        vector = self.diff(gt, out)

        self.assertEqual(vector["visibility"]["mismatch_count"], 1)
        self.assertEqual(
            vector["visibility"]["mismatches"][0],
            {"label": "Muted", "gt": "low_contrast", "out": "painted"},
        )

    def test_later_opaque_rectangle_hides_text(self) -> None:
        text = line_of("Covered", 72, 100)
        cover = rect_op(60, 80, 150, 110)

        vector = self.diff("\n".join([text, cover]), "\n".join([cover, text]))

        self.assertEqual(vector["visibility"]["mismatch_count"], 1)
        self.assertEqual(vector["visibility"]["mismatches"][0]["gt"], "hidden")

    def test_later_translucent_rectangle_does_not_hide_text(self) -> None:
        text = line_of("Tinted", 72, 100)
        tint = rect_op(60, 80, 150, 110, alpha=0.5)

        vector = self.diff("\n".join([text, tint]), "\n".join([tint, text]))

        self.assertEqual(vector["visibility"]["mismatch_count"], 0)

    def test_minor_text_color_substitution_is_not_a_visibility_mismatch(self) -> None:
        background = rect_op(0, 0, 595.2, 841.92, color="1 1 1")
        gt = "\n".join([background, line_of("Stable", 72, 100)])
        out = "\n".join(
            [
                background,
                line_of("Stable", 72, 100).replace(
                    'color="0 0 0"', 'color=".02 .02 .02"'
                ),
            ]
        )

        vector = self.diff(gt, out)

        self.assertEqual(vector["visibility"]["mismatch_count"], 0)


class ReadingTest(unittest.TestCase):
    def test_reading_mentions_wrap_and_missing_content(self) -> None:
        gt = compare_layout.parse_trace(
            trace_document("\n".join([line_of("abcdef", 72, 100), line_of("ghi", 72, 112)]))
        )[0]
        out = compare_layout.parse_trace(
            trace_document("\n".join([line_of("abc", 72, 100), line_of("defghi", 72, 112)]))
        )[0]
        vector = compare_layout.diff_page(gt, out)
        reading = compare_layout.render_reading([vector])
        self.assertIn("wrap", reading.lower())

    def test_reading_names_a_large_repeated_label_shift(self) -> None:
        gt = compare_layout.parse_trace(
            trace_document("\n".join([line_of("Sales", 337, 133), line_of("Sales", 553, 286)]))
        )[0]
        out = compare_layout.parse_trace(
            trace_document("\n".join([line_of("Sales", 457, 134), line_of("Sales", 526, 286)]))
        )[0]

        reading = compare_layout.render_reading(
            [compare_layout.diff_page(gt, out, large_shift=5.0)]
        )

        self.assertIn("Sales [1/2]", reading)
        self.assertIn("+120.00pt", reading)


if __name__ == "__main__":
    unittest.main()

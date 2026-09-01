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
from contextlib import redirect_stdout
from io import StringIO
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


def rotated_text_op(text: str, origin_x: float, origin_y: float) -> str:
    """One 45-degree fill_text whose glyphs share a text-space baseline."""
    glyphs = "\n".join(
        f'<g unicode="{char}" glyph="2" x="{index * 10}" y="100" adv=".5"/>'
        for index, char in enumerate(text)
    )
    return (
        '<fill_text transform=".70710678 -.70710678 .70710678 .70710678 '
        f'{origin_x} {origin_y}">\n'
        '<span font="AAAAAA+ArialMT" wmode="0" trm="44 0 0 44">\n'
        f"{glyphs}\n"
        "</span>\n</fill_text>"
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


class AffineTextPositionTest(unittest.TestCase):
    def test_rotated_text_uses_the_complete_affine_transform(self) -> None:
        trace = trace_page(rotated_text_op("AB", 500.0, 600.0), numbered=True)

        with mock.patch.object(compare_render.subprocess, "run") as run:
            run.return_value = mock.Mock(returncode=0, stdout=trace)
            lines = compare_render.baseline_lines(Path("rotated.pdf"))

        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0].text, "AB")
        self.assertAlmostEqual(lines[0].x_min, 570.710678, places=5)
        self.assertAlmostEqual(lines[0].y_min, 663.6396102, places=5)


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

    def test_requested_vision_artifacts_fail_if_imagemagick_is_absent(self) -> None:
        with only_on_path("pdftoppm"):
            with self.assertRaisesRegex(SystemExit, "--artifacts-dir requires ImageMagick"):
                compare_render.require_vision_artifact_dependencies(Path("artifacts"))

    def test_no_artifact_request_does_not_require_imagemagick(self) -> None:
        with only_on_path("pdftoppm"):
            compare_render.require_vision_artifact_dependencies(None)


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

    def test_element_shift_prevents_false_antialiasing_verdict(self) -> None:
        self.geometry.update(
            {
                "large_shift_count": 1.0,
                "large_shift_threshold": 5.0,
                "worst_dx": 120.0,
            }
        )

        report = self.render({"intersection": 1.0, "shift": 0.0, "ink_delta": 0.0})

        self.assertIn("matched text instance", report)
        self.assertNotIn("No axis shows a material difference", report)


class RepeatedTextGeometryTest(unittest.TestCase):
    """Repeated labels must be paired as spatial instances, never discarded."""

    def setUp(self) -> None:
        self.gt_lines = [
            compare_render.TextLine(0, 337.0, 133.0, "Sales"),
            compare_render.TextLine(0, 553.0, 286.0, "Sales"),
        ]
        self.output_lines = [
            compare_render.TextLine(0, 457.0, 134.0, "Sales"),
            compare_render.TextLine(0, 526.0, 286.0, "Sales"),
        ]

    def test_repeated_labels_are_matched_by_spatial_instance(self) -> None:
        matches = compare_render.match_text_line_instances(self.gt_lines, self.output_lines)

        self.assertEqual(len(matches), 2)
        self.assertEqual([match.label for match in matches], ["Sales [1/2]", "Sales [2/2]"])
        self.assertEqual([round(match.dx) for match in matches], [120, -27])

    def test_report_names_the_displaced_repeated_instance(self) -> None:
        with mock.patch.object(
            compare_render, "text_lines", side_effect=[self.gt_lines, self.output_lines]
        ):
            output = StringIO()
            with redirect_stdout(output):
                result = compare_render.report_geometry(
                    Path("gt.pdf"), Path("output.pdf"), large_shift=5.0
                )

        self.assertEqual(result["matched"], 2.0)
        self.assertEqual(result["large_shift_count"], 2.0)
        self.assertAlmostEqual(result["worst_dx"], 120.0)
        self.assertIn("Sales [1/2]", output.getvalue())
        self.assertIn("+120.00pt", output.getvalue())

    def test_report_keeps_coarse_summary_and_adds_fine_shift_gate(self) -> None:
        gt_lines = [compare_render.TextLine(0, 10.0, 20.0, "Fine detail")]
        output_lines = [compare_render.TextLine(0, 10.0, 20.75, "Fine detail")]
        with mock.patch.object(
            compare_render, "text_lines", side_effect=[gt_lines, output_lines]
        ):
            result = compare_render.report_geometry(
                Path("gt.pdf"),
                Path("output.pdf"),
                large_shift=5.0,
                fine_shift=0.5,
            )

        self.assertEqual(result["large_shift_count"], 0.0)
        self.assertEqual(result["large_shift_threshold"], 5.0)
        self.assertEqual(result["fine_shift_count"], 1.0)
        self.assertEqual(result["fine_shift_threshold"], 0.5)

    def test_report_scopes_geometry_to_the_requested_page(self) -> None:
        gt_lines = [
            compare_render.TextLine(0, 10.0, 20.0, "First page"),
            compare_render.TextLine(1, 30.0, 40.0, "Second page"),
        ]
        output_lines = [
            compare_render.TextLine(0, 110.0, 120.0, "First page"),
            compare_render.TextLine(1, 31.0, 42.0, "Second page"),
        ]

        with mock.patch.object(
            compare_render, "text_lines", side_effect=[gt_lines, output_lines]
        ):
            result = compare_render.report_geometry(
                Path("gt.pdf"), Path("output.pdf"), page=2, large_shift=5.0
            )

        self.assertEqual(result["matched"], 1.0)
        self.assertEqual(result["large_shift_count"], 0.0)
        self.assertAlmostEqual(result["worst_dx"], 1.0)
        self.assertAlmostEqual(result["worst_dy"], 2.0)

    def test_shift_crop_contains_both_repeated_label_locations(self) -> None:
        match = compare_render.match_text_line_instances(self.gt_lines, self.output_lines)[0]

        left, top, width, height = compare_render.shift_crop_box(
            match, dpi=144, image_width=1440, image_height=1080
        )

        self.assertLessEqual(left, round(match.reference.x_min * 2))
        self.assertGreaterEqual(left + width, round(match.candidate.x_min * 2))
        self.assertLessEqual(top, round(match.reference.y_min * 2))
        self.assertGreaterEqual(top + height, round(match.candidate.y_min * 2))


class DiffClusterParseTest(unittest.TestCase):
    """The cluster census turns the diff mask into dispositionable regions.

    Issue #1029's squared-off panel corners survived a 5% fuzz sweep because
    the sweep printed one count for the whole page; nothing named where the
    differing pixels sat. The census parses ImageMagick's
    connected-components listing so each contiguous diff region becomes a
    line item with coordinates.
    """

    # Verbatim shape of `-define connected-components:verbose=true` output.
    # Component 0 is the black background, 235/257 are the #1029 corner
    # clusters, 2 is a glyph-rasterisation blob, 99 is a sub-threshold speck.
    VERBOSE = "\n".join(
        [
            "Objects (id: bounding-box centroid area mean-color):",
            "  0: 2000x1125+0+0 987.4,553.9 2.2002e+06 gray(0)",
            "  235: 203x329+1797+796 1941.2,1038.4 22130 gray(255)",
            "  257: 198x313+1004+812 1060.3,1040.6 20988 gray(255)",
            "  2: 67x63+1624+118 1652.8,155.0 1343 gray(255)",
            "  99: 5x5+10+10 12.0,12.0 25 gray(255)",
        ]
    )

    def clusters(self) -> list:
        return compare_render.parse_diff_clusters(self.VERBOSE, dpi=150)

    def test_reports_only_white_components(self) -> None:
        regions = self.clusters()
        self.assertEqual(len(regions), 3)

    def test_sorts_largest_first(self) -> None:
        areas = [region.area_pt2 for region in self.clusters()]
        self.assertEqual(areas, sorted(areas, reverse=True))

    def test_converts_pixels_to_points_at_the_given_dpi(self) -> None:
        largest = self.clusters()[0]
        # 203x329px at +1797+796, 150 DPI: 1px = 0.48pt.
        self.assertAlmostEqual(largest.x_pt, 862.56, places=2)
        self.assertAlmostEqual(largest.y_pt, 382.08, places=2)
        self.assertAlmostEqual(largest.width_pt, 97.44, places=2)
        self.assertAlmostEqual(largest.height_pt, 157.92, places=2)
        self.assertAlmostEqual(largest.area_pt2, 22130 * 0.48 * 0.48, places=1)

    def test_drops_specks_below_the_area_floor(self) -> None:
        # 25px at 150 DPI is 5.76pt² — glyph antialiasing, not a defect region.
        areas_px = [region.area_pt2 / (0.48 * 0.48) for region in self.clusters()]
        self.assertNotIn(25, [round(area) for area in areas_px])

    def test_names_the_page_region_from_the_centroid(self) -> None:
        regions = self.clusters()
        # Page is 2000x1125px; centroids in the bottom-right and bottom-center
        # thirds. Both #1029 corner clusters read as bottom-edge regions.
        self.assertEqual(regions[0].region, "bottom-right")
        self.assertEqual(regions[1].region, "bottom")
        self.assertEqual(regions[2].region, "top-right")

    def test_a_merged_component_still_counts_as_white(self) -> None:
        # area-threshold merging leaves near-pure means such as gray(254.7).
        verbose = "\n".join(
            [
                "Objects (id: bounding-box centroid area mean-color):",
                "  0: 100x100+0+0 50.0,50.0 9000 gray(0.3)",
                "  1: 40x40+30+30 50.0,50.0 1000 gray(254.7)",
            ]
        )
        regions = compare_render.parse_diff_clusters(verbose, dpi=150)
        self.assertEqual(len(regions), 1)
        self.assertEqual(regions[0].region, "center")


class DiagnoseWithClustersTest(unittest.TestCase):
    """A dominant contiguous cluster routes attention to structure, not noise."""

    GEOMETRY = {
        "mad_y": 0.1,
        "mad_x": 0.1,
        "page_mismatch": 0.0,
        "matched": 40.0,
        "coverage": 0.9,
    }
    HISTOGRAM = {"intersection": 1.0, "shift": 0.0, "ink_delta": 0.0}

    def render(self, clusters) -> str:
        from io import StringIO
        from contextlib import redirect_stdout

        buffer = StringIO()
        with redirect_stdout(buffer):
            compare_render.diagnose(self.GEOMETRY, self.HISTOGRAM, clusters)
        return buffer.getvalue()

    def big_cluster(self) -> object:
        return compare_render.DiffCluster(
            x_pt=862.6,
            y_pt=382.1,
            width_pt=97.4,
            height_pt=157.9,
            area_pt2=5100.0,
            region="bottom-right",
        )

    def test_a_dominant_cluster_is_a_finding_even_when_the_axes_agree(self) -> None:
        report = self.render([self.big_cluster()])
        self.assertNotIn("No axis shows a material difference", report)
        self.assertIn("outline", report)

    def test_small_clusters_do_not_override_agreement(self) -> None:
        small = compare_render.DiffCluster(
            x_pt=10.0, y_pt=10.0, width_pt=8.0, height_pt=8.0,
            area_pt2=40.0, region="top-left",
        )
        report = self.render([small])
        self.assertIn("No axis shows a material difference", report)

    def test_no_clusters_keeps_the_old_reading(self) -> None:
        report = self.render(None)
        self.assertIn("No axis shows a material difference", report)



if __name__ == "__main__":
    unittest.main()

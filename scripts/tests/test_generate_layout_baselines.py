"""Unit tests for the layout-baseline producer.

The producer's subprocess boundaries (office2pdf conversion, mutool traces,
the GT integrity gate) are replaced with fakes so the pipeline logic —
per-format noise floors, the zero-page and invalid-GT hard failures, float
rounding, and the schema of the emitted document — is exercised without
external tools.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import generate_layout_baselines as producer


def manifest_case(case_id: str = "docx-invoice-en", source_format: str = "docx") -> dict:
    extension = {"docx": ".docx", "pptx": ".pptx", "xlsx": ".xlsx"}[source_format]
    return {
        "id": case_id,
        "format": source_format,
        "source": f"sources/{source_format}/01_case{extension}",
        "expected_pdf": f"expected/{source_format}/01_case.pdf",
        "expected_pages": 1,
    }


def synthetic_page_layout() -> "producer.compare_layout.PageLayout":
    compare_layout = producer.compare_layout
    glyph = compare_layout.Glyph(x=72.0, y=100.0, unicode="A", size=11.0, advance=6.0)
    line = compare_layout.Line(y=100.0, glyphs=[glyph])
    return compare_layout.PageLayout(lines=[line], rects=[])


class FakePipeline:
    """Stub out every subprocess boundary the producer crosses."""

    def __init__(
        self,
        test_case: unittest.TestCase,
        gt_pages: int = 1,
        out_pages: int = 1,
        gt_invalid: bool = False,
        conversion_error: str | None = None,
    ) -> None:
        module = producer
        originals = (
            module.run_ground_truth_gate,
            module.convert_source,
            module.trace_pages,
        )
        test_case.addCleanup(
            lambda: (
                setattr(module, "run_ground_truth_gate", originals[0]),
                setattr(module, "convert_source", originals[1]),
                setattr(module, "trace_pages", originals[2]),
            )
        )
        module.run_ground_truth_gate = lambda gt, source: {
            "pages": gt_pages,
            "periodicity": 3 if gt_invalid else None,
            "invalid": gt_invalid,
        }

        def fake_convert(office2pdf_binary, source, output_pdf):
            if conversion_error is not None:
                raise producer.BaselineError(conversion_error)
            Path(output_pdf).write_bytes(b"%PDF-fake")

        module.convert_source = fake_convert
        # The GT lives under the corpus `expected/` tree; the converted PDF
        # lives in the producer's scratch directory.
        module.trace_pages = lambda pdf: [
            synthetic_page_layout()
            for _ in range(gt_pages if "expected" in str(pdf) else out_pages)
        ]


class NoiseFloorTests(unittest.TestCase):
    def test_word_and_powerpoint_use_the_quantisation_floor(self) -> None:
        self.assertEqual(producer.NOISE_FLOOR_PT["docx"], 0.12)
        self.assertEqual(producer.NOISE_FLOOR_PT["pptx"], 0.12)

    def test_excel_uses_the_whole_point_floor(self) -> None:
        self.assertEqual(producer.NOISE_FLOOR_PT["xlsx"], 0.5)

    def test_case_record_carries_its_format_floor(self) -> None:
        FakePipeline(self)
        with tempfile.TemporaryDirectory() as tmp:
            record = producer.build_case_record(
                manifest_case("xlsx-budget-ko", "xlsx"),
                Path("office2pdf"),
                Path(tmp),
                Path(tmp),
            )
        self.assertEqual(record["noise_floor_pt"], 0.5)


class HardFailureTests(unittest.TestCase):
    def test_zero_page_trace_is_a_hard_failure_not_a_clean_vector(self) -> None:
        # A trace parsed to zero pages looks exactly like a perfect comparison
        # (issue #808); the producer must refuse to record it.
        FakePipeline(self, gt_pages=0)
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(producer.BaselineError, "no pages parsed"):
                producer.build_case_record(
                    manifest_case(), Path("office2pdf"), Path(tmp), Path(tmp)
                )

    def test_invalid_ground_truth_blocks_recording(self) -> None:
        # Recording a baseline from a corrupt GT bakes the corruption in
        # (issue #616); the gate result must stop the producer.
        FakePipeline(self, gt_invalid=True)
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(producer.BaselineError, "integrity"):
                producer.build_case_record(
                    manifest_case(), Path("office2pdf"), Path(tmp), Path(tmp)
                )

    def test_conversion_failure_names_the_case(self) -> None:
        FakePipeline(self, conversion_error="conversion exploded")
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(producer.BaselineError, "conversion exploded"):
                producer.build_case_record(
                    manifest_case(), Path("office2pdf"), Path(tmp), Path(tmp)
                )


class CaseRecordTests(unittest.TestCase):
    def test_happy_path_record_shape(self) -> None:
        FakePipeline(self, gt_pages=2, out_pages=2)
        with tempfile.TemporaryDirectory() as tmp:
            record = producer.build_case_record(
                manifest_case(), Path("office2pdf"), Path(tmp), Path(tmp)
            )
        self.assertEqual(record["id"], "docx-invoice-en")
        self.assertEqual(record["format"], "docx")
        self.assertEqual(record["gt_pages"], 2)
        self.assertEqual(record["out_pages"], 2)
        self.assertTrue(record["page_parity"])
        self.assertEqual(len(record["pages"]), 2)
        for page_vector in record["pages"]:
            self.assertIn("lines", page_vector)
            self.assertIn("baseline", page_vector)
            self.assertIn("pitch", page_vector)
            self.assertIn("rects", page_vector)

    def test_page_count_mismatch_still_records_with_parity_false(self) -> None:
        FakePipeline(self, gt_pages=3, out_pages=2)
        with tempfile.TemporaryDirectory() as tmp:
            record = producer.build_case_record(
                manifest_case(), Path("office2pdf"), Path(tmp), Path(tmp)
            )
        self.assertFalse(record["page_parity"])
        self.assertEqual(len(record["pages"]), 2)


class DocumentTests(unittest.TestCase):
    def test_floats_round_recursively_for_stable_diffs(self) -> None:
        rounded = producer.round_floats(
            {"a": 1.23456, "b": [{"c": -0.005}, 2, "text"], "d": {"e": 0.1}}
        )
        self.assertEqual(rounded, {"a": 1.23, "b": [{"c": -0.01}, 2, "text"], "d": {"e": 0.1}})

    def test_document_metadata_and_summary(self) -> None:
        record = {
            "id": "docx-invoice-en",
            "format": "docx",
            "noise_floor_pt": 0.12,
            "gt_pages": 1,
            "out_pages": 1,
            "page_parity": True,
            "pages": [
                {
                    "lines": {"missing": 1, "extra": 0, "matched": 5, "deviant": 2},
                    "instances": {"large_shift_count": 1},
                    "visibility": {"mismatch_count": 2},
                    "visible_fills": {"mismatch_count": 3},
                    "rects": {"geometry_mismatch_count": 4},
                }
            ],
        }
        document = producer.build_baseline_document(
            [record], repository_commit="abc123", office2pdf_version="office2pdf 0.6.3"
        )
        self.assertEqual(document["schema_version"], 2)
        self.assertEqual(document["repository_commit"], "abc123")
        self.assertEqual(document["office2pdf_version"], "office2pdf 0.6.3")
        self.assertEqual(document["large_shift_pt"], 5.0)
        self.assertEqual(document["noise_floors_pt"]["xlsx"], 0.5)
        summary = document["summary"]
        self.assertEqual(summary["cases"], 1)
        self.assertEqual(summary["page_parity_cases"], 1)
        self.assertEqual(summary["pages_compared"], 1)
        self.assertEqual(summary["missing_lines"], 1)
        self.assertEqual(summary["large_shifts"], 1)
        self.assertEqual(summary["visibility_mismatches"], 2)
        self.assertEqual(summary["visible_fill_mismatches"], 3)
        self.assertEqual(summary["rect_geometry_mismatches"], 4)

    def test_serialisation_is_deterministic(self) -> None:
        record = {
            "id": "docx-invoice-en",
            "format": "docx",
            "noise_floor_pt": 0.12,
            "gt_pages": 1,
            "out_pages": 1,
            "page_parity": True,
            "pages": [],
        }
        document = producer.build_baseline_document(
            [record], repository_commit="abc123", office2pdf_version="office2pdf 0.6.3"
        )
        first = producer.serialize_baseline(document)
        second = producer.serialize_baseline(json.loads(first))
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()

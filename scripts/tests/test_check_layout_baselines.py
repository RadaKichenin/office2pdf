"""Unit tests for the layout-baseline consumer.

The consumer is a pure function of two baseline documents, so every test
builds small in-memory documents and asserts on the findings and exit
semantics — no external tools are involved.
"""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_layout_baselines as checker


def page_vector(**overrides: object) -> dict:
    vector = {
        "lines": {
            "gt": 10,
            "out": 10,
            "matched": 10,
            "missing": 0,
            "extra": 0,
            "deviant": 0,
            "missing_text": [],
            "extra_text": [],
        },
        "baseline": {
            "mean_abs_dy": 0.5,
            "worst_dy": 1.0,
            "worst_dy_signed": -1.0,
            "worst_line": "Line",
        },
        "dx0": {"mean_abs": 0.2, "worst": 0.4},
        "width": {"mean_abs_pct": 0.3, "worst_pct": 0.9},
        "instances": {
            "large_shift_threshold": 5.0,
            "large_shift_count": 0,
            "large_shifts": [],
        },
        "visibility": {"mismatch_count": 0, "mismatches": []},
        "visible_fills": {"mismatch_count": 0, "mismatches": []},
        "pitch": {"pairs": 9, "worst_delta": 0.3},
        "wraps": {"count": 0, "samples": []},
        "reflow": {"gt_lines": 0, "out_lines": 0, "samples": []},
        "rects": {
            "gt_count": 4,
            "out_count": 4,
            "matched": 4,
            "mean_center_delta": 0.5,
            "geometry_mismatch_count": 0,
        },
        "noise_floor": 0.12,
    }
    for dotted_key, value in overrides.items():
        section, _, key = dotted_key.partition("__")
        if key:
            vector[section][key] = value
        else:
            vector[section] = value
    return vector


def baseline_document(cases: list[dict] | None = None) -> dict:
    if cases is None:
        cases = [
            {
                "id": "docx-invoice-en",
                "format": "docx",
                "noise_floor_pt": 0.12,
                "gt_pages": 1,
                "out_pages": 1,
                "page_parity": True,
                "pages": [page_vector()],
            }
        ]
    return {
        "schema_version": 2,
        "repository_commit": "abc123",
        "office2pdf_version": "office2pdf 0.6.3",
        "large_shift_pt": 5.0,
        "noise_floors_pt": {"docx": 0.12, "pptx": 0.12, "xlsx": 0.5},
        "summary": {},
        "cases": cases,
    }


def xlsx_document(**page_overrides: object) -> dict:
    document = baseline_document()
    case = document["cases"][0]
    case["id"] = "xlsx-budget-ko"
    case["format"] = "xlsx"
    case["noise_floor_pt"] = 0.5
    case["pages"] = [page_vector(noise_floor=0.5, **page_overrides)]
    return document


class ToleranceTests(unittest.TestCase):
    def test_word_tolerance_matches_the_gt_quantisation_lattice(self) -> None:
        self.assertEqual(checker.PT_TOLERANCE["docx"], 0.24)
        self.assertEqual(checker.PT_TOLERANCE["pptx"], 0.24)

    def test_excel_tolerance_covers_whole_point_advance_rounding(self) -> None:
        self.assertEqual(checker.PT_TOLERANCE["xlsx"], 1.0)

    def test_pt_shift_within_tolerance_is_noise(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["baseline"]["mean_abs_dy"] = 0.7  # +0.2 <= 0.24
        self.assertEqual(checker.compare_baselines(stored, fresh), [])

    def test_pt_shift_beyond_tolerance_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["baseline"]["mean_abs_dy"] = 1.0  # +0.5 > 0.24
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "regression")
        self.assertEqual(findings[0]["metric"], "baseline.mean_abs_dy")

    def test_excel_gets_the_wider_tolerance(self) -> None:
        stored = xlsx_document()
        fresh = copy.deepcopy(stored)
        # +0.9pt would regress a Word case but sits inside Excel's ±1.0pt.
        fresh["cases"][0]["pages"][0]["baseline"]["mean_abs_dy"] = 1.4
        self.assertEqual(checker.compare_baselines(stored, fresh), [])

    def test_pt_improvement_beyond_tolerance_is_reported(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["baseline"]["mean_abs_dy"] = 0.1  # -0.4
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "improvement")

    def test_width_uses_the_percent_tolerance(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["width"]["worst_pct"] = 1.3  # +0.4 <= 0.5%
        self.assertEqual(checker.compare_baselines(stored, fresh), [])
        fresh["cases"][0]["pages"][0]["width"]["worst_pct"] = 2.0  # +1.1 > 0.5%
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(findings[0]["metric"], "width.worst_pct")


class CountTests(unittest.TestCase):
    def test_any_count_increase_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["lines"]["missing"] = 1
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "regression")
        self.assertEqual(findings[0]["metric"], "lines.missing")

    def test_any_count_decrease_is_an_improvement(self) -> None:
        stored = baseline_document()
        stored["cases"][0]["pages"][0]["instances"]["large_shift_count"] = 2
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["instances"]["large_shift_count"] = 0
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "improvement")

    def test_visibility_mismatch_increase_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["visibility"]["mismatch_count"] = 1
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "regression")
        self.assertEqual(findings[0]["metric"], "visibility.mismatch_count")

    def test_visible_fill_mismatch_increase_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["visible_fills"]["mismatch_count"] = 1
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "regression")
        self.assertEqual(findings[0]["metric"], "visible_fills.mismatch_count")

    def test_older_baseline_without_visible_fills_treats_new_finding_as_regression(
        self,
    ) -> None:
        stored = baseline_document()
        del stored["cases"][0]["pages"][0]["visible_fills"]
        fresh = baseline_document()
        fresh["cases"][0]["pages"][0]["visible_fills"]["mismatch_count"] = 1

        findings = checker.compare_baselines(stored, fresh)

        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["metric"], "visible_fills.mismatch_count")

    def test_rect_census_gap_widening_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["rects"]["out_count"] = 7  # gap 0 -> 3
        findings = checker.compare_baselines(stored, fresh)
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["metric"], "rects.census_gap")
        self.assertEqual(findings[0]["kind"], "regression")

    def test_rect_geometry_mismatch_increase_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["rects"]["geometry_mismatch_count"] = 1

        findings = checker.compare_baselines(stored, fresh)

        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["kind"], "regression")
        self.assertEqual(findings[0]["metric"], "rects.geometry_mismatch_count")

    def test_older_baseline_without_rect_geometry_treats_new_finding_as_regression(
        self,
    ) -> None:
        stored = baseline_document()
        del stored["cases"][0]["pages"][0]["rects"]["geometry_mismatch_count"]
        fresh = baseline_document()
        fresh["cases"][0]["pages"][0]["rects"]["geometry_mismatch_count"] = 1

        findings = checker.compare_baselines(stored, fresh)

        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0]["metric"], "rects.geometry_mismatch_count")


class PageParityTests(unittest.TestCase):
    def test_output_page_count_drifting_from_gt_is_a_regression(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["out_pages"] = 2
        fresh["cases"][0]["page_parity"] = False
        findings = checker.compare_baselines(stored, fresh)
        kinds = {finding["metric"]: finding["kind"] for finding in findings}
        self.assertEqual(kinds.get("page_parity_gap"), "regression")

    def test_gt_page_count_change_makes_documents_incomparable(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["gt_pages"] = 3
        with self.assertRaisesRegex(checker.BaselineComparisonError, "gt_pages"):
            checker.compare_baselines(stored, fresh)


class ComparabilityTests(unittest.TestCase):
    def test_case_set_mismatch_is_an_error(self) -> None:
        stored = baseline_document()
        fresh = baseline_document()
        fresh["cases"][0]["id"] = "docx-something-else"
        with self.assertRaisesRegex(checker.BaselineComparisonError, "case"):
            checker.compare_baselines(stored, fresh)

    def test_noise_floor_mismatch_is_an_error(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["noise_floor_pt"] = 0.5
        with self.assertRaisesRegex(checker.BaselineComparisonError, "noise floor"):
            checker.compare_baselines(stored, fresh)

    def test_schema_version_mismatch_is_an_error(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["schema_version"] = 1
        with self.assertRaisesRegex(checker.BaselineComparisonError, "schema"):
            checker.compare_baselines(stored, fresh)


class ReportTests(unittest.TestCase):
    def test_report_names_case_page_metric_and_delta(self) -> None:
        stored = baseline_document()
        fresh = copy.deepcopy(stored)
        fresh["cases"][0]["pages"][0]["baseline"]["mean_abs_dy"] = 1.0
        findings = checker.compare_baselines(stored, fresh)
        report = checker.render_report(findings)
        self.assertIn("docx-invoice-en", report)
        self.assertIn("page 1", report)
        self.assertIn("baseline.mean_abs_dy", report)
        self.assertIn("+0.50", report)

    def test_identical_documents_report_no_material_change(self) -> None:
        stored = baseline_document()
        report = checker.render_report(
            checker.compare_baselines(stored, copy.deepcopy(stored))
        )
        self.assertIn("no material change", report)

    def test_exit_code_is_nonzero_only_on_regressions(self) -> None:
        improvement = {"kind": "improvement"}
        regression = {"kind": "regression"}
        self.assertEqual(checker.exit_code([]), 0)
        self.assertEqual(checker.exit_code([improvement]), 0)
        self.assertEqual(checker.exit_code([improvement, regression]), 1)


if __name__ == "__main__":
    unittest.main()

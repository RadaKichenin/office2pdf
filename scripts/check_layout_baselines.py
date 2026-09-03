#!/usr/bin/env python3
"""Compare a fresh layout baseline against the stored one under tolerance.

The stored baseline (``tests/golden_mocks/business/baselines/layout.json``)
records per-page deviation vectors from ``compare_layout``. A fix PR
regenerates the file with ``generate_layout_baselines.py``; this tool diffs
the two documents and classifies every material change as a regression or an
improvement, so the numeric before/after and any collateral damage surface
without a full manual re-audit.

Tolerance policy — the vectors carry measurement noise that must not re-file
known quantisation artefacts as regressions (issues #630 and #649 both turned
on deltas smaller than the GT's own lattice):

- pt metrics move materially only past 0.24pt for Word/PowerPoint cases (the
  native export quantises coordinates to a 0.24pt grid) and 1.0pt for Excel
  cases (the Quartz export rounds every advance to a whole point);
- width percentages move materially only past 0.5%;
- counts (missing/extra lines, wraps, reflows, deviant lines, large shifts,
  painted-text visibility mismatches, visible-fill occlusions, rectangle
  geometry mismatches, rect census gap, page parity gap) compare exactly — any
  increase is a regression.

Safe split/join topology groups are informational and therefore are not a
count regression. Their recovered fragments still contribute to the position,
width, pitch, and visibility metrics above.

Exit status is nonzero only when a regression is found; improvements alone
exit zero so a fix PR can paste the report as its acceptance evidence.

Usage:
    check_layout_baselines.py --fresh PATH [--baseline PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE_PATH = (
    PROJECT_ROOT / "tests" / "golden_mocks" / "business" / "baselines" / "layout.json"
)

SUPPORTED_SCHEMA_VERSION = 2
PT_TOLERANCE = {"docx": 0.24, "pptx": 0.24, "xlsx": 1.0}
WIDTH_PCT_TOLERANCE = 0.5

# Lower is better for every metric below; count metrics compare exactly.
COUNT_METRICS = (
    ("lines", "missing"),
    ("lines", "extra"),
    ("lines", "deviant"),
    ("wraps", "count"),
    ("reflow", "gt_lines"),
    ("instances", "large_shift_count"),
    ("visibility", "mismatch_count"),
    ("visible_fills", "mismatch_count"),
    ("rects", "geometry_mismatch_count"),
)
PT_METRICS = (
    ("baseline", "mean_abs_dy"),
    ("baseline", "worst_dy"),
    ("dx0", "mean_abs"),
    ("pitch", "worst_delta"),
    ("rects", "mean_center_delta"),
)
PCT_METRICS = (
    ("width", "mean_abs_pct"),
    ("width", "worst_pct"),
)


class BaselineComparisonError(RuntimeError):
    """The two documents cannot be compared metric-by-metric."""


def finding(
    kind: str,
    case_id: str,
    page: int | None,
    metric: str,
    stored: float,
    fresh: float,
    tolerance: float | None,
) -> dict:
    return {
        "kind": kind,
        "case": case_id,
        "page": page,
        "metric": metric,
        "stored": stored,
        "fresh": fresh,
        "delta": fresh - stored,
        "tolerance": tolerance,
    }


def classify_delta(stored: float, fresh: float, tolerance: float) -> str | None:
    delta = fresh - stored
    if delta > tolerance:
        return "regression"
    if delta < -tolerance:
        return "improvement"
    return None


def compare_page(
    case_id: str,
    page_number: int,
    pt_tolerance: float,
    stored_vector: dict,
    fresh_vector: dict,
) -> list[dict]:
    findings: list[dict] = []
    for section, key in COUNT_METRICS:
        # Material checks are added compatibly within schema v2. An older
        # baseline without one of these counts means zero recorded findings;
        # a fresh finding therefore surfaces as a regression.
        stored_count = int(stored_vector.get(section, {}).get(key, 0))
        fresh_count = int(fresh_vector.get(section, {}).get(key, 0))
        if fresh_count != stored_count:
            kind = "regression" if fresh_count > stored_count else "improvement"
            findings.append(
                finding(
                    kind, case_id, page_number, f"{section}.{key}",
                    stored_count, fresh_count, None,
                )
            )
    for metrics, tolerance in ((PT_METRICS, pt_tolerance), (PCT_METRICS, WIDTH_PCT_TOLERANCE)):
        for section, key in metrics:
            stored_value = abs(float(stored_vector[section][key]))
            fresh_value = abs(float(fresh_vector[section][key]))
            kind = classify_delta(stored_value, fresh_value, tolerance)
            if kind is not None:
                findings.append(
                    finding(
                        kind, case_id, page_number, f"{section}.{key}",
                        stored_value, fresh_value, tolerance,
                    )
                )
    stored_gap = abs(
        int(stored_vector["rects"]["gt_count"]) - int(stored_vector["rects"]["out_count"])
    )
    fresh_gap = abs(
        int(fresh_vector["rects"]["gt_count"]) - int(fresh_vector["rects"]["out_count"])
    )
    if fresh_gap != stored_gap:
        kind = "regression" if fresh_gap > stored_gap else "improvement"
        findings.append(
            finding(kind, case_id, page_number, "rects.census_gap", stored_gap, fresh_gap, None)
        )
    return findings


def compare_case(stored_case: dict, fresh_case: dict) -> list[dict]:
    case_id = str(stored_case["id"])
    if stored_case["noise_floor_pt"] != fresh_case["noise_floor_pt"]:
        raise BaselineComparisonError(
            f"{case_id}: noise floor differs between documents "
            f"({stored_case['noise_floor_pt']} vs {fresh_case['noise_floor_pt']}) — "
            "the vectors were measured under different parameters"
        )
    if stored_case["gt_pages"] != fresh_case["gt_pages"]:
        raise BaselineComparisonError(
            f"{case_id}: gt_pages changed ({stored_case['gt_pages']} vs "
            f"{fresh_case['gt_pages']}) — the ground truth itself differs, "
            "so regenerate the stored baseline from the current corpus first"
        )

    findings: list[dict] = []
    stored_gap = abs(int(stored_case["gt_pages"]) - int(stored_case["out_pages"]))
    fresh_gap = abs(int(fresh_case["gt_pages"]) - int(fresh_case["out_pages"]))
    if fresh_gap != stored_gap:
        kind = "regression" if fresh_gap > stored_gap else "improvement"
        findings.append(finding(kind, case_id, None, "page_parity_gap", stored_gap, fresh_gap, None))

    pt_tolerance = PT_TOLERANCE[str(stored_case["format"])]
    stored_pages = stored_case["pages"]
    fresh_pages = fresh_case["pages"]
    for index in range(min(len(stored_pages), len(fresh_pages))):
        findings.extend(
            compare_page(case_id, index + 1, pt_tolerance, stored_pages[index], fresh_pages[index])
        )
    return findings


def compare_baselines(stored: dict, fresh: dict) -> list[dict]:
    for label, document in (("stored", stored), ("fresh", fresh)):
        if document.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
            raise BaselineComparisonError(
                f"{label} document has schema_version {document.get('schema_version')}; "
                f"this tool compares schema_version {SUPPORTED_SCHEMA_VERSION}"
            )
    stored_cases = {str(case["id"]): case for case in stored["cases"]}
    fresh_cases = {str(case["id"]): case for case in fresh["cases"]}
    if set(stored_cases) != set(fresh_cases):
        missing = sorted(set(stored_cases) - set(fresh_cases))
        added = sorted(set(fresh_cases) - set(stored_cases))
        raise BaselineComparisonError(
            f"case sets differ (missing from fresh: {missing}, new in fresh: {added}) — "
            "regenerate both documents from the same manifest"
        )

    findings: list[dict] = []
    for case_id in sorted(stored_cases):
        findings.extend(compare_case(stored_cases[case_id], fresh_cases[case_id]))
    findings.sort(key=lambda item: (item["kind"] != "regression", item["case"], item["page"] or 0))
    return findings


def render_finding(item: dict) -> str:
    location = f"page {item['page']}" if item["page"] is not None else "case"
    tolerance = (
        f" (tolerance ±{item['tolerance']})" if item["tolerance"] is not None else ""
    )
    return (
        f"{item['kind'].upper()}  {item['case']} {location} {item['metric']}: "
        f"{item['stored']:g} -> {item['fresh']:g} ({item['delta']:+.2f}){tolerance}"
    )


def render_report(findings: list[dict]) -> str:
    if not findings:
        return "no material change against the stored baseline"
    lines = [render_finding(item) for item in findings]
    regressions = sum(1 for item in findings if item["kind"] == "regression")
    improvements = len(findings) - regressions
    lines.append("")
    lines.append(f"{regressions} regression(s), {improvements} improvement(s)")
    return "\n".join(lines)


def exit_code(findings: list[dict]) -> int:
    return 1 if any(item["kind"] == "regression" for item in findings) else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--baseline",
        type=Path,
        default=DEFAULT_BASELINE_PATH,
        help="stored baseline (default: baselines/layout.json)",
    )
    parser.add_argument(
        "--fresh",
        type=Path,
        required=True,
        help="freshly generated baseline to compare against the stored one",
    )
    args = parser.parse_args()

    stored = json.loads(args.baseline.read_text(encoding="utf-8"))
    fresh = json.loads(args.fresh.read_text(encoding="utf-8"))
    try:
        findings = compare_baselines(stored, fresh)
    except BaselineComparisonError as error:
        print(f"baseline comparison failed: {error}", file=sys.stderr)
        return 1
    print(render_report(findings))
    return exit_code(findings)


if __name__ == "__main__":
    sys.exit(main())

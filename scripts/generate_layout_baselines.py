#!/usr/bin/env python3
"""Produce the per-page layout deviation baseline for the business golden mocks.

For every case in ``tests/golden_mocks/business/manifest.json`` this pipeline:

1. runs the GT integrity gate (``check_gt_integrity``) and refuses to record
   anything from an invalid export — a corrupt GT baked into a baseline is how
   issue #616 propagated;
2. converts the source with a locally built ``office2pdf`` binary;
3. traces both PDFs with ``mutool draw -F trace`` and stores the
   ``compare_layout`` deviation vector for every page, using the per-format
   noise floor (0.12pt for Word/PowerPoint GT, whose exports quantise to a
   0.24pt grid; 0.5pt for Excel GT, whose Quartz export rounds every advance
   to a whole point), including informational safe split/join topology groups,
   painted-text visibility mismatches caused by z-order or flat-background
   contrast, plus visible-fill occlusions where a later opaque, differently
   coloured rectangle cuts into a thin rule, and matched axis-aligned
   rectangle/line geometry deviations after same-paint operation splits merge.

A trace that parses to zero pages is a hard failure, never an empty vector: on
mutool 1.23.x that state used to look exactly like a perfect comparison
(issue #808), which would have recorded a baseline that passes everything
silently, forever.

The output is deterministic for a given tree (floats rounded, keys ordered),
so regenerating it in a fix PR makes ``git diff`` on the baseline file the
quantitative before/after. ``scripts/check_layout_baselines.py`` compares two
generated documents under the tolerance policy.

Usage:
    generate_layout_baselines.py --office2pdf PATH [--output PATH]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_gt_integrity
import compare_layout

PROJECT_ROOT = Path(__file__).resolve().parents[1]
CORPUS_ROOT = PROJECT_ROOT / "tests" / "golden_mocks" / "business"
MANIFEST_PATH = CORPUS_ROOT / "manifest.json"
DEFAULT_BASELINE_PATH = CORPUS_ROOT / "baselines" / "layout.json"

SCHEMA_VERSION = 2
# Word and PowerPoint native exports quantise coordinates to a 0.24pt grid;
# Excel's Quartz export rounds every advance to a whole point.
NOISE_FLOOR_PT = {"docx": 0.12, "pptx": 0.12, "xlsx": 0.5}
LARGE_SHIFT_PT = 5.0
FLOAT_PRECISION = 2


class BaselineError(RuntimeError):
    """A condition under which recording a baseline would be misleading."""


def round_floats(value: object) -> object:
    """Round every float to a stable precision so regenerated files diff cleanly."""
    if isinstance(value, float):
        return round(value, FLOAT_PRECISION)
    if isinstance(value, dict):
        return {key: round_floats(item) for key, item in value.items()}
    if isinstance(value, list):
        return [round_floats(item) for item in value]
    return value


def run_ground_truth_gate(gt_pdf: Path, source: Path) -> dict:
    return check_gt_integrity.check(gt_pdf, source)


def convert_source(office2pdf_binary: Path, source: Path, output_pdf: Path) -> None:
    result = subprocess.run(
        [str(office2pdf_binary), str(source), "-o", str(output_pdf)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not output_pdf.is_file():
        raise BaselineError(
            f"conversion failed for {source.name}: {result.stderr.strip() or 'no output'}"
        )


def trace_pages(pdf: Path) -> list[compare_layout.PageLayout]:
    return compare_layout.parse_trace(compare_layout.run_mutool(pdf))


def build_case_record(
    case: dict,
    office2pdf_binary: Path,
    corpus_root: Path,
    work_directory: Path,
) -> dict:
    case_id = str(case["id"])
    source_format = str(case["format"])
    source = corpus_root / str(case["source"])
    gt_pdf = corpus_root / str(case["expected_pdf"])

    gate = run_ground_truth_gate(gt_pdf, source)
    if gate["invalid"]:
        raise BaselineError(
            f"{case_id}: GT failed the integrity gate "
            f"(periodicity={gate['periodicity']}) — re-export before recording a baseline"
        )

    converted_pdf = work_directory / f"{case_id}.pdf"
    try:
        convert_source(office2pdf_binary, source, converted_pdf)
    except BaselineError as error:
        raise BaselineError(f"{case_id}: {error}") from error

    gt_pages = trace_pages(gt_pdf)
    out_pages = trace_pages(converted_pdf)
    for label, pdf, pages in (("GT", gt_pdf, gt_pages), ("output", converted_pdf, out_pages)):
        if not pages:
            raise BaselineError(
                f"{case_id}: no pages parsed from the {label} trace ({pdf}) — "
                "an empty trace is indistinguishable from a perfect comparison "
                "and must never be recorded"
            )

    noise_floor = NOISE_FLOOR_PT[source_format]
    vectors = [
        compare_layout.diff_page(
            gt_pages[index],
            out_pages[index],
            noise_floor=noise_floor,
            large_shift=LARGE_SHIFT_PT,
        )
        for index in range(min(len(gt_pages), len(out_pages)))
    ]
    return {
        "id": case_id,
        "format": source_format,
        "noise_floor_pt": noise_floor,
        "gt_pages": len(gt_pages),
        "out_pages": len(out_pages),
        "page_parity": len(gt_pages) == len(out_pages),
        "pages": vectors,
    }


def build_baseline_document(
    case_records: list[dict], repository_commit: str, office2pdf_version: str
) -> dict:
    pages = [vector for record in case_records for vector in record["pages"]]
    summary = {
        "cases": len(case_records),
        "page_parity_cases": sum(1 for record in case_records if record["page_parity"]),
        "pages_compared": len(pages),
        "missing_lines": sum(vector["lines"]["missing"] for vector in pages),
        "extra_lines": sum(vector["lines"]["extra"] for vector in pages),
        "deviant_lines": sum(vector["lines"]["deviant"] for vector in pages),
        "large_shifts": sum(vector["instances"]["large_shift_count"] for vector in pages),
        "visibility_mismatches": sum(
            vector.get("visibility", {"mismatch_count": 0})["mismatch_count"]
            for vector in pages
        ),
        "visible_fill_mismatches": sum(
            vector.get("visible_fills", {"mismatch_count": 0})["mismatch_count"]
            for vector in pages
        ),
        "rect_geometry_mismatches": sum(
            vector.get("rects", {}).get("geometry_mismatch_count", 0)
            for vector in pages
        ),
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "generator": "scripts/generate_layout_baselines.py",
        "repository_commit": repository_commit,
        "office2pdf_version": office2pdf_version,
        "noise_floors_pt": dict(NOISE_FLOOR_PT),
        "large_shift_pt": LARGE_SHIFT_PT,
        "summary": summary,
        "cases": case_records,
    }


def serialize_baseline(document: dict) -> str:
    return json.dumps(round_floats(document), indent=1, sort_keys=True) + "\n"


def describe_repository_commit() -> str:
    commit = subprocess.run(
        ["git", "-C", str(PROJECT_ROOT), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    status = subprocess.run(
        ["git", "-C", str(PROJECT_ROOT), "status", "--porcelain"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    return f"{commit}-dirty" if status else commit


def describe_office2pdf_version(office2pdf_binary: Path) -> str:
    return subprocess.run(
        [str(office2pdf_binary), "--version"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--office2pdf",
        type=Path,
        required=True,
        help="path to a locally built office2pdf binary",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=MANIFEST_PATH,
        help="corpus manifest (default: the business golden mock manifest)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_BASELINE_PATH,
        help="baseline file to write (default: baselines/layout.json)",
    )
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    corpus_root = args.manifest.resolve().parent
    case_records: list[dict] = []
    try:
        with tempfile.TemporaryDirectory() as scratch:
            for case in manifest["cases"]:
                case_records.append(
                    build_case_record(case, args.office2pdf, corpus_root, Path(scratch))
                )
                record = case_records[-1]
                visibility_mismatches = sum(
                    vector.get("visibility", {"mismatch_count": 0})["mismatch_count"]
                    for vector in record["pages"]
                )
                visible_fill_mismatches = sum(
                    vector.get("visible_fills", {"mismatch_count": 0})[
                        "mismatch_count"
                    ]
                    for vector in record["pages"]
                )
                rect_geometry_mismatches = sum(
                    vector.get("rects", {}).get("geometry_mismatch_count", 0)
                    for vector in record["pages"]
                )
                print(
                    f"{record['id']}: {record['out_pages']}/{record['gt_pages']} pages, "
                    f"{sum(v['lines']['missing'] for v in record['pages'])} missing, "
                    f"{sum(v['instances']['large_shift_count'] for v in record['pages'])} "
                    "large shifts, "
                    f"{visibility_mismatches} text visibility mismatches, "
                    f"{visible_fill_mismatches} visible fill mismatches, "
                    f"{rect_geometry_mismatches} rectangle geometry mismatches"
                )
    except BaselineError as error:
        print(f"baseline generation failed: {error}", file=sys.stderr)
        return 1

    document = build_baseline_document(
        case_records,
        repository_commit=describe_repository_commit(),
        office2pdf_version=describe_office2pdf_version(args.office2pdf),
    )
    args.output.write_text(serialize_baseline(document), encoding="utf-8")
    summary = document["summary"]
    print(
        f"wrote {args.output}: {summary['cases']} cases, "
        f"{summary['pages_compared']} pages, "
        f"{summary['page_parity_cases']} with page parity"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

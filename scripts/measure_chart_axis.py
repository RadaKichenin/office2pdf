#!/usr/bin/env python3
"""Measure a renderer's automatic value-axis scale, one data maximum per file.

`nice_axis` in the chart backend has to reproduce what Excel picks for a chart
that declares no `c:max`, `c:min` or `c:majorUnit`. Excel documents only that
the axis clears the data by a twentieth of its range; the major unit it then
chooses is undocumented, and no fixture in this repository carries a chart with
a committed native ground truth, so the rule cannot be read off the corpus.

This harness produces the sample instead. `tests/fixtures/xlsx/poi/WithChart.xlsx`
has two value series and a fully auto-scaled value axis, so multiplying every
plotted point by one factor moves the data maximum without changing anything
else about the chart. Rendering each rescaled workbook and reading the tick
labels back gives one (data maximum, axis maximum, major unit) triple per file.

The renderer is LibreOffice, which is not Excel. It agrees with Excel on the two
maxima this repository has native exports for — 17 (issue #634) and 23,334
(issue #553) — and running it here is what makes the table in
`chart_value_label_tests::MEASURED_AUTO_SCALE` reproducible rather than asserted.
Where an Excel export is available, prefer it and record which entries came from
where.

Usage:
    measure_chart_axis.py 17 23334 [...]        # measure the listed maxima
    measure_chart_axis.py --rust 0.44 1.9       # emit Rust table rows

Requires ``libreoffice`` and ``pdftotext`` (poppler-utils).
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FIXTURE = REPO_ROOT / "tests/fixtures/xlsx/poi/WithChart.xlsx"

# The fixture's two series, and the maximum across both. Rescaling multiplies
# every one of these by `target / FIXTURE_MAX`, which keeps the ratio of the
# smallest point to the largest — and so the axis minimum's own auto rule —
# unchanged while the maximum sweeps.
SERIES_A: tuple[float, ...] = (1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
SERIES_B: tuple[float, ...] = (10.0, 12.0, 14.0, 9.0, 15.0, 17.0)
FIXTURE_MAX: float = 17.0

NUMERIC_WORD = re.compile(r"^-?[\d,]+(?:\.\d+)?$")
BBOX_WORD = re.compile(
    r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">([^<]*)</word>'
)


def format_number(value: float) -> str:
    """Render a value the way the fixture's XML spells it, so replacement matches."""
    if abs(value - round(value)) < 1e-9:
        return str(int(round(value)))
    return repr(round(value, 10))


def rust_float(value: float) -> str:
    """Spell a value as an `f64` literal, which always needs a decimal point."""
    text: str = repr(round(value, 10))
    return text if "." in text or "e" in text else f"{text}.0"


def rescale_workbook(source: Path, target_max: float, destination: Path) -> None:
    """Write `source` to `destination` with every plotted point scaled so the
    data maximum becomes `target_max`.

    Both the sheet cells and the chart's `numCache` carry the values; Excel and
    LibreOffice read the cells, but a stale cache would leave the file
    self-contradictory, so both are rewritten.
    """
    factor: float = target_max / FIXTURE_MAX
    scaled_a: list[float] = [value * factor for value in SERIES_A]
    scaled_b: list[float] = [value * factor for value in SERIES_B]

    with zipfile.ZipFile(source) as archive_in:
        with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive_out:
            for item in archive_in.infolist():
                payload: bytes = archive_in.read(item.filename)
                if item.filename == "xl/worksheets/sheet1.xml":
                    payload = _rescale_sheet(payload, scaled_a, scaled_b)
                elif item.filename == "xl/charts/chart1.xml":
                    payload = _rescale_cache(payload, scaled_a, scaled_b)
                archive_out.writestr(item, payload)


def _rescale_sheet(payload: bytes, scaled_a: list[float], scaled_b: list[float]) -> bytes:
    text: str = payload.decode("utf-8")
    for row in range(6):
        for column, original, scaled in (
            ("A", SERIES_A, scaled_a),
            ("B", SERIES_B, scaled_b),
        ):
            old: str = f'<c r="{column}{row + 1}"><v>{format_number(original[row])}</v></c>'
            new: str = f'<c r="{column}{row + 1}"><v>{format_number(scaled[row])}</v></c>'
            if old not in text:
                raise SystemExit(f"fixture cell {column}{row + 1} did not match {old}")
            text = text.replace(old, new)
    return text.encode("utf-8")


def _rescale_cache(payload: bytes, scaled_a: list[float], scaled_b: list[float]) -> bytes:
    """Replace the cached point values in document order, leaving series names alone."""
    text: str = payload.decode("utf-8")
    replacements: list[str] = [format_number(v) for v in scaled_a + scaled_b]
    pieces: list[str] = []
    index: int = 0
    cursor: int = 0
    for match in re.finditer(r"<c:v>([^<]*)</c:v>", text):
        if not NUMERIC_WORD.match(match.group(1).strip()):
            continue  # a c:tx series name, not a plotted point
        pieces.append(text[cursor : match.start(1)])
        pieces.append(replacements[index])
        cursor = match.end(1)
        index += 1
    if index != len(replacements):
        raise SystemExit(f"rewrote {index} cached points, expected {len(replacements)}")
    pieces.append(text[cursor:])
    return "".join(pieces).encode("utf-8")


def render_pdf(workbook: Path, out_dir: Path) -> Path:
    """Convert with LibreOffice, giving it a private HOME so concurrent runs of
    this script do not fight over one user profile."""
    subprocess.run(
        ["libreoffice", "--headless", "--convert-to", "pdf", "--outdir", str(out_dir), str(workbook)],
        check=True,
        capture_output=True,
        timeout=180,
        env={**os.environ, "HOME": str(out_dir)},
    )
    rendered: Path = out_dir / f"{workbook.stem}.pdf"
    if not rendered.is_file():
        raise SystemExit(f"LibreOffice produced no PDF for {workbook}")
    return rendered


def value_axis_labels(pdf: Path) -> list[float]:
    """Read the value axis's tick labels out of the rendered page.

    The value-axis labels are right-aligned against the axis, so they share a
    right edge that no other text on the page does; taking the x-cluster with the
    most distinct baselines picks them out without needing to know the layout.
    The category labels sit on one baseline and so never win.
    """
    trace: str = subprocess.run(
        ["pdftotext", "-bbox", str(pdf), "-"],
        check=True,
        capture_output=True,
        timeout=60,
    ).stdout.decode("utf-8")

    words: list[tuple[float, float, float]] = []
    for match in BBOX_WORD.finditer(trace):
        text: str = match.group(5).strip()
        if NUMERIC_WORD.match(text):
            words.append((float(match.group(3)), float(match.group(2)), float(text.replace(",", ""))))
    if not words:
        return []

    best: list[tuple[float, float, float]] = []
    for anchor in sorted({round(right, 1) for right, _, _ in words}):
        column = [word for word in words if abs(word[0] - anchor) < 3.0]
        if len({round(top, 1) for _, top, _ in column}) > len(
            {round(top, 1) for _, top, _ in best}
        ):
            best = column

    labels: list[float] = []
    for _, _, value in sorted(best, key=lambda word: word[1]):
        if not labels or abs(value - labels[-1]) > 1e-9:
            labels.append(value)
    return labels


def measure(fixture: Path, target_max: float, work_root: Path) -> tuple[float, float] | None:
    """Return the (axis maximum, major unit) a render chooses for `target_max`."""
    case_dir: Path = work_root / f"max-{format_number(target_max)}"
    shutil.rmtree(case_dir, ignore_errors=True)
    case_dir.mkdir(parents=True)
    workbook: Path = case_dir / "chart.xlsx"
    rescale_workbook(fixture, target_max, workbook)

    labels: list[float] = value_axis_labels(render_pdf(workbook, case_dir))
    if len(labels) < 2:
        return None
    ordered: list[float] = sorted(labels)
    steps: set[float] = {
        round(ordered[i + 1] - ordered[i], 10) for i in range(len(ordered) - 1)
    }
    if len(steps) != 1:
        print(f"  irregular tick spacing {sorted(steps)} for {target_max}", file=sys.stderr)
        return None
    return ordered[-1], steps.pop()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("maxima", type=float, nargs="+", help="data maxima to measure")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--rust", action="store_true", help="emit Rust table rows")
    parser.add_argument("--keep", type=Path, default=None, help="keep artefacts here")
    args = parser.parse_args()

    if not args.fixture.is_file():
        raise SystemExit(f"fixture not found: {args.fixture}")

    temporary: tempfile.TemporaryDirectory | None = None
    if args.keep:
        args.keep.mkdir(parents=True, exist_ok=True)
        work_root: Path = args.keep
    else:
        temporary = tempfile.TemporaryDirectory(prefix="chart-axis-")
        work_root = Path(temporary.name)

    try:
        if not args.rust:
            print(f"{'data max':>12} {'axis max':>12} {'major unit':>12}")
        for target_max in args.maxima:
            measured: tuple[float, float] | None = measure(args.fixture, target_max, work_root)
            if measured is None:
                print(f"{format_number(target_max):>12}   no axis found", file=sys.stderr)
                continue
            axis_max, step = measured
            if args.rust:
                row: str = ", ".join(rust_float(v) for v in (target_max, axis_max, step))
                print(f"        ({row}),")
            else:
                print(f"{format_number(target_max):>12} {format_number(axis_max):>12} {format_number(step):>12}")
    finally:
        if temporary is not None:
            temporary.cleanup()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

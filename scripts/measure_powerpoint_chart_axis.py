#!/usr/bin/env python3
"""Measure PowerPoint's automatic horizontal value-axis scale.

The chart renderer must reproduce what PowerPoint chooses when a bar chart
declares no ``c:max``, ``c:min``, or ``c:majorUnit``. Those choices differ from
Excel's and are undocumented, so this harness rewrites the committed PPTX
fixture, exports each probe with desktop PowerPoint, and reads the resulting
tick labels back from the PDF.

Usage:
    measure_powerpoint_chart_axis.py --rust 0.44 1.9 8.2 23334
    measure_powerpoint_chart_axis.py --mode text-size 10 12 18 24 36 44
    measure_powerpoint_chart_axis.py --mode frame-width 200 240 400 480 560

The default ``maximum`` mode rescales both the chart cache and its embedded
workbook. The other modes keep the fixture's 8.2 maximum and rewrite only the
chart text size or graphic-frame width. Frame-width output also reports the
plot width recovered from the longest bar, which is the width the scaling rule
actually consumes.

Every probe package and every exported PDF is staged inside PowerPoint's own
sandbox container and the stage is removed when the run ends, so an unattended
run never stops on a "Grant Access" dialog (#1082). ``--keep`` copies the
packages and PDFs out to a directory of your choice before that cleanup.

Requires macOS, Microsoft PowerPoint, ``pdftotext`` (Poppler), and, for
``frame-width``, ``mutool`` (MuPDF).
"""

from __future__ import annotations

import argparse
import io
import re
import shutil
import subprocess
import zipfile
from pathlib import Path

import probe_harness

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FIXTURE = REPO_ROOT / "tests/fixtures/pptx/bar-chart.pptx"
POWERPOINT_EXPORTER = REPO_ROOT / "scripts/macos/export_powerpoint_pdfs.applescript"
STAGE_NAME = "powerpoint-chart-axis"

SERIES_VALUES: tuple[float, ...] = (8.2, 3.2, 1.4, 1.2)
FIXTURE_MAX = max(SERIES_VALUES)
EMU_PER_POINT = 12_700.0

NUMERIC_WORD = re.compile(r"^-?[\d,]+(?:\.\d+)?$")
BBOX_WORD = re.compile(
    r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">([^<]*)</word>'
)


def format_number(value: float) -> str:
    if abs(value - round(value)) < 1e-9:
        return str(int(round(value)))
    return f"{value:.12g}"


def rust_float(value: float) -> str:
    text = repr(round(value, 10))
    return text if "." in text or "e" in text else f"{text}.0"


def slug(value: float) -> str:
    return format_number(value).replace("-", "neg-").replace(".", "p")


def _rewrite_embedded_workbook(payload: bytes, values: tuple[float, ...]) -> bytes:
    source = io.BytesIO(payload)
    output = io.BytesIO()
    with zipfile.ZipFile(source) as archive_in, zipfile.ZipFile(
        output, "w", zipfile.ZIP_DEFLATED
    ) as archive_out:
        for item in archive_in.infolist():
            content = archive_in.read(item.filename)
            if item.filename == "xl/worksheets/sheet1.xml":
                text = content.decode("utf-8")
                for row, value in enumerate(values, start=2):
                    pattern = rf'(<c r="B{row}"[^>]*>\s*<v>)[^<]*(</v>)'
                    text, count = re.subn(
                        pattern, rf"\g<1>{format_number(value)}\g<2>", text
                    )
                    if count != 1:
                        raise SystemExit(f"expected one embedded B{row}, got {count}")
                content = text.encode("utf-8")
            archive_out.writestr(item, content)
    return output.getvalue()


def _rewrite_chart_values(payload: bytes, values: tuple[float, ...]) -> bytes:
    text = payload.decode("utf-8")
    match = re.search(r"<c:numCache>.*?</c:numCache>", text, flags=re.DOTALL)
    if match is None:
        raise SystemExit("fixture chart has no c:numCache")
    cache = match.group(0)
    for index, value in enumerate(values):
        pattern = rf'(<c:pt idx="{index}">\s*<c:v>)[^<]*(</c:v>\s*</c:pt>)'
        cache, count = re.subn(
            pattern, rf"\g<1>{format_number(value)}\g<2>", cache
        )
        if count != 1:
            raise SystemExit(f"expected one chart cache point {index}, got {count}")
    return (text[: match.start()] + cache + text[match.end() :]).encode("utf-8")


def _rewrite_chart_text_size(payload: bytes, points: float) -> bytes:
    text = payload.decode("utf-8")
    text, count = re.subn(
        r'<a:defRPr sz="\d+"/>', f'<a:defRPr sz="{round(points * 100)}"/>', text
    )
    if count != 1:
        raise SystemExit(f"expected one chart default text size, got {count}")
    return text.encode("utf-8")


def _rewrite_frame_width(payload: bytes, points: float) -> bytes:
    text = payload.decode("utf-8")
    pattern = r"(<p:graphicFrame>.*?<p:xfrm>.*?<a:ext cx=\")\d+(\" cy=\"\d+\")"
    text, count = re.subn(
        pattern,
        rf"\g<1>{round(points * EMU_PER_POINT)}\g<2>",
        text,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        raise SystemExit(f"expected one chart graphic-frame width, got {count}")
    return text.encode("utf-8")


def build_probe(source: Path, destination: Path, mode: str, value: float) -> None:
    scaled_values = tuple(
        source_value * value / FIXTURE_MAX for source_value in SERIES_VALUES
    )
    with zipfile.ZipFile(source) as archive_in, zipfile.ZipFile(
        destination, "w", zipfile.ZIP_DEFLATED
    ) as archive_out:
        for item in archive_in.infolist():
            payload = archive_in.read(item.filename)
            if mode == "maximum":
                if item.filename == "ppt/charts/chart1.xml":
                    payload = _rewrite_chart_values(payload, scaled_values)
                elif item.filename == "ppt/embeddings/Microsoft_Excel_Worksheet1.xlsx":
                    payload = _rewrite_embedded_workbook(payload, scaled_values)
            elif mode == "text-size" and item.filename == "ppt/charts/chart1.xml":
                payload = _rewrite_chart_text_size(payload, value)
            elif mode == "frame-width" and item.filename == "ppt/slides/slide1.xml":
                payload = _rewrite_frame_width(payload, value)
            archive_out.writestr(item, payload)


def container_stage() -> Path:
    """Directory PowerPoint can open and save in without a consent dialog.

    The app is sandboxed: a package it opens from, or a PDF it saves to,
    anywhere outside its own container costs a per-file "Grant Access" dialog,
    and an unattended run stalls on the first one (#1051, #1082). The probe
    harness already maps a format to its app's container, so reuse that rather
    than spelling the path out a second time. Its own subdirectory keeps a
    concurrent ``probe_harness.py`` run from clearing these packages.
    """
    return probe_harness.default_stage_root("office", ".pptx") / STAGE_NAME


def mirror_stage(stage: Path, destination: Path) -> None:
    """Copy the staged probe packages and their PDFs out of the container."""
    for name in ("pptx", "pdf"):
        source = stage / name
        if not source.is_dir():
            continue
        target = destination / name
        target.mkdir(parents=True, exist_ok=True)
        for item in sorted(source.iterdir()):
            if item.is_file():
                shutil.copyfile(item, target / item.name)


def export_probes(probes: list[tuple[str, Path]], out_dir: Path) -> None:
    command = ["osascript", str(POWERPOINT_EXPORTER), str(out_dir)]
    for identifier, probe in probes:
        command.extend((identifier, str(probe.resolve())))
    subprocess.run(command, check=True, timeout=max(180, 130 * len(probes)))


def value_axis_labels(pdf: Path) -> list[float]:
    trace = subprocess.run(
        ["pdftotext", "-bbox", str(pdf), "-"],
        check=True,
        capture_output=True,
        timeout=60,
    ).stdout.decode("utf-8")
    words: list[tuple[float, float, float, float]] = []
    for match in BBOX_WORD.finditer(trace):
        text = match.group(5).strip()
        if NUMERIC_WORD.fullmatch(text):
            words.append(
                (
                    float(match.group(1)),
                    float(match.group(2)),
                    float(match.group(3)),
                    float(text.replace(",", "")),
                )
            )

    best: list[tuple[float, float, float, float]] = []
    for top in sorted({round(word[1], 1) for word in words}):
        row = [word for word in words if abs(word[1] - top) < 2.0]
        if len({word[3] for word in row}) > len({word[3] for word in best}):
            best = row
    return sorted({word[3] for word in best})


def plot_width(pdf: Path, axis_max: float) -> float:
    trace = subprocess.run(
        ["mutool", "draw", "-F", "trace", "-o", "-", str(pdf)],
        check=True,
        capture_output=True,
        timeout=60,
    ).stdout.decode("utf-8")
    paths = re.findall(
        r'<fill_path[^>]*color="\.3098039 \.5058824 \.7411765".*?</fill_path>',
        trace,
        flags=re.DOTALL,
    )
    widths: list[float] = []
    for path in paths:
        xs = [float(value) for value in re.findall(r'<(?:move|line)to x="([\d.]+)"', path)]
        if xs:
            widths.append(max(xs) - min(xs))
    if not widths:
        raise SystemExit(f"could not find the fixture's blue bars in {pdf}")
    return max(widths) * axis_max / FIXTURE_MAX


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("values", type=float, nargs="+")
    parser.add_argument(
        "--mode",
        choices=("maximum", "text-size", "frame-width"),
        default="maximum",
    )
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--rust", action="store_true", help="emit Rust rows")
    parser.add_argument(
        "--keep", type=Path, help="copy the generated PPTX/PDF probes out to here"
    )
    args = parser.parse_args(argv)

    for required in (args.fixture, POWERPOINT_EXPORTER):
        if not required.is_file():
            raise SystemExit(f"required file not found: {required}")

    # Both the packages PowerPoint opens and the PDFs it writes stay in its own
    # sandbox container for the whole run; a leftover stage from an aborted run
    # would otherwise feed stale PDFs to the measurement below.
    work_root = container_stage()
    shutil.rmtree(work_root, ignore_errors=True)

    try:
        probe_dir = work_root / "pptx"
        pdf_dir = work_root / "pdf"
        probe_dir.mkdir(parents=True)
        pdf_dir.mkdir(parents=True)
        probes: list[tuple[str, Path]] = []
        for value in args.values:
            identifier = f"{args.mode}-{slug(value)}"
            probe = probe_dir / f"{identifier}.pptx"
            build_probe(args.fixture, probe, args.mode, value)
            probes.append((identifier, probe))
        export_probes(probes, pdf_dir)
        # Copy out before measuring, so an analysis failure still leaves the
        # artifacts behind for inspection.
        if args.keep is not None:
            mirror_stage(work_root, args.keep)

        if not args.rust:
            suffix = "     plot pt" if args.mode == "frame-width" else ""
            print(f"{'probe':>12} {'axis max':>12} {'major unit':>12}{suffix}")
        for value, (identifier, _) in zip(args.values, probes, strict=True):
            pdf = pdf_dir / f"{identifier}.pdf"
            labels = value_axis_labels(pdf)
            if len(labels) < 2:
                raise SystemExit(f"fewer than two axis labels found in {pdf}: {labels}")
            axis_max = labels[-1]
            steps = {
                round(labels[index + 1] - labels[index], 10)
                for index in range(len(labels) - 1)
            }
            if len(steps) != 1:
                raise SystemExit(f"irregular axis steps in {pdf}: {sorted(steps)}")
            step = steps.pop()
            if args.rust:
                row = ", ".join(rust_float(v) for v in (value, axis_max, step))
                print(f"        ({row}),")
            elif args.mode == "frame-width":
                width = plot_width(pdf, axis_max)
                print(
                    f"{format_number(value):>12} {format_number(axis_max):>12} "
                    f"{format_number(step):>12} {width:12.3f}"
                )
            else:
                print(
                    f"{format_number(value):>12} {format_number(axis_max):>12} "
                    f"{format_number(step):>12}"
                )
    finally:
        shutil.rmtree(work_root, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

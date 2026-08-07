#!/usr/bin/env python3
"""Compare a rendered PDF against a native Office export on three axes.

No single measure is trustworthy alone, and each one's blind spot is
another's strength:

- **Geometry** catches position, size, and pitch. It is what actually
  changes when a layout bug is fixed, and it is the only axis that can
  distinguish "moved to the right place" from "moved somewhere else".
  Blind to colour and to elements that are absent entirely.
- **Histogram** catches fill colour, recolouring, and missing elements,
  because it counts what is drawn without caring where. Blind to position,
  size, and font: those keep the ink total the same.
- **Pixel difference** is the catch-all that notices what the other two
  were not looking for. It is the weakest signal of the three: `AE` counts
  differing pixels without weighing how different they are, so it scores a
  layout shift and a colour inversion alike, and it can *rise* when a fix is
  correct but the element is still displaced by an unrelated defect.

Read them together. A fix that improves geometry while leaving the
histogram flat has moved something without changing what is drawn, which is
usually exactly what a positioning fix should do.

Usage:
    compare_render.py GT.pdf OUTPUT.pdf [--page N] [--dpi 150]
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

PAGE_RE = re.compile(r'<page width="[\d.]+" height="[\d.]+">(.*?)</page>', re.S)
WORD_RE = re.compile(
    r'<word xMin="([\d.-]+)" yMin="([\d.-]+)" xMax="([\d.-]+)" yMax="([\d.-]+)">(.*?)</word>',
    re.S,
)
FILL_TEXT_RE = re.compile(
    r'<fill_text[^>]*transform="([-0-9.]+) [-0-9.]+ [-0-9.]+ ([-0-9.]+) '
    r'([-0-9.]+) ([-0-9.]+)"[^>]*>(.*?)</fill_text>',
    re.S,
)
# mutool 1.23.x opens a page as `<page mediabox="...">`; later builds add a
# `number` attribute. Splitting on the numbered form alone yields zero pages and
# silently drops the geometry axis.
TRACE_PAGE_RE = re.compile(r"<page\b")
GLYPH_RE = re.compile(r'<g unicode="([^"]*)" glyph="[^"]*" x="([-0-9.]+)" y="([-0-9.]+)"')
HISTOGRAM_BINS = 32


@dataclass(frozen=True)
class TextLine:
    page: int
    x_min: float
    y_min: float
    text: str


def render_page(pdf: Path, page: int, dpi: int, out_dir: Path, role: str) -> Path:
    """Rasterise one page, returning the PNG path.

    `role` names the output, because the GT and the candidate usually share
    a file stem: rendering both under that stem made the second overwrite
    the first, and every comparison then ran an image against itself and
    reported a perfect match.
    """
    prefix = out_dir / role
    subprocess.run(
        ["pdftoppm", "-r", str(dpi), "-png", "-f", str(page), "-l", str(page),
         str(pdf), str(prefix)],
        check=True,
        capture_output=True,
    )
    pages = sorted(out_dir.glob(f"{role}-*.png"))
    if not pages:
        raise SystemExit(f"{pdf}: page {page} did not render")
    return pages[0]


def has_mutool() -> bool:
    """Whether `mutool` is on PATH (it ships in `mupdf-tools`)."""
    return shutil.which("mutool") is not None


def baseline_lines(pdf: Path) -> list[TextLine]:
    """Text lines positioned by their true baseline, via `mutool draw -F trace`.

    A `<fill_text>` carries the text-space matrix, so a glyph's baseline is
    `ty + d * gy` and its x is `a * gx + tx`. Office exports on macOS place
    text in a scaled space (`transform=".24 0 0 -.24 0 201.8"` with `trm="92"`
    for 22pt text) while ours are plain translations; one formula covers both.

    Baselines jitter by fractions of a point inside one visual line, so rows
    are bucketed to 1pt before being joined.
    """
    trace = subprocess.run(
        ["mutool", "draw", "-F", "trace", "-o", "-", str(pdf)],
        capture_output=True,
        text=True,
    )
    if trace.returncode != 0:
        return []
    lines: list[TextLine] = []
    for page_index, page in enumerate(TRACE_PAGE_RE.split(trace.stdout)[1:]):
        rows: dict[int, list[tuple[float, float, str]]] = {}
        for match in FILL_TEXT_RE.finditer(page):
            scale_x, scale_y = float(match.group(1)), float(match.group(2))
            translate_x, translate_y = float(match.group(3)), float(match.group(4))
            for glyph in GLYPH_RE.finditer(match.group(5)):
                char = glyph.group(1)
                if not char.strip():
                    continue
                baseline = translate_y + scale_y * float(glyph.group(3))
                rows.setdefault(round(baseline), []).append(
                    (translate_x + scale_x * float(glyph.group(2)), baseline, char)
                )
        for key in sorted(rows):
            glyphs = sorted(rows[key])
            text = re.sub(r"\s+", " ", "".join(glyph[2] for glyph in glyphs)).strip()
            if text:
                # The 1pt bucket only groups glyphs into a line; reporting its
                # key as the position would quantise every measurement to 1pt
                # and hide the sub-point row-pitch differences this axis exists
                # to find. Report the line's own baseline instead.
                lines.append(
                    TextLine(
                        page_index,
                        min(g[0] for g in glyphs),
                        min(g[1] for g in glyphs),
                        text,
                    )
                )
    return lines


def descriptor_box_lines(pdf: Path) -> list[TextLine]:
    """Fallback line tops from `pdftotext -bbox`, used only without `mutool`.

    `yMin` is each glyph's *font-descriptor box*, not its ink or its baseline.
    The two PDFs always embed different subsets, so the drift this yields
    carries an error proportional to font size — on the newsletter mock it
    reported +2.90pt for a 22pt heading whose baseline is really 1.07pt the
    other way, which is how #501 came to be filed against a defect that did
    not exist (issue #505).
    """
    xml = subprocess.run(
        ["pdftotext", "-bbox", str(pdf), "-"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    lines: list[TextLine] = []
    for page_index, page in enumerate(PAGE_RE.findall(xml)):
        rows: dict[int, list[tuple[float, float, str]]] = {}
        for match in WORD_RE.finditer(page):
            x_min, y_min = float(match.group(1)), float(match.group(2))
            rows.setdefault(round(y_min), []).append((x_min, y_min, match.group(5)))
        for key in sorted(rows):
            words = sorted(rows[key])
            text = re.sub(r"\s+", " ", " ".join(word[2] for word in words)).strip()
            if text:
                lines.append(
                    TextLine(page_index, min(w[0] for w in words),
                             min(w[1] for w in words), text)
                )
    return lines


def text_lines(pdf: Path) -> list[TextLine]:
    """Lines for the geometry axis, by true baseline where `mutool` allows."""
    if has_mutool():
        lines = baseline_lines(pdf)
        if lines:
            return lines
    return descriptor_box_lines(pdf)


def unique_lines(lines: list[TextLine]) -> dict[str, TextLine]:
    """Lines keyed by their text, keeping only texts that occur exactly once.

    A repeated string cannot be paired between two renderings without guessing
    which occurrence is which, so repeats are dropped rather than mismatched.
    """
    seen: dict[str, list[TextLine]] = {}
    for line in lines:
        seen.setdefault(line.text, []).append(line)
    return {text: found[0] for text, found in seen.items() if len(found) == 1}


def report_geometry(gt: Path, other: Path) -> dict[str, float]:
    """Vertical and horizontal drift of every line matched by unique text."""
    gt_lines = unique_lines(text_lines(gt))
    other_lines = unique_lines(text_lines(other))
    dy: list[float] = []
    dx: list[float] = []
    page_mismatch = 0
    for text, reference in gt_lines.items():
        candidate = other_lines.get(text)
        if candidate is None:
            continue
        if candidate.page != reference.page:
            page_mismatch += 1
            continue
        dy.append(candidate.y_min - reference.y_min)
        dx.append(candidate.x_min - reference.x_min)

    print("## Geometry — position, size, pitch")
    if not has_mutool():
        print("  APPROXIMATE: mutool absent, so positions come from font-descriptor")
        print("  boxes rather than baselines. The error scales with font size and can")
        print("  invert the sign. Install mupdf-tools before trusting these numbers.")
    if not dy:
        print("  no lines matched by unique text; compare pages manually")
        return {}
    mad_y = sum(abs(value) for value in dy) / len(dy)
    mad_x = sum(abs(value) for value in dx) / len(dx)
    coverage = len(dy) / len(gt_lines) if gt_lines else 0.0
    print(f"  matched lines      {len(dy)} of {len(gt_lines)} "
          f"({coverage * 100:.0f}% of the GT's unique lines)")
    print(f"  vertical   MAD {mad_y:7.2f}pt   worst {max(dy, key=abs):+8.2f}pt")
    print(f"  horizontal MAD {mad_x:7.2f}pt   worst {max(dx, key=abs):+8.2f}pt")
    if page_mismatch:
        print(f"  on a different page: {page_mismatch} line(s) — pagination differs")
    return {
        "mad_y": mad_y,
        "mad_x": mad_x,
        "page_mismatch": float(page_mismatch),
        "matched": float(len(dy)),
        "coverage": coverage,
    }


def histogram(png: Path) -> tuple[list[int], int]:
    """Per-channel binned colour counts, flattened R|G|B, and the pixel total."""
    txt = subprocess.run(
        ["magick", str(png), "-depth", "8", "txt:-"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    counts = [0] * (3 * HISTOGRAM_BINS)
    total = 0
    step = 256 // HISTOGRAM_BINS
    for line in txt.splitlines():
        head, _, rest = line.partition("#")
        if not head or len(rest) < 6:
            continue
        try:
            channels = [int(rest[i : i + 2], 16) for i in (0, 2, 4)]
        except ValueError:
            continue
        for index, value in enumerate(channels):
            counts[index * HISTOGRAM_BINS + min(value // step, HISTOGRAM_BINS - 1)] += 1
        total += 1
    return counts, total


def ink_fraction(counts: list[int], total: int) -> float:
    """Share of pixels that are not near-white, averaged over the channels."""
    if total == 0:
        return 0.0
    dark = sum(
        counts[channel * HISTOGRAM_BINS + b]
        for channel in range(3)
        for b in range(HISTOGRAM_BINS - 2)
    )
    return dark / (3.0 * total)


def report_histogram(gt_png: Path, other_png: Path) -> dict[str, float]:
    """Colour-distribution agreement, independent of where the pixels sit."""
    gt_counts, gt_total = histogram(gt_png)
    counts, total = histogram(other_png)
    gt_sum = sum(gt_counts) or 1
    other_sum = sum(counts) or 1
    reference = [value / gt_sum for value in gt_counts]
    candidate = [value / other_sum for value in counts]
    intersection = sum(min(a, b) for a, b in zip(reference, candidate))
    # Bin-wise agreement punishes a one-level shift as hard as a recolour: a
    # smooth gradient dithers by a channel step or two between renderers, and
    # every one of those pixels lands in a neighbouring bin. Three decks
    # scored 0.9745-0.9860 on intersection with their gradients pixel-identical
    # to within +-2 per channel. Comparing the *cumulative* distributions
    # instead measures how far colour has to move, so a one-bin shift costs
    # almost nothing while a genuine recolour still shows.
    shift = cumulative_distance(reference, candidate)
    gt_ink = ink_fraction(gt_counts, gt_total)
    ink = ink_fraction(counts, total)

    print("## Histogram — fill colour, recolouring, missing elements")
    print(f"  intersection       {intersection:.4f}   (1.0000 = identical distribution)")
    print(f"  colour shift       {shift:.4f}   (0.0000 = identical; tolerates dithering)")
    print(f"  ink coverage       {ink * 100:6.3f}%  against GT {gt_ink * 100:6.3f}%"
          f"   ({(ink - gt_ink) * 100:+.3f}%)")
    return {
        "intersection": intersection,
        "shift": shift,
        "ink_delta": (ink - gt_ink) * 100.0,
    }


def cumulative_distance(reference: list[float], candidate: list[float]) -> float:
    """Mean per-channel distance between the cumulative distributions.

    Insensitive to a colour landing one bin either side of where it did in
    the reference, which is what renderer dithering produces, while still
    growing with a real change in what colour is present.
    """
    channels = 3
    total = 0.0
    for channel in range(channels):
        start = channel * HISTOGRAM_BINS
        run_reference = 0.0
        run_candidate = 0.0
        for index in range(start, start + HISTOGRAM_BINS):
            run_reference += reference[index]
            run_candidate += candidate[index]
            total += abs(run_reference - run_candidate)
    return total / (channels * HISTOGRAM_BINS)


def report_pixels(gt_png: Path, other_png: Path, out_dir: Path) -> None:
    """Whole-page difference, as a coarse catch-all."""
    normalised = out_dir / "gt-normalised.png"
    size = subprocess.run(
        ["magick", "identify", "-format", "%wx%h", str(other_png)],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    # pdftoppm can differ by a pixel between two PDFs of the same paper size,
    # which would otherwise make every comparison fail outright.
    subprocess.run(
        ["magick", str(gt_png), "-background", "white", "-extent", size, str(normalised)],
        check=True, capture_output=True,
    )

    print("## Pixel difference — coarse catch-all, read last")
    for label, args in (
        ("AE  5% fuzz", ["-metric", "AE", "-fuzz", "5%"]),
        ("AE  1% fuzz", ["-metric", "AE", "-fuzz", "1%"]),
        ("RMSE       ", ["-metric", "RMSE"]),
    ):
        result = subprocess.run(
            ["magick", "compare", *args, str(normalised), str(other_png), "null:"],
            capture_output=True, text=True,
        )
        print(f"  {label}      {result.stderr.strip()}")


def diagnose(geometry: dict[str, float], histogram_result: dict[str, float]) -> None:
    """Say what the combination of axes means, and what to look at next.

    This is the point of running all three: a single number invites the
    wrong conclusion. A pixel count that rises can accompany a correct fix,
    and one that does not move at all can hide a large geometric
    improvement. The pattern across axes is what identifies the defect
    class.
    """
    print("## Reading")
    if not geometry:
        print("  Geometry could not be measured, so the other axes stand alone.")
        print("  Compare the pages by eye before trusting them.")
        return

    # A drift figure averaged over a handful of lines is noise. Korean
    # sheets match poorly because word segmentation differs between the two
    # PDFs, and one such page reported 4.11pt of "drift" from nine matched
    # lines out of sixty — a number with no meaning that would have sent the
    # next investigation chasing a row-height bug that is not there.
    matched: float = geometry.get("matched", 0.0)
    coverage: float = geometry.get("coverage", 0.0)
    if matched < 10 or coverage < 0.25:
        print(f"  Only {matched:.0f} lines matched ({coverage * 100:.0f}% of the GT).")
        print("  Treat the geometry figures as unreliable: too few samples, and")
        print("  the ones that matched are not a random selection. Compare the")
        print("  pages by eye, or measure a specific element directly, before")
        print("  drawing any conclusion from the drift above.")
        print()

    mad_y: float = geometry["mad_y"]
    mad_x: float = geometry["mad_x"]
    pages_differ: bool = geometry["page_mismatch"] > 0
    intersection: float = histogram_result.get("intersection", 1.0)
    ink_delta: float = histogram_result.get("ink_delta", 0.0)

    # Thresholds are deliberately loose: they route attention, they do not
    # decide correctness. A point of drift is invisible; ten is not.
    drifts_vertically: bool = mad_y > 2.0
    drifts_horizontally: bool = mad_x > 1.0
    # Judge colour on the shift, not the bin-wise intersection: the latter
    # flags smooth gradients that are pixel-identical to within dithering.
    # Measured separation on this corpus: renderer dithering across a smooth
    # gradient reaches 0.0003, and the half-width cell borders of #487
    # reached 0.0016 before the fix and 0.0004 after it.
    colour_differs: bool = histogram_result.get("shift", 0.0) > 0.001
    ink_differs: bool = abs(ink_delta) > 0.2

    findings: list[str] = []
    if pages_differ:
        findings.append(
            "Pagination differs — content sits on the wrong page. Fix this "
            "first: every per-line measurement below it is contaminated by "
            "the accumulated drift that pushed it over."
        )
    if drifts_vertically:
        findings.append(
            f"Vertical drift {mad_y:.2f}pt — line advance, paragraph spacing, "
            "or row height. Compare consecutive-line pitch against the GT "
            "rather than absolute positions, so a constant offset near the "
            "top does not read as a spacing bug."
        )
    if drifts_horizontally:
        findings.append(
            f"Horizontal drift {mad_x:.2f}pt — indent, column width, or "
            "margin. If it grows across the page it is per-column and "
            "cumulative; if it is constant it is an indent or margin."
        )
    if colour_differs:
        findings.append(
            f"Colour distribution differs (shift {histogram_result.get('shift', 0.0):.4f}) — "
            "a fill, theme colour, or shading is wrong, or an element is "
            "missing entirely. Position measurements will not show this."
        )
    if ink_differs and not colour_differs:
        findings.append(
            f"Ink coverage is off by {ink_delta:+.3f}% while the colour "
            "distribution matches — the right things are drawn in the right "
            "colours but at the wrong size, or a font renders at a different "
            "weight."
        )

    if not findings:
        print("  No axis shows a material difference. What remains is font")
        print("  rasterisation and antialiasing; inspect crops at full")
        print("  resolution before concluding anything is wrong.")
        return
    for index, finding in enumerate(findings, start=1):
        print(f"  {index}. {finding}")

    if not colour_differs and not ink_differs and (drifts_vertically or drifts_horizontally):
        print()
        print("  Colour and ink are unchanged while geometry moves: this is a")
        print("  pure layout difference. A pixel count may rise even as the")
        print("  fix is correct, because a displaced element that grows")
        print("  toward its true size overlaps GT less, not more.")


def report_matched_lines(gt: Path, other: Path) -> None:
    """Per-line baselines for every line matched by unique text.

    Aggregate drift says a page is wrong; this says which line. Pairing the two
    PDFs by hand — taking the topmost line, or grepping for a prefix — silently
    matches the wrong line and produces impossible numbers, so the pairing here
    is the same unique-text match the geometry axis already trusts.
    """
    gt_lines = unique_lines(text_lines(gt))
    other_lines = unique_lines(text_lines(other))
    rows: list[tuple[TextLine, TextLine]] = [
        (reference, other_lines[text])
        for text, reference in gt_lines.items()
        if text in other_lines and other_lines[text].page == reference.page
    ]
    rows.sort(key=lambda pair: (pair[0].page, pair[0].y_min))

    print("## Matched lines — baseline of each, and the pitch between them")
    if not rows:
        print("  none matched by unique text")
        return
    print(f"  {'page':>4} {'GT':>9} {'output':>9} {'delta':>8} "
          f"{'GT pitch':>9} {'out pitch':>9}  text")
    previous: tuple[float, float] | None = None
    for reference, candidate in rows:
        gt_pitch = "" if previous is None else f"{reference.y_min - previous[0]:9.2f}"
        out_pitch = "" if previous is None else f"{candidate.y_min - previous[1]:9.2f}"
        print(f"  {reference.page + 1:>4} {reference.y_min:9.2f} {candidate.y_min:9.2f} "
              f"{candidate.y_min - reference.y_min:+8.2f} {gt_pitch:>9} {out_pitch:>9}  "
              f"{reference.text[:44]}")
        previous = (reference.y_min, candidate.y_min)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("gt", type=Path, help="native Office export")
    parser.add_argument("output", type=Path, help="office2pdf output")
    parser.add_argument("--page", type=int, default=1)
    parser.add_argument("--dpi", type=int, default=150, help="at least 150")
    parser.add_argument(
        "--lines",
        action="store_true",
        help="list every matched line's baselines, so a single paragraph can be "
        "inspected without hand-pairing text between the two PDFs",
    )
    args = parser.parse_args()

    if args.dpi < 150:
        raise SystemExit("--dpi must be at least 150; hairlines vanish below that")

    print(f"GT     {args.gt}")
    print(f"output {args.output}")
    print(f"page {args.page} at {args.dpi} DPI\n")

    geometry = report_geometry(args.gt, args.output)
    print()
    if args.lines:
        report_matched_lines(args.gt, args.output)
        print()
    with tempfile.TemporaryDirectory() as raw_dir:
        out_dir = Path(raw_dir)
        gt_png = render_page(args.gt, args.page, args.dpi, out_dir, "gt")
        other_png = render_page(args.output, args.page, args.dpi, out_dir, "candidate")
        histogram_result = report_histogram(gt_png, other_png)
        print()
        report_pixels(gt_png, other_png, out_dir)
    print()
    diagnose(geometry, histogram_result)


if __name__ == "__main__":
    sys.exit(main())

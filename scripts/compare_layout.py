#!/usr/bin/env python3
"""Trace-based layout differ: a per-page deviation vector instead of a pixel scalar.

Parses ``mutool draw -F trace`` for a ground-truth PDF and an office2pdf
output, reconstructs text lines and vector rects in device points, matches
lines by their text, and reports typed deviations:

- matched / missing / extra lines, and wrap-point differences (text that is
  present but breaks at a different word) counted separately from real loss;
- baseline dy statistics, per-line dx0, line-width drift, inter-line pitch
  deltas between consecutive matched lines;
- a fill/stroke rect census with nearest-match position deltas.

A noise floor (default 0.12pt — native Word exports quantise coordinates to a
0.24pt grid; use 0.5 for Excel GT, whose Quartz export rounds every advance to
a whole point) separates measurement noise from deviations: raw statistics are
always reported, but a line only counts as *deviant* past the floor.

Pixel metrics (`compare_render.py`) remain the tripwire for whatever this tool
does not model — colour, images, effects. This tool is the primary signal for
position, size, pitch, wrap, and element presence, which is what actually
changes when a layout defect is fixed.

Usage:
    compare_layout.py GT.pdf OUTPUT.pdf [--page N] [--noise-floor PT] [--json]
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import statistics
import subprocess
import sys
from dataclasses import dataclass, field
from difflib import SequenceMatcher
from pathlib import Path

PAGE_RE = re.compile(r'<page number="\d+"[^>]*>(.*?)</page>', re.S)
FILL_TEXT_RE = re.compile(r"<fill_text\b([^>]*)>(.*?)</fill_text>", re.S)
SPAN_RE = re.compile(r"<span\b([^>]*)>(.*?)</span>", re.S)
GLYPH_RE = re.compile(
    r'<g unicode="([^"]*)" glyph="[^"]*" x="([-0-9.e]+)" y="([-0-9.e]+)" adv="([-0-9.e]+)"'
)
PATH_RE = re.compile(r"<(fill_path|stroke_path)\b([^>]*)>(.*?)</\1>", re.S)
POINT_RE = re.compile(r'<(?:moveto|lineto|curveto)[^>]*x="([-0-9.e]+)" y="([-0-9.e]+)"')
TRANSFORM_RE = re.compile(
    r'transform="([-0-9.e]+) ([-0-9.e]+) ([-0-9.e]+) ([-0-9.e]+) ([-0-9.e]+) ([-0-9.e]+)"'
)
TRM_RE = re.compile(r'trm="([-0-9.e]+) ')

LINE_Y_TOLERANCE_PT = 0.6
RECT_MATCH_RADIUS_PT = 6.0
XML_ESCAPES = {"&amp;": "&", "&lt;": "<", "&gt;": ">", "&quot;": '"', "&apos;": "'"}


@dataclass(frozen=True)
class Glyph:
    x: float
    y: float
    unicode: str
    size: float
    advance: float


@dataclass(frozen=True)
class Rect:
    kind: str  # "fill" | "stroke"
    x0: float
    y0: float
    x1: float
    y1: float

    @property
    def center(self) -> tuple[float, float]:
        return ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2)


@dataclass
class Line:
    y: float
    glyphs: list[Glyph] = field(default_factory=list)

    @property
    def text(self) -> str:
        return "".join(g.unicode for g in self.glyphs)

    @property
    def key(self) -> str:
        """Whitespace-free text; GT exports carry stray trailing space glyphs."""
        return "".join(g.unicode for g in self.glyphs if not g.unicode.isspace())

    @property
    def x0(self) -> float:
        return self.glyphs[0].x

    @property
    def x1(self) -> float:
        last = self.glyphs[-1]
        return last.x + last.advance

    @property
    def width(self) -> float:
        return self.x1 - self.x0


@dataclass
class PageLayout:
    lines: list[Line]
    rects: list[Rect]


def unescape(text: str) -> str:
    for entity, char in XML_ESCAPES.items():
        text = text.replace(entity, char)
    return text


def parse_transform(attrs: str) -> tuple[float, float, float, float, float, float] | None:
    match = TRANSFORM_RE.search(attrs)
    if not match:
        return None
    return tuple(float(value) for value in match.groups())  # type: ignore[return-value]


def parse_trace(trace_xml: str) -> list[PageLayout]:
    pages: list[PageLayout] = []
    for page_match in PAGE_RE.finditer(trace_xml):
        content = page_match.group(1)
        glyphs: list[Glyph] = []
        for op_attrs, op_body in FILL_TEXT_RE.findall(content):
            transform = parse_transform(op_attrs)
            if transform is None:
                continue
            a, _b, _c, d, e, f = transform
            for span_attrs, span_body in SPAN_RE.findall(op_body):
                trm = TRM_RE.search(span_attrs)
                size_units = float(trm.group(1)) if trm else 0.0
                size_pt = abs(size_units * a)
                for unicode_char, gx, gy, adv in GLYPH_RE.findall(span_body):
                    glyphs.append(
                        Glyph(
                            x=a * float(gx) + e,
                            y=d * float(gy) + f,
                            unicode=unescape(unicode_char),
                            size=size_pt,
                            advance=float(adv) * size_units * abs(a),
                        )
                    )
        rects: list[Rect] = []
        for kind, op_attrs, op_body in PATH_RE.findall(content):
            transform = parse_transform(op_attrs) or (1, 0, 0, 1, 0, 0)
            a, _b, _c, d, e, f = transform
            xs: list[float] = []
            ys: list[float] = []
            for px, py in POINT_RE.findall(op_body):
                xs.append(a * float(px) + e)
                ys.append(d * float(py) + f)
            if xs:
                rects.append(
                    Rect(
                        kind="fill" if kind == "fill_path" else "stroke",
                        x0=min(xs),
                        y0=min(ys),
                        x1=max(xs),
                        y1=max(ys),
                    )
                )
        pages.append(PageLayout(lines=build_lines(glyphs), rects=rects))
    return pages


def build_lines(glyphs: list[Glyph], y_tolerance: float = LINE_Y_TOLERANCE_PT) -> list[Line]:
    lines: list[Line] = []
    for glyph in sorted(glyphs, key=lambda g: (g.y, g.x)):
        target = None
        for line in lines:
            if abs(line.y - glyph.y) <= y_tolerance:
                target = line
                break
        if target is None:
            target = Line(y=glyph.y)
            lines.append(target)
        target.glyphs.append(glyph)
    for line in lines:
        line.glyphs.sort(key=lambda g: g.x)
        line.y = statistics.median(g.y for g in line.glyphs)
    lines.sort(key=lambda line: (line.y, line.x0))
    return [line for line in lines if line.key]


def match_lines(
    gt_lines: list[Line], out_lines: list[Line]
) -> tuple[list[tuple[Line, Line]], list[Line], list[Line]]:
    matcher = SequenceMatcher(
        a=[line.key for line in gt_lines], b=[line.key for line in out_lines], autojunk=False
    )
    matches: list[tuple[Line, Line]] = []
    missing: list[Line] = []
    extra: list[Line] = []
    for tag, a0, a1, b0, b1 in matcher.get_opcodes():
        if tag == "equal":
            matches.extend(zip(gt_lines[a0:a1], out_lines[b0:b1]))
        else:
            missing.extend(gt_lines[a0:a1])
            extra.extend(out_lines[b0:b1])
    return matches, missing, extra


def take_wrap_differences(
    missing: list[Line], extra: list[Line]
) -> tuple[list[str], list[Line], list[Line]]:
    """Pull re-wrapped paragraphs out of the missing/extra pools.

    A wrap difference is text that exists on both sides but breaks at a
    different point, so joined runs of unmatched lines carry the same
    characters. Greedy prefix consumption keeps it linear and is sufficient
    for the contiguous runs SequenceMatcher produces.
    """
    wraps: list[str] = []
    remaining_missing = list(missing)
    remaining_extra = list(extra)
    while remaining_missing and remaining_extra:
        gt_join = "".join(line.key for line in remaining_missing)
        out_join = "".join(line.key for line in remaining_extra)
        if not gt_join or gt_join != out_join:
            break
        wraps.append(remaining_missing[0].key[:60])
        remaining_missing.clear()
        remaining_extra.clear()
    return wraps, remaining_missing, remaining_extra


def take_reflows(missing: list[Line], extra: list[Line]) -> tuple[dict, list[Line], list[Line]]:
    """Classify unmatched lines whose characters all survive as reflow, not loss.

    Same multiset of characters on both sides means the text is present but
    grouped into different lines (baseline splits, cell order) — the audit's
    invoice wrap-flip and budget baseline-split cases. Real loss keeps its
    asymmetric remainder in missing/extra.
    """
    gt_chars = sorted("".join(line.key for line in missing))
    out_chars = sorted("".join(line.key for line in extra))
    if gt_chars and gt_chars == out_chars:
        info = {
            "gt_lines": len(missing),
            "out_lines": len(extra),
            "samples": [line.key[:60] for line in missing[:3]],
        }
        return info, [], []
    return {"gt_lines": 0, "out_lines": 0, "samples": []}, missing, extra


def diff_page(
    gt: PageLayout, out: PageLayout, noise_floor: float = 0.12
) -> dict:
    matches, missing, extra = match_lines(gt.lines, out.lines)
    wraps, missing, extra = take_wrap_differences(missing, extra)
    reflow, missing, extra = take_reflows(missing, extra)

    dys = [out_line.y - gt_line.y for gt_line, out_line in matches]
    dx0s = [out_line.x0 - gt_line.x0 for gt_line, out_line in matches]
    width_pcts = [
        (out_line.width - gt_line.width) / gt_line.width * 100
        for gt_line, out_line in matches
        if gt_line.width > 1.0
    ]

    pitch_deltas: list[float] = []
    matched_gt = {id(gt_line) for gt_line, _ in matches}
    ordered = [pair for pair in matches]
    ordered.sort(key=lambda pair: pair[0].y)
    for (gt_a, out_a), (gt_b, out_b) in zip(ordered, ordered[1:]):
        if id(gt_a) in matched_gt and id(gt_b) in matched_gt:
            pitch_deltas.append((out_b.y - out_a.y) - (gt_b.y - gt_a.y))

    worst_dy_index = max(range(len(dys)), key=lambda i: abs(dys[i]), default=None)

    rect_pairs = 0
    rect_center_deltas: list[float] = []
    unclaimed = list(out.rects)
    for gt_rect in gt.rects:
        best = None
        best_distance = RECT_MATCH_RADIUS_PT
        for candidate in unclaimed:
            if candidate.kind != gt_rect.kind:
                continue
            distance = max(
                abs(candidate.center[0] - gt_rect.center[0]),
                abs(candidate.center[1] - gt_rect.center[1]),
            )
            if distance <= best_distance:
                best, best_distance = candidate, distance
        if best is not None:
            unclaimed.remove(best)
            rect_pairs += 1
            rect_center_deltas.append(best_distance)

    def stats(values: list[float]) -> dict:
        if not values:
            return {"mean_abs": 0.0, "worst": 0.0}
        return {
            "mean_abs": sum(abs(v) for v in values) / len(values),
            "worst": max(values, key=abs),
        }

    dy_stats = stats(dys)
    return {
        "lines": {
            "gt": len(gt.lines),
            "out": len(out.lines),
            "matched": len(matches),
            "missing": len(missing),
            "extra": len(extra),
            "deviant": sum(1 for dy in dys if abs(dy) > noise_floor),
            "missing_text": [line.key[:60] for line in missing[:5]],
            "extra_text": [line.key[:60] for line in extra[:5]],
        },
        "baseline": {
            "mean_abs_dy": dy_stats["mean_abs"],
            "worst_dy": abs(dy_stats["worst"]),
            "worst_dy_signed": dy_stats["worst"],
            "worst_line": (
                matches[worst_dy_index][0].key[:60] if worst_dy_index is not None else ""
            ),
        },
        "dx0": stats(dx0s),
        "width": {
            "mean_abs_pct": stats(width_pcts)["mean_abs"],
            "worst_pct": abs(stats(width_pcts)["worst"]),
        },
        "pitch": {"pairs": len(pitch_deltas), "worst_delta": abs(stats(pitch_deltas)["worst"])},
        "wraps": {"count": len(wraps), "samples": wraps[:5]},
        "reflow": reflow,
        "rects": {
            "gt_count": len(gt.rects),
            "out_count": len(out.rects),
            "matched": rect_pairs,
            "mean_center_delta": stats(rect_center_deltas)["mean_abs"],
        },
        "noise_floor": noise_floor,
    }


def render_reading(vectors: list[dict]) -> str:
    lines = ["## Reading", ""]
    for index, vector in enumerate(vectors, start=1):
        page_notes: list[str] = []
        line_info = vector["lines"]
        if line_info["missing"] or line_info["extra"]:
            page_notes.append(
                f"content differs: {line_info['missing']} GT line(s) unmatched "
                f"(e.g. {line_info['missing_text'][:1]}), {line_info['extra']} extra"
            )
        if vector["wraps"]["count"]:
            page_notes.append(
                f"{vector['wraps']['count']} paragraph(s) wrap at a different word "
                "(text present, break point moved) — check advances, kerning, or spacing"
            )
        if vector["reflow"]["gt_lines"]:
            page_notes.append(
                f"{vector['reflow']['gt_lines']} GT line(s) re-grouped into "
                f"{vector['reflow']['out_lines']} output line(s) with no text lost — "
                "baseline splits or a moved wrap, not missing content"
            )
        if line_info["deviant"]:
            page_notes.append(
                f"{line_info['deviant']}/{line_info['matched']} matched lines sit past the "
                f"{vector['noise_floor']}pt floor; worst {vector['baseline']['worst_dy_signed']:+.2f}pt "
                f"on '{vector['baseline']['worst_line']}'"
            )
        if vector["pitch"]["worst_delta"] > vector["noise_floor"]:
            page_notes.append(
                f"line pitch drifts up to {vector['pitch']['worst_delta']:.2f}pt — "
                "line-height model rather than block placement"
            )
        if vector["rects"]["gt_count"] != vector["rects"]["out_count"]:
            page_notes.append(
                f"rect census differs: GT {vector['rects']['gt_count']} vs "
                f"output {vector['rects']['out_count']} draw ops — can be op-splitting "
                "(per-cell border segments vs one merged stroke), so confirm visually "
                "before reading it as missing ink"
            )
        if not page_notes:
            page_notes.append("no deviation past the noise floor")
        lines.append(f"- page {index}: " + "; ".join(page_notes))
    return "\n".join(lines)


def run_mutool(pdf: Path) -> str:
    mutool = shutil.which("mutool")
    if mutool is None:
        sys.exit("mutool is required (brew install mupdf-tools)")
    result = subprocess.run(
        [mutool, "draw", "-F", "trace", str(pdf)],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("gt", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--page", type=int, help="1-based page to compare (default: all)")
    parser.add_argument(
        "--noise-floor",
        type=float,
        default=0.12,
        help="pt threshold under which a delta is measurement noise (0.12 Word GT, 0.5 Excel GT)",
    )
    parser.add_argument("--json", action="store_true", help="emit the deviation vectors as JSON")
    args = parser.parse_args()

    gt_pages = parse_trace(run_mutool(args.gt))
    out_pages = parse_trace(run_mutool(args.output))

    if args.page is not None:
        gt_pages = gt_pages[args.page - 1 : args.page]
        out_pages = out_pages[args.page - 1 : args.page]

    count = min(len(gt_pages), len(out_pages))
    vectors = [
        diff_page(gt_pages[i], out_pages[i], noise_floor=args.noise_floor) for i in range(count)
    ]

    if args.json:
        print(json.dumps({"pages": vectors, "gt_pages": len(gt_pages), "out_pages": len(out_pages)}, indent=1))
        return 0

    if len(gt_pages) != len(out_pages):
        print(f"PAGE COUNT MISMATCH: GT {len(gt_pages)} vs output {len(out_pages)}\n")
    for index, vector in enumerate(vectors, start=1):
        line_info = vector["lines"]
        print(
            f"page {index}: {line_info['matched']} matched "
            f"({line_info['missing']} missing, {line_info['extra']} extra, "
            f"{vector['wraps']['count']} re-wrapped) | "
            f"dy mean {vector['baseline']['mean_abs_dy']:.2f}pt worst "
            f"{vector['baseline']['worst_dy_signed']:+.2f}pt | "
            f"pitch worst {vector['pitch']['worst_delta']:.2f}pt | "
            f"width worst {vector['width']['worst_pct']:.1f}% | "
            f"rects {vector['rects']['gt_count']}/{vector['rects']['out_count']}"
        )
    print()
    print(render_reading(vectors))
    return 0


if __name__ == "__main__":
    sys.exit(main())

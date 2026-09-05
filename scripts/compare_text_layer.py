#!/usr/bin/env python3
"""Text-layer codepoint census: is the selectable and searchable text intact?

Some defects are invisible in any raster and in any geometry comparison, because
they change *what a reader can select and search for* rather than where ink
lands. Two from the 2026-07-29 audit wave:

- U+2060 WORD JOINER and U+00A0 injected into slide text (issue #664), which
  render identically but break search and copy;
- ``ffi`` ligatures swallowing letter-spacing so the run extracted as
  ``o ffi c e 2 p d f`` instead of ``office2pdf`` (issue #684) — 17 codepoints
  where the word is 10, matching no search for it.

Measured on the pre-fix output of ``office2pdf_introduction_ko.pptx``:
``grep -c "o ffi c e 2 p d f"`` returned 24 and ``grep -c "ofce2pdf"``
returned 0. ``pdftotext`` renders the merged glyph as the three letters
``ffi``, and the tracking puts a space between every glyph, so characters are
*gained* rather than lost — which is why this shows up below as a
``space:SPACE`` delta and not a ``ligature:`` one. (Issue #684's body says the
text reads ``ofce2pdf``; that string does not occur in the output, and the
issue carries a correction to that effect.)

Both are one extraction pass from automatic detection, and cost milliseconds.

This tool answers two questions separately:

1. **Census** — how many codepoints of each class does each side carry?
   Injected joiners, NBSPs, ligature codepoints and control characters show up
   as a class delta even when the rendered page is pixel-identical.
2. **Content** — after undoing ligatures and collapsing whitespace, is the text
   sequence the same? A residual requires checking extraction order and
   missing or extra text; sequence inequality alone does not prove text loss.

Reporting them apart matters: a ligature changes the census without changing the
content, while a dropped word changes content. Conflating them makes a cosmetic
difference look like data loss.

Usage:
    compare_text_layer.py GT.pdf OUTPUT.pdf [--page N] [--json]

Requires ``pdftotext`` (poppler-utils).
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import unicodedata
from collections import Counter
from pathlib import Path

# Codepoints that a correct converter should never inject into the text layer.
# Each renders as nothing or as an ordinary space, so a raster comparison cannot
# see them, but each breaks search and copy.
INVISIBLE_FORMATTERS: dict[str, str] = {
    "⁠": "WORD JOINER",
    "​": "ZERO WIDTH SPACE",
    "‌": "ZERO WIDTH NON-JOINER",
    "‍": "ZERO WIDTH JOINER",
    "﻿": "ZERO WIDTH NO-BREAK SPACE",
    "­": "SOFT HYPHEN",
}

# Spaces that are not U+0020. NBSP is the one converters reach for when they
# want a space that does not collapse, which is exactly what breaks a search.
NONSTANDARD_SPACES: dict[str, str] = {
    " ": "NO-BREAK SPACE",
    " ": "FIGURE SPACE",
    " ": "THIN SPACE",
    " ": "NARROW NO-BREAK SPACE",
    "　": "IDEOGRAPHIC SPACE",
}

# Precomposed ligatures. A ligature is correct typography but wrong text: the
# reader searching for "office" finds nothing if the file holds U+FB03.
LIGATURES: dict[str, str] = {
    "ﬀ": "ff",
    "ﬁ": "fi",
    "ﬂ": "fl",
    "ﬃ": "ffi",
    "ﬄ": "ffl",
    "ﬅ": "st",
    "ﬆ": "st",
}


def extract_text(pdf: Path, page: int | None) -> str:
    """The PDF's text layer, as a reader's copy-paste would see it."""
    command = ["pdftotext"]
    if page is not None:
        command += ["-f", str(page), "-l", str(page)]
    command += [str(pdf), "-"]
    try:
        result = subprocess.run(command, capture_output=True, check=True)
    except FileNotFoundError:
        sys.exit("pdftotext not found — install poppler-utils.")
    except subprocess.CalledProcessError as error:
        sys.exit(f"pdftotext failed on {pdf}: {error}")
    return result.stdout.decode("utf-8", errors="replace")


def census(text: str) -> dict[str, int]:
    """Counts per codepoint class, not per codepoint.

    Classes are what a reviewer can act on: "3 word joiners appeared" is a
    finding, "codepoint 8288 appeared 3 times" is a lookup.
    """
    counts: Counter[str] = Counter()
    for char in text:
        if char in INVISIBLE_FORMATTERS:
            counts[f"invisible:{INVISIBLE_FORMATTERS[char]}"] += 1
        elif char in NONSTANDARD_SPACES:
            counts[f"space:{NONSTANDARD_SPACES[char]}"] += 1
        elif char in LIGATURES:
            counts[f"ligature:{LIGATURES[char]}"] += 1
        elif char == " ":
            counts["space:SPACE"] += 1
        elif char in "\n\r\t":
            continue  # layout, not content
        elif unicodedata.category(char) == "Cc":
            counts["control"] += 1
    return dict(counts)


def normalize(text: str) -> str:
    """Undo ligatures and collapse whitespace, retaining extraction order.

    Comparing normalized forms separates "the text is different" from "the text
    is encoded differently" — a distinction a raw diff cannot make.
    """
    for ligature, expansion in LIGATURES.items():
        text = text.replace(ligature, expansion)
    for invisible in INVISIBLE_FORMATTERS:
        text = text.replace(invisible, "")
    for space in NONSTANDARD_SPACES:
        text = text.replace(space, " ")
    return re.sub(r"\s+", " ", text).strip()


def compare(gt_text: str, out_text: str) -> dict:
    """Census deltas and a content residual, reported apart."""
    gt_census, out_census = census(gt_text), census(out_text)
    deltas = {
        key: out_census.get(key, 0) - gt_census.get(key, 0)
        for key in set(gt_census) | set(out_census)
        if out_census.get(key, 0) != gt_census.get(key, 0)
    }
    gt_norm, out_norm = normalize(gt_text), normalize(out_text)
    return {
        "census_gt": gt_census,
        "census_output": out_census,
        "census_deltas": deltas,
        "content_matches": gt_norm == out_norm,
        "content_length_gt": len(gt_norm),
        "content_length_output": len(out_norm),
    }


def render_report(result: dict) -> str:
    """A report that says what to do, not just what differs."""
    lines: list[str] = ["## Text layer census", ""]
    deltas = result["census_deltas"]
    if not deltas:
        lines.append("No codepoint class differs.")
    else:
        lines.append("| class | GT | output | delta |")
        lines.append("| --- | ---: | ---: | ---: |")
        for key in sorted(deltas):
            lines.append(
                f"| {key} | {result['census_gt'].get(key, 0)} | "
                f"{result['census_output'].get(key, 0)} | {deltas[key]:+d} |"
            )
    lines += ["", "## Content", ""]
    if result["content_matches"]:
        lines.append(
            "Normalized content is identical — any class delta above is an "
            "encoding difference, not text loss."
        )
    else:
        lines.append(
            f"Normalized content DIFFERS: {result['content_length_gt']} chars in "
            f"GT against {result['content_length_output']} in the output. Review "
            "extraction order and check for missing or extra text."
        )

    lines += ["", "## Reading", ""]
    injected = {k: v for k, v in deltas.items() if v > 0 and k.startswith("invisible:")}
    ligatures = {k: v for k, v in deltas.items() if v > 0 and k.startswith("ligature:")}
    nbsp = {k: v for k, v in deltas.items() if v > 0 and k.startswith("space:") and "SPACE" != k.split(":")[1]}
    if injected:
        lines.append(
            "Invisible formatting characters were injected. They render as "
            "nothing, so no raster check can see them, but they break search "
            "and copy (see issue #664)."
        )
    if ligatures:
        lines.append(
            "The output carries ligature codepoints GT does not. The page may "
            "look right while the word is unsearchable (see issue #684)."
        )
    if nbsp:
        lines.append(
            "Non-standard spaces were introduced. A search for the plain-space "
            "form will not match."
        )
    if not (injected or ligatures or nbsp) and result["content_matches"]:
        lines.append("Text layer is intact.")
    elif not (injected or ligatures or nbsp):
        lines.append(
            "No class was injected. Compare the extractions directly: a sequence "
            "mismatch can reflect extraction order or missing or extra text."
        )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gt", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--page", type=int, default=None)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    result = compare(
        extract_text(args.gt, args.page),
        extract_text(args.output, args.page),
    )
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(render_report(result))
    # A census delta alone is not a failure; a sequence mismatch needs review.
    return 0 if result["content_matches"] else 1


if __name__ == "__main__":
    sys.exit(main())

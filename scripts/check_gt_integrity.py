#!/usr/bin/env python3
"""Integrity gate for ground-truth exports, run before any metric is derived.

A corrupt GT poisons everything downstream silently: page counts, text lengths,
baselines and pixel deltas all inherit the corruption and look like converter
defects. The workbook-per-sheet duplication of issue #616 survived undetected
for exactly this reason — nothing validated the export itself.

Three checks, each aimed at a failure this corpus has actually seen:

1. **Page periodicity** — render at low DPI, hash each page, and look for an
   exact repeating cycle. A GT exported once per worksheet but containing every
   worksheet produces a perfectly periodic page sequence. This is what would
   have caught #616 the day it was written.
2. **Page-count plausibility** — for XLSX, compare the GT page count against the
   worksheets the workbook would actually print. A GT with fewer pages than
   those cannot be complete; an exact multiple is the duplication signature
   again. Sheets marked `hidden` or `veryHidden` are excluded, because Excel
   never pages them (issue #1065) and counting them failed correct exports of
   every workbook carrying a hidden lookup sheet (issue #1211).
3. **Font census** — list the fonts the GT actually embeds and compare against
   the families the source declares. A substitution (Calibri rendered as Malgun
   Gothic) or lost emphasis is a *GT-side* artefact; reporting it here stops an
   auditor filing it as a converter defect, which has already cost audit time
   twice.

Findings are annotations, not automatic failures, except where the GT cannot be
trusted at all: only periodicity and an impossible page count exit non-zero,
because those mean every derived metric is invalid. A font substitution is
worth knowing but does not invalidate geometry.

Usage:
    check_gt_integrity.py GT.pdf [--source FILE.xlsx|.docx|.pptx] [--json]

Requires ``pdftoppm`` and ``pdffonts`` (both poppler-utils).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

# Low enough to be fast and to ignore antialiasing noise, high enough that two
# genuinely different pages never collide.
PERIODICITY_DPI = 20


def page_hashes(pdf: Path, dpi: int = PERIODICITY_DPI) -> list[str]:
    """One hash per page, from a low-DPI raster."""
    with tempfile.TemporaryDirectory() as tmp:
        prefix = Path(tmp) / "p"
        try:
            subprocess.run(
                ["pdftoppm", "-r", str(dpi), "-png", str(pdf), str(prefix)],
                capture_output=True,
                check=True,
            )
        except FileNotFoundError:
            sys.exit("pdftoppm not found — install poppler-utils.")
        except subprocess.CalledProcessError as error:
            sys.exit(f"pdftoppm failed on {pdf}: {error}")
        return [
            hashlib.sha256(page.read_bytes()).hexdigest()
            for page in sorted(Path(tmp).glob("p*.png"))
        ]


def detect_periodicity(hashes: list[str]) -> int | None:
    """The smallest cycle length the page sequence repeats on, if it repeats.

    Only an *exact* whole-sequence repeat counts. A deck that happens to reuse
    one layout is not periodic; a workbook exported once per sheet is.

    Known limit: a document whose pages are *pixel-identical* in a strict
    alternation — [A, B, A, B, ...] with A and B each byte-identical to their
    counterparts — would be flagged. That needs pages carrying identical ink at
    the render DPI, which differing text defeats, so it is a pathological case
    rather than an ordinary repeated template. Treat a flag as "inspect this
    GT", not as proof of corruption.
    """
    count = len(hashes)
    if count < 4:
        return None  # too short for a repeat to mean anything
    for period in range(1, count // 2 + 1):
        if count % period:
            continue
        if all(hashes[i] == hashes[i % period] for i in range(count)):
            return period
    return None


# `state` is optional on `<sheet>`; ECMA-376's ST_SheetState defaults it to
# `visible`. `veryHidden` differs from `hidden` only in keeping the sheet out
# of Excel's unhide dialog — neither ever reaches paper (issue #1065).
HIDDEN_SHEET_STATES = frozenset({"hidden", "veryhidden"})

# `<sheets>`, `<sheetPr>` and `<sheetView>` share the prefix, so the word
# boundary is what makes this select sheet declarations alone. The optional
# prefix group covers producers that write the main namespace explicitly.
SHEET_ELEMENT = re.compile(r"<(?:[\w.-]+:)?sheet\b[^>]*>")
SHEET_STATE_ATTRIBUTE = re.compile(r'\bstate\s*=\s*"([^"]*)"')


def sheet_states(workbook_xml: str) -> list[str]:
    """The visibility state of every sheet the workbook declares, in order."""
    states: list[str] = []
    for element in SHEET_ELEMENT.findall(workbook_xml):
        state = SHEET_STATE_ATTRIBUTE.search(element)
        states.append(state.group(1).strip().lower() if state else "visible")
    return states


def printable_sheet_count(workbook_xml: str) -> int:
    """Sheets Excel would put on paper: every declared sheet bar the hidden ones.

    Counting the hidden ones made this gate reject correct exports: a native
    export of a workbook with a hidden lookup sheet — an ordinary template
    shape — legitimately has fewer pages than the workbook declares sheets, and
    the gate called that INVALID (issue #1211).
    """
    return sum(
        1 for state in sheet_states(workbook_xml) if state not in HIDDEN_SHEET_STATES
    )


def hidden_sheet_count(workbook_xml: str) -> int:
    """Sheets the workbook hides, reported so a low page count explains itself."""
    return sum(
        1 for state in sheet_states(workbook_xml) if state in HIDDEN_SHEET_STATES
    )


def read_workbook_xml(source: Path) -> str | None:
    """`xl/workbook.xml` from an XLSX, or None for other or unreadable formats."""
    if source.suffix.lower() != ".xlsx":
        return None
    try:
        with zipfile.ZipFile(source) as archive:
            return archive.read("xl/workbook.xml").decode("utf-8", "replace")
    except (OSError, KeyError, zipfile.BadZipFile):
        return None


def declared_fonts(source: Path) -> set[str]:
    """Font families the source asks for, across the OOXML formats."""
    families: set[str] = set()
    try:
        archive = zipfile.ZipFile(source)
    except (OSError, zipfile.BadZipFile):
        return families
    with archive:
        for name in archive.namelist():
            if not name.endswith(".xml"):
                continue
            try:
                text = archive.read(name).decode("utf-8", "replace")
            except OSError:
                continue
            # XLSX `<name val="Calibri"/>`, DOCX `w:ascii`, DrawingML `typeface`.
            families.update(re.findall(r'<[a-z]*:?name val="([^"]+)"', text))
            families.update(re.findall(r'w:ascii="([^"]+)"', text))
            families.update(re.findall(r'typeface="([^"]+)"', text))
    return {f for f in families if f and not f.startswith("+")}


def embedded_fonts(pdf: Path) -> set[str]:
    """Font families the GT actually embeds, with subset prefixes stripped.

    Read from ``pdffonts`` rather than ``mutool info -F``: the latter prints
    document metadata alongside the font list, and scraping names out of that
    picks up words from the Producer and Author strings.
    """
    try:
        result = subprocess.run(
            ["pdffonts", str(pdf)], capture_output=True, check=False
        )
    except FileNotFoundError:
        return set()
    families: set[str] = set()
    for line in result.stdout.decode("utf-8", "replace").splitlines()[2:]:
        name = line.split(" ", 1)[0].strip()
        if not name:
            continue
        # `BAAAAA+LiberationSerif` -> `LiberationSerif`
        families.add(name.split("+", 1)[-1])
    return families


def check(gt: Path, source: Path | None) -> dict:
    hashes = page_hashes(gt)
    period = detect_periodicity(hashes)
    result: dict = {
        "pages": len(hashes),
        "periodicity": period,
        "printable_worksheets": None,
        "hidden_worksheets": None,
        "font_substitutions": [],
        "invalid": False,
    }
    if period is not None:
        result["invalid"] = True

    if source is not None:
        workbook_xml = read_workbook_xml(source)
        if workbook_xml is not None:
            sheets = printable_sheet_count(workbook_xml)
            result["printable_worksheets"] = sheets
            result["hidden_worksheets"] = hidden_sheet_count(workbook_xml)
            if sheets:
                # Fewer pages than sheets cannot be a complete export.
                if len(hashes) < sheets:
                    result["invalid"] = True
                # An exact multiple is the duplication signature again.
                elif (
                    sheets > 1
                    and len(hashes) % sheets == 0
                    and len(hashes) // sheets > 1
                ):
                    result["suspicious_multiple"] = len(hashes) // sheets
        declared = declared_fonts(source)
        embedded = embedded_fonts(gt)
        if declared and embedded:
            missing = sorted(d for d in declared if not any(d.replace(" ", "") in e for e in embedded))
            result["font_substitutions"] = missing
    return result


def render_report(result: dict) -> str:
    lines = ["## GT integrity", "", f"Pages: {result['pages']}"]
    if result["printable_worksheets"] is not None:
        lines.append(f"Printable worksheets in source: {result['printable_worksheets']}")
    if result.get("hidden_worksheets"):
        lines.append(
            f"Hidden worksheets, which Excel never prints: {result['hidden_worksheets']}"
        )
    lines.append("")

    if result["periodicity"] is not None:
        lines += [
            f"**INVALID — page sequence repeats on a cycle of {result['periodicity']}.**",
            "",
            "Every page is a duplicate of one in the first cycle, which is the",
            "signature of a per-sheet export that contained the whole workbook",
            "(issue #616). Do not derive any metric from this GT.",
        ]
    elif result["invalid"]:
        lines += [
            "**INVALID — fewer pages than the source has printable worksheets.**",
            "",
            "The export cannot be complete, so page-indexed comparisons will be",
            "misaligned. Re-export before measuring.",
        ]
    else:
        lines.append("No structural corruption detected.")

    if result.get("suspicious_multiple"):
        lines += [
            "",
            f"Note: page count is exactly {result['suspicious_multiple']}x the "
            "worksheet count. Not conclusive on its own — check whether sheets "
            "genuinely span that many pages.",
        ]

    if result["font_substitutions"]:
        lines += [
            "",
            "## GT-side font substitutions",
            "",
            "These families are declared by the source but absent from the GT's",
            "embedded fonts. They are artefacts of the exporting application, not",
            "converter defects — do not file them as such.",
            "",
        ]
        lines += [f"- {name}" for name in result["font_substitutions"]]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gt", type=Path)
    parser.add_argument("--source", type=Path, default=None)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    result = check(args.gt, args.source)
    print(json.dumps(result, indent=2) if args.json else render_report(result))
    return 1 if result["invalid"] else 0


if __name__ == "__main__":
    sys.exit(main())

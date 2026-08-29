#!/usr/bin/env python3
"""Sweep the row height native Excel recomputes for a dimension-less row (#1150).

Excel ignores a sheet's declared ``defaultRowHeight`` for a row that records no
``ht`` unless the sheet marks it ``customHeight``; it lays such a row out from
the workbook's Normal font instead (#1047). The recomputed height belongs to the
*face*, not to a size curve shared across faces, so modelling one family needs a
size sweep of its own against a native export.

This drives that sweep. It patches ``xl/styles.xml``'s first ``<font>`` — the
Normal font — to one ``family``/``size`` pair per variant, touching nothing
else, and reads ``standard height of worksheet 1`` back through AppleScript.
Reading the worksheet property rather than measuring a PDF keeps a variant to
one workbook open instead of one PDF export, which is what makes a full
family x size matrix practical.

A no-patch re-zip control is a hard gate, as in ``probe_harness.py``: if
repackaging alone moves the reading, the sweep measures the repackaging.

The Excel app is sandboxed, so the packages are staged under
``~/Library/Containers/com.microsoft.Excel/Data/probes/`` — anywhere else costs
a per-file "Grant Access" dialog that stalls an unattended run, and an external
volume hangs outright with AppleEvent timeout -1712 (#1051). The report is
copied back out to ``target/probes/`` when the run finishes.

Usage:
    measure_excel_row_height.py --families Arial "Times New Roman"
                                [--sizes 8 9 10 11 12] [--base FIXTURE.xlsx]
                                [--stage-root DIR] [--report-dir DIR]
                                [--name SWEEP]
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

from probe_harness import ProbeError, build_package, ensure_internal_disk

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parent
READ_ROW_HEIGHTS = SCRIPTS_DIR / "macos" / "read_excel_row_heights.applescript"
EXCEL_CONTAINER_STAGE = (
    Path.home() / "Library" / "Containers" / "com.microsoft.Excel" / "Data" / "probes"
)
RESULTS_ROOT = REPO_ROOT / "target" / "probes"
STYLES_PART = "xl/styles.xml"

# The fixture the #1102 and #1150 sweeps both ran on: its Normal font names a
# face outright (no `<scheme>`), and its sheet declares a `defaultRowHeight`
# that disagrees with every recompute, so a reading equal to the declaration
# would mean the patch never reached Excel.
DEFAULT_BASE = REPO_ROOT / "tests" / "fixtures" / "xlsx" / "issue_1066_blip_effect_picture.xlsx"
DEFAULT_SIZES = (
    8.0,
    9.0,
    10.0,
    11.0,
    12.0,
    13.0,
    14.0,
    15.0,
    16.0,
    17.0,
    18.0,
    20.0,
    22.0,
    24.0,
)

NORMAL_FONT_PATTERN = re.compile(r"(<fonts\b[^>]*>)<font>.*?</font>", re.DOTALL)


def patched_styles(styles_xml: str, family: str, size_pt: float) -> str:
    """Replace the Normal font — ``fonts[0]`` — with ``family`` at ``size_pt``."""
    normal_font = f'<font><sz val="{format_number(size_pt)}"/><name val="{family}"/></font>'
    patched, count = NORMAL_FONT_PATTERN.subn(
        lambda match: match.group(1) + normal_font, styles_xml, count=1
    )
    if count != 1:
        raise ProbeError("no <fonts><font> element to patch in " + STYLES_PART)
    return patched


def format_number(value: float) -> str:
    return f"{value:g}"


def variant_file_name(family_index: int, family: str, size_pt: float) -> str:
    """A stage file name unique per (family, size).

    The index carries the uniqueness rather than the slug: two families whose
    names share no ASCII at all — every Korean family here — slug to the same
    string, and the collision silently overwrites one variant's package with
    the other's, reporting one family's column for both.
    """
    slug = "".join(
        character if character.isascii() and character.isalnum() else "-"
        for character in family
    ).strip("-").lower()
    return f"{family_index:02d}-{slug}-{format_number(size_pt)}.xlsx"


def read_heights(stage: Path, file_names: list[str]) -> dict[str, float]:
    completed = subprocess.run(
        ["osascript", str(READ_ROW_HEIGHTS), str(stage), *file_names],
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise ProbeError(f"reading row heights failed:\n{completed.stderr.strip()}")
    heights: dict[str, float] = {}
    for line in completed.stdout.splitlines():
        if not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) < 2:
            raise ProbeError(f"unparsable reading: {line!r}")
        heights[fields[0]] = float(fields[1])
    missing = sorted(set(file_names) - set(heights))
    if missing:
        raise ProbeError(f"no reading came back for: {', '.join(missing)}")
    return heights


def render_report(
    base: Path, families: list[str], sizes: list[float], heights: dict[str, float]
) -> str:
    lines = [
        "# Excel recomputed standard row height by Normal font",
        "",
        f"- base: {base.name}",
        "- patch: `xl/styles.xml` `fonts[0]`, one family/size pair per variant",
        "- reading: `standard height of worksheet 1` (points)",
        "",
        "| family | " + " | ".join(format_number(size) for size in sizes) + " |",
        "| --- | " + " | ".join("---:" for _ in sizes) + " |",
    ]
    for family_index, family in enumerate(families):
        cells = [
            format_number(heights[variant_file_name(family_index, family, size)])
            for size in sizes
        ]
        lines.append(f"| {family} | " + " | ".join(cells) + " |")
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--families", nargs="+", required=True)
    parser.add_argument("--sizes", nargs="+", type=float, default=list(DEFAULT_SIZES))
    parser.add_argument("--base", type=Path, default=DEFAULT_BASE)
    parser.add_argument("--stage-root", type=Path, default=EXCEL_CONTAINER_STAGE)
    parser.add_argument("--report-dir", type=Path, default=RESULTS_ROOT)
    parser.add_argument("--name", default="row-height-by-face")
    arguments = parser.parse_args(argv)

    try:
        ensure_internal_disk(arguments.stage_root)
        base: Path = arguments.base.resolve()
        if not base.is_file():
            raise ProbeError(f"base fixture not found: {base}")

        stage: Path = arguments.stage_root / arguments.name
        if stage.exists():
            shutil.rmtree(stage)
        stage.mkdir(parents=True)

        import zipfile

        with zipfile.ZipFile(base) as archive:
            styles_xml = archive.read(STYLES_PART).decode("utf-8")

        # The control repackages the base with no patch at all: it isolates
        # repackaging from the factor being swept.
        control_name = "control.xlsx"
        build_package(base, stage / control_name, {})
        shutil.copyfile(base, stage / "base.xlsx")

        file_names = ["base.xlsx", control_name]
        for family_index, family in enumerate(arguments.families):
            for size in arguments.sizes:
                name = variant_file_name(family_index, family, size)
                build_package(
                    base,
                    stage / name,
                    {STYLES_PART: patched_styles(styles_xml, family, size).encode("utf-8")},
                )
                file_names.append(name)

        heights = read_heights(stage, file_names)
        if heights[control_name] != heights["base.xlsx"]:
            raise ProbeError(
                "control gate: the re-zipped base reads "
                f"{heights[control_name]}pt against the base's {heights['base.xlsx']}pt — "
                "repackaging moved the reading, so the sweep measures repackaging"
            )

        report = render_report(base, arguments.families, arguments.sizes, heights)
        arguments.report_dir.mkdir(parents=True, exist_ok=True)
        report_path = arguments.report_dir / f"{arguments.name}.md"
        report_path.write_text(report, encoding="utf-8")
        print(report)
        print(f"report: {report_path}")
    except ProbeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

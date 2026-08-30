#!/usr/bin/env python3
"""Semi-automated one-factor native-export probe harness (#698).

Takes a base OOXML fixture plus a list of one-factor XML patches, generates a
variant package per patch alongside a re-zip control (repackaged with no patch
at all), batch-exports everything through a chosen backend, and runs the layout
differ (``compare_layout.py``) over the results into one table per probe: one
row per variant, keyed by the varied factor's value.

The control is a hard gate, not an option: a variant differing from the control
by anything other than the patched factor invalidates the whole series, and
repackaging is the step most likely to introduce that. The harness aborts when
the control's export deviates from the base's.

Backends:

- ``office2pdf`` (default) — our own converter. Answers "what does our code do
  with this attribute", runs anywhere in seconds.
- ``soffice`` — headless LibreOffice, the only reference available without a
  Mac.
- ``office`` — native Office via ``scripts/macos/export_*_pdfs.applescript``
  (picked by the base fixture's extension). Answers "what does Office do".

  The Office apps are sandboxed, which constrains the stage twice over. A file
  outside the driven app's own container needs per-file consent, and the
  "Grant Access" dialog stalls an unattended run, so this backend stages under
  ``~/Library/Containers/<bundle id>/Data/probes/`` — ``com.microsoft.Word``,
  ``com.microsoft.Powerpoint`` or ``com.microsoft.Excel``, matched to the
  format, since another app's container prompts just the same — and copies the
  report and the PDFs back out to ``target/probes/`` when the run finishes
  (#1051). And the sandbox cannot write to external volumes at all (a save
  under ``/Volumes`` hangs with AppleEvent timeout -1712), so an explicit
  ``--stage-root`` must still sit on the internal disk.

A differ failure always propagates: a probe row built from a differ that
silently reported nothing would show the variant as identical to the control,
which is exactly the answer a one-factor probe is looking for and exactly the
wrong one (#810).

Probe spec (JSON; paths resolve relative to the spec file)::

    {
      "name": "spcAft-linearity",
      "base": "base.pptx",
      "part": "ppt/slides/slide1.xml",
      "factor": "a:spcAft val",
      "page": 1,            // optional, forwarded to the differ
      "noise_floor": 0.12,  // optional, forwarded to the differ
      "variants": [
        {"value": "500", "find": "val=\\"1000\\"", "replace": "val=\\"500\\""},
        {"value": "ctr", "find": "anchor=\\"[a-z]+\\"", "replace": "anchor=\\"ctr\\"",
         "regex": true, "count": 1}
      ]
    }

Usage:
    probe_harness.py SPEC.json [SPEC.json ...] [--backend office2pdf|soffice|office]
                     [--converter PATH] [--stage-root DIR]
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import zipfile
from dataclasses import dataclass, field
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPTS_DIR.parent
COMPARE_LAYOUT = SCRIPTS_DIR / "compare_layout.py"
APPLESCRIPT_BY_EXTENSION = {
    ".docx": SCRIPTS_DIR / "macos" / "export_word_pdfs.applescript",
    ".pptx": SCRIPTS_DIR / "macos" / "export_powerpoint_pdfs.applescript",
    ".xlsx": SCRIPTS_DIR / "macos" / "export_excel_pdfs.applescript",
}
OFFICE_BUNDLE_ID_BY_EXTENSION = {
    ".docx": "com.microsoft.Word",
    ".pptx": "com.microsoft.Powerpoint",
    ".xlsx": "com.microsoft.Excel",
}
CONTAINERS_ROOT = Path.home() / "Library" / "Containers"
RESULTS_ROOT = REPO_ROOT / "target" / "probes"

SPEC_KEYS = {"name", "base", "part", "factor", "page", "noise_floor", "variants"}
SPEC_REQUIRED_KEYS = ("name", "base", "part", "variants")
VARIANT_KEYS = {"value", "find", "replace", "regex", "count"}
VARIANT_REQUIRED_KEYS = ("value", "find", "replace")


class ProbeError(Exception):
    """A condition that invalidates the probe; the run must stop, not report."""


@dataclass(frozen=True)
class Variant:
    value: str
    find: str
    replace: str
    regex: bool = False
    count: int | None = None


@dataclass(frozen=True)
class Spec:
    name: str
    base: Path
    part: str
    factor: str
    variants: list[Variant] = field(default_factory=list)
    page: int | None = None
    noise_floor: float | None = None


def run_command(argv: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True)


def load_spec(path: Path) -> Spec:
    try:
        raw = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ProbeError(f"cannot read probe spec {path}: {error}") from error
    if not isinstance(raw, dict):
        raise ProbeError(f"probe spec {path} must be a JSON object")
    for key in SPEC_REQUIRED_KEYS:
        if key not in raw:
            raise ProbeError(f"probe spec {path} is missing required key '{key}'")
    unknown = sorted(set(raw) - SPEC_KEYS)
    if unknown:
        raise ProbeError(f"probe spec {path} has unknown key(s): {', '.join(unknown)}")

    variants: list[Variant] = []
    for index, entry in enumerate(raw["variants"]):
        if not isinstance(entry, dict):
            raise ProbeError(f"variant #{index + 1} must be a JSON object")
        for key in VARIANT_REQUIRED_KEYS:
            if key not in entry:
                raise ProbeError(f"variant #{index + 1} is missing required key '{key}'")
        unknown = sorted(set(entry) - VARIANT_KEYS)
        if unknown:
            raise ProbeError(f"variant #{index + 1} has unknown key(s): {', '.join(unknown)}")
        variants.append(
            Variant(
                value=str(entry["value"]),
                find=entry["find"],
                replace=entry["replace"],
                regex=bool(entry.get("regex", False)),
                count=entry["count"] if entry.get("count") is not None else None,
            )
        )
    if not variants:
        raise ProbeError(f"probe spec {path} has no variants — nothing to probe")
    values = [variant.value for variant in variants]
    duplicates = sorted({value for value in values if values.count(value) > 1})
    if duplicates:
        raise ProbeError(f"duplicate variant value(s): {', '.join(duplicates)}")

    return Spec(
        name=str(raw["name"]),
        base=(Path(path).resolve().parent / raw["base"]).resolve(),
        part=raw["part"],
        factor=str(raw.get("factor", "variant")),
        variants=variants,
        page=int(raw["page"]) if raw.get("page") is not None else None,
        noise_floor=float(raw["noise_floor"]) if raw.get("noise_floor") is not None else None,
    )


def apply_patch(text: str, variant: Variant) -> tuple[str, int]:
    """Apply one variant's patch, returning the patched text and match count.

    Zero matches is an error: a no-op patch would export a variant identical
    to the control, and the table would read "this factor does nothing".
    """
    if variant.regex:
        patched, count = re.subn(variant.find, variant.replace, text)
    else:
        count = text.count(variant.find)
        patched = text.replace(variant.find, variant.replace)
    if count == 0:
        raise ProbeError(f"variant '{variant.value}': patch matched nothing in the part")
    if variant.count is not None and count != variant.count:
        raise ProbeError(
            f"variant '{variant.value}': expected {variant.count} match(es), found {count}"
        )
    return patched, count


def suspicious_members(names: list[str]) -> list[str]:
    """AppleDouble / Finder members that mark a fixture repackaged by macOS tools.

    ``_rels/.rels`` is legitimate OOXML, so only the macOS artifacts are
    flagged, not every dot-named member.
    """
    flagged = []
    for name in names:
        basename = name.rsplit("/", 1)[-1]
        if basename.startswith("._") or basename == ".DS_Store":
            flagged.append(name)
    return flagged


def build_package(base: Path, out: Path, replacements: dict[str, bytes]) -> None:
    """Write a fresh archive with the base's members, applying ``replacements``.

    Always opens the target in write mode: ``zip -r`` *updates* an existing
    archive, so a second variant silently inherits the first's members — the
    harness must never do that. Member order, timestamps, and attributes are
    copied from the base so rebuilding the same package is byte-deterministic,
    which is what lets the control gate byte-compare exported PDFs.
    """
    with zipfile.ZipFile(base) as source:
        missing = sorted(set(replacements) - set(source.namelist()))
        if missing:
            raise ProbeError(f"part(s) not present in {base.name}: {', '.join(missing)}")
        with zipfile.ZipFile(out, "w") as target:
            for info in source.infolist():
                member = zipfile.ZipInfo(info.filename, date_time=info.date_time)
                member.compress_type = zipfile.ZIP_DEFLATED
                member.external_attr = info.external_attr
                member.create_system = 3
                data = replacements.get(info.filename, source.read(info.filename))
                target.writestr(member, data)


def ensure_internal_disk(path: Path) -> None:
    parts = Path(path).parts
    if len(parts) >= 2 and parts[0] == "/" and parts[1] == "Volumes":
        raise ProbeError(
            f"{path} sits on an external volume; the Office sandbox cannot write there "
            "(AppleEvent timeout -1712) — stage on the internal disk"
        )


def default_stage_root(backend: str, extension: str) -> Path:
    """Where a run stages packages and PDFs when ``--stage-root`` is not given.

    The Office apps are sandboxed: opening a package from, or saving a PDF to,
    anywhere outside the app's own container raises a per-file "Grant Access"
    dialog, and an unattended run stalls on the first one (#1051). Inside the
    container nothing is asked, so the office backend stages there — in the
    container of the app this fixture's format drives, because reaching into
    another app's container prompts just the same.
    """
    if backend != "office":
        return RESULTS_ROOT
    bundle_id = OFFICE_BUNDLE_ID_BY_EXTENSION.get(extension.lower())
    if bundle_id is None:
        raise ProbeError(f"no Office app container for '{extension.lstrip('.')}' fixtures")
    return CONTAINERS_ROOT / bundle_id / "Data" / "probes"


def mirror_results(stage: Path, destination: Path) -> Path:
    """Copy a container-staged run's report and PDFs out; return the report path.

    The variant packages stay behind: they rebuild deterministically from the
    spec, while the report and the exported PDFs are what the run produced and
    what the caller came for.
    """
    (destination / "pdfs").mkdir(parents=True, exist_ok=True)
    for pdf in sorted((stage / "pdfs").glob("*.pdf")):
        shutil.copyfile(pdf, destination / "pdfs" / pdf.name)
    report = destination / "report.md"
    shutil.copyfile(stage / "report.md", report)
    return report


def applescript_for(extension: str) -> Path:
    script = APPLESCRIPT_BY_EXTENSION.get(extension.lower())
    if script is None:
        raise ProbeError(f"no native export AppleScript for '{extension.lstrip('.')}' fixtures")
    return script


def _check_export(argv: list[str], result: subprocess.CompletedProcess) -> None:
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "").strip()
        raise ProbeError(f"export command failed ({argv[0]}): {detail}")


def _unite_excel_sheets(job_id: str, pdf_dir: Path, runner) -> None:
    sheets = sorted(pdf_dir.glob(f"{job_id}-sheet-*.pdf"))
    united = pdf_dir / f"{job_id}.pdf"
    if not sheets:
        raise ProbeError(f"no Excel sheet PDFs found for {job_id}")
    if len(sheets) == 1:
        shutil.copyfile(sheets[0], united)
        return
    argv = ["pdfunite", *[str(sheet) for sheet in sheets], str(united)]
    _check_export(argv, runner(argv))


def export_all(
    backend: str,
    jobs: list[tuple[str, Path]],
    pdf_dir: Path,
    converter: str = "office2pdf",
    runner=run_command,
) -> None:
    """Export every (job_id, package) to ``pdf_dir/<job_id>.pdf`` via ``backend``."""
    if backend == "office2pdf":
        for job_id, package in jobs:
            argv = [converter, str(package), "-o", str(pdf_dir / f"{job_id}.pdf")]
            _check_export(argv, runner(argv))
    elif backend == "soffice":
        for _, package in jobs:
            argv = [
                "soffice",
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                str(pdf_dir),
                str(package),
            ]
            _check_export(argv, runner(argv))
    elif backend == "office":
        ensure_internal_disk(pdf_dir)
        for _, package in jobs:
            ensure_internal_disk(package)
        extensions = {package.suffix.lower() for _, package in jobs}
        if len(extensions) != 1:
            raise ProbeError(f"mixed package extensions in one export batch: {sorted(extensions)}")
        extension = next(iter(extensions))
        argv = ["osascript", str(applescript_for(extension)), str(pdf_dir)]
        for job_id, package in jobs:
            argv += [job_id, str(package)]
        _check_export(argv, runner(argv))
        if extension == ".xlsx":
            for job_id, _ in jobs:
                _unite_excel_sheets(job_id, pdf_dir, runner)
    else:
        raise ProbeError(f"unknown backend '{backend}'")

    for job_id, _ in jobs:
        pdf = pdf_dir / f"{job_id}.pdf"
        if not pdf.is_file() or pdf.stat().st_size == 0:
            raise ProbeError(f"exporter produced no PDF for {job_id} ({pdf})")


def run_differ(
    gt: Path,
    output: Path,
    page: int | None = None,
    noise_floor: float | None = None,
    runner=run_command,
) -> dict:
    """Run compare_layout.py --json and return its report.

    A nonzero exit always propagates. Before #810 a zero-page trace printed an
    empty report with exit status 0, which reads as "no differences" — the
    exact answer a one-factor probe is looking for, and exactly the wrong one.
    """
    argv = [sys.executable, str(COMPARE_LAYOUT), str(gt), str(output), "--json"]
    if page is not None:
        argv += ["--page", str(page)]
    if noise_floor is not None:
        argv += ["--noise-floor", str(noise_floor)]
    result = runner(argv)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "").strip()
        raise ProbeError(f"layout differ failed on {output.name}: {detail}")
    try:
        return json.loads(result.stdout)
    except ValueError as error:
        raise ProbeError(f"layout differ emitted no JSON for {output.name}: {error}") from error


def layout_identical_failures(report: dict) -> list[str]:
    """Reasons the report is not layout-identical; empty means identical."""
    failures: list[str] = []
    if report["gt_pages"] != report["out_pages"]:
        failures.append(f"page count {report['gt_pages']} vs {report['out_pages']}")
    for index, vector in enumerate(report["pages"], start=1):
        lines = vector["lines"]
        for key in ("missing", "extra", "deviant"):
            if lines[key]:
                failures.append(f"page {index}: {lines[key]} {key} line(s)")
        if vector["wraps"]["count"]:
            failures.append(f"page {index}: {vector['wraps']['count']} re-wrapped paragraph(s)")
        if vector["reflow"]["gt_lines"]:
            failures.append(f"page {index}: {vector['reflow']['gt_lines']} re-grouped line(s)")
        if vector["instances"]["large_shift_count"]:
            failures.append(
                f"page {index}: {vector['instances']['large_shift_count']} large shift(s)"
            )
        visibility = vector.get("visibility", {"mismatch_count": 0})
        if visibility["mismatch_count"]:
            failures.append(
                f"page {index}: {visibility['mismatch_count']} visibility mismatch(es)"
            )
        rects = vector["rects"]
        if rects["gt_count"] != rects["out_count"]:
            failures.append(f"page {index}: rects {rects['gt_count']} vs {rects['out_count']}")
    return failures


def verify_control(
    base_pdf: Path,
    control_pdf: Path,
    page: int | None = None,
    noise_floor: float | None = None,
    runner=run_command,
) -> str:
    """Gate the run on the re-zip control; return the verdict for the report.

    Byte equality is the strongest verdict; native Office stamps timestamps
    into every export, so a clean differ report is the fallback there.
    """
    if base_pdf.read_bytes() == control_pdf.read_bytes():
        return "byte-identical re-zip"
    report = run_differ(base_pdf, control_pdf, page=page, noise_floor=noise_floor, runner=runner)
    failures = layout_identical_failures(report)
    if failures:
        raise ProbeError(
            "re-zip control deviates from the base export — repackaging changed the "
            "output and no variant row can be trusted: " + "; ".join(failures)
        )
    return "layout-identical re-zip (bytes differ)"


def variant_row(value: str, patched: int, report: dict) -> dict:
    """Collapse a differ report into one table row for this variant."""
    vectors = report["pages"]
    matched = sum(vector["lines"]["matched"] for vector in vectors)
    weighted_dy = sum(
        vector["baseline"]["mean_abs_dy"] * vector["lines"]["matched"] for vector in vectors
    )
    worst_dy = max(
        (vector["baseline"]["worst_dy_signed"] for vector in vectors), key=abs, default=0.0
    )
    return {
        "value": value,
        "patched": patched,
        "pages": f"{report['gt_pages']}/{report['out_pages']}",
        "matched": matched,
        "missing": sum(vector["lines"]["missing"] for vector in vectors),
        "extra": sum(vector["lines"]["extra"] for vector in vectors),
        "wraps": sum(vector["wraps"]["count"] for vector in vectors),
        "deviant": sum(vector["lines"]["deviant"] for vector in vectors),
        "mean_abs_dy": weighted_dy / matched if matched else 0.0,
        "worst_dy": worst_dy,
        "worst_pitch": max((vector["pitch"]["worst_delta"] for vector in vectors), default=0.0),
        "worst_width": max((vector["width"]["worst_pct"] for vector in vectors), default=0.0),
        "large_shifts": sum(vector["instances"]["large_shift_count"] for vector in vectors),
        "visibility": sum(
            vector.get("visibility", {"mismatch_count": 0})["mismatch_count"]
            for vector in vectors
        ),
        "rects": "{}/{}".format(
            sum(vector["rects"]["gt_count"] for vector in vectors),
            sum(vector["rects"]["out_count"] for vector in vectors),
        ),
    }


def render_table(spec: Spec, backend: str, control_verdict: str, rows: list[dict]) -> str:
    """One markdown table per probe: one row per variant, keyed by the factor."""
    columns = (
        "patched",
        "pages",
        "matched",
        "missing",
        "extra",
        "wraps",
        "deviant",
        "mean abs dy (pt)",
        "worst dy (pt)",
        "worst pitch (pt)",
        "worst width (%)",
        "large shifts",
        "visibility",
        "rects",
    )
    lines = [
        f"# Probe: {spec.name}",
        "",
        f"- base: {spec.base.name}",
        f"- part: {spec.part}",
        f"- backend: {backend}",
        f"- control: {control_verdict}",
        "",
        "| " + " | ".join((spec.factor, *columns)) + " |",
        "| " + " | ".join("---" for _ in range(len(columns) + 1)) + " |",
    ]
    for row in rows:
        cells = (
            row["value"],
            str(row["patched"]),
            row["pages"],
            str(row["matched"]),
            str(row["missing"]),
            str(row["extra"]),
            str(row["wraps"]),
            str(row["deviant"]),
            f"{row['mean_abs_dy']:.2f}",
            f"{row['worst_dy']:+.2f}",
            f"{row['worst_pitch']:.2f}",
            f"{row['worst_width']:.1f}",
            str(row["large_shifts"]),
            str(row["visibility"]),
            row["rects"],
        )
        lines.append("| " + " | ".join(cells) + " |")
    lines.append("")
    return "\n".join(lines)


def _slug(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", value)


def run_probe(
    spec_path: Path,
    backend: str = "office2pdf",
    converter: str = "office2pdf",
    stage_root: Path | None = None,
    runner=run_command,
) -> Path:
    """Run one probe spec end to end; return the written report path."""
    spec = load_spec(spec_path)
    if not spec.base.is_file():
        raise ProbeError(f"base fixture not found: {spec.base}")
    extension = spec.base.suffix.lower()

    staged_in_container = stage_root is None and backend == "office"
    stage = (stage_root or default_stage_root(backend, extension)) / spec.name
    if backend == "office":
        ensure_internal_disk(stage)
    packages_dir = stage / "packages"
    pdf_dir = stage / "pdfs"
    packages_dir.mkdir(parents=True, exist_ok=True)
    pdf_dir.mkdir(parents=True, exist_ok=True)
    if staged_in_container:
        print(f"[{spec.name}] staging inside the app sandbox container: {stage}", file=sys.stderr)

    with zipfile.ZipFile(spec.base) as archive:
        names = archive.namelist()
        if spec.part not in names:
            raise ProbeError(f"part '{spec.part}' not present in {spec.base.name}")
        part_text = archive.read(spec.part).decode("utf-8")
    flagged = suspicious_members(names)
    if flagged:
        print(
            f"[{spec.name}] warning: base carries macOS artifact member(s) "
            f"({', '.join(flagged)}); they are preserved, but the fixture was likely "
            "repackaged by Finder/tar and is worth rebuilding",
            file=sys.stderr,
        )

    # Patch every variant before exporting anything, so a bad patch aborts
    # the run without a half-finished export batch behind it.
    patched: list[tuple[Variant, int, Path]] = []
    for variant in spec.variants:
        text, count = apply_patch(part_text, variant)
        package = packages_dir / f"variant-{_slug(variant.value)}{extension}"
        build_package(spec.base, package, {spec.part: text.encode("utf-8")})
        patched.append((variant, count, package))

    base_package = packages_dir / f"base{extension}"
    shutil.copyfile(spec.base, base_package)
    control_package = packages_dir / f"control{extension}"
    build_package(spec.base, control_package, {})

    jobs = [("base", base_package), ("control", control_package)]
    jobs += [(package.stem, package) for _, _, package in patched]
    print(f"[{spec.name}] exporting {len(jobs)} package(s) via {backend}", file=sys.stderr)
    export_all(backend, jobs, pdf_dir, converter=converter, runner=runner)

    control_verdict = verify_control(
        pdf_dir / "base.pdf",
        pdf_dir / "control.pdf",
        page=spec.page,
        noise_floor=spec.noise_floor,
        runner=runner,
    )
    print(f"[{spec.name}] control gate: {control_verdict}", file=sys.stderr)

    rows = [
        variant_row(
            variant.value,
            count,
            run_differ(
                pdf_dir / "control.pdf",
                pdf_dir / f"{package.stem}.pdf",
                page=spec.page,
                noise_floor=spec.noise_floor,
                runner=runner,
            ),
        )
        for variant, count, package in patched
    ]

    report = render_table(spec, backend, control_verdict, rows)
    report_path = stage / "report.md"
    report_path.write_text(report, encoding="utf-8")
    sys.stdout.write(report)
    if staged_in_container:
        report_path = mirror_results(stage, RESULTS_ROOT / spec.name)
        print(f"[{spec.name}] results copied out to {report_path.parent}", file=sys.stderr)
    return report_path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("specs", nargs="+", type=Path, help="probe spec JSON file(s)")
    parser.add_argument(
        "--backend",
        choices=("office2pdf", "soffice", "office"),
        default="office2pdf",
        help="exporter: our converter (default), headless LibreOffice, or native Office",
    )
    parser.add_argument(
        "--converter",
        default="office2pdf",
        help="office2pdf binary for the office2pdf backend (default: from PATH)",
    )
    parser.add_argument(
        "--stage-root",
        type=Path,
        default=None,
        help=(
            "directory to stage packages/PDFs/reports under (default: target/probes; "
            "--backend office defaults into the driven app's sandbox container and "
            "copies the results back to target/probes)"
        ),
    )
    args = parser.parse_args(argv)

    try:
        for spec_path in args.specs:
            run_probe(
                spec_path,
                backend=args.backend,
                converter=args.converter,
                stage_root=args.stage_root,
            )
    except ProbeError as error:
        print(f"probe failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

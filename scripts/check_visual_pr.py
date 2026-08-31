#!/usr/bin/env python3
"""Fail a pull request when its visual audit or evidence is incomplete."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


VISUAL_ROOT = Path("assets/bugfixes")
EVIDENCE_PATH = re.compile(
    r"^assets/bugfixes/issue-(?P<issue>\d+)/(?P<name>gt|before|after|compare)\.(?P<ext>[^/]+)$"
)
LAYOUT_AUDIT_PATH = re.compile(
    r"^assets/bugfixes/issue-(?P<issue>\d+)/layout-audit\.json$"
)
# Prose living under assets/bugfixes/ carries no pixels, so it is neither evidence
# to validate nor a rendered change to audit. Without this the file documenting the
# evidence convention could never be edited (#539). Image suffixes stay outside the
# set so a stray screenshot is still rejected rather than silently waved through.
BOOKKEEPING_SUFFIXES = frozenset({".md", ".txt"})
AUDIT_ROWS = (
    "Page count/order",
    "Element presence",
    "Position/size",
    "Rotation/flip",
    "Fill",
    "Stroke/border",
    "Shape outline geometry",
    "Text content",
    "Font family/weight/style",
    "Text color",
    "Alignment",
    "Line/paragraph spacing",
    "Clipping/overflow",
)
INSPECTION_ITEMS = (
    "Rendered all evidence at 150 DPI or higher",
    "Stored progressive JPEG quality 86 assets with metadata stripped",
    "Used Codex/Claude vision to inspect the full GT/output pages, diff, and matched crops",
    "Inspected matched region crops at full resolution",
    "Ran compare_layout.py --audit and dispositioned every large text-instance shift and painted-visibility mismatch",
    "Ran the 5% fuzz pixel-difference sweep",
    "Inventoried hairlines and border dash styles",
    "Inventoried font weight, italic, and underline emphasis",
)
FIX_PREVIEW_LABELS = ("GT", "Before", "After")
DEFECT_PREVIEW_LABELS = ("Compare",)
ALLOWED_RESULTS = ("Matches GT", "Fixed", "No deviation observed")
VISION_WORDS = re.compile(
    r"(?i)\b(?:page|diff|crop|text|title|label|line|shape|image|chart|table|"
    r"position|align(?:ment|ed)?|offset|colour|color|fill|stroke|border|font|"
    r"spacing|clip(?:ping|ped)?|overflow|rotation|size|weight)\b"
)


@dataclass(frozen=True)
class JpegInfo:
    width: int
    height: int
    progressive: bool
    density_dpi: tuple[float, float] | None
    metadata_markers: tuple[str, ...]


def extract_section(body: str, heading: str) -> str:
    pattern = re.compile(
        rf"(?ms)^{re.escape(heading)}\s*$\n(?P<section>.*?)(?=^##\s|\Z)"
    )
    match = pattern.search(body)
    return match.group("section") if match else ""


def checked(section: str, label: str) -> bool:
    return bool(re.search(rf"(?mi)^- \[[xX]\] {re.escape(label)}\s*$", section))


def field(section: str, name: str) -> str | None:
    match = re.search(rf"(?mi)^- {re.escape(name)}:\s*(.*?)\s*$", section)
    if not match:
        return None
    value = match.group(1).strip().strip("`")
    if not value or "<!--" in value:
        return None
    return value


def audit_table(section: str) -> dict[str, str]:
    rows: dict[str, str] = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) == 2 and cells[0] in AUDIT_ROWS:
            rows[cells[0]] = cells[1]
    return rows


def rendered_preview_urls(section: str, labels: tuple[str, ...]) -> dict[str, str]:
    rendered_markup = re.sub(r"(?s)<!--.*?-->", "", section)
    rendered_markup = re.sub(r"(?s)```.*?```", "", rendered_markup)
    rendered_markup = re.sub(r"`[^`\n]*`", "", rendered_markup)
    urls: dict[str, str] = {}
    for label in labels:
        match = re.search(
            rf"!\[{re.escape(label)}\]\((?P<url>https?://[^\s)]+)\)",
            rendered_markup,
        )
        if match:
            urls[label] = match.group("url")
    return urls


def is_visual_asset(path: str) -> bool:
    """Report whether a changed path is an image under the visual evidence root."""
    if not path.startswith(f"{VISUAL_ROOT.as_posix()}/"):
        return False
    return PurePosixPath(path).suffix.lower() not in BOOKKEEPING_SUFFIXES


def validate_pr_body(body: str, changed_paths: list[str]) -> list[str]:
    errors: list[str] = []
    impact = extract_section(body, "## Visual impact")
    no_change = checked(impact, "No rendered PDF change")
    visual = checked(impact, "Rendered PDF change or visual evidence added")
    visual_assets_changed = any(is_visual_asset(path) for path in changed_paths)

    if no_change == visual:
        errors.append("Select exactly one Visual impact checkbox.")
        return errors

    if no_change:
        reason = field(impact, "Reason")
        if not reason:
            errors.append("Explain why the PR has no rendered PDF change in Visual impact > Reason.")
        if visual_assets_changed:
            errors.append("assets/bugfixes changes require 'Rendered PDF change or visual evidence added'.")
        return errors

    audit = extract_section(body, "## Visual audit")
    if not audit:
        return ["Rendered changes require a ## Visual audit section."]

    issue_value = field(audit, "Issue")
    issue_match = re.fullmatch(r"#(\d+)", issue_value or "")
    if not issue_match:
        errors.append("Visual audit > Issue must be a single issue reference such as #123.")
        issue_number = None
    else:
        issue_number = issue_match.group(1)

    for name in ("Fixture", "Page(s)"):
        if not field(audit, name):
            errors.append(f"Visual audit > {name} is required.")

    renderer = field(audit, "Renderer and DPI")
    dpi_match = re.search(r"(?i)(\d+(?:\.\d+)?)\s*DPI", renderer or "")
    if not dpi_match or float(dpi_match.group(1)) < 150:
        errors.append("Visual audit > Renderer and DPI must record at least 150 DPI.")

    mode = field(audit, "Evidence mode")
    if mode not in {"fix", "defect"}:
        errors.append("Visual audit > Evidence mode must be `fix` or `defect`.")
    elif issue_number:
        expected = (
            ("GT", "gt"),
            ("Before", "before"),
            ("After", "after"),
        ) if mode == "fix" else (("Compare", "compare"),)
        for field_name, basename in expected:
            expected_path = f"assets/bugfixes/issue-{issue_number}/{basename}.jpg"
            if field(audit, field_name) != expected_path:
                errors.append(f"Visual audit > {field_name} must be `{expected_path}`.")

    preview_labels: tuple[str, ...] = ()
    if mode == "fix":
        preview_labels = FIX_PREVIEW_LABELS
    elif mode == "defect":
        preview_labels = DEFECT_PREVIEW_LABELS
    preview_urls = rendered_preview_urls(audit, preview_labels)
    for label in preview_labels:
        if label not in preview_urls:
            errors.append(
                f"Visual audit must include a rendered preview for {label}: "
                f"![{label}](https://...)."
            )
    if len(preview_urls) == len(preview_labels) and len(set(preview_urls.values())) != len(
        preview_labels
    ):
        errors.append("Each rendered preview must reference a different evidence image.")

    follow_up_value = field(audit, "New follow-up issues found in this audit")
    if not follow_up_value:
        errors.append("Visual audit must summarize newly discovered follow-up issues or say None.")
    elif follow_up_value != "None":
        follow_up_issues = {int(number) for number in re.findall(r"#(\d+)", follow_up_value)}
        if not follow_up_issues:
            errors.append("New follow-up issues must use issue references such as #123, or None.")
        else:
            remaining_issues = remaining_issue_numbers(body)
            unclassified = follow_up_issues - remaining_issues
            if unclassified:
                references = ", ".join(f"#{number}" for number in sorted(unclassified))
                errors.append(
                    f"New follow-up issues must also classify a Remaining deviation: {references}."
                )

    vision_findings = field(audit, "Model vision findings")
    descriptive_words = re.findall(r"[A-Za-z]{3,}", vision_findings or "")
    if (
        not vision_findings
        or len(vision_findings) < 20
        or len(descriptive_words) < 4
        or not VISION_WORDS.search(vision_findings)
    ):
        errors.append(
            "Visual audit > Model vision findings must record a substantive direct "
            "inspection of the full pages, diff, and crops; numeric metrics alone do not count."
        )

    for item in INSPECTION_ITEMS:
        if not checked(audit, item):
            errors.append(f"Required visual inspection is not checked: {item}.")

    rows = audit_table(audit)
    for row in AUDIT_ROWS:
        result = rows.get(row, "")
        if not result or "<!--" in result:
            errors.append(f"Deviation audit row is incomplete: {row}.")
        elif result.startswith("Remaining:"):
            if not re.search(r"#\d+", result):
                errors.append(f"Remaining deviation must reference an issue: {row}.")
        elif not result.startswith(ALLOWED_RESULTS):
            errors.append(
                f"Deviation audit row '{row}' must start with Matches GT, Fixed, "
                "No deviation observed, or Remaining: #N."
            )

    if not visual_assets_changed:
        errors.append("Rendered visual changes require evidence under assets/bugfixes/issue-<number>/.")

    return errors


def remaining_issue_numbers(body: str) -> set[int]:
    audit = extract_section(body, "## Visual audit")
    numbers: set[int] = set()
    for result in audit_table(audit).values():
        if result.startswith("Remaining:"):
            numbers.update(int(number) for number in re.findall(r"#(\d+)", result))
    return numbers


def layout_audit_categories(report: object) -> dict[str, bool]:
    """Return the material finding categories recorded by compare_layout JSON."""

    if not isinstance(report, dict):
        raise ValueError("the top level must be an object")
    pages = report.get("pages")
    gt_pages = report.get("gt_pages")
    out_pages = report.get("out_pages")
    if (
        not isinstance(pages, list)
        or type(gt_pages) is not int
        or type(out_pages) is not int
        or gt_pages <= 0
        or out_pages <= 0
    ):
        raise ValueError("pages must be a list and page counts must be positive integers")
    if len(pages) != min(gt_pages, out_pages):
        raise ValueError("page vectors must cover every comparable page")

    has_text_flow_findings = False
    has_large_shifts = False
    for page_number, page in enumerate(pages, start=1):
        if not isinstance(page, dict):
            raise ValueError(f"page {page_number} must be an object")
        try:
            line_counts = page["lines"]
            wraps = page["wraps"]
            reflow = page["reflow"]
            instances = page["instances"]
            visibility = page.get("visibility", {"mismatch_count": 0})
            counts = (
                line_counts["missing"],
                line_counts["extra"],
                wraps["count"],
                reflow["gt_lines"],
                reflow["out_lines"],
                visibility["mismatch_count"],
                instances["large_shift_count"],
            )
        except (KeyError, TypeError) as exc:
            raise ValueError(f"page {page_number} is missing compare_layout fields") from exc
        if any(type(count) is not int or count < 0 for count in counts):
            raise ValueError(f"page {page_number} finding counts must be non-negative integers")

        has_text_flow_findings |= any(counts[:6])
        has_large_shifts |= counts[6] > 0

    return {
        "page count": gt_pages != out_pages,
        "text flow": has_text_flow_findings,
        "large shifts": has_large_shifts,
    }


def disposition_issue_numbers(value: str | None) -> set[int] | None:
    if value == "Pass":
        return set()
    if not value or not re.fullmatch(r"#\d+(?:\s*,\s*#\d+)*", value):
        return None
    return {int(number) for number in re.findall(r"#(\d+)", value)}


def decoded_pixel_delta(before: Path, after: Path) -> int:
    """Return ImageMagick's exact decoded-pixel AE count for two images."""

    if shutil.which("magick") is not None:
        command = ["magick", "compare"]
    elif shutil.which("compare") is not None:
        command = ["compare"]
    else:
        raise RuntimeError(
            "ImageMagick is required to verify text-layer-only pixel equality"
        )

    result = subprocess.run(
        [*command, "-metric", "AE", str(before), str(after), "null:"],
        check=False,
        capture_output=True,
        text=True,
    )
    metric_output = result.stderr.strip() or result.stdout.strip()
    metric_match = re.search(r"(?:^|\s)(\d+)(?:\s|$)", metric_output)
    if result.returncode not in {0, 1} or not metric_match:
        detail = metric_output or f"exit status {result.returncode}"
        raise RuntimeError(f"ImageMagick could not compare the evidence: {detail}")
    return int(metric_match.group(1))


def validate_layout_audit(body: str, changed_paths: list[str], root: Path) -> list[str]:
    """Tie machine-detected layout failures to classified open issue references."""

    impact = extract_section(body, "## Visual impact")
    if checked(impact, "No rendered PDF change"):
        return []

    audit = extract_section(body, "## Visual audit")
    if field(audit, "Evidence mode") != "fix":
        return []
    issue_match = re.fullmatch(r"#(\d+)", field(audit, "Issue") or "")
    if not issue_match:
        return []

    issue_number = issue_match.group(1)
    expected_path = f"assets/bugfixes/issue-{issue_number}/layout-audit.json"
    errors: list[str] = []
    if field(audit, "Layout audit report") != expected_path:
        errors.append(f"Visual audit > Layout audit report must be `{expected_path}`.")
    if expected_path not in changed_paths:
        errors.append(f"{expected_path}: layout audit report must be changed in this pull request.")
    expected_after = f"assets/bugfixes/issue-{issue_number}/after.jpg"
    text_layer_only = field(audit, "Text-layer-only") == "Yes"
    if text_layer_only and field(audit, "Pixel delta") != "0":
        errors.append("Visual audit > Pixel delta must be 0 for a text-layer-only fix.")
    if expected_after not in changed_paths and not text_layer_only:
        errors.append(
            f"{expected_path}: {expected_after} must be changed with the layout audit report."
        )
    if text_layer_only:
        before_path = root / f"assets/bugfixes/issue-{issue_number}/before.jpg"
        after_path = root / expected_after
        missing_evidence = False
        for evidence_path in (before_path, after_path):
            if not evidence_path.is_file():
                errors.append(
                    f"{evidence_path.relative_to(root)}: current evidence is required."
                )
                missing_evidence = True
        if not missing_evidence:
            evidence_errors = [
                *validate_jpeg(before_path),
                *validate_jpeg(after_path),
            ]
            errors.extend(evidence_errors)
            if not evidence_errors:
                try:
                    actual_delta = decoded_pixel_delta(before_path, after_path)
                except RuntimeError as exc:
                    errors.append(f"Text-layer-only evidence comparison failed: {exc}.")
                else:
                    if actual_delta != 0:
                        errors.append(
                            "Text-layer-only evidence must have zero decoded-pixel "
                            f"delta, got {actual_delta}."
                        )

    report_path = root / expected_path
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
        findings = layout_audit_categories(report)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        errors.append(f"{expected_path}: invalid layout audit report: {exc}.")
        return errors

    remaining_issues = remaining_issue_numbers(body)
    fields = {
        "page count": "Layout audit page count",
        "text flow": "Layout audit text flow",
        "large shifts": "Layout audit large shifts",
    }
    for category, has_findings in findings.items():
        field_name = fields[category]
        disposition = disposition_issue_numbers(field(audit, field_name))
        if has_findings:
            if not disposition:
                errors.append(
                    f"Visual audit > {field_name} has {category} findings and requires "
                    "one or more issue references."
                )
                continue
            unclassified = disposition - remaining_issues
            if unclassified:
                references = ", ".join(f"#{number}" for number in sorted(unclassified))
                errors.append(
                    f"Visual audit > {field_name} references must also appear in a "
                    f"Remaining deviation row: {references}."
                )
        elif disposition != set():
            errors.append(
                f"Visual audit > {field_name} has no findings and must be Pass."
            )

    return errors


def validate_open_issues(issue_numbers: set[int], repository: str, token: str | None) -> list[str]:
    if not issue_numbers:
        return []
    if not token:
        return ["GITHUB_TOKEN is required to verify remaining visual issues."]

    errors: list[str] = []
    for number in sorted(issue_numbers):
        url = f"https://api.github.com/repos/{repository}/issues/{number}"
        request = urllib.request.Request(
            url,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "User-Agent": "office2pdf-visual-pr-gate",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                issue = json.load(response)
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
            errors.append(f"Could not verify remaining visual issue #{number}: {exc}.")
            continue
        if "pull_request" in issue:
            errors.append(f"Remaining reference #{number} is a pull request, not an issue.")
        elif issue.get("state") != "open":
            errors.append(f"Remaining visual issue #{number} is not open.")
    return errors


def read_jpeg_info(path: Path) -> JpegInfo:
    data = path.read_bytes()
    if not data.startswith(b"\xff\xd8"):
        raise ValueError("not a JPEG file")

    offset = 2
    width = height = None
    progressive = False
    density: tuple[float, float] | None = None
    metadata: list[str] = []

    while offset + 1 < len(data):
        if data[offset] != 0xFF:
            offset += 1
            continue
        while offset < len(data) and data[offset] == 0xFF:
            offset += 1
        if offset >= len(data):
            break
        marker = data[offset]
        offset += 1
        if marker in {0xD8, 0xD9, 0x01} or 0xD0 <= marker <= 0xD7:
            continue
        if offset + 2 > len(data):
            raise ValueError("truncated JPEG segment")
        length = struct.unpack(">H", data[offset : offset + 2])[0]
        if length < 2 or offset + length > len(data):
            raise ValueError("invalid JPEG segment length")
        payload = data[offset + 2 : offset + length]
        offset += length

        if marker == 0xE0 and payload.startswith(b"JFIF\x00") and len(payload) >= 12:
            units = payload[7]
            x_density = struct.unpack(">H", payload[8:10])[0]
            y_density = struct.unpack(">H", payload[10:12])[0]
            if units == 1:
                density = (float(x_density), float(y_density))
            elif units == 2:
                density = (x_density * 2.54, y_density * 2.54)
        elif marker in {0xE1, 0xE2, 0xED, 0xFE}:
            metadata.append({0xE1: "APP1", 0xE2: "APP2", 0xED: "APP13", 0xFE: "COM"}[marker])
        elif marker in {0xC0, 0xC1, 0xC2} and len(payload) >= 5:
            height = struct.unpack(">H", payload[1:3])[0]
            width = struct.unpack(">H", payload[3:5])[0]
            progressive = marker == 0xC2
        elif marker == 0xDA:
            break

    if width is None or height is None:
        raise ValueError("JPEG dimensions were not found")
    return JpegInfo(width, height, progressive, density, tuple(metadata))


def validate_jpeg(path: Path) -> list[str]:
    try:
        info = read_jpeg_info(path)
    except (OSError, ValueError) as exc:
        return [f"{path}: {exc}"]

    errors: list[str] = []
    if not info.progressive:
        errors.append(f"{path}: evidence must be a progressive JPEG.")
    if not info.density_dpi or min(info.density_dpi) < 150:
        errors.append(f"{path}: JPEG density must be at least 150 DPI.")
    if info.metadata_markers:
        markers = ", ".join(info.metadata_markers)
        errors.append(f"{path}: metadata was not stripped ({markers}).")
    if path.name == "compare.jpg" and info.width % 2:
        errors.append(f"{path}: side-by-side comparison width must split into equal panels.")
    return errors


def validate_evidence(changed_paths: list[str], root: Path) -> list[str]:
    errors: list[str] = []
    touched: dict[str, set[str]] = {}

    for raw_path in changed_paths:
        if LAYOUT_AUDIT_PATH.fullmatch(raw_path):
            continue
        if not is_visual_asset(raw_path):
            continue
        match = EVIDENCE_PATH.fullmatch(raw_path)
        if not match:
            errors.append(
                f"{raw_path}: visual evidence must be gt.jpg, before.jpg, after.jpg, or compare.jpg "
                "under assets/bugfixes/issue-<number>/."
            )
            continue
        issue = match.group("issue")
        name = match.group("name")
        if match.group("ext").lower() != "jpg":
            errors.append(f"{raw_path}: visual evidence must use the .jpg extension.")
            continue
        touched.setdefault(issue, set()).add(name)

    for issue, names in touched.items():
        issue_dir = root / VISUAL_ROOT / f"issue-{issue}"
        required = {"gt", "before", "after"} if names & {"gt", "before", "after"} else set()
        for name in sorted(required | ({"compare"} if "compare" in names else set())):
            path = issue_dir / f"{name}.jpg"
            if not path.is_file():
                errors.append(f"{path}: required evidence file is missing.")
            else:
                errors.extend(validate_jpeg(path))

    return errors


def git_changed_paths(base: str, head: str, root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=ACMR", base, head],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in result.stdout.splitlines() if line]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", type=Path, required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    event = json.loads(args.event.read_text(encoding="utf-8"))
    body = event.get("pull_request", {}).get("body") or ""
    changed_paths = git_changed_paths(args.base, args.head, args.root)
    errors = validate_pr_body(body, changed_paths)
    errors.extend(validate_evidence(changed_paths, args.root))
    errors.extend(validate_layout_audit(body, changed_paths, args.root))
    errors.extend(
        validate_open_issues(
            remaining_issue_numbers(body),
            args.repository,
            os.environ.get("GITHUB_TOKEN"),
        )
    )

    if errors:
        for error in errors:
            print(f"::error::{error}")
        return 1

    print("Visual pull request contract is complete.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

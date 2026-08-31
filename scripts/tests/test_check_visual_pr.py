import io
import json
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.check_visual_pr import (
    AUDIT_ROWS,
    INSPECTION_ITEMS,
    read_jpeg_info,
    validate_evidence,
    validate_layout_audit,
    validate_open_issues,
    validate_pr_body,
)


ROOT = Path(__file__).resolve().parents[2]


def visual_body(result_overrides=None, include_previews=True):
    results = {row: "Matches GT" for row in AUDIT_ROWS}
    results.update(result_overrides or {})
    follow_ups = sorted(
        {number for result in results.values() for number in re.findall(r"#\d+", result)}
    )
    follow_up_value = ", ".join(follow_ups) if follow_ups else "None"
    inspections = "\n".join(f"- [x] {item}" for item in INSPECTION_ITEMS)
    rows = "\n".join(f"| {row} | {results[row]} |" for row in AUDIT_ROWS)
    previews = (
        """### Visual comparison

| GT | Before | After |
| --- | --- | --- |
| ![GT](https://example.com/gt.jpg) | ![Before](https://example.com/before.jpg) | ![After](https://example.com/after.jpg) |

"""
        if include_previews
        else ""
    )
    return f"""## Visual impact

- [ ] No rendered PDF change
- [x] Rendered PDF change or visual evidence added
- Reason:

## Visual audit

- Issue: #186
- Fixture: tests/fixtures/xlsx/pr_186_contributor_acceptance.xlsx
- Page(s): 1
- Renderer and DPI: pdftoppm, 150 DPI
- Evidence mode: `fix`
- Layout audit report: `assets/bugfixes/issue-186/layout-audit.json`
- Layout audit page count: Pass
- Layout audit text flow: Pass
- Layout audit large shifts: Pass
- New follow-up issues found in this audit: {follow_up_value}
- Model vision findings: Full pages, pixel diff, and matched crops were opened; no untracked visual deviation remains.
- GT: `assets/bugfixes/issue-186/gt.jpg`
- Before: `assets/bugfixes/issue-186/before.jpg`
- After: `assets/bugfixes/issue-186/after.jpg`

{previews}
### Required inspection

{inspections}

### Deviation audit

| Check | Result |
| --- | --- |
{rows}
"""


def layout_report(
    *,
    gt_pages=1,
    out_pages=1,
    missing=0,
    extra=0,
    wraps=0,
    reflow_gt=0,
    reflow_out=0,
    large_shifts=0,
    visibility=0,
):
    return {
        "pages": [
            {
                "lines": {"missing": missing, "extra": extra},
                "wraps": {"count": wraps},
                "reflow": {"gt_lines": reflow_gt, "out_lines": reflow_out},
                "instances": {"large_shift_count": large_shifts},
                "visibility": {"mismatch_count": visibility},
            }
        ],
        "gt_pages": gt_pages,
        "out_pages": out_pages,
    }


def validate_report(body, report):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        report_path = root / "assets/bugfixes/issue-186/layout-audit.json"
        report_path.parent.mkdir(parents=True)
        report_path.write_text(json.dumps(report), encoding="utf-8")
        return validate_layout_audit(
            body,
            [
                "assets/bugfixes/issue-186/after.jpg",
                "assets/bugfixes/issue-186/layout-audit.json",
            ],
            root,
        )


def defect_body():
    return (
        visual_body()
        .replace("- Evidence mode: `fix`", "- Evidence mode: `defect`")
        .replace(
            """- GT: `assets/bugfixes/issue-186/gt.jpg`
- Before: `assets/bugfixes/issue-186/before.jpg`
- After: `assets/bugfixes/issue-186/after.jpg`""",
            "- Compare: `assets/bugfixes/issue-186/compare.jpg`",
        )
        .replace(
            """| GT | Before | After |
| --- | --- | --- |
| ![GT](https://example.com/gt.jpg) | ![Before](https://example.com/before.jpg) | ![After](https://example.com/after.jpg) |""",
            """| Compare |
| --- |
| ![Compare](https://example.com/compare.jpg) |""",
        )
    )


class PullRequestBodyTests(unittest.TestCase):
    def test_non_visual_pr_requires_reason(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Documentation-only workflow change
"""
        self.assertEqual(validate_pr_body(body, ["README.md"]), [])

    def test_complete_visual_audit_passes(self):
        errors = validate_pr_body(
            visual_body({"Fill": "Remaining: #328"}),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertEqual(errors, [])

    def test_visual_audit_requires_rendered_evidence_previews(self):
        errors = validate_pr_body(
            visual_body(include_previews=False),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertTrue(any("rendered preview" in error for error in errors))

    def test_visual_audit_requires_layout_finding_disposition(self):
        required = (
            "- [x] Ran compare_layout.py --audit and dispositioned every "
            "large text-instance shift and painted-visibility mismatch"
        )
        errors = validate_pr_body(
            visual_body().replace(required, required.replace("[x]", "[ ]")),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertTrue(any("painted-visibility mismatch" in error for error in errors))

    def test_visual_audit_requires_model_vision_inspection(self):
        required = (
            "- [x] Used Codex/Claude vision to inspect the full GT/output pages, "
            "diff, and matched crops"
        )
        errors = validate_pr_body(
            visual_body().replace(required, required.replace("[x]", "[ ]")),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertTrue(any("Codex/Claude vision" in error for error in errors))

    def test_visual_audit_requires_substantive_model_vision_findings(self):
        body = visual_body().replace(
            "Model vision findings: Full pages, pixel diff, and matched crops were opened; "
            "no untracked visual deviation remains.",
            "Model vision findings: None",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("substantive direct inspection" in error for error in errors))

    def test_visual_audit_rejects_numeric_only_model_vision_findings(self):
        body = visual_body().replace(
            "Model vision findings: Full pages, pixel diff, and matched crops were opened; "
            "no untracked visual deviation remains.",
            "Model vision findings: 123456789012345678901234567890",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("numeric metrics alone" in error for error in errors))

    def test_visual_audit_requires_distinct_preview_urls(self):
        body = visual_body().replace(
            "https://example.com/before.jpg",
            "https://example.com/gt.jpg",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("different evidence image" in error for error in errors))

    def test_commented_preview_does_not_count_as_rendered(self):
        body = visual_body().replace(
            "![GT](https://example.com/gt.jpg)",
            "<!-- ![GT](https://example.com/gt.jpg) -->",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("rendered preview for GT" in error for error in errors))

    def test_backticked_preview_does_not_count_as_rendered(self):
        body = visual_body().replace(
            "![GT](https://example.com/gt.jpg)",
            "`![GT](https://example.com/gt.jpg)`",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("rendered preview for GT" in error for error in errors))

    def test_defect_audit_requires_only_rendered_compare_preview(self):
        errors = validate_pr_body(
            defect_body(),
            ["assets/bugfixes/issue-186/compare.jpg"],
        )
        self.assertEqual(errors, [])

    def test_remaining_deviation_requires_issue(self):
        errors = validate_pr_body(
            visual_body({"Fill": "Remaining: still different"}),
            ["assets/bugfixes/issue-186/after.jpg"],
        )
        self.assertTrue(any("Remaining deviation must reference an issue" in error for error in errors))

    def test_new_follow_up_must_classify_a_remaining_deviation(self):
        body = visual_body().replace(
            "New follow-up issues found in this audit: None",
            "New follow-up issues found in this audit: #328",
        )
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("must also classify a Remaining deviation" in error for error in errors))

    def test_visual_assets_cannot_be_marked_non_visual(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Documentation only
"""
        errors = validate_pr_body(body, ["assets/bugfixes/issue-186/after.jpg"])
        self.assertTrue(any("assets/bugfixes changes require" in error for error in errors))

    def test_layout_report_cannot_be_marked_non_visual(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Report only
"""
        errors = validate_pr_body(
            body,
            ["assets/bugfixes/issue-186/layout-audit.json"],
        )
        self.assertTrue(any("assets/bugfixes changes require" in error for error in errors))

    def test_evidence_documentation_is_not_a_rendered_change(self):
        body = """## Visual impact

- [x] No rendered PDF change
- [ ] Rendered PDF change or visual evidence added
- Reason: Documents the evidence convention
"""
        self.assertEqual(validate_pr_body(body, ["assets/bugfixes/README.md"]), [])


class LayoutAuditTests(unittest.TestCase):
    def test_clean_report_accepts_pass_dispositions(self):
        self.assertEqual(validate_report(visual_body(), layout_report()), [])

    def test_page_count_failure_rejects_pass_disposition(self):
        errors = validate_report(
            visual_body(),
            layout_report(gt_pages=2, out_pages=1),
        )
        self.assertTrue(any("page count" in error and "issue reference" in error for error in errors))

    def test_text_flow_failure_rejects_none_disposition(self):
        errors = validate_report(
            visual_body(),
            layout_report(missing=2, extra=5, wraps=1, reflow_gt=2, reflow_out=1),
        )
        self.assertTrue(any("text flow" in error and "issue reference" in error for error in errors))

    def test_large_shift_failure_rejects_pass_disposition(self):
        errors = validate_report(
            visual_body(),
            layout_report(large_shifts=5),
        )
        self.assertTrue(any("large shifts" in error and "issue reference" in error for error in errors))

    def test_visibility_failure_rejects_pass_text_flow_disposition(self):
        errors = validate_report(
            visual_body(),
            layout_report(visibility=1),
        )
        self.assertTrue(
            any("text flow" in error and "issue reference" in error for error in errors)
        )

    def test_failed_categories_accept_remaining_issue_references(self):
        body = visual_body(
            {
                "Position/size": "Remaining: #328",
                "Line/paragraph spacing": "Remaining: #329",
            }
        )
        body = body.replace(
            "Layout audit text flow: Pass",
            "Layout audit text flow: #329",
        ).replace(
            "Layout audit large shifts: Pass",
            "Layout audit large shifts: #328, #329",
        )
        errors = validate_report(
            body,
            layout_report(missing=1, reflow_gt=2, reflow_out=1, large_shifts=3),
        )
        self.assertEqual(errors, [])

    def test_failed_category_rejects_issue_absent_from_remaining_rows(self):
        body = visual_body().replace(
            "Layout audit large shifts: Pass",
            "Layout audit large shifts: #328",
        )
        errors = validate_report(body, layout_report(large_shifts=1))
        self.assertTrue(any("must also appear in a Remaining" in error for error in errors))

    def test_clean_category_rejects_issue_disposition(self):
        body = visual_body({"Position/size": "Remaining: #328"}).replace(
            "Layout audit large shifts: Pass",
            "Layout audit large shifts: #328",
        )
        errors = validate_report(body, layout_report())
        self.assertTrue(any("has no findings and must be Pass" in error for error in errors))

    def test_fix_audit_requires_changed_machine_report(self):
        with tempfile.TemporaryDirectory() as directory:
            errors = validate_layout_audit(visual_body(), [], Path(directory))
        self.assertTrue(any("must be changed in this pull request" in error for error in errors))

    def test_fix_audit_report_must_accompany_fresh_image_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report_path = root / "assets/bugfixes/issue-186/layout-audit.json"
            report_path.parent.mkdir(parents=True)
            report_path.write_text(json.dumps(layout_report()), encoding="utf-8")
            errors = validate_layout_audit(
                visual_body(),
                ["assets/bugfixes/issue-186/layout-audit.json"],
                root,
            )
        self.assertTrue(any("after.jpg must be changed" in error for error in errors))

    def test_fix_audit_report_requires_changed_after_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report_path = root / "assets/bugfixes/issue-186/layout-audit.json"
            report_path.parent.mkdir(parents=True)
            report_path.write_text(json.dumps(layout_report()), encoding="utf-8")
            errors = validate_layout_audit(
                visual_body(),
                [
                    "assets/bugfixes/issue-186/gt.jpg",
                    "assets/bugfixes/issue-186/layout-audit.json",
                ],
                root,
            )
        self.assertTrue(any("after.jpg must be changed" in error for error in errors))

    def test_text_layer_only_fix_accepts_identical_decoded_before_and_after_images(self):
        body = visual_body().replace(
            "- Evidence mode: `fix`",
            "- Evidence mode: `fix`\n- Text-layer-only: Yes\n- Pixel delta: 0",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            issue_dir = root / "assets/bugfixes/issue-186"
            issue_dir.mkdir(parents=True)
            (issue_dir / "layout-audit.json").write_text(
                json.dumps(layout_report()), encoding="utf-8"
            )
            evidence = (ROOT / "assets/bugfixes/issue-186/after.jpg").read_bytes()
            (issue_dir / "before.jpg").write_bytes(evidence)
            (issue_dir / "after.jpg").write_bytes(evidence)
            errors = validate_layout_audit(
                body,
                ["assets/bugfixes/issue-186/layout-audit.json"],
                root,
            )
        self.assertEqual(errors, [])

    def test_text_layer_only_fix_rejects_different_decoded_before_and_after_images(self):
        body = visual_body().replace(
            "- Evidence mode: `fix`",
            "- Evidence mode: `fix`\n- Text-layer-only: Yes\n- Pixel delta: 0",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            issue_dir = root / "assets/bugfixes/issue-186"
            issue_dir.mkdir(parents=True)
            (issue_dir / "layout-audit.json").write_text(
                json.dumps(layout_report()), encoding="utf-8"
            )
            (issue_dir / "before.jpg").write_bytes(
                (ROOT / "assets/bugfixes/issue-186/gt.jpg").read_bytes()
            )
            (issue_dir / "after.jpg").write_bytes(
                (ROOT / "assets/bugfixes/issue-186/after.jpg").read_bytes()
            )
            errors = validate_layout_audit(
                body,
                ["assets/bugfixes/issue-186/layout-audit.json"],
                root,
            )
        self.assertTrue(any("decoded-pixel delta" in error for error in errors))

    def test_text_layer_only_fix_requires_zero_pixel_delta(self):
        body = visual_body().replace(
            "- Evidence mode: `fix`",
            "- Evidence mode: `fix`\n- Text-layer-only: Yes\n- Pixel delta: 1",
        )
        errors = validate_report(body, layout_report())
        self.assertTrue(any("Pixel delta must be 0" in error for error in errors))

    def test_text_layer_only_fix_requires_current_before_image(self):
        body = visual_body().replace(
            "- Evidence mode: `fix`",
            "- Evidence mode: `fix`\n- Text-layer-only: Yes\n- Pixel delta: 0",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            issue_dir = root / "assets/bugfixes/issue-186"
            issue_dir.mkdir(parents=True)
            (issue_dir / "layout-audit.json").write_text(
                json.dumps(layout_report()), encoding="utf-8"
            )
            (issue_dir / "after.jpg").write_bytes(
                (ROOT / "assets/bugfixes/issue-186/after.jpg").read_bytes()
            )
            errors = validate_layout_audit(
                body,
                ["assets/bugfixes/issue-186/layout-audit.json"],
                root,
            )
        self.assertTrue(any("before.jpg: current evidence is required" in error for error in errors))

    def test_fix_audit_requires_issue_scoped_report_path(self):
        body = visual_body().replace(
            "assets/bugfixes/issue-186/layout-audit.json",
            "assets/bugfixes/issue-999/layout-audit.json",
        )
        errors = validate_report(body, layout_report())
        self.assertTrue(any("Layout audit report must be" in error for error in errors))

    def test_malformed_report_is_rejected(self):
        errors = validate_report(visual_body(), {"pages": "not-a-list"})
        self.assertTrue(any("invalid layout audit report" in error for error in errors))

    def test_incomplete_page_vectors_are_rejected(self):
        errors = validate_report(
            visual_body(),
            layout_report(gt_pages=2, out_pages=2),
        )
        self.assertTrue(any("invalid layout audit report" in error for error in errors))


class EvidenceTests(unittest.TestCase):
    def test_repository_evidence_is_progressive_150_dpi_jpeg(self):
        path = ROOT / "assets/bugfixes/issue-186/gt.jpg"
        info = read_jpeg_info(path)
        self.assertTrue(info.progressive)
        self.assertEqual(info.density_dpi, (150.0, 150.0))
        self.assertEqual(info.metadata_markers, ())

    def test_changed_trio_validates_all_three_files(self):
        errors = validate_evidence(
            ["assets/bugfixes/issue-186/after.jpg"],
            ROOT,
        )
        self.assertEqual(errors, [])

    def test_png_evidence_is_rejected(self):
        errors = validate_evidence(
            ["assets/bugfixes/issue-186/after.png"],
            ROOT,
        )
        self.assertTrue(any(".jpg extension" in error for error in errors))

    def test_evidence_readme_is_not_evidence(self):
        self.assertEqual(validate_evidence(["assets/bugfixes/README.md"], ROOT), [])

    def test_layout_audit_report_is_validated_separately_from_jpegs(self):
        self.assertEqual(
            validate_evidence(
                ["assets/bugfixes/issue-186/layout-audit.json"],
                ROOT,
            ),
            [],
        )

    def test_issue_directory_notes_are_not_evidence(self):
        self.assertEqual(
            validate_evidence(["assets/bugfixes/issue-186/notes.txt"], ROOT),
            [],
        )

    def test_unnamed_image_is_still_rejected(self):
        errors = validate_evidence(["assets/bugfixes/issue-186/extra.jpg"], ROOT)
        self.assertTrue(any("visual evidence must be" in error for error in errors))


class OpenIssueTests(unittest.TestCase):
    @patch("scripts.check_visual_pr.urllib.request.urlopen")
    def test_remaining_issue_must_be_open(self, urlopen):
        response = io.BytesIO(b'{"state":"closed"}')
        urlopen.return_value = response
        errors = validate_open_issues({328}, "developer0hye/office2pdf", "token")
        self.assertEqual(errors, ["Remaining visual issue #328 is not open."])


if __name__ == "__main__":
    unittest.main()

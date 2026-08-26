"""Unit tests for the one-factor native-export probe harness (#698).

Every export and differ invocation goes through an injectable runner, so the
platform-bound stages (AppleScript export, mutool) are exercised here without a
Mac or a converter binary. The zip traps pinned below are the ones the manual
loop hit in practice: `zip` updating an existing archive instead of replacing
it, and dotfile/AppleDouble members leaking into a repackaged fixture. The
differ tests pin the #810 contract: a differ failure must propagate, never
read as an empty (all-identical) probe row.
"""

from __future__ import annotations

import io
import json
import subprocess
import sys
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import probe_harness


SLIDE_XML = '<p:sld><a:bodyPr anchor="t"/><a:spcAft val="1000"/></p:sld>'
EXCEL_EXPORTER = (
    Path(__file__).resolve().parent.parent / "macos" / "export_excel_pdfs.applescript"
)


def write_base_package(path: Path, slide_xml: str = SLIDE_XML, extra: dict | None = None) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("[Content_Types].xml", "<Types/>")
        archive.writestr("ppt/presentation.xml", "<p:presentation/>")
        archive.writestr("ppt/slides/slide1.xml", slide_xml)
        for name, data in (extra or {}).items():
            archive.writestr(name, data)


def write_spec(directory: Path, overrides: dict | None = None, base_name: str = "base.pptx") -> Path:
    spec = {
        "name": "spcAft-linearity",
        "base": base_name,
        "part": "ppt/slides/slide1.xml",
        "factor": "a:spcAft val",
        "variants": [
            {"value": "500", "find": 'val="1000"', "replace": 'val="500"'},
            {"value": "2000", "find": 'val="1000"', "replace": 'val="2000"'},
        ],
    }
    spec.update(overrides or {})
    path = directory / "probe.json"
    path.write_text(json.dumps(spec))
    return path


def completed(argv: list[str], returncode: int = 0, stdout: str = "", stderr: str = "") -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(argv, returncode, stdout=stdout, stderr=stderr)


def page_vector(
    matched: int = 12,
    missing: int = 0,
    extra: int = 0,
    deviant: int = 0,
    wraps: int = 0,
    reflow: int = 0,
    large_shifts: int = 0,
    mean_abs_dy: float = 0.0,
    worst_dy: float = 0.0,
    worst_pitch: float = 0.0,
    worst_width: float = 0.0,
    rects: tuple[int, int] = (3, 3),
) -> dict:
    """A differ page vector with the same shape compare_layout.py --json emits."""
    return {
        "lines": {
            "gt": matched + missing,
            "out": matched + extra,
            "matched": matched,
            "missing": missing,
            "extra": extra,
            "deviant": deviant,
            "missing_text": [],
            "extra_text": [],
        },
        "baseline": {
            "mean_abs_dy": mean_abs_dy,
            "worst_dy": abs(worst_dy),
            "worst_dy_signed": worst_dy,
            "worst_line": "",
        },
        "dx0": {"mean_abs": 0.0, "worst": 0.0},
        "width": {"mean_abs_pct": 0.0, "worst_pct": worst_width},
        "instances": {
            "large_shift_threshold": 5.0,
            "large_shift_count": large_shifts,
            "large_shifts": [],
        },
        "pitch": {"pairs": max(matched - 1, 0), "worst_delta": worst_pitch},
        "wraps": {"count": wraps, "samples": []},
        "reflow": {"gt_lines": reflow, "out_lines": reflow},
        "rects": {
            "gt_count": rects[0],
            "out_count": rects[1],
            "matched": min(rects),
            "mean_center_delta": 0.0,
        },
        "noise_floor": 0.12,
    }


def differ_report(*vectors: dict, gt_pages: int | None = None, out_pages: int | None = None) -> dict:
    return {
        "pages": list(vectors),
        "gt_pages": len(vectors) if gt_pages is None else gt_pages,
        "out_pages": len(vectors) if out_pages is None else out_pages,
    }


class SpecTest(unittest.TestCase):
    def test_base_path_resolves_relative_to_the_spec_file(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec = probe_harness.load_spec(write_spec(tmp))
            self.assertEqual(spec.base, (tmp / "base.pptx").resolve())
            self.assertEqual(spec.name, "spcAft-linearity")
            self.assertEqual([v.value for v in spec.variants], ["500", "2000"])

    def test_a_missing_required_key_is_a_probe_error(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            path = write_spec(tmp)
            spec = json.loads(path.read_text())
            del spec["part"]
            path.write_text(json.dumps(spec))
            with self.assertRaisesRegex(probe_harness.ProbeError, "part"):
                probe_harness.load_spec(path)

    def test_duplicate_variant_values_are_rejected(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            path = write_spec(
                tmp,
                {
                    "variants": [
                        {"value": "500", "find": "a", "replace": "b"},
                        {"value": "500", "find": "a", "replace": "c"},
                    ]
                },
            )
            with self.assertRaisesRegex(probe_harness.ProbeError, "duplicate"):
                probe_harness.load_spec(path)

    def test_an_empty_variant_list_is_rejected(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            path = write_spec(tmp, {"variants": []})
            with self.assertRaisesRegex(probe_harness.ProbeError, "variant"):
                probe_harness.load_spec(path)


class PatchTest(unittest.TestCase):
    def variant(self, **overrides) -> probe_harness.Variant:
        fields = {"value": "500", "find": 'val="1000"', "replace": 'val="500"'}
        fields.update(overrides)
        return probe_harness.Variant(**fields)

    def test_a_literal_patch_replaces_and_reports_the_count(self):
        patched, count = probe_harness.apply_patch(SLIDE_XML, self.variant())
        self.assertIn('val="500"', patched)
        self.assertNotIn('val="1000"', patched)
        self.assertEqual(count, 1)

    def test_a_patch_that_matches_nothing_is_an_error(self):
        # A no-op patch would export a variant identical to the control and
        # read as "this factor does nothing" — the probe must refuse to run.
        with self.assertRaisesRegex(probe_harness.ProbeError, "500"):
            probe_harness.apply_patch(SLIDE_XML, self.variant(find='val="9999"'))

    def test_a_regex_patch_substitutes_by_pattern(self):
        patched, count = probe_harness.apply_patch(
            SLIDE_XML,
            self.variant(find=r'anchor="[a-z]+"', replace='anchor="ctr"', regex=True),
        )
        self.assertIn('anchor="ctr"', patched)
        self.assertEqual(count, 1)

    def test_an_expected_count_mismatch_is_an_error(self):
        with self.assertRaisesRegex(probe_harness.ProbeError, "expected 2"):
            probe_harness.apply_patch(SLIDE_XML, self.variant(count=2))

    def test_multiple_matches_are_allowed_and_counted(self):
        xml = SLIDE_XML + '<a:spcBef val="1000"/>'
        _, count = probe_harness.apply_patch(xml, self.variant())
        self.assertEqual(count, 2)


class MemberHygieneTest(unittest.TestCase):
    def test_appledouble_and_dotfile_members_are_flagged(self):
        flagged = probe_harness.suspicious_members(
            ["ppt/slides/slide1.xml", "._slide1.xml", ".DS_Store", "ppt/._x.xml"]
        )
        self.assertEqual(flagged, ["._slide1.xml", ".DS_Store", "ppt/._x.xml"])

    def test_ordinary_ooxml_members_are_not_flagged(self):
        names = ["[Content_Types].xml", "_rels/.rels", "ppt/slides/slide1.xml"]
        self.assertEqual(probe_harness.suspicious_members(names), [])


class BuildPackageTest(unittest.TestCase):
    def test_the_control_preserves_every_member_name_and_byte(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            base = tmp / "base.pptx"
            write_base_package(base)
            control = tmp / "control.pptx"
            probe_harness.build_package(base, control, {})
            with zipfile.ZipFile(base) as before, zipfile.ZipFile(control) as after:
                self.assertEqual(before.namelist(), after.namelist())
                for name in before.namelist():
                    self.assertEqual(before.read(name), after.read(name))

    def test_a_variant_changes_only_the_patched_part(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            base = tmp / "base.pptx"
            write_base_package(base)
            variant = tmp / "variant.pptx"
            patched = SLIDE_XML.replace('val="1000"', 'val="500"')
            probe_harness.build_package(
                base, variant, {"ppt/slides/slide1.xml": patched.encode()}
            )
            with zipfile.ZipFile(base) as before, zipfile.ZipFile(variant) as after:
                self.assertEqual(before.namelist(), after.namelist())
                self.assertEqual(after.read("ppt/slides/slide1.xml"), patched.encode())
                for name in before.namelist():
                    if name != "ppt/slides/slide1.xml":
                        self.assertEqual(before.read(name), after.read(name))

    def test_an_existing_archive_is_replaced_not_updated(self):
        # `zip -r` updates an archive in place, so a second variant silently
        # inherits the first's members; the harness must always write fresh.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            base = tmp / "base.pptx"
            write_base_package(base)
            out = tmp / "out.pptx"
            with zipfile.ZipFile(out, "w") as stale:
                stale.writestr("stale/leftover.xml", "<stale/>")
            probe_harness.build_package(base, out, {})
            with zipfile.ZipFile(out) as archive:
                self.assertNotIn("stale/leftover.xml", archive.namelist())

    def test_rebuilding_the_same_package_is_byte_deterministic(self):
        # The control gate byte-compares exported PDFs; a nondeterministic
        # repackaging step would turn that gate into noise.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            base = tmp / "base.pptx"
            write_base_package(base)
            first = tmp / "first.pptx"
            second = tmp / "second.pptx"
            probe_harness.build_package(base, first, {})
            probe_harness.build_package(base, second, {})
            self.assertEqual(first.read_bytes(), second.read_bytes())


class ExportTest(unittest.TestCase):
    def collect_runner(self, pdfs_to_write: dict[str, bytes] | None = None):
        calls: list[list[str]] = []

        def runner(argv: list[str]) -> subprocess.CompletedProcess:
            calls.append(list(argv))
            for name, data in (pdfs_to_write or {}).items():
                Path(name).write_bytes(data)
            return completed(argv)

        return calls, runner

    def test_the_office2pdf_backend_converts_each_package(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "control.pptx").write_bytes(b"zip")
            calls = []

            def runner(argv):
                calls.append(list(argv))
                Path(argv[3]).write_bytes(b"%PDF")
                return completed(argv)

            probe_harness.export_all(
                "office2pdf",
                [("control", tmp / "control.pptx")],
                pdf_dir,
                converter="/tmp/office2pdf",
                runner=runner,
            )
            self.assertEqual(
                calls,
                [["/tmp/office2pdf", str(tmp / "control.pptx"), "-o", str(pdf_dir / "control.pdf")]],
            )

    def test_the_soffice_backend_converts_into_the_pdf_directory(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "control.pptx").write_bytes(b"zip")
            calls = []

            def runner(argv):
                calls.append(list(argv))
                (pdf_dir / "control.pdf").write_bytes(b"%PDF")
                return completed(argv)

            probe_harness.export_all(
                "soffice", [("control", tmp / "control.pptx")], pdf_dir, runner=runner
            )
            self.assertEqual(
                calls,
                [
                    [
                        "soffice",
                        "--headless",
                        "--convert-to",
                        "pdf",
                        "--outdir",
                        str(pdf_dir),
                        str(tmp / "control.pptx"),
                    ]
                ],
            )

    def test_the_office_backend_batches_one_osascript_call(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "base.docx").write_bytes(b"zip")
            (tmp / "variant-500.docx").write_bytes(b"zip2")
            calls = []

            def runner(argv):
                calls.append(list(argv))
                (pdf_dir / "base.pdf").write_bytes(b"%PDF")
                (pdf_dir / "variant-500.pdf").write_bytes(b"%PDF")
                return completed(argv)

            probe_harness.export_all(
                "office",
                [("base", tmp / "base.docx"), ("variant-500", tmp / "variant-500.docx")],
                pdf_dir,
                runner=runner,
            )
            self.assertEqual(len(calls), 1)
            self.assertEqual(calls[0][0], "osascript")
            self.assertTrue(calls[0][1].endswith("export_word_pdfs.applescript"))
            self.assertEqual(
                calls[0][2:],
                [
                    str(pdf_dir),
                    "base",
                    str(tmp / "base.docx"),
                    "variant-500",
                    str(tmp / "variant-500.docx"),
                ],
            )

    def test_the_office_backend_picks_the_applescript_by_extension(self):
        self.assertTrue(str(probe_harness.applescript_for(".docx")).endswith("export_word_pdfs.applescript"))
        self.assertTrue(str(probe_harness.applescript_for(".pptx")).endswith("export_powerpoint_pdfs.applescript"))
        self.assertTrue(str(probe_harness.applescript_for(".xlsx")).endswith("export_excel_pdfs.applescript"))
        with self.assertRaisesRegex(probe_harness.ProbeError, "odt"):
            probe_harness.applescript_for(".odt")

    def test_excel_export_walks_chartsheets_in_workbook_order(self):
        # The exporter itself requires macOS and Excel, so portable CI pins the
        # workbook-order enumeration feeding the tested PDF-union path (#1316).
        source = EXCEL_EXPORTER.read_text()
        self.assertIn(
            "set workbookSheets to get every sheet of openedWorkbook",
            source,
        )
        self.assertIn(
            "repeat with sheetIndex from 1 to (count workbookSheets)",
            source,
        )
        self.assertIn(
            "set currentSheet to item sheetIndex of workbookSheets",
            source,
        )
        self.assertIn(
            'outputId & "-sheet-" & my paddedIndex(sheetIndex)',
            source,
        )
        self.assertNotIn("count worksheets of openedWorkbook", source)

    def test_a_visible_chartsheet_satisfies_the_excel_export_contract(self):
        source = EXCEL_EXPORTER.read_text()
        self.assertIn(
            'if visibleSheetCount is 0 then error "workbook has no visible sheets"',
            source,
        )

    def test_the_office_backend_refuses_an_external_volume_stage(self):
        # The Office sandbox cannot write to external volumes; a save there
        # hangs with AppleEvent timeout -1712 and produces nothing.
        calls, runner = self.collect_runner()
        with self.assertRaisesRegex(probe_harness.ProbeError, "internal disk"):
            probe_harness.export_all(
                "office",
                [("base", Path("/tmp/base.docx"))],
                Path("/Volumes/FakeDisk/probe/pdfs"),
                runner=runner,
            )
        self.assertEqual(calls, [])

    def test_an_export_failure_propagates_its_stderr(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "control.pptx").write_bytes(b"zip")

            def runner(argv):
                return completed(argv, returncode=1, stderr="conversion exploded")

            with self.assertRaisesRegex(probe_harness.ProbeError, "conversion exploded"):
                probe_harness.export_all(
                    "office2pdf", [("control", tmp / "control.pptx")], pdf_dir, runner=runner
                )

    def test_a_missing_output_pdf_is_an_error(self):
        # An exporter that reports success but writes nothing (or an empty
        # file) must not produce a probe row.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "control.pptx").write_bytes(b"zip")

            def runner(argv):
                return completed(argv)

            with self.assertRaisesRegex(probe_harness.ProbeError, "control"):
                probe_harness.export_all(
                    "office2pdf", [("control", tmp / "control.pptx")], pdf_dir, runner=runner
                )

    def test_excel_sheet_pdfs_are_united_per_job(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "base.xlsx").write_bytes(b"zip")
            calls = []

            def runner(argv):
                calls.append(list(argv))
                if argv[0] == "osascript":
                    (pdf_dir / "base-sheet-01.pdf").write_bytes(b"%PDF1")
                    (pdf_dir / "base-sheet-02.pdf").write_bytes(b"%PDF2")
                if argv[0] == "pdfunite":
                    Path(argv[-1]).write_bytes(b"%PDF12")
                return completed(argv)

            probe_harness.export_all(
                "office", [("base", tmp / "base.xlsx")], pdf_dir, runner=runner
            )
            self.assertEqual(calls[0][0], "osascript")
            self.assertTrue(calls[0][1].endswith("export_excel_pdfs.applescript"))
            self.assertEqual(
                calls[1],
                [
                    "pdfunite",
                    str(pdf_dir / "base-sheet-01.pdf"),
                    str(pdf_dir / "base-sheet-02.pdf"),
                    str(pdf_dir / "base.pdf"),
                ],
            )

    def test_a_single_excel_sheet_is_copied_without_pdfunite(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            pdf_dir = tmp / "pdfs"
            pdf_dir.mkdir()
            (tmp / "base.xlsx").write_bytes(b"zip")
            calls = []

            def runner(argv):
                calls.append(list(argv))
                (pdf_dir / "base-sheet-01.pdf").write_bytes(b"%PDF1")
                return completed(argv)

            probe_harness.export_all(
                "office", [("base", tmp / "base.xlsx")], pdf_dir, runner=runner
            )
            self.assertEqual(len(calls), 1)
            self.assertEqual((pdf_dir / "base.pdf").read_bytes(), b"%PDF1")


class StageRootTest(unittest.TestCase):
    """Where a run stages when --stage-root is not given (#1051)."""

    def test_the_office_backend_stages_in_the_driven_apps_own_container(self):
        # Anywhere outside the sandbox container raises a per-file "Grant
        # Access" dialog, which stalls an unattended run; a cross-app
        # container prompts too, so each format goes to its own app's.
        for extension, bundle_id in (
            (".docx", "com.microsoft.Word"),
            (".pptx", "com.microsoft.Powerpoint"),
            (".xlsx", "com.microsoft.Excel"),
        ):
            with self.subTest(extension=extension):
                self.assertEqual(
                    probe_harness.default_stage_root("office", extension),
                    probe_harness.CONTAINERS_ROOT / bundle_id / "Data" / "probes",
                )

    def test_an_extension_no_office_app_opens_is_an_error(self):
        with self.assertRaisesRegex(probe_harness.ProbeError, "odt"):
            probe_harness.default_stage_root("office", ".odt")

    def test_the_other_backends_stage_under_the_repo_results_root(self):
        # Only the sandboxed apps need the container; our converter and
        # LibreOffice read and write anywhere.
        for backend in ("office2pdf", "soffice"):
            with self.subTest(backend=backend):
                self.assertEqual(
                    probe_harness.default_stage_root(backend, ".pptx"),
                    probe_harness.RESULTS_ROOT,
                )

    def test_results_are_copied_out_of_the_container(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            stage = tmp / "container" / "spcAft"
            (stage / "pdfs").mkdir(parents=True)
            (stage / "packages").mkdir()
            (stage / "report.md").write_text("# Probe: spcAft\n")
            (stage / "pdfs" / "base.pdf").write_bytes(b"%PDF base")
            (stage / "pdfs" / "variant-500.pdf").write_bytes(b"%PDF 500")
            destination = tmp / "target" / "probes" / "spcAft"

            report_path = probe_harness.mirror_results(stage, destination)

            self.assertEqual(report_path, destination / "report.md")
            self.assertEqual(report_path.read_text(), "# Probe: spcAft\n")
            self.assertEqual(
                (destination / "pdfs" / "variant-500.pdf").read_bytes(), b"%PDF 500"
            )
            self.assertEqual((destination / "pdfs" / "base.pdf").read_bytes(), b"%PDF base")


class DifferTest(unittest.TestCase):
    def test_the_differ_report_is_parsed_from_json(self):
        report = differ_report(page_vector())

        def runner(argv):
            self.assertEqual(argv[0], sys.executable)
            self.assertTrue(argv[1].endswith("compare_layout.py"))
            self.assertIn("--json", argv)
            return completed(argv, stdout=json.dumps(report))

        parsed = probe_harness.run_differ(Path("control.pdf"), Path("variant.pdf"), runner=runner)
        self.assertEqual(parsed["gt_pages"], 1)

    def test_page_and_noise_floor_are_forwarded(self):
        seen = []

        def runner(argv):
            seen.append(list(argv))
            return completed(argv, stdout=json.dumps(differ_report(page_vector())))

        probe_harness.run_differ(
            Path("control.pdf"), Path("variant.pdf"), page=2, noise_floor=0.5, runner=runner
        )
        self.assertIn("--page", seen[0])
        self.assertIn("2", seen[0])
        self.assertIn("--noise-floor", seen[0])
        self.assertIn("0.5", seen[0])

    def test_a_differ_failure_propagates_instead_of_reading_as_identical(self):
        # The #810 trap: a differ that parses zero pages once exited 0 with an
        # empty report, which reads as "variant identical to control" — the
        # exact answer a one-factor probe is looking for, and exactly wrong.
        def runner(argv):
            return completed(
                argv, returncode=1, stderr="no pages parsed from the GT trace — cannot compare"
            )

        with self.assertRaisesRegex(probe_harness.ProbeError, "no pages parsed"):
            probe_harness.run_differ(Path("control.pdf"), Path("variant.pdf"), runner=runner)

    def test_undecodable_differ_output_is_an_error(self):
        def runner(argv):
            return completed(argv, stdout="not json")

        with self.assertRaisesRegex(probe_harness.ProbeError, "JSON"):
            probe_harness.run_differ(Path("control.pdf"), Path("variant.pdf"), runner=runner)


class ControlGateTest(unittest.TestCase):
    def test_byte_identical_pdfs_pass_without_the_differ(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            (tmp / "base.pdf").write_bytes(b"%PDF same")
            (tmp / "control.pdf").write_bytes(b"%PDF same")

            def runner(argv):
                raise AssertionError("differ must not run for byte-identical PDFs")

            verdict = probe_harness.verify_control(
                tmp / "base.pdf", tmp / "control.pdf", runner=runner
            )
            self.assertIn("byte-identical", verdict)

    def test_a_layout_identical_control_passes_with_a_note(self):
        # Native Office stamps timestamps into every export, so byte equality
        # is unreachable there; a clean differ report is the fallback gate.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            (tmp / "base.pdf").write_bytes(b"%PDF a")
            (tmp / "control.pdf").write_bytes(b"%PDF b")

            def runner(argv):
                return completed(argv, stdout=json.dumps(differ_report(page_vector())))

            verdict = probe_harness.verify_control(
                tmp / "base.pdf", tmp / "control.pdf", runner=runner
            )
            self.assertIn("layout-identical", verdict)

    def test_a_control_deviation_aborts_the_run(self):
        # A control that differs from the base invalidates every variant row;
        # nothing downstream can be trusted.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            (tmp / "base.pdf").write_bytes(b"%PDF a")
            (tmp / "control.pdf").write_bytes(b"%PDF b")

            def runner(argv):
                return completed(
                    argv, stdout=json.dumps(differ_report(page_vector(missing=1)))
                )

            with self.assertRaisesRegex(probe_harness.ProbeError, "control"):
                probe_harness.verify_control(tmp / "base.pdf", tmp / "control.pdf", runner=runner)

    def test_a_control_page_count_mismatch_aborts_the_run(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            (tmp / "base.pdf").write_bytes(b"%PDF a")
            (tmp / "control.pdf").write_bytes(b"%PDF b")

            def runner(argv):
                return completed(
                    argv,
                    stdout=json.dumps(differ_report(page_vector(), gt_pages=2, out_pages=1)),
                )

            with self.assertRaisesRegex(probe_harness.ProbeError, "page"):
                probe_harness.verify_control(tmp / "base.pdf", tmp / "control.pdf", runner=runner)


class RowTest(unittest.TestCase):
    def test_multi_page_reports_sum_counts_and_keep_worst_magnitudes(self):
        report = differ_report(
            page_vector(matched=10, missing=1, wraps=1, mean_abs_dy=0.2, worst_dy=-0.5, worst_pitch=0.3),
            page_vector(matched=30, extra=2, deviant=3, mean_abs_dy=0.6, worst_dy=1.4, worst_pitch=0.1, rects=(4, 5)),
        )
        row = probe_harness.variant_row("500", 1, report)
        self.assertEqual(row["value"], "500")
        self.assertEqual(row["patched"], 1)
        self.assertEqual(row["pages"], "2/2")
        self.assertEqual(row["matched"], 40)
        self.assertEqual(row["missing"], 1)
        self.assertEqual(row["extra"], 2)
        self.assertEqual(row["wraps"], 1)
        self.assertEqual(row["deviant"], 3)
        self.assertAlmostEqual(row["mean_abs_dy"], (0.2 * 10 + 0.6 * 30) / 40)
        self.assertAlmostEqual(row["worst_dy"], 1.4)
        self.assertAlmostEqual(row["worst_pitch"], 0.3)
        self.assertEqual(row["rects"], "7/8")


class TableTest(unittest.TestCase):
    def test_one_row_per_variant_keyed_by_the_factor_value(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec = probe_harness.load_spec(write_spec(tmp))
        rows = [
            probe_harness.variant_row("500", 1, differ_report(page_vector(worst_dy=-5.0))),
            probe_harness.variant_row("2000", 1, differ_report(page_vector(worst_dy=20.0))),
        ]
        table = probe_harness.render_table(spec, "office2pdf", "byte-identical re-zip", rows)
        self.assertIn("a:spcAft val", table)
        self.assertIn("byte-identical re-zip", table)
        lines = [line for line in table.splitlines() if line.startswith("| ")]
        self.assertEqual(len(lines), 4)  # header + separator + two variants
        self.assertIn("| 500 |", table)
        self.assertIn("| 2000 |", table)
        self.assertIn("-5.00", table)
        self.assertIn("+20.00", table)


class EndToEndTest(unittest.TestCase):
    """Full run with a fake exporter that derives PDF bytes from the patched part.

    Base and control carry identical slide XML, so their fake PDFs are
    byte-identical and the control gate passes without the differ; variants
    differ and get a differ row each.
    """

    def fake_runner(self, differ_result=None):
        def runner(argv):
            if argv[0] == "fake-office2pdf":
                with zipfile.ZipFile(argv[1]) as archive:
                    content = archive.read("ppt/slides/slide1.xml")
                Path(argv[3]).write_bytes(b"%PDF " + content)
                return completed(argv)
            if argv[1].endswith("compare_layout.py"):
                if differ_result is not None:
                    return differ_result(argv)
                return completed(argv, stdout=json.dumps(differ_report(page_vector())))
            raise AssertionError(f"unexpected command {argv}")

        return runner

    def test_a_probe_run_writes_one_table_with_one_row_per_variant(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec_path = write_spec(tmp)
            stage_root = tmp / "stage"
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                report_path = probe_harness.run_probe(
                    spec_path,
                    backend="office2pdf",
                    converter="fake-office2pdf",
                    stage_root=stage_root,
                    runner=self.fake_runner(),
                )
            report = report_path.read_text()
            self.assertIn("| 500 |", report)
            self.assertIn("| 2000 |", report)
            self.assertIn("byte-identical", report)
            self.assertEqual(report, stdout.getvalue())
            packages = stage_root / "spcAft-linearity" / "packages"
            self.assertTrue((packages / "control.pptx").exists())
            self.assertTrue((packages / "variant-500.pptx").exists())

    def office_runner(self):
        """A fake `osascript` export deriving each PDF from its package's part."""
        calls: list[list[str]] = []

        def runner(argv):
            calls.append(list(argv))
            if argv[0] == "osascript":
                pdf_dir = Path(argv[2])
                for index in range(3, len(argv), 2):
                    job_id, package = argv[index], argv[index + 1]
                    with zipfile.ZipFile(package) as archive:
                        content = archive.read("ppt/slides/slide1.xml")
                    (pdf_dir / f"{job_id}.pdf").write_bytes(b"%PDF " + content)
                return completed(argv)
            if argv[1].endswith("compare_layout.py"):
                return completed(argv, stdout=json.dumps(differ_report(page_vector())))
            raise AssertionError(f"unexpected command {argv}")

        return calls, runner

    def test_an_office_run_stages_in_the_container_and_copies_results_out(self):
        # Every path PowerPoint is handed must sit inside its own container,
        # or the sandbox raises a Grant Access dialog per file (#1051); the
        # results still have to land where the caller looks for them.
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec_path = write_spec(tmp)
            containers = tmp / "Containers"
            results = tmp / "target" / "probes"
            calls, runner = self.office_runner()
            with patch.object(probe_harness, "CONTAINERS_ROOT", containers), patch.object(
                probe_harness, "RESULTS_ROOT", results
            ), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                report_path = probe_harness.run_probe(
                    spec_path, backend="office", runner=runner
                )

            stage = containers / "com.microsoft.Powerpoint" / "Data" / "probes" / "spcAft-linearity"
            exports = [argv for argv in calls if argv[0] == "osascript"]
            self.assertEqual(len(exports), 1)
            for argument in exports[0][2:]:
                if "/" in argument:
                    self.assertTrue(
                        Path(argument).is_relative_to(stage),
                        f"{argument} is outside the app container",
                    )
            self.assertEqual(report_path, results / "spcAft-linearity" / "report.md")
            self.assertIn("| 500 |", report_path.read_text())
            self.assertEqual(
                (results / "spcAft-linearity" / "pdfs" / "variant-500.pdf").read_bytes(),
                (stage / "pdfs" / "variant-500.pdf").read_bytes(),
            )

    def test_an_explicit_stage_root_is_honoured_and_still_guarded(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec_path = write_spec(tmp)
            _, runner = self.office_runner()
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                report_path = probe_harness.run_probe(
                    spec_path, backend="office", stage_root=tmp / "explicit", runner=runner
                )
            self.assertEqual(report_path, tmp / "explicit" / "spcAft-linearity" / "report.md")

            with self.assertRaisesRegex(probe_harness.ProbeError, "internal disk"):
                probe_harness.run_probe(
                    spec_path,
                    backend="office",
                    stage_root=Path("/Volumes/FakeDisk/probes"),
                    runner=runner,
                )

    def test_a_differ_failure_fails_the_whole_run(self):
        def differ_result(argv):
            return completed(argv, returncode=1, stderr="no pages parsed from the GT trace")

        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec_path = write_spec(tmp)
            with self.assertRaisesRegex(probe_harness.ProbeError, "no pages parsed"):
                probe_harness.run_probe(
                    spec_path,
                    backend="office2pdf",
                    converter="fake-office2pdf",
                    stage_root=tmp / "stage",
                    runner=self.fake_runner(differ_result=differ_result),
                )

    def test_main_reports_a_probe_error_and_exits_nonzero(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            write_base_package(tmp / "base.pptx")
            spec_path = write_spec(
                tmp,
                {"variants": [{"value": "500", "find": "no-such-text", "replace": "x"}]},
            )
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                code = probe_harness.main(
                    [str(spec_path), "--stage-root", str(tmp / "stage")]
                )
            self.assertEqual(code, 1)
            self.assertIn("500", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()

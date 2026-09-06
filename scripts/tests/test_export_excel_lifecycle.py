"""Opt-in native Excel lifecycle regressions (#1592).

Run OFFICE2PDF_TEST_EXCEL=1 python3 -m unittest discover -s scripts/tests
-p test_export_excel_lifecycle.py. Excel, Poppler and MuPDF must be installed.
The test owns its sandbox workbooks, preserves other open workbooks, and restores
Excel's prior display-alert setting. A legacy application-wide quit is replaced
with error -128 in the test copy, safely reproducing the observed shutdown
failure without allowing the old exporter to close unrelated user workbooks.
The fixed exporter has no quit, so its test copy is byte-identical to the source.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
EXPORTER = ROOT / "scripts/macos/export_excel_pdfs.applescript"
FIXTURE = ROOT / "tests/fixtures/xlsx/issue_1181_fit_to_height.xlsx"
ENABLED = sys.platform == "darwin" and os.environ.get("OFFICE2PDF_TEST_EXCEL") == "1"


def apple(source: str) -> str:
    return subprocess.check_output(["osascript", "-e", source], text=True, timeout=180).strip()


def quoted(value: Path | str) -> str:
    return json.dumps(str(value))


@unittest.skipUnless(ENABLED, "requires opt-in native Microsoft Excel")
class ExcelLifecycleTests(unittest.TestCase):
    def setUp(self):
        parent = Path.home() / "Library/Containers/com.microsoft.Excel/Data/probes"
        parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="issue-1592-", dir=parent)
        self.stage = Path(self.temp.name)
        self.original_alerts = apple('tell application "Microsoft Excel" to get display alerts')
        self.sentinel = self.stage / (self.stage.name + "-sentinel.xlsx")
        self.source = self.stage / (self.stage.name + "-input.xlsx")
        self.addCleanup(self.cleanup_excel)
        for path in [self.sentinel, self.source]:
            shutil.copy2(FIXTURE, path)
        self.digest = hashlib.sha256(self.source.read_bytes()).hexdigest()
        apple(f'''tell application "Microsoft Excel"
            open workbook workbook file name {quoted(self.sentinel)} update links do not update links read only true
            set value of range "A1" of worksheet 1 of workbook {quoted(self.sentinel.name)} to "lifecycle sentinel"
            set display alerts to true
        end tell''')
        self.output = self.stage / "pdfs"
        self.script = self.stage / "export.applescript"
        source = EXPORTER.read_text()
        safe, injected = re.subn(r'(?m)^(\s*)quit(?: saving no)?\s*$', r'\1error "Injected final quit rejection" number -128', source)
        self.script.write_text(safe)
        self.injected = injected

    def cleanup_excel(self):
        try:
            try:
                for path in [self.source, self.sentinel]:
                    apple(f'''tell application "Microsoft Excel"
                        if exists workbook {quoted(path.name)} then close workbook {quoted(path.name)} saving no
                    end tell''')
            finally:
                apple(f'tell application "Microsoft Excel" to set display alerts to {self.original_alerts}')
        finally:
            evidence = os.environ.get("OFFICE2PDF_EXCEL_TEST_ARTIFACTS")
            if evidence:
                destination = Path(evidence).resolve() / self._testMethodName
                shutil.copytree(self.stage, destination, dirs_exist_ok=True)
            self.temp.cleanup()

    def export(self, jobs):
        args = ["osascript", str(self.script), str(self.output)]
        for name, path in jobs:
            args.extend([name, str(path)])
        result = subprocess.run(args, text=True, capture_output=True, timeout=180)
        state = {
            "returncode": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "legacy_quit_injections": self.injected,
            "sentinel_open": apple(f'tell application "Microsoft Excel" to get exists workbook {quoted(self.sentinel.name)}'),
            "sentinel_value": apple(f'tell application "Microsoft Excel" to get value of range "A1" of worksheet 1 of workbook {quoted(self.sentinel.name)}'),
            "input_open": apple(f'tell application "Microsoft Excel" to get exists workbook {quoted(self.source.name)}'),
            "alerts": apple('tell application "Microsoft Excel" to get display alerts'),
        }
        (self.stage / "result.json").write_text(json.dumps(state, indent=2) + "\n")
        return result, state

    def check_pdf(self, name):
        files = sorted(self.output.glob(name + "-sheet-*.pdf"))
        self.assertEqual(len(files), 2)
        self.assertTrue(all(f.stat().st_size > 0 for f in files))
        combined = self.stage / (name + ".pdf")
        subprocess.run(["pdfunite", *map(str, files), str(combined)], check=True)
        result = subprocess.run([sys.executable, str(ROOT / "scripts/check_gt_integrity.py"), str(combined), "--source", str(self.source), "--json"], text=True, capture_output=True)
        (self.stage / (name + "-integrity.json")).write_text(result.stdout)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return [ET.tostring(p) for p in ET.fromstring(subprocess.check_output(["mutool", "draw", "-F", "trace", str(combined)], stderr=subprocess.DEVNULL)).findall("page")]

    def check_lifecycle(self, state, alerts="true"):
        self.assertEqual(state["sentinel_open"], "true")
        self.assertEqual(state["sentinel_value"], "lifecycle sentinel")
        self.assertEqual(state["input_open"], "false")
        self.assertEqual(state["alerts"], alerts)
        self.assertEqual(hashlib.sha256(self.source.read_bytes()).hexdigest(), self.digest)

    def test_completed_batch_restores_alerts_and_preserves_other_workbook(self):
        result, state = self.export([("good", self.source)])
        self.check_pdf("good")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.check_lifecycle(state)

    def test_disabled_alert_setting_is_preserved(self):
        apple('tell application "Microsoft Excel" to set display alerts to false')
        result, state = self.export([("good", self.source)])
        self.check_pdf("good")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.check_lifecycle(state, alerts="false")

    def test_failed_workbook_keeps_error_and_later_successful_exports(self):
        result, state = self.export([("before", self.source), ("missing", self.stage / "missing.xlsx"), ("after", self.source)])
        self.assertEqual(self.check_pdf("before"), self.check_pdf("after"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing:", result.stderr)
        self.assertNotIn("Injected final quit rejection", result.stderr)
        self.check_lifecycle(state)


if __name__ == "__main__":
    unittest.main()

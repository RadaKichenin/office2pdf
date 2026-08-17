"""Unit tests for the business golden native-export script (#1128).

Word, PowerPoint and Excel are sandboxed: every source they open and every PDF
they save has to sit inside that app's own container, or macOS raises a
per-file "Grant Access" dialog and an unattended run stalls on the first one
(#1051, #1082). This script drives all three apps in one run, so it needs three
containers, not one shared stage. The tests below pin that staging contract and
the copy back out to the caller's stage root, with `osascript` and the Poppler
tools stubbed, so they need neither a Mac nor a copy of Office.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
EXPORT_SCRIPT = REPO_ROOT / "scripts" / "macos" / "export_business_golden_pdfs.sh"
CORPUS_SOURCES = REPO_ROOT / "tests" / "golden_mocks" / "business" / "sources"

BUNDLE_ID_BY_EXPORTER = {
    "export_word_pdfs.applescript": "com.microsoft.Word",
    "export_powerpoint_pdfs.applescript": "com.microsoft.Powerpoint",
    "export_excel_pdfs.applescript": "com.microsoft.Excel",
}

# Stands in for `osascript`: records the argv the exporter was driven with and
# writes the PDFs that invocation promised, so the rest of the pipeline runs.
OSASCRIPT_STUB = r"""#!/usr/bin/env python3
import hashlib
import json
import os
import sys
from pathlib import Path

exporter = Path(sys.argv[1]).name
output_directory = Path(sys.argv[2])
pairs = list(zip(sys.argv[3::2], sys.argv[4::2]))
log = Path(os.environ["OSASCRIPT_LOG"])
calls = json.loads(log.read_text()) if log.exists() else []
# The stage is torn down when the run ends, so the digest of what the app was
# handed has to be taken here, while the file still exists.
calls.append(
    {
        "argv": sys.argv[1:],
        "digests": {
            identifier: hashlib.sha256(Path(source).read_bytes()).hexdigest()
            for identifier, source in pairs
        },
    }
)
log.write_text(json.dumps(calls))

output_directory.mkdir(parents=True, exist_ok=True)
for identifier, source in pairs:
    if exporter == "export_excel_pdfs.applescript":
        names = [f"{identifier}-sheet-01.pdf", f"{identifier}-sheet-02.pdf"]
    else:
        names = [f"{identifier}.pdf"]
    for name in names:
        (output_directory / name).write_bytes(
            b"%PDF-1.7 " + Path(source).read_bytes()[:16]
        )
"""

PDFUNITE_STUB = r"""#!/usr/bin/env python3
import sys
from pathlib import Path

*sheets, united = sys.argv[1:]
Path(united).write_bytes(b"".join(Path(sheet).read_bytes() for sheet in sheets))
"""

TRUE_STUB = "#!/bin/sh\nexit 0\n"
SW_VERS_STUB = '#!/bin/sh\ncase "$1" in\n-buildVersion) echo 25G99 ;;\n*) echo 26.6.1 ;;\nesac\n'
SHASUM_STUB = '#!/bin/sh\nif [ "$1" = "-a" ]; then shift 2; fi\nfor path in "$@"; do\n  echo "0000000000000000000000000000000000000000000000000000000000000000  $path"\ndone\n'
PLIST_BUDDY_STUB = "#!/bin/sh\necho 16.90\n"


def write_stub(path: Path, body: str) -> None:
    path.write_text(body)
    path.chmod(0o755)


def source_digests(extension: str) -> dict[str, str]:
    return {
        source.stem: hashlib.sha256(source.read_bytes()).hexdigest()
        for source in CORPUS_SOURCES.joinpath(extension).glob(f"*.{extension}")
    }


class ExportRun:
    """One stubbed run of the export script against a throwaway HOME."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.home = root / "home"
        self.stage_root = root / "staged"
        self.bin = root / "bin"
        self.log = root / "osascript-calls.json"
        self.bin.mkdir(parents=True)
        self.home.mkdir(parents=True)
        write_stub(self.bin / "osascript", OSASCRIPT_STUB)
        write_stub(self.bin / "pdfunite", PDFUNITE_STUB)
        write_stub(self.bin / "pdfinfo", TRUE_STUB)
        write_stub(self.bin / "sw_vers", SW_VERS_STUB)
        write_stub(self.bin / "shasum", SHASUM_STUB)
        write_stub(self.bin / "plistbuddy", PLIST_BUDDY_STUB)

    def run(self) -> subprocess.CompletedProcess:
        environment = dict(os.environ)
        environment.update(
            HOME=str(self.home),
            PATH=f"{self.bin}:{environment['PATH']}",
            OSASCRIPT_LOG=str(self.log),
            PLIST_BUDDY_BIN=str(self.bin / "plistbuddy"),
        )
        return subprocess.run(
            ["bash", str(EXPORT_SCRIPT), str(self.stage_root)],
            capture_output=True,
            text=True,
            env=environment,
            timeout=300,
        )

    @property
    def containers(self) -> Path:
        return self.home / "Library" / "Containers"

    def calls(self) -> list[dict]:
        return json.loads(self.log.read_text())


class StagingContractTest(unittest.TestCase):
    """Where the sandboxed apps are pointed (#1128)."""

    def setUp(self) -> None:
        self._tmp = TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.run_context = ExportRun(Path(self._tmp.name))
        self.result = self.run_context.run()
        self.assertEqual(
            self.result.returncode,
            0,
            f"export script failed: {self.result.stdout}{self.result.stderr}",
        )

    def test_every_path_an_office_app_touches_sits_in_its_own_container(self):
        calls = self.run_context.calls()
        self.assertEqual(len(calls), 3, "one exporter invocation per format")
        for call in calls:
            argv = call["argv"]
            exporter = Path(argv[0]).name
            bundle_id = BUNDLE_ID_BY_EXPORTER[exporter]
            container = self.run_context.containers / bundle_id / "Data"
            for argument in argv[1:]:
                if "/" not in argument:
                    continue
                self.assertTrue(
                    Path(argument).is_relative_to(container),
                    f"{exporter} was handed {argument}, outside {bundle_id}",
                )

    def test_each_app_opens_a_byte_identical_copy_of_its_own_sources(self):
        # Staging must copy the corpus, not stand in for it: every source the
        # app is handed has to be the tracked fixture, byte for byte.
        expected = {
            "export_word_pdfs.applescript": source_digests("docx"),
            "export_powerpoint_pdfs.applescript": source_digests("pptx"),
            "export_excel_pdfs.applescript": source_digests("xlsx"),
        }
        for call in self.run_context.calls():
            exporter = Path(call["argv"][0]).name
            self.assertEqual(call["digests"], expected[exporter], exporter)

    def test_the_containers_are_left_clean_when_the_run_finishes(self):
        # A staged copy of the corpus left inside a container outlives the run
        # and is the next run's stale input.
        for bundle_id in BUNDLE_ID_BY_EXPORTER.values():
            container = self.run_context.containers / bundle_id / "Data"
            leftovers = sorted(
                path.name
                for path in container.rglob("*")
                if path.suffix in {".docx", ".pptx", ".xlsx", ".pdf"}
            )
            self.assertEqual(leftovers, [], f"{bundle_id} still holds {leftovers}")

    def test_the_stage_root_layout_is_unchanged(self):
        stage_root = self.run_context.stage_root
        for extension, subdirectory in (("docx", "docx"), ("pptx", "pptx")):
            for source in sorted(CORPUS_SOURCES.joinpath(extension).glob(f"*.{extension}")):
                pdf = stage_root / subdirectory / f"{source.stem}.pdf"
                self.assertTrue(pdf.is_file(), f"missing {pdf}")
                self.assertTrue(pdf.read_bytes().startswith(b"%PDF-1.7 "))
        for source in sorted(CORPUS_SOURCES.joinpath("xlsx").glob("*.xlsx")):
            sheets = sorted(
                (stage_root / "xlsx-sheets").glob(f"{source.stem}-sheet-*.pdf")
            )
            self.assertEqual(len(sheets), 2, f"missing sheet PDFs for {source.stem}")
            united = stage_root / "xlsx" / f"{source.stem}.pdf"
            self.assertEqual(
                united.read_bytes(),
                b"".join(sheet.read_bytes() for sheet in sheets),
            )
        provenance = (stage_root / "provenance.txt").read_text()
        self.assertIn("Microsoft Word=16.90", provenance)
        self.assertIn("macos=26.6.1 build 25G99", provenance)
        self.assertIn(str(stage_root / "docx" / "01_invoice_en.pdf"), provenance)

    def test_the_stage_root_is_never_handed_to_a_sandboxed_app(self):
        # The caller's stage root is the one path the script cannot control;
        # the Poppler steps that read it afterwards are not sandboxed.
        stage_root = str(self.run_context.stage_root)
        for call in self.run_context.calls():
            for argument in call["argv"][1:]:
                self.assertFalse(
                    argument.startswith(stage_root),
                    f"{argument} exposes the stage root to a sandboxed app",
                )


if __name__ == "__main__":
    unittest.main()

"""Unit tests for the PowerPoint chart-axis measurement harness (#1082).

Microsoft PowerPoint is sandboxed: every package it opens and every PDF it
saves has to sit inside its own container, or macOS raises a per-file "Grant
Access" dialog and an unattended run stalls on the first one (#1051). These
tests pin that staging contract — and the copy back out of the container — with
the export stage stubbed, so they need neither a Mac nor a copy of PowerPoint.
"""

from __future__ import annotations

import io
import sys
import unittest
import zipfile
from contextlib import redirect_stdout
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import measure_powerpoint_chart_axis as chart_axis
import probe_harness


class StageLocationTest(unittest.TestCase):
    """Where the run stages the probes PowerPoint has to open (#1082)."""

    def test_the_stage_sits_inside_the_powerpoint_sandbox_container(self):
        stage = chart_axis.container_stage()
        container = probe_harness.default_stage_root("office", ".pptx")
        self.assertEqual(stage.parent, container)
        self.assertIn(
            probe_harness.OFFICE_BUNDLE_ID_BY_EXTENSION[".pptx"], stage.parts
        )

    def test_the_stage_is_its_own_directory_under_the_container(self):
        # A concurrent probe_harness run stages siblings there; sharing one
        # directory would let either run delete the other's packages.
        self.assertNotEqual(
            chart_axis.container_stage(),
            probe_harness.default_stage_root("office", ".pptx"),
        )


class MirrorTest(unittest.TestCase):
    def test_probes_and_pdfs_are_copied_out_of_the_container(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            stage = tmp / "container" / "powerpoint-chart-axis"
            (stage / "pptx").mkdir(parents=True)
            (stage / "pdf").mkdir(parents=True)
            (stage / "pptx" / "maximum-8p2.pptx").write_bytes(b"PK probe")
            (stage / "pdf" / "maximum-8p2.pdf").write_bytes(b"%PDF probe")
            destination = tmp / "keep"

            chart_axis.mirror_stage(stage, destination)

            self.assertEqual(
                (destination / "pptx" / "maximum-8p2.pptx").read_bytes(), b"PK probe"
            )
            self.assertEqual(
                (destination / "pdf" / "maximum-8p2.pdf").read_bytes(), b"%PDF probe"
            )


class RunTest(unittest.TestCase):
    """A full run: nothing PowerPoint touches may sit outside the container."""

    def stubbed_run(self, tmp: Path, argv: list[str]) -> tuple[dict, str]:
        stage = tmp / "container" / "powerpoint-chart-axis"
        seen: dict = {}

        def fake_export(probes, out_dir: Path) -> None:
            seen["probes"] = [str(probe) for _, probe in probes]
            seen["out_dir"] = str(out_dir)
            for identifier, _ in probes:
                (out_dir / f"{identifier}.pdf").write_bytes(b"%PDF probe")

        buffer = io.StringIO()
        with patch.object(chart_axis, "container_stage", return_value=stage), patch.object(
            chart_axis, "export_probes", fake_export
        ), patch.object(
            chart_axis, "value_axis_labels", return_value=[0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
        ), redirect_stdout(buffer):
            self.assertEqual(chart_axis.main(argv), 0)
        seen["stage"] = stage
        return seen, buffer.getvalue()

    def test_every_path_handed_to_powerpoint_sits_inside_the_stage(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            seen, report = self.stubbed_run(tmp, ["8.2", "1.9"])

            self.assertEqual(len(seen["probes"]), 2)
            for probe in seen["probes"]:
                self.assertTrue(
                    probe.startswith(str(seen["stage"])),
                    f"probe staged outside the container: {probe}",
                )
                self.assertTrue(zipfile.is_zipfile(probe) or not Path(probe).exists())
            self.assertTrue(seen["out_dir"].startswith(str(seen["stage"])))
            self.assertIn("10", report)

    def test_the_container_stage_is_removed_when_the_run_finishes(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            seen, _ = self.stubbed_run(tmp, ["8.2"])
            self.assertFalse(seen["stage"].exists())

    def test_keep_copies_the_artifacts_out_before_the_stage_is_cleaned(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            keep = tmp / "keep"
            seen, _ = self.stubbed_run(tmp, ["8.2", "--keep", str(keep)])

            self.assertTrue((keep / "pptx" / "maximum-8p2.pptx").is_file())
            self.assertTrue((keep / "pdf" / "maximum-8p2.pdf").is_file())
            self.assertFalse(seen["stage"].exists())


class ExportCommandTest(unittest.TestCase):
    def test_the_exporter_is_driven_with_the_staged_paths(self):
        with TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            probe = tmp / "maximum-8p2.pptx"
            probe.write_bytes(b"PK probe")
            recorded: list[list[str]] = []

            with patch.object(
                chart_axis.subprocess, "run", lambda *args, **kwargs: recorded.append(list(args[0]))
            ):
                chart_axis.export_probes([("maximum-8p2", probe)], tmp / "pdf")

            self.assertEqual(recorded[0][0], "osascript")
            self.assertTrue(recorded[0][1].endswith("export_powerpoint_pdfs.applescript"))
            self.assertEqual(recorded[0][2], str(tmp / "pdf"))
            self.assertEqual(recorded[0][3:], ["maximum-8p2", str(probe.resolve())])


if __name__ == "__main__":
    unittest.main()

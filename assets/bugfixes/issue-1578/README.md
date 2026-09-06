# Chart axis and marker paint order

The category axis now paints before line/scatter series and their markers.
An opaque selected-period circle covers the rule beneath it, matching native
Excel. Hollow and disabled markers retain the visible rule. Geometry, colors,
opacity, widths and values are unchanged.

## Native evidence

Source: `tests/fixtures/xlsx/issue_1181_fit_to_height.xlsx`, SHA-256
`2b4a2d8dceda58758593c88409875efbda05780559154c02bd13fef4f7a1c65b`.
Native Excel 16.112.3 and the converter use Excel's DFonts. The horizontal
category rule declares 0.25pt at 50% opacity; the selected-period marker
declares 14pt with its own opaque fill and 0.75pt outline.

Five native cases cover the original workbook, unchanged re-zip control,
red fill, no fill, and disabled marker. Each exports the two printable sheets
and passes the integrity gate. Native and converter re-zip controls have
identical complete paint traces. The native export script returned -128 at
its final quit after writing all ten sheet PDFs; the completed sheets were
preserved, united in workbook order, and independently validated.

In the red control's page-2 trace, native paints the axis before the marker.
The old output reversed that order; the fixed output agrees with native.
All five before/after cases preserve every paint primitive, including its
geometry and paint, and have identical page-1 traces. Layout and text-layer
reports retain their baseline findings. Non-whitespace text matches native;
the stricter text-layer report retains eight extra space-census entries over
the five-page collection, so it is not an exact text-layer match.

A portable compiled-frame regression covers opaque, hollow and disabled
markers at two overlapping values. The four enabled-marker cases fail before
the fix and pass after it; disabled markers remain absent and the category
rule remains present.

## Collection

| Page | Source |
| --- | --- |
| 1 | Original workbook, first printed sheet |
| 2 | Original workbook, second printed sheet |
| 3 | Red-fill control, second printed sheet |
| 4 | No-fill control, second printed sheet |
| 5 | Disabled-marker control, second printed sheet |

Before is the implementation at `39111d5b82861b3f9fd3d2d46d4bd49a83628a82`.
The candidate was also rebuilt on `75d40462e6abdb9388b0485214ca75a012aa3e8b`;
all five two-page probe paint traces remained identical. All 2,052 material
clusters across the five collection pages have explicit passing dispositions.
The original issue-report `compare.jpg` is retained. Full-page evidence uses
pdftoppm at 300 DPI, progressive JPEG quality 86, and metadata stripped before
restoring the density. The layout audit uses a 0.5pt fine-detail threshold.

Remaining independent findings: chart frame #1272; fitted drawing origin
#1542; column width #1543; fixed-row text #1545; title-row height #1550;
wrapped-note pitch #1551; footer baseline #1552; worksheet rules #1564;
missing chart separators #1566; cached marker value #1577. The disabled-marker
crop exposes the separately tracked round-cap/join loss #1590. Native marker
and sparkline rasterization also differs from vector output. These findings
must not be mistaken for the corrected axis-over-marker order.

## Reproduction

Run `python3 scripts/probe_harness.py scripts/probes/issue-1578-marker-overlap.json --backend office`.
For each source check the native export with
`python3 scripts/check_gt_integrity.py NATIVE.pdf --source SOURCE.xlsx --json`.
Build each version with `cargo build --locked -p office2pdf-cli`, preserve its
binary, and convert with `office2pdf SOURCE.xlsx -o OUTPUT.pdf --font-path FONT_DIRECTORY`.
Use Microsoft Excel.app's `Contents/Resources/DFonts` directory. Assemble
native and converter collections in the table's order with `mutool merge`.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`. Their existing
findings are described above. For every page run
`python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 300 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`.
Inspect full pages, crops and pixel sweeps, then rerun with
`--cluster-dispositions PATH --strict-clusters` using reviewed exact IDs.

# Declared chart-series stroke geometry

Chart-series lines and their legend samples now retain the series' declared
round, flat, or square cap and round, miter, or bevel join. Miter limits are
carried through to the rendered stroke. Values, color, width, marker styling,
and all other paint primitives remain unchanged.

## Native evidence

Source: `tests/fixtures/xlsx/issue_1181_fit_to_height.xlsx`, SHA-256
`2b4a2d8dceda58758593c88409875efbda05780559154c02bd13fef4f7a1c65b`.
Native Excel 16.112.3 and the converter use Excel's DFonts. The cash-flow
series declares 2.25pt, fitted to 1.755pt on the worksheet. Before is main
`38a69278f2ab911eaa258845d0191cc65fe13bd6`; its complete paint traces match
the frozen #1578 candidate used to prepare these probes.

The original and unchanged re-zip control have identical complete native and
converter paint traces. One-factor controls change the local cap to flat or
square, or the join to miter or bevel. A separate omitted-limit miter control
has complete native and converter page traces identical to the explicit
`lim="800000"` control: native exports limit 8. This is measured Excel behavior.
All seven required cases agree with native after the fix. Every native
workbook exports two sheets and passes the integrity gate. The exporter
returned -128 only at its final quit; completed sheet PDFs were preserved,
united in workbook order, and independently validated. That exporter bug is
tracked in #1592.

Fifteen before/after cases, including related-style controls, preserve every
paint primitive except the cash-flow stroke's cap, join and miter limit. Their
first pages are wholly unchanged. Layout and text-layer findings are identical
to the baseline. Non-whitespace text matches with `pdftotext -layout`. Default extraction
retains baseline table reading-order differences and ten extra space-census
entries, so this is not an exact text-layer match.

A portable compiled-frame regression covers six cap/join/limit combinations
across line, mixed column/line, and radar charts. It reads both plot and legend
strokes after compilation, verifies their widths, and prevents nested marker
or data-point styles from replacing the series declaration. The regression
fails before the implementation and passes afterward.

## Collection

| Page | Source |
| --- | --- |
| 1 | Original workbook, first printed sheet |
| 2 | Original workbook, second printed sheet; round cap and join |
| 3 | Flat-cap control, second printed sheet |
| 4 | Square-cap control, second printed sheet |
| 5 | Miter-join control, second printed sheet; limit 8 |
| 6 | Bevel-join control, second printed sheet |

Page 5 also represents the omitted-limit control, whose complete paint traces
match the explicit-limit control. Full-page evidence uses pdftoppm at 300 DPI,
progressive JPEG quality 86, metadata stripped before restoring the density,
and a 0.5pt fine-detail layout threshold. All 2,533 material clusters have explicit passing dispositions across the
six accompanying reports. Full contexts, original-scale endpoint/join/marker
crops and final JPEG typography/rule crops were visually inspected.

Remaining independent findings: chart frame #1272; fitted drawing origin
#1542; column width #1543; fixed-row text #1545; title-row height #1550;
wrapped-note pitch #1551; footer baseline #1552; worksheet rules #1564;
missing chart separators #1566; cached marker value #1577. Native marker and
sparkline bitmap edges also differ from vector output. At this baseline, omitted local stroke
properties did not inherit the related chart style. The follow-up fix and
style-only round/flat/square controls are documented in
[issue #1593](../issue-1593/README.md). Explicit local properties retain precedence.

## Reproduction

Run `python3 scripts/probe_harness.py scripts/probes/issue-1590-series-stroke.json --backend office`.
The portable spec recreates all five measured variant packages byte for byte.
Check each native export with
`python3 scripts/check_gt_integrity.py NATIVE.pdf --source SOURCE.xlsx --json`.
Build each version with `cargo build --locked -p office2pdf-cli`, preserve its
binary, and convert with `office2pdf SOURCE.xlsx -o OUTPUT.pdf --font-path FONT_DIRECTORY`.
Use Microsoft Excel.app's `Contents/Resources/DFonts` directory. Assemble
collections in the table's order with `mutool merge`.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`. Their existing
findings are described above. For every page run
`python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 300 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`.
Inspect full pages, matched crops and pixel sweeps, then rerun with
`--cluster-dispositions PATH --strict-clusters` using reviewed exact IDs.

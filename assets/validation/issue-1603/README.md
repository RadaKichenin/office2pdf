# Source-font chart metrics (#1603)

The public fixture `tests/fixtures/xlsx/issue_1603_gift_budget.xlsx` is the
unchanged attachment from #982, SHA-256
`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`.
The baseline is main `33b71cafb28650e060796aa05b970c5cf77468f3`.

Segoe UI Regular and Bold have a 2210/-514/0 `hhea` line box at 2048 units per
em. Bundled Selawik Regular has 2027/-431/0, despite identical Basic Latin
advances. Using the substitute's line metrics makes filled legend keys too
short and the automatic value-axis gutter too narrow. `font-provenance.json`
records the locally measured font hashes; no Office font binary is distributed.

`native-font-size-controls.json` records native and converter key bounds at
6/9/12/18pt and native horizontal plot getters. The weight control changes only
`a:defRPr/@b`. Tests also exercise source metrics without Office font discovery.
The compiled public-fixture regression checks all three original legend keys
against native bounds with the unchanged 0.5pt gate.

| Printed key height | Native | Before | Candidate |
| --- | ---: | ---: | ---: |
| Fitted, 82% | 4.41690pt | 3.98585pt | 4.41719pt |
| Unscaled, 100% | 5.38650pt | 4.86079pt | 5.38682pt |

All original fitted key bounds are within 0.5pt. Unscaled keys retain the
upward legend-baseline offset tracked in #1607; their corrected height does not
resolve that independent chart-content inset. The 6pt row spacing (#1616),
small legend marker size (#1617), and vector legend marker placement (#1618)
remain open. Plot marker placement remains #1577.

## Reproduction

Run from the repository root with Microsoft Excel available on macOS:

```sh
python3 scripts/probe_harness.py assets/validation/issue-1603/legend-size-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1603/axis-size-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1603/legend-weight-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1603/axis-weight-probe.json --backend office
```

The specs change one chart text style at a time and retain the harness's
no-patch control gate. Their patched chart XML was checked against every
measured control. Repackaging methods can change ZIP hashes without changing
member contents; use the recorded hashes to identify the measured artifacts.
Native PDF exports can also differ in metadata between runs.

Read native rectangles with `mutool draw -F trace`. Plot `inside left` and
`inside width` were read from Excel's `plot area object` immediately after PDF
export and before closing the workbook. The native left getter excludes the
4pt chart-area inset. Do not use its inside-height getter as a printed-height
measurement: it can retain stale geometry after export.

For the unscaled control, change only `xl/worksheets/sheet2.xml`
`pageSetUpPr/@fitToPage` from `true` to `false`; keep `pageSetup/@scale="100"`.
Export with `scripts/macos/export_excel_pdfs.applescript`, staging in Excel's
own container as required by the project rules. The original has two pages;
the unscaled control has three.

Build with `cargo build --locked -p office2pdf-cli`. Convert both sources with
`--font-path '/Applications/Microsoft Excel.app/Contents/Resources/DFonts'`.
Use `compare_layout.py --json --audit --fine-shift 0.5`,
`compare_text_layer.py --json`, and `compare_render.py --dpi 300` on the same
native/current PDFs. Full-page images retain their original dimensions, with
white padding to the widest page before vertical concatenation; supplemental
legend crops show native, baseline, and candidate in that order.

The full workbook retains independent defects #1543, #1577, #1598, #1600,
#1604, #1605, #1606, #1607, and #1618. A strict cluster report means every
reported cluster has an explicit disposition; it does not mean the workbook
has no remaining visual defect.

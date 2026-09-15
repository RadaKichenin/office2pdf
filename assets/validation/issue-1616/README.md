# Excel bottom-legend entry widths (#1616)

The public fixture `tests/fixtures/xlsx/issue_1603_gift_budget.xlsx` is the
unchanged attachment from #982, SHA-256
`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`. Every
controlled package changes one thing in `xl/charts/chart1.xml`: the legend's
`a:defRPr/@sz`, or all four series names at once. The probe specs beside this
file reproduce them; `native-legend-row-measurements.json` records each
package's SHA-256, its native page-2 export's SHA-256, and the legend row read
off that export's `mutool draw -F trace` output.

## Rule

Native Excel for Mac 16.112 advances every entry of a worksheet axis-chart
bottom legend by

    19.2 key + 2.025 gap + label design advance + clearance

with no width floor, and one clearance shared by the row:

    clearance = 6.617 + 0.16 x mean(label design advances of the row)

Twenty-four of the twenty-five measured rows fit within 0.016pt; the 9pt
`Budget` row sits 0.22pt under the rule. The 78pt floor the converter applied
held the 6pt fixture row 12.27 printed points left of native.

| legend | labels | mean label | native clearance | rule |
| --- | --- | ---: | ---: | ---: |
| 9pt | fixture four names | 63.313 | 16.743 | 16.747 |
| 9pt | `It` x4 | 5.445 | 7.486 | 7.488 |
| 9pt | `Gift Budget Amount` x4 | 79.830 | 19.388 | 19.390 |
| 6pt | fixture four names | 42.209 | 13.383 | 13.370 |
| 6pt | `Gift` x4 | 9.480 | 8.138 | 8.134 |
| 18pt | fixture four names | 126.626 | 26.865 | 26.877 |
| 18pt | `Holiday Budget` x4 | 122.670 | 26.236 | 26.244 |

The earlier per-face slopes (#1249) were this share of each face's own mean
label advance, and the 0.16-per-em terminal-glyph effect was the same share of
the four renamed labels' mean. Above 23pt the fixture row leaves the rule: the
24pt row keeps the 22pt clearance and the 28pt row compresses to 8.9pt; that
constrained regime is tracked separately.

## Reproduction

Run from the repository root with Microsoft Excel available on macOS:

```sh
python3 scripts/probe_harness.py assets/validation/issue-1616/legend-size-and-label-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1616/label-width-9pt-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1616/label-width-6pt-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1616/label-width-18pt-probe.json --backend office
```

The 6pt and 18pt label-width specs take a base package with only the legend
size changed; build it from the fixture with the same `sz` patch as the first
spec before running them. Convert with
`--font-path '/Applications/Microsoft Excel.app/Contents/Resources/DFonts'`
and compare page 2 with `compare_layout.py --audit --fine-shift 0.5`,
`compare_text_layer.py`, and `compare_render.py --dpi 300`.

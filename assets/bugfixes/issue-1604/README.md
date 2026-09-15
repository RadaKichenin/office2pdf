# Issue 1604 evidence

`gt.jpg`, `before.jpg` and `after.jpg` show page 2 of the public Gift Budget
and Tracker1.xlsx attachment from #982 (tracked as
`tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`, SHA-256
`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`) at 300 DPI:
the Microsoft Excel for Mac 16.112 export (SHA-256
`2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`), the
output of `main` at `fcb6ca95` before the fix, and the output of the fix. Both
conversions passed `--font-path` for the Office cloud-font cache (Segoe UI) and
Excel's DFonts bundle. Page 1 is pixel-identical before and after.

`layout-audit.json` is the two-page
`compare_layout.py --audit --fine-shift 0.5 --noise-floor 0.5 --json` report and
`render-clusters-page-2.json` the strict 300 DPI cluster report of page 2.

## The defect and the correction

The chart's category axis (`a:ln w="9360"`, 0.60pt at the 0.82 print scale)
and its `Amount Spent` line series (`a:ln w="28440"`, 1.84pt, round caps) share
y=329.80pt wherever the series is zero: February-March and August-December.
Native strokes the axis after the columns and before the line, so the line's
turquoise is the final colour there. The converter stroked the axis after the
line, leaving a grey core between two teal edges.

`generate_chart_axis` now emits the axis lines and their tick marks between the
column loop and the overlaid line-family series, the order the line-chart
generator already used (#1578). Widths, colours, caps, point geometry and marker
placement are unchanged; only the paint order moved, so the before/after pixel
difference on page 2 is confined to the two zero-value runs of the series.

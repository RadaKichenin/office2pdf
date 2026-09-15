# Issue 1600 evidence

`compare.jpg` records the defect: unscaled workbook page 3 at 300 DPI, native
Excel on the left and office2pdf at `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da`
on the right. The centered Delivered? header and Yes cell shift left while
their vertical baselines match.

`gt.jpg`, `before.jpg` and `after.jpg` are the fix evidence for the same page
(page 3, the chart sheet's second horizontal print tile) at 300 DPI: the fresh
native export, the output of `main` at `42240aa3` before the fix, and the
output of the fix. `layout-audit.json` is the all-page
`compare_layout.py --audit --fine-shift 0.5 --noise-floor 0.5 --json` report
and `render-clusters-page-2.json` / `render-clusters-page-3.json` are the
strict 300 DPI cluster reports of both chart-sheet tiles.

Use the public Gift Budget and Tracker1.xlsx attachment from #982 (tracked as
`tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`). Change only
`xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true` to `false`,
leaving `pageSetup/@scale="100"` unchanged. The unscaled package SHA-256 is
`af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`.

## The rule

Excel starts a centred sheet line on a whole sheet point from the cell's left
gridline: `floor((column - W + 2) / 2)` in an unwrapped cell and
`floor((column - W + 1) / 2)` in a wrapped one, where `W` is the sum of the
line's whole-point glyph advances. Probe-measured with
`scripts/probe_harness.py --backend office` (Excel 16.112) on 15 strings of
even and odd `W` in 65, 66, 80, 81, 99 and 100pt columns at 11, 14, 20 and
24pt, every row exact; the same rule reproduces all 191 centred string cells of
the ten business golden mocks, which are all wrapped. The renderer moves each
centred line that fits its column from Typst's exact centring to that origin
(`centered_sheet_line_start_shift_pt`); the individual lines of a wrapped cell
that needs more than one line are #1738.

After the fix every centred single-line cell of both tiles starts within the
tile's own 0.225/0.275pt centring offset of the native origin, the same offset
every left-aligned run carries: Delivered? 486.00 native against 485.78
(before 484.59), Yes 504.00 against 503.78 (before 503.44), Purchased? 980.00
against 980.28 (before 979.09), Month 512.00 against 512.28 (before 511.38).

## Provenance

- Original workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Unscaled workbook SHA-256: `af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`
- Native PDF SHA-256: `0935ca38e497a657d2f972fb7ddfc3163cf69e8edfe6d79d230202b5db5d7bfa`
  (the whole-workbook Excel 16.112 export recorded in #1598)
- Before PDF (`main` at `42240aa3`) SHA-256: `f1d9797c57bda35be11f8b56cbb50a6f425ab10fcd51e6de9e011fcf6195be45`
- After PDF SHA-256: `6364bea3c5389f2a991b174f2d3bd6aca0b8bf78b265bbdcb99e24788455e91e`
- Both conversions pass `--font-path` for the Office cloud-font cache
  (Segoe UI) and Excel's bundled DFonts (Aptos).

The images preserve the source PNG dimensions and use progressive JPEG
quality 86 with metadata stripped and 300 DPI density restored afterward.
Remaining differences on both tiles are tracked in #1607, #1604, #1659, #1735,
#1738 and, for the text layer only, #1736.

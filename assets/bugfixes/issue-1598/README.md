# Issue 1598 evidence

`compare.jpg` records the defect: page 3 at 300 DPI, fresh Microsoft Excel on
the left, office2pdf at `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da` on the
right, with the chart continuation above the table absent from the output.

`gt.jpg`, `before.jpg` and `after.jpg` are the fix evidence for the same page
(page 3, the chart sheet's second horizontal print tile) at 300 DPI: the fresh
native export, the output of `main` at `769042c4` before the fix, and the
output of the fix. `layout-audit.json` is the all-page
`compare_layout.py --audit --fine-shift 0.5 --noise-floor 0.5 --json` report
and `render-clusters-page-2.json` / `render-clusters-page-3.json` are the
strict 300 DPI cluster reports of both chart-sheet tiles.

Source: the public Gift Budget and Tracker1.xlsx attachment in issue #982.
Change only `xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true`
to `false`; leave `pageSetup/@scale="100"` unchanged. The resulting workbook
has three printed pages in both exporters.

The chart's `twoCellAnchor` in `xl/drawings/drawing1.xml` crosses from column 5,
row 2 to column 16, row 3. Native Excel clips it on the first horizontal tile
and paints its continuation on the second. Width pagination now keeps every
anchored chart on each page-column it reaches, shifted left by the width the
earlier tiles printed, and the drawing foreground clips it to that
page-column's window (`SheetChartPlacement::clip_window`).

## Provenance

- Original workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Unscaled workbook SHA-256: `af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`
- Native PDF SHA-256: `0935ca38e497a657d2f972fb7ddfc3163cf69e8edfe6d79d230202b5db5d7bfa`
  (the whole-workbook export; a fresh per-sheet export of the chart sheet on
  2026-09-15 with `scripts/macos/export_excel_pdfs.applescript` is
  pixel-identical to its pages 2 and 3 at 72 DPI)
- Defect-time converter PDF SHA-256: `60788eab1c0aa8af0d55db81be7adc48ec1a7ff97b2a1e148422e2d8da9c9805`
- Before PDF (`main` at `769042c4`) SHA-256: `bcaeec78afedf2bfdbab008b384bcfb839e610478c15493ca552254c2255cb2f`
- Both before and after conversions pass `--font-path` for the Office
  cloud-font cache (Segoe UI) and Excel's bundled DFonts (Aptos).

The images preserve the source PNG dimensions and use progressive JPEG
quality 86 with metadata stripped and 300 DPI density restored afterward.
Remaining differences on both tiles are tracked in #1607, #1604, #1600, #1735
and, for the text layer only, #1736.

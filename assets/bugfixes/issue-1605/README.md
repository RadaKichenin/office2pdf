# Issue 1605 evidence

Page 1 of the public #982 workbook (tracked as
`tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`) at 300 DPI. `gt.jpg` is
the native Microsoft Excel export recorded in #1600, `before.jpg` is
office2pdf at `4919bfc1` (`main` before the fix) and `after.jpg` is the fixed
converter. `compare.jpg` is the original defect evidence at
`6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da`.

Native clips every printed page's cell fills to the page's grid region inset
by one sheet point on the top and left edges. The unscaled first sheet traces
the clip as `[51, 55, 474, 418]`pt around a raw rose title fill of
`[50, 54, 474, 145]`; the fitted 0.82 explicit-area sheet clips at
`[55.76, 54.12]` for a grid origin of `[54.94, 53.30]`; and the unscaled
explicit-area sheet's continuation page clips at x=473 for a raw fill that
starts at x=472 (`/Volumes/T7/scratch/issue-1598/gt/`, #1598). The bottom
and right edges keep the one-point bleed, and later rows and columns keep
their raw origin.

The fix trims the first row's top strip and the first column's left strip of
each page's fill layer by one sheet point (`excel_fill_paint.rs`). After the
fix the rose title paints `[51, 55, 474, 145]` and the pale body starts at
y=144 under it, both matching native; text placement and the page count are
unchanged, and page 2 is pixel-identical before and after.

`layout-audit.json` is `compare_layout.py --page 1 --noise-floor 0.5 --audit
--fine-shift 0.5 --json`: 12 of 12 lines matched, no missing/extra/re-wrapped
text, no fine or large shift, visible fills 0, and the two raw rectangle
findings (the clipped output rectangle against native's unclipped raw
rectangle under its clip) resolve as equivalent visible coverage.
`render-clusters-page-1.json` is the strict `compare_render.py --dpi 300
--fine-shift 0.5` report: 71 of 71 clusters dispositioned, 68 glyph-sized
clusters on the title and body runs to #1659 (whole-point glyph advances
drift about 0.8pt by the end of the long lines) and 3 footer glyph outlines
as glyph-edge rasterization. No cluster touches the fill edges.

## Provenance

- Workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Native PDF SHA-256: `2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`
- Both conversions used `--font-path` for the Office cloud-font cache and
  Excel's DFonts bundle.

The images preserve the source raster dimensions, use progressive JPEG
quality 86 with metadata stripped and 300 DPI density restored afterward.

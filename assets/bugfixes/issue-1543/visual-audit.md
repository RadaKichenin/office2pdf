# Issue #1543 visual audit

Source: `Gift Budget and Tracker1.xlsx` from #982, tracked as
`tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`
(`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`).
The native reference is a fresh Excel for Mac 16.112 export of both worksheets,
concatenated in workbook order; it passes `check_gt_integrity.py`. The before
PDF is the `main` CLI, the after PDF the branch CLI, both run with
`--font-path /Applications/Microsoft\ Excel.app/Contents/Resources/DFonts`.
Hashes are recorded in `assets/validation/issue-1543/provenance.json`.

## Rule

Excel paints every anchored worksheet column edge on a whole unscaled sheet
point and clips the column to the plot rectangle. Ten native exports through
`scripts/probes/issue-1543-column-gap-width.json` (`c:gapWidth` 0, 50, 100,
150, 219, 300, 500) and `scripts/probes/issue-1543-column-overlap.json`
(`c:overlap` 0, 50, -27) paint 50 columns; every edge but one is
`round(continuous edge)` and the remaining one is the gapWidth-0 first column
stopping at the plot's exact left edge. Column tops stay continuous. The full
per-edge table is `assets/validation/issue-1543/native-column-edge-analysis.txt`.

## Before-change enumeration (page 2)

- Page count/order: native, before and after each hold the same two pages.
- Element presence: sidebar, body copy, three gift images, chart, legend, both
  rose title bands, tracker table and footer are present.
- Position/size: the June column is 26.53pt wide at 735.96..762.47pt against
  native 27.06pt at 735.54..762.60pt; the other painted columns (Jan, Apr, Jun
  stack, Jul) carry the same fractional band width. The sixteen centred tracker
  cells sit 0.53..0.79pt left of native (#1600).
- Rotation/flip: none.
- Fill: series colours, title bands and occasion fills match.
- Stroke/border: eleven grey gridlines, the zero axis and the teal series are
  present and solid; the grey axis overpaints the zero-valued teal segments
  (#1604).
- Shape outline geometry: circular markers and rectangular columns.
- Text content: all 34 native trace lines match; no line missing or extra.
- Font family/weight/style: Segoe UI/Aptos substitutions unchanged; titles and
  the five white occasion labels remain bold.
- Text colour, alignment, line spacing: unchanged from #1542's audit.
- Clipping/overflow: none on page 2; the page-1 title band still paints beyond
  the native used-range clip (#1605).

## After-change audit

- All five painted columns now trace at exactly the native page-space edges:
  337.84..364.90, 537.10..563.34, 669.12..696.18 (both June segments) and
  735.54..762.60pt. `compare_layout.py --audit --fine-shift 0.5 --noise-floor
  0.5` reports 0 rectangle-geometry findings on page 2 (1 before), 34/34 lines
  matched, no missing/extra/reflowed lines, no visibility or visible-fill
  findings, and the same 16 fine text shifts before and after (#1600).
- Page 1 is pixel-identical before and after (exact AE 0). Page 2 differs by
  2,183 exact pixels, all on the column edges.
- The selectable-text census is identical before and after; the two-space
  delta against native is the native `Birt hday Bu dget` extraction artifact
  recorded in #1510.
- `compare_render.py --page 1/2 --dpi 300 --fine-shift 0.5 --strict-clusters`
  passes with 72 and 34 clusters dispositioned. Page 1 keeps the 71 glyph-edge
  IDs reviewed in #1542 plus the smaller title-band rim of #1605. Page 2 keeps
  32 reviewed glyph-edge IDs; the six #1599 fill seams are gone since #1615,
  and the two remaining zero-line clusters are the #1604 axis overpaint.
- Full pages, the 5% diff, and matched 300 DPI crops of the Jan..Apr and
  Jun..Jul columns and of the Feb..Mar and Aug..Dec zero line were inspected.

## Evidence

- Full pages 1 and 2 stacked: `gt.jpg`, `before.jpg`, `after.jpg` (300 DPI,
  progressive JPEG, quality 86, metadata stripped)
- Vector/layout audit: `layout-audit.json`
- Strict pixel census: `render-clusters-page-1.json`, `render-clusters-page-2.json`

# Worksheet fill paint order (#1599)

The public [workbook attached to #982](https://github.com/user-attachments/files/30941041/Gift.Budget.and.Tracker1.xlsx)
has SHA-256 `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`.
Native references were exported with Microsoft Excel for Mac 16.112. The original
prints two pages; a control changing only sheet2 `pageSetUpPr/@fitToPage` to
`false`, retaining `pageSetup/@scale="100"`, prints three. Control SHA-256:
`af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`.
The baseline is `64433df1fac9ae65d84a8347bfad1d2819b5260e`; its converter code is
unchanged from the verified #1542 CLI. [Provenance and control evidence](../../validation/issue-1599/)
record input, reference, baseline, candidate and output hashes.

## Cause and correction

G7:G11 select conditional fills through dxfIds 11–14; their direct borderId 0
has no stroke. Earlier cells' bottom/right extensions were painted as child
content after every main table fill, so they covered later neighboring colors.
The same cause affects the two sidebar/rose joins and the first sheet's rose
title/pale body boundary. E2/E5 use fillId 3; the adjoining merged rose bands use
fillId 2. A1:B1 uses fillId 2; A2/B2 use fillId 3. These are fill boundaries,
not declared borders.

The correction expands the completed Excel table fill rectangles within their
original background layer and paints those fills in row order, then column
order. Table labels and source spans restrict the pass to Excel boundary-band
tables. Borders, text, other table types and frame positions retain their paint
slots. Using completed layout rectangles covers automatic rows, merged cells
and split table frames without a second alpha application.

| Boundary | Native | Before | After |
| --- | --- | --- | --- |
| Four fitted Occasion row seams (pt) | 478.06, 502.66, 527.26, 551.86 | Native +0.82 | Native |
| Two fitted sidebar/rose joins, x (pt) | 276.34 | 277.16 | 276.34 |
| First-sheet title/body seam, y (pt) | 144 | 145 | 144 |
| Four unscaled Occasion row seams (pt) | 572, 602, 632, 662 | Native +1 | Native |
| Two unscaled sidebar/rose joins, x (pt) | 329 | 330.275 | 329.275 |

The unscaled grid's existing 0.275pt x phase is preserved. The fix removes its
additional 1pt overpaint; it does not claim exact horizontal coordinate parity.
The audit gate remains **0.5pt**. The diagnostic initially sampled at native
x329 ±0.1pt and crossed this existing phase; the retained control proof records
that failed probe and samples ±0.5pt, without changing the geometry gate.
Both sides of every tested boundary have the expected final color. See
[primary](../../validation/issue-1599/boundary-color-proof.json) and
[control](../../validation/issue-1599/control-boundary-color-proof.json) color proofs.

All 130 original and 133 control text-paint operations are byte-identical
before/after. Selectable text also remains identical. Native extraction splits
`Birthday Budget` into `Birt hday Bu dget`; the control additionally retains the
known chart-label pagination mismatch #1598. Neither is introduced by this fix.

## Reproduction and checks

Convert both packages with the baseline/current CLI and Excel's DFonts folder:
`--font-path /Applications/Microsoft\ Excel.app/Contents/Resources/DFonts`.
Use the tracked lockfile for builds. Export both printable sheets from native
Excel and concatenate in workbook order. Run:

- `compare_layout.py --json --audit --fine-shift 0.5 GT.pdf output.pdf`
- `compare_text_layer.py --json GT.pdf output.pdf`
- `compare_render.py GT.pdf output.pdf --page N --dpi 300 --fine-shift 0.5 --cluster-report REPORT --cluster-dispositions DISPOSITIONS --strict-clusters`

Repeat the render comparison for all five compared pages. The strict reports
retain exact cluster IDs and dispositions. Original `gt.jpg`, `before.jpg` and
`after.jpg` stack two pages; supplemental `control-*.jpg` stack three. Each page
retains its 300 DPI pixel dimensions, with white padding for the narrower first
page. JPEGs are progressive quality 86, metadata stripped, density reset to 300.
The existing `compare.jpg` remains the original filing evidence.

Eight compiled-paint cases failed before the correction: normal/merged cells,
row/column adjacency, scales 1 and 0.82. They now verify final visible ownership
and outer extension. Additional tests cover the winning merged bottom border,
row order at shared corners, single-alpha automatic rows, and isolation from
Word/centered-stroke tables. The real `ExcelTables.xlsx` fill-extension
regression now checks compiled bounds instead of the removed overlay markup.
Current validation: 2,884 library tests (including the 999 generator tests),
206 XLSX fixture tests, workspace Clippy with warnings denied, formatting and diff checks.

## Full visual audit and model vision findings

All original/control GT and output pages, 5% diffs, every material cluster crop,
36 large-cluster panels and all 22 emitted text-shift crops were opened and
inspected. Large strips were divided into matched panels for fine-detail review.
The checklist below covers page order, presence, position, size, rotation/flip,
fill, stroke/dashes, outline geometry, text, font/emphasis/color, alignment,
line/paragraph spacing and clipping. It records remaining defects rather than
claiming full workbook parity.

- Original page 1 and control page 1 were inspected separately. Title, body,
  Note and footer remain in order with matching text and wrapping. Note remains
  bold; other runs are regular, with no italic or underline. Glyph interiors,
  color and paragraph spacing remain. The title/body transition matches native;
  only the connected top/left background clip remains #1605. Title shape/size
  and orientation otherwise agree. No thin rules are present.
- Original page 2 preserves both rose headings, sidebar wraps, five table rows,
  complete chart/legend, three gift packages and footer. The four Occasion
  seams are continuous and level with native; the two pale sidebar strips no
  longer cover rose fills. All five white Occasion labels retain bold weight
  and baselines. Other table and chart text stays regular. The photo keeps its
  position, size and orientation, with native resampling versus sharper output.
  Centered-cell shifts remain #1600; June bar width #1543 and filled legend key
  height/placement #1603 remain. Zero-series crops show a grey interior stroke
  in output (#1604) and displaced marker centers (#1577).
- Control page 2 preserves the same content and emphasis. All four row seams
  now meet at native y; both sidebar joins retain only the 0.275pt grid phase.
  Nine sidebar lines remain 1.14355pt above native, preserving 21pt pitch and
  wraps (#1606). Chart gridlines/series retain their vertical phase difference
  (#1607). The chart still crosses the right tile margin and shows October on
  the first tile (#1598). Individual centered-cell shifts remain #1600; photo
  texture differences are resampling with matching pose and bounds.
- Control page 3 retains both rose bands, Delivered/Notes headings, all five row
  entries and footer. Native's chart continuation (eleven horizontal grey rules,
  turquoise zero series, three markers and Oct/Nov/Dec labels) is absent from
  output (#1598). Delivered? and Yes retain their horizontal shifts (#1600).
  Present text remains regular; no emphasis or underline is missing. Small rose
  edge raster residuals retain their exact-ID rendering disposition.

Hairline inventory: eleven solid horizontal chart gridlines are present on the
first chart tile at both scales; their phase differs in the control (#1607).
The category axis overlays the coincident turquoise series (#1604); marker
placement is separately #1577. The control continuation's eleven rules are
missing (#1598). No dash pattern, thin table border or underline is declared in
these compared regions. Colored cell seams are fill ownership, not strokes.
No new rotation, flip, outline, font-family, weight, color or clipping defect was
found in the current comparison.

## Dispositions and remaining findings

Original after clusters: **72/72**, **34/34**. Control: **72/72**, **158/158**,
**24/24**. Six seam clusters disappear from each page-2 comparison. The page-1
connected cluster changes ID because the bottom overlap is gone; its remaining
top/left edge is assigned only to #1605. Every other current ID was individually
re-audited against the corresponding region, with no page-wide approval.

The current layout harness reports fine text shifts [0,16] in the original and
[0,39,2] in the control, preserving #1600/#1606/#1607. Each page 2 has four
rectangle findings: three filled legend keys (#1603) and the June bar (#1543,
with control vertical placement also #1607). Original has no missing/extra text,
reflow or visibility mismatch. Control chart category lines have missing/extra
pagination findings (#1598). Neither package has a changed-wrap, painted-text
visibility or visible-fill occlusion finding. The primary and control layout
commands therefore intentionally return nonzero for those remaining defects.

Remaining converter defects: #1543, #1577, #1598, #1600, #1603, #1604, #1605,
#1606 and #1607. The corrected audit limitations #1608/#1609 are already merged;
no remaining finding is assigned to them or to the fixed #1599 paint order.

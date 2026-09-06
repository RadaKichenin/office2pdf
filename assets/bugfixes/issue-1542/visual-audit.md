# Fitted worksheet drawing origin (#1542)

The public [Gift Budget and Tracker1.xlsx attachment](https://github.com/user-attachments/files/30941041/Gift.Budget.and.Tracker1.xlsx)
from #982 is unchanged. SHA-256:
`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`.
The baseline is `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da`.
Native references were exported with Microsoft Excel for Mac 16.112; both
exports passed `check_gt_integrity.py`. Converter runs use the Excel DFonts
folder and the tracked lockfile. Exact file hashes and supplemental control evidence are in
[`assets/validation/issue-1542`](../../validation/issue-1542/).

## Scope and reproduction

The fitted drawing foreground kept the physical page origin after the sheet
grid moved to its fitted origin. The correction applies the sheet paint offset
to anchored images/text boxes and chart area paint. It preserves the chart's
existing content calibration, including plot, text and legend positions. The
unscaled content-inset discrepancy is separately tracked in #1607; this change
does not establish a native rule that chart content has a different origin.

The original workbook has two pages. The control changes only
`xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true` to `false`,
retaining `pageSetup/@scale="100"`; it prints three pages. Control SHA-256:
`af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`.

For each package, export both printable sheets from native Excel and concatenate
in workbook order. Convert the same package with the frozen baseline and current
CLI using `--font-path /Applications/Microsoft\ Excel.app/Contents/Resources/DFonts`.
Run `compare_layout.py --json --audit --fine-shift 0.5`,
`compare_text_layer.py --json`, and `compare_render.py --page N --dpi 300
--fine-shift 0.5 --cluster-report REPORT --cluster-dispositions DISPOSITIONS
--strict-clusters` for every page. The strict reports retain each disposition's
exact cluster ID and rationale.

## Verified correction

| Page-2 paint | Native | Before | After |
| --- | --- | --- | --- |
| Chart frame top-left (pt) | 289.44970, 117.27030 | 289.63468, 117.97034 | 289.44969, 117.27033 |
| Gift image top-left (pt) | 63.90705, 436.57063 | 64.09206, 437.27055 | 63.90706, 436.57060 |

The frame dimensions remain within 0.0004pt of native. Image dimensions remain
within 0.00002pt; [image-origin.json](../../validation/issue-1542/image-origin.json) records the full transforms. No tolerance
was widened. Page 1 has exactly zero changed decoded pixels at 300 DPI.
The unscaled candidate PDF is byte-identical to its baseline, so its three
page renders and unchanged strict dispositions also validate the candidate.
The original and control page-1 native/output rasters are separately verified
pixel-identical, allowing the same page-1 census to be reused.

All original-workbook selectable text is identical before/after. Native alone
extracts the visually correct legend label as `Birt hday Bu dget`; the converter
extracts `Birthday Budget`. This native extraction segmentation is not converter
text loss.

The six focused compiled-paint regressions cover fitted frame/image bounds,
unscaled bounds, chart text, grid positions, filled legend geometry, and text
flow at 0.64/0.78 scales with Column/Bar/Line charts. After the Clippy correction,
`cargo test --locked -p office2pdf --lib render::typst_gen` passed 1,000 tests,
and workspace Clippy passed with warnings denied. The rebuilt CLI produced
byte-identical original and unscaled PDFs, preserving this visual evidence.
Earlier candidates that moved the matching plot or line legend were
rejected; the retained candidate changes only the intended paint origin.

## Full visual audit

Every full GT/output page, the 5% diff, and matched text/chart/image regions
were inspected at 300 DPI. The checklist covers presence, position, size,
rotation/flip, fill, stroke/dashes, outline, text, font/emphasis/color, alignment,
spacing and clipping. This is a dispositioned audit, not a claim that the
workbook has no remaining defects.

- Original page 1 / control page 1: title, all body paragraphs and footer remain
  in order. Title/body text stays regular and `Note:` remains bold. No italic,
  underlined run or ruled border is present. Glyph interiors and line/paragraph
  spacing agree. The title's top/left clip differs (#1605); its bottom rose
  extension covers the next pale row (#1599). Both causes share one connected
  pixel cluster. The pale body's raw rectangle mismatch is the composition
  blind spot in #1608, not missing visible coverage.
- Original page 2: sidebar wraps and regular purple text, two rose headings,
  five table rows, all chart elements, three gift packages and footer remain.
  The corrected image/frame positions agree with native. Native downsamples the
  photograph; converter texture is sharper, with matching pose and size.
  Five Occasion labels remain white and bold; other table/header/chart text is
  regular, with no italic/underline loss. Four Occasion boundaries and two
  sidebar/rose seams retain the fill-order defect #1599. Individual centered
  cells retain #1600 even though the combined-line anchor passes (#1609).
  The June column width is #1543; filled legend height/baseline is #1603.
- Control page 2: the nine wrapped B4 lines sit 1.14355pt above native while
  preserving their 21pt pitch, text, wraps, regular weight and color (#1606).
  Chart content sits about 0.854pt above native (#1607). Native clips the chart
  at the tile edge; output crosses into the margin and includes October on the
  first tile (#1598). Cell fill seams remain #1599. The four material centered
  cell clusters are #1600; other above-threshold cell shifts are recorded in
  [control-cell-positions.json](../../validation/issue-1542/control-cell-positions.json), including shifts beneath the pixel-cluster floor.
- Control page 3: native chart continuation has eleven solid grey gridlines,
  a turquoise zero line, three markers and Oct/Nov/Dec labels; all are absent
  from output (#1598). Both rose bands, five table rows and footer remain.
  Delivered? and Yes retain their leftward displacement (#1600). Present table
  text remains regular; no bold, italic or underlined run is present.

Hairline inventory: the chart's eleven solid grey horizontal gridlines remain
on each first chart tile, without missing rules or dash changes. The grey
category axis paints over the coincident turquoise series in output, producing
a grey center between two teal edges (#1604). Native paints the series last.
Circular markers have independently different native raster placement (#1577).
Those defects remain in both scales; moving the chart frame does not fix them.
There are no declared thin table rules in these compared regions; the colored
cell seams are fill ownership, not stroke widths.

## Findings and evidence

- Original after clusters: page 1 **72/72**, page 2 **40/40**. Before page 2 has
  **45/45**; the five removed clusters were the displaced gift photograph.
- Control clusters: pages 1/2/3 **72/72**, **164/164**, **24/24**. Every exact ID
  is assigned to a known issue or an inspected renderer difference.
- `layout-audit.json` retains one page-1 and ten page-2 rectangle findings:
  composite fill matching #1608, filled legend keys #1603, June column #1543.
  It reports no missing/extra/reflowed lines, painted visibility mismatches or
  fine combined-line shifts. This does not prove per-cell alignment:
  [cell-positions.json](../../validation/issue-1542/cell-positions.json) exposes #1600, hidden by the combined-line matcher #1609.
- [control-layout-audit.json](../../validation/issue-1542/control-layout-audit.json) retains the same fill/legend/column findings and
  20 page-2 / two page-3 combined-line shifts, assigned to #1606, #1607 and #1600.
  Per-cell control shifts are separately enumerated.
- `gt.jpg`, `before.jpg`, `after.jpg` contain the original pages top-to-bottom;
  the supplemental `assets/validation/issue-1542/control-*.jpg` files contain all
  three control pages. Pages retain their original
  300 DPI pixel dimensions, with white right padding for the narrower first
  page. JPEGs are progressive quality 86, metadata stripped and density reset
  to 300 DPI. No page was resized.

Remaining converter defects: #1543, #1577, #1598, #1599, #1600, #1603, #1604,
#1605, #1606, #1607. Audit harness limitations: #1608, #1609. Fix these separately
without treating a mechanically complete cluster census as full visual parity.

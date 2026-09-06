# Issue 1605 evidence

`compare.jpg` shows the unchanged public #982 workbook's first page at 300 DPI:
native Microsoft Excel on the left, office2pdf at
`6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da` on the right.

Native clips the worksheet fills to `[51,55,474,418]`pt. The raw rose title is
`[50,54,474,145]`, but the clip removes its top and left 1pt. The converter
paints the title from x50/y54 and the pale body from x50. Matched high-DPI
crops show the edge discrepancy while title text positions agree.

The first sheet declares dimension A1:B8, merged title A1:B1, no explicit
print area, fitToPage=false, scale=100, A4 portrait and gridLines=false.
The title's direct style uses borderId 0 without edge strokes. This evidence
shows fill clipping; it does not establish a general rule for other print
areas/scales. Those controls are required before fixing #1605.

The title's bottom edge has a separate paint-order defect (#1599): native's
later pale fill starts at y144, while the converter's late rose extension
covers it through y145. The raw pale-body rectangle mismatch is tracked in
#1608 because fill extensions already provide the visible coverage. Neither
finding is part of the top/left clipping root cause.

All body text and paragraphs remain; `Note:` retains bold emphasis, and other
visible runs stay regular. No italic, underlined run or thin rule is present.
The 72 exact pixel clusters are dispositioned: one connected title-boundary
cluster belongs to #1605 with the bottom overlap explicitly noted as #1599;
the other 71 are inspected glyph-edge rasterization. This is defect evidence,
not a conversion fix or a claim that the complete workbook matches native.

## Provenance

- Workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Native PDF SHA-256: `2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`
- Converter PDF SHA-256: `81b93282a27291eb3ebf77ca02229a4d36e5dabc9916b742a083ce0130763c25`

The comparison retains the original raster dimensions, uses progressive JPEG
quality 86, strips metadata and restores 300 DPI density. No runtime code changes
are included.

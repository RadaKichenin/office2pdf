# Issue 1598 evidence

`compare.jpg` shows page 3 at 300 DPI: fresh Microsoft Excel on the left,
office2pdf at `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da` on the right.
The chart continuation above the table is absent from office2pdf output.
This is defect evidence, not a claim that the workbook passes the visual audit.

Source: the public Gift Budget and Tracker1.xlsx attachment in issue #982.
Change only `xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true`
to `false`; leave `pageSetup/@scale="100"` unchanged. The resulting workbook
has three printed pages in both exporters.

The chart's `twoCellAnchor` in `xl/drawings/drawing1.xml` crosses from column 5,
row 2 to column 16, row 3. Native Excel clips it on the first horizontal tile
and paints its continuation on the second. The converter's
`split_sheet_page_by_width` retains charts only in the first column group.
The drawing foreground also lacks a clip to that group's printable window.

## Provenance

- Original workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Unscaled workbook SHA-256: `af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`
- Native PDF SHA-256: `0935ca38e497a657d2f972fb7ddfc3163cf69e8edfe6d79d230202b5db5d7bfa`
- Converter PDF SHA-256: `60788eab1c0aa8af0d55db81be7adc48ec1a7ff97b2a1e148422e2d8da9c9805`

The comparison preserves the source PNG dimensions and uses progressive JPEG
quality 86 with metadata stripped and 300 DPI density restored afterward.
Other text-position differences remain under investigation in the #1542
control audit. No renderer code changes are included here.

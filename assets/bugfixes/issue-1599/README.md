# Issue 1599 evidence

`compare.jpg` shows page 2 at 300 DPI: fresh Microsoft Excel on the left,
office2pdf at `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da` on the right.
The four internal Occasion fill boundaries in G7:G11 are 0.82pt too low.
This records a defect; it does not claim the full workbook audit has passed.

The public Gift Budget and Tracker1.xlsx attachment from issue #982 is unchanged.
It prints at the fitted scale of 0.82. Its G7:G11 conditional-formatting rules
select the Occasion colors. The direct cell styles declare no edge strokes.

Native paints each subsequent row's fill after the preceding row's positive
extension. For example, G8 starts at y=478.06pt and covers G7's fill, whose raw
rectangle extends to y=478.88pt. The converter paints all main fills before the
cell-content overlays, so the later G7 bottom strip covers G8 until y=478.88pt.
The same error occurs at the other three internal color boundaries. Text and
row positions already agree; moving the rows would introduce another error.

## Provenance

- Workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Native PDF SHA-256: `2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`
- Converter PDF SHA-256: `81b93282a27291eb3ebf77ca02229a4d36e5dabc9916b742a083ce0130763c25`

The image preserves the source PNG dimensions and uses progressive JPEG quality
86 with metadata stripped and 300 DPI density restored afterward. Other
independent differences remain in the #1542 audit. No runtime code changes
are included in this evidence commit.

# Shared-baseline cell anchors (#1609)

`native.xml` and `output.xml` contain unchanged `fill_text` operations from
only the header and July rows of page 2 of the public workbook attached to
[issue #982](https://github.com/developer0hye/office2pdf/issues/982).
These are text-position fixtures; page fills and clips were omitted, so they
do not reproduce the full page's visibility or appearance.

Source: [Gift Budget and Tracker1.xlsx](https://github.com/user-attachments/files/30941041/Gift.Budget.and.Tracker1.xlsx).
SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`.
Native exporter: Microsoft Excel for Mac 16.112. Converter commit:
`6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da`, using the Excel bundled fonts.

PDF SHA-256 values:

- Native: `2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`.
- Output: `81b93282a27291eb3ebf77ca02229a4d36e5dabc9916b742a083ce0130763c25`.

To reproduce the extraction, run `mutool draw -F trace -o trace.xml FILE.pdf`
for each PDF. Parse page 2 with `compare_layout.parse_trace`, select the lines
whose normalized keys contain `Month` or `July`, and retain the text operations
whose offsets match those lines' `Glyph.paint_index` values. Preserve the page
media box and wrap the operations in a trace document.

At the unchanged 0.5pt fine threshold, these two rows expose six shifted cells.
The full fitted page exposes 16; the previous combined-line audit reported zero.
Pinned dx values include Month -0.54931pt, Purchased? -0.79040pt, Delivered?
-0.79031pt, and July -0.76732pt. Their converter defect is tracked in #1600.

Run the regression with:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_compare_layout.py'
```

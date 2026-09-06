# Composed fill coverage (#1608)

These traces preserve page 2's paint, clip, and group operations from the public
[Gift Budget workbook in #982](https://github.com/developer0hye/office2pdf/issues/982).
Text operations were removed; this is a fill-layer fixture, not a renderable
full-page document. Image operators retain placement metadata only.

Source: [Gift Budget and Tracker1.xlsx](https://github.com/user-attachments/files/30941041/Gift.Budget.and.Tracker1.xlsx).
SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`.
Native exporter: Microsoft Excel for Mac 16.112. Converter commit:
`6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da`, with the Excel bundled fonts.

PDF SHA-256 values:

- Native: `2aa03cafa6a156d553a2f1e03e0458257319ff6416a188f1132ce7817ecaeae1`.
- Output: `81b93282a27291eb3ebf77ca02229a4d36e5dabc9916b742a083ce0130763c25`.

Reproduce with `mutool draw -F trace -o trace.xml FILE.pdf`: keep page 2's
operations matched by `compare_layout.COVERAGE_EVENT_RE`, omit `fill_text` and
`ignore_text`, retain their surrounding clip push/pop operations, and preserve
the page media box inside a trace document.

The first rose band has a raw right-edge deficit of 0.82pt, but composed
coverage proves that edge equivalent. Its left-side overpaint and lower-left
ownership error remain visible (#1599), so its geometry finding stays active.
The report names those actual differing regions instead of claiming the right
extension is missing. Separate synthetic tests cover fully equivalent split
fills, clipping, opaque occlusion, unknown images/opacity, and dense coordinate
partitions that must not hide a material overlap.

```sh
python3 -m unittest discover -s scripts/tests -p 'test_compare_layout.py'
```

# PowerPoint legend key side length

PowerPoint draws an axis chart's legend key as a square whose side is 45% of the
**legend face's** line box, not a fixed multiple of the chart text size.
office2pdf carried the multiple that relationship produces in Calibri
(`0.5493 em`) and applied it to every face, so an Arial deck's key came out 9.3%
too large. The gap the entry leaves before its label is half that side, less a
fixed 0.375pt, and inherited the same error at half the magnitude.

## Native evidence

Two one-factor sweeps of `tests/fixtures/pptx/bar-chart.pptx`, each gated by its
layout-identical re-zip control:

```sh
python3 scripts/probe_harness.py \
  scripts/probes/issue-1439-legend-key-face.json \
  scripts/probes/issue-1439-legend-key-arial-size.json --backend office
```

`issue-1439-legend-key-face.json` rewrites the chart space's `<a:latin
typeface>` one value at a time at a fixed 18pt; the control is the theme's own
Calibri. Key side and key-to-label gap read off `mutool draw -F trace` of each
exported PDF, in points, against PowerPoint 16.112 on macOS 26.6.2 (build
25G83):

| face | `hhea` ascent + descent | native key side | 0.45 x line box | native gap | side/2 - 0.375 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Calibri (control) | 1.220703 | 9.8887 | 9.8877 | 4.5694 | 4.5689 |
| Arial | 1.117188 | 9.0495 | 9.0492 | 4.1498 | 4.1498 |
| Times New Roman | 1.107422 | 8.9707 | 8.9701 | 4.1104 | 4.1104 |
| Courier New | 1.132813 | 9.1755 | 9.1758 | 4.2127 | 4.2128 |
| Georgia | 1.136230 | 9.2025 | 9.2035 | 4.2262 | 4.2263 |
| Trebuchet MS | 1.161133 | 9.4050 | 9.4052 | 4.3275 | 4.3275 |
| Verdana | 1.215332 | 9.8438 | 9.8442 | 4.5469 | 4.5469 |

`issue-1439-legend-key-arial-size.json` holds the face at Arial and moves the
chart space's `<a:defRPr sz>` instead, which shows the same line box scaling
with the size and no fixed term:

| chart text | native key side | 0.45 x line box x size |
| ---: | ---: | ---: |
| 10pt | 5.0288 | 5.0273 |
| 12pt | 6.0345 | 6.0328 |
| 18pt | 9.0495 | 9.0492 |
| 24pt | 12.0646 | 12.0656 |
| 36pt | 18.0989 | 18.0984 |

No export in either sweep is further than 0.0017pt from a flat 0.45 share, and
the ratio itself spans 0.44995..0.45005. That is the share
already measured for the height of Excel's flat worksheet key (#1169), so the
two hosts now read one `LEGEND_KEY_LINE_BOX_SHARE` constant.

## Page-8 check

Page 8 of `GENERAL SERVICES.pptx` (#1220, input SHA-256
`17924ec3b27646a2c1b2bb711b845360b713cc7052b3cdde9df752f9ceded3a9`) is the
independent case: three entries, 11.97pt Arial legend text. Key square, page
points:

| | side | left edge |
| --- | ---: | ---: |
| native PowerPoint 16.112 | 6.0166 | 745.3548 |
| LibreOffice Impress reference (GT) | 7.1720 | 743.9810 |
| office2pdf before | 6.5751 | 744.5061 |
| office2pdf after | 6.0177 | 745.3422 |

The side error against the native export falls from +0.5585pt to +0.0011pt. The
left edge follows it — the block is right-fitted by its widest label (#1436), so
the label pen positions are byte-identical either side of this change and only
the key start moves.

## Evidence

`gt.jpg`, `before.jpg` and `after.jpg` are page 8 at 150 DPI. The ground truth
is the LibreOffice reference attached to #1220, SHA-256
`8c8f471d8baaf0abd79e752ef49951c240752d36e4512651f6cb9a7cfffb650f`, which is
what the page's other open issues are measured against. That reference draws its
own 7.1720pt key, so the key difference remaining against it is reference-side
and stays with #1670.

`compare.jpg` is the 300 DPI defect image filed with the issue.

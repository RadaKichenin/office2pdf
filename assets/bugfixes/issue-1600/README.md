# Issue 1600 evidence

`compare.jpg` shows unscaled workbook page 3 at 300 DPI: native Excel on the
left and office2pdf at `6d78f1aa26f7ac3f12b4602e77afdf8ff506d1da` on the right.
The centered Delivered? header and Yes cell shift left while their vertical
baselines match. The missing chart above them is separately tracked in #1598.
This is defect evidence, not a completed workbook audit or a renderer fix.

Use the public Gift Budget and Tracker1.xlsx attachment from #982. Change only
`xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true` to `false`,
leaving `pageSetup/@scale="100"` unchanged. The unscaled package SHA-256 is
`af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`.

P6 declares centered Segoe UI 12pt; P10 declares centered Segoe UI 11pt. The
combined Delivered?/Notes trace line starts at x=486pt in native and
x=484.58555pt in output (delta -1.41445pt). The Yes line starts at x=504pt
versus x=503.44126pt (delta -0.55874pt). Both baseline deltas are zero.
The first number is a combined line measurement, not an individual-cell claim.

The full-page comparison is the same image as issue 1598's evidence, retaining
both panels at their original pixel dimensions. It is progressive JPEG
quality 86, with metadata stripped and 300 DPI density restored afterward.

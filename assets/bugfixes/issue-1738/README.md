# Issue 1738 evidence

`compare.jpg` records the defect: unscaled workbook page 2, the tracker's
`Amount budgeted` / `Amount spent` header cells (I6:J6) at 300 DPI, native
Excel on the left and office2pdf with the #1600 fix on the right. Both are
wrapped, centred cells whose text takes two lines; the second line `budgeted`
starts 0.51pt left of native because each wrapped line keeps Typst's exact
centring where Excel seats it on the whole-point grid. This is defect
evidence, not a fix.

Use the public Gift Budget and Tracker1.xlsx attachment from #982 (tracked as
`tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`). Change only
`xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true` to `false`,
leaving `pageSetup/@scale="100"` unchanged. The unscaled package SHA-256 is
`af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`.

The crop is 700x160px from each 4961x3508px page render, appended side by
side; progressive JPEG quality 86, metadata stripped and 300 DPI density
restored afterward.

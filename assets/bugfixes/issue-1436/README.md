# PowerPoint column-chart right-legend horizontal fit

A PowerPoint chart with `<c:legendPos val="r"/>` and no manual `<c:layout>`
right-fits its stacked legend against the **chart area's** right edge. office2pdf
measured that fit from the plot's right edge instead, and a column plot — unlike
a bar plot — keeps an 11pt inset of its own inside the chart area. The inset was
therefore taken off twice and the whole block landed 11pt left of both
references.

## Native evidence

`scripts/probes/issue-1435-column-legend-center.json` repackages
`tests/fixtures/pptx/bar-chart.pptx` as a column chart and rewrites the chart
space's `<a:defRPr sz>` one value at a time on a 480 x 320pt frame. Run it with

```sh
python3 scripts/probe_harness.py \
  scripts/probes/issue-1435-column-legend-center.json --backend office
```

and read the legend key's left edge off each exported PDF with
`mutool draw -F trace`. The unpatched re-zip control gates the series.

PowerPoint 16.112 on macOS 26.6.2 (build 25G83), key left edge inside the frame:

| chart text | native column | native bar control | office2pdf before | after |
| ---: | ---: | ---: | ---: | ---: |
| 10pt | 441.4465 | 441.4465 | 430.4420 | 441.4420 |
| 12pt | 435.6760 | 435.6760 | 424.6809 | 435.6809 |
| 18pt | 418.4018 | 418.4018 | 407.3973 | 418.3973 |
| 24pt | 401.1207 | 401.1207 | 390.1137 | 401.1137 |
| 36pt | 366.5520 | 366.5520 | 355.5466 | 366.5466 |

The column and bar exports land on one x to the last emitted digit, which is
what identifies the anchor as the chart area rather than the plot. The constant
error is `CHART_COLUMN_RIGHT_PAD_PT`, the 11pt a column plot keeps inside that
edge; a bar plot keeps none, which is why #999 and #1000 fitted the frame
clearance alone and only the column family was wrong. After the change the two
families emit the identical x and sit within 0.0054pt of the native export; that
residual is the 10.127pt frame clearance #1000 fitted against these same
exports, and this pull request does not retune it.

## Page-8 check

Page 8 of `GENERAL SERVICES.pptx` (#1220, input SHA-256
`17924ec3b27646a2c1b2bb711b845360b713cc7052b3cdde9df752f9ceded3a9`) is the
independent case: three entries, 11.97pt legend text, an 824.0pt chart-area
right edge. Legend label pen x, page points:

| Entry | Native PowerPoint | LibreOffice reference | Before | After |
| --- | ---: | ---: | ---: | ---: |
| Total Sales | 754.005 | 753.985 | 742.994 | 753.994 |
| Total Cogs | 754.005 | 753.985 | 742.994 | 753.994 |
| Net Profit | 754.005 | 753.985 | 742.994 | 753.994 |

The horizontal error falls from -11.011pt to -0.011pt against the native export
and to +0.009pt against the reference. The keys keep a 0.849pt offset because
office2pdf draws them 6.575pt wide against the native 6.016pt — that side
length is #1439.

## Evidence

`gt.jpg`, `before.jpg` and `after.jpg` are page 8 at 150 DPI. The ground truth
is the LibreOffice reference attached to #1220, SHA-256
`8c8f471d8baaf0abd79e752ef49951c240752d36e4512651f6cb9a7cfffb650f`, which is
what the page's other open issues are measured against; it agrees with the
native export on this fit to 0.02pt. The legend rows still differ from it
vertically — that reference runs LibreOffice Impress's own side-legend layout,
which #1670 records as reference-side.

`compare.jpg` is the 300 DPI defect image filed with the issue by #1441.

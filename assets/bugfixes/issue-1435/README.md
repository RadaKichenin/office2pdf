# PowerPoint column-chart right-legend block centre

A PowerPoint chart with `<c:legendPos val="r"/>` and no manual `<c:layout>`
puts its legend at one page position per chart-text size, whichever way the
bars run. office2pdf centres the stack on the plot plus its axis-label
gutters, and a column chart's bottom gutter is the category band where a bar
chart's is the value tick band, so the two families need separate corrections
even though the native target is the same. Only the bar correction existed;
a column legend therefore landed low. This adds the column pair.

## Native evidence

`scripts/probes/issue-1435-column-legend-center.json` repackages
`tests/fixtures/pptx/bar-chart.pptx` as a column chart and rewrites the chart
space's `<a:defRPr sz>` one value at a time, on a 480 x 320pt frame. Run it
with

```sh
python3 scripts/probe_harness.py \
  scripts/probes/issue-1435-column-legend-center.json --backend office
```

and read the legend key's top edge off each exported PDF with
`mutool draw -F trace`. The unpatched re-zip control gates the series.

PowerPoint 16.112 on macOS 26.6.2 (build 25G83):

| chart text | native key top | key top before | correction |
| ---: | ---: | ---: | ---: |
| 10pt | 279.0788 | 285.6054 | -6.5266 |
| 12pt | 279.9912 | 286.2424 | -6.2512 |
| 18pt | 282.7380 | 288.1537 | -5.4157 |
| 24pt | 285.4861 | 290.0649 | -4.5788 |
| 36pt | 290.9797 | 313.5074 | -22.5277 |

`-7.920088 + 0.139188 * size` fits the first four to within 0.0016pt. The 36pt
export is outside the fit: PowerPoint wraps its category labels onto a second
line there and drops the value axis from ten ticks to three, while office2pdf
keeps one line and ten, so the plot rectangle rather than the legend rule
carries that residual. #1437 owns it.

The 18pt column variant's legend key lands on the unpatched bar control's
`y[282.7380, 292.6268]` to the last emitted digit, which is what makes the
native target family-independent and the two corrections' difference purely
our own.

## Page-8 check

Page 8 of `GENERAL SERVICES.pptx` (#1220, input SHA-256
`17924ec3b27646a2c1b2bb711b845360b713cc7052b3cdde9df752f9ceded3a9`) is the
independent case: three entries instead of one, 11.97pt legend text, and a
690 x 220.8pt frame. Legend text baselines, page y in points:

| Entry | Native PowerPoint | Before | After |
| --- | ---: | ---: | ---: |
| Total Sales | 380.400 | 387.103 | 380.849 |
| Total Cogs | 399.600 | 406.255 | 400.001 |
| Net Profit | 418.800 | 425.407 | 419.153 |

The block centre moves from 6.66pt below the native centre to 0.40pt below it.

## Evidence

`gt.jpg`, `before.jpg` and `after.jpg` are page 8 at 150 DPI. The ground truth
is the LibreOffice reference attached to #1220, SHA-256
`8c8f471d8baaf0abd79e752ef49951c240752d36e4512651f6cb9a7cfffb650f`, which is
what the page's other open issues are measured against. That reference runs
LibreOffice Impress's own automatic side-legend layout — a 15.62pt row pitch
and a block centre 15.42pt above PowerPoint's — so the legend rows still differ
from it after the fix. #1670 records that as reference-side; the native export
above is the authority for the PowerPoint layout this issue is about.

`compare.jpg` is the 300 DPI defect image filed with the issue by #1441.

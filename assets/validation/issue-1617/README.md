# Excel legend line-marker size cap (#1617)

The public fixture `tests/fixtures/xlsx/issue_1603_gift_budget.xlsx` is the
unchanged attachment from #982, SHA-256
`25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`. Its line
series declares a circle marker of `c:size` 5 and its bottom legend is 9pt
Segoe UI. Every probe package changes `xl/charts/chart1.xml` alone; the four
specs beside this file reproduce them, and
`native-legend-marker-measurements.json` records each package's SHA-256, its
native page-2 export's SHA-256, and the legend row read off that export's
`mutool draw -F trace` output in chart points (page points / the 0.82 fit).

## Rule

Native Excel for Mac 16.112 draws the legend key's marker as a vector path no
larger than the legend's own allowance:

    legend marker = min(declared c:size, floor(0.6 x legend size in pt))

With the series declaring a 24pt marker, the key marker measures 3, 4, 4, 5,
6, 6, 7, 8 and 10 chart points at legend sizes 6, 7, 8, 9, 10, 11, 12, 14 and
18, on Segoe UI and identically on Verdana. The painted key rectangle
(0.45 x the face's hhea box) is not the cap: at 10pt it is 5.985pt on Segoe UI
and 5.470pt on Verdana while the marker is 6pt on both. Declared sizes under
the allowance pass through unchanged: a 9pt legend draws 2, 3 and 5 and clamps
7 and 12 to 5; a 6pt legend draws 2 and clamps 4, 5 and 7 to 3. The plotted
markers keep their declared size in every export.

| legend | declared marker | native key marker | rule |
| --- | ---: | ---: | ---: |
| 6pt Segoe UI / Verdana | 24 | 3 | 3 |
| 7pt Segoe UI | 24 | 4 | 4 |
| 8pt Segoe UI / Verdana | 24 | 4 | 4 |
| 9pt Segoe UI / Verdana | 24 | 5 | 5 |
| 10pt Segoe UI / Verdana | 24 | 6 | 6 |
| 12pt Segoe UI / Verdana | 24 | 7 | 7 |
| 14pt Segoe UI / Verdana | 24 | 8 | 8 |
| 18pt Segoe UI | 24 | 10 | 10 |
| 9pt Segoe UI | 2 / 3 / 5 / 7 / 12 | 2 / 3 / 5 / 5 / 5 | same |
| 6pt Segoe UI | 2 / 4 / 5 / 7 | 2 / 3 / 3 / 3 | same |

The same exports show the 6pt legend's sample stroke at 2.0pt where the series
declares 2.24pt (#1751) and the marker's origin on whole chart points left and
above the sample midpoint (#1618); neither is part of this rule.

## Reproduction

Run from the repository root with Microsoft Excel available on macOS. Build
the three base packages from the fixture first: `base-marker24.xlsx` changes
`<c:size val="5"/>` to `24`; `base-marker24-verdana.xlsx` also changes the
legend's `<a:latin typeface="Segoe UI"/>` to `Verdana`; `base-legend6.xlsx`
changes only the legend's `a:defRPr/@sz` from `900` to `600` (SHA-256
`3f40e2a801046f458f2ac8a4631ddfbaa35802803d9229b65cec74d825e0c295`, the #1617
controlled package). Then:

```sh
python3 scripts/probe_harness.py assets/validation/issue-1617/legend-size-marker24-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1617/legend-size-verdana-marker24-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1617/marker-size-9pt-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1617/marker-size-6pt-probe.json --backend office
```

Convert with `--font-path '/Applications/Microsoft Excel.app/Contents/Resources/DFonts'`
and compare page 2 with `compare_layout.py --audit --fine-shift 0.5`,
`compare_text_layer.py`, and `compare_render.py --dpi 300`.

# Issue 1607 evidence

`gt.jpg`, `before.jpg` and `after.jpg` are pages 2 and 3 of the unscaled
control workbook stacked top to bottom at 300 DPI: the fresh native Excel for
Mac 16.112 export, the output of `main` at `bcbe2b48` before the fix, and the
output of the fix. `layout-audit.json` is the all-page
`compare_layout.py --audit --fine-shift 0.5 --noise-floor 0.5 --json` report
of the fix against the native export, and `render-clusters-page-2.json` /
`render-clusters-page-3.json` are the strict 300 DPI cluster reports of both
chart-sheet tiles.

Source: the public Gift Budget and Tracker1.xlsx attachment in issue #982,
tracked as `tests/fixtures/xlsx/issue_1603_gift_budget.xlsx`. Change only
`xl/worksheets/sheet2.xml`'s `pageSetUpPr/@fitToPage` from `true` to `false`;
leave `pageSetup/@scale="100"` unchanged. Both exporters print three pages.

The chart's plot, gridlines, value and category labels and legend were laid
out from the converter's physical page origin while the frame was painted on
Excel's fitted sheet origin (#1542). At the original workbook's 0.82 fit scale
the two origins differ by 0.854 sheet points and the #1250 chrome model had
absorbed that gap (10.146pt top inset, 11.853pt bottom pad); on this unscaled
control the origins coincide and the whole plot printed 0.854pt high. The
content now shares the frame's origin with Excel's flat 11pt top and bottom
insets, and the bottom-legend seat is re-based by the same 0.853pt.

Native page-2 rules against the two outputs, page points:

```text
rule          native     before     after
plot top      143.013    142.159    143.013
$180          168.000    167.000    168.000
$120          242.000    241.000    242.000
$40           342.000    341.000    342.000
axis ($0)     391.192    390.340    391.193
Jan baseline  408.000    407.147    408.000
legend row    428.000    427.147    428.000
```

The original fitted workbook keeps its matched vertical geometry (page-2
gridlines unchanged to 0.001pt) and its gridline x moves from 388.342 to
388.116 sheet points against native 388.109.

## Provenance

- Original workbook SHA-256: `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613`
- Unscaled workbook SHA-256: `af4160ca276c3add9e3100f80c8df7156afbdf6ec7a0090161e015a7ccf5d5cb`
- Native PDF SHA-256: `0935ca38e497a657d2f972fb7ddfc3163cf69e8edfe6d79d230202b5db5d7bfa`
  (the whole-workbook export recorded for #1598)
- Before PDF (`main` at `bcbe2b48`) SHA-256: `af3057a4e0fdbe1539ac4554c4155833a1632964f833c85b50eed690f71016e2`
- After PDF SHA-256: `e6c0b95504cd7ae6c219055c794a894dd37f629d45ec64b9577879c16b03ccdb`
- Both conversions pass `--font-path` for the Office cloud-font cache
  (Segoe UI) and Excel's bundled DFonts (Aptos).

The images preserve the source PNG dimensions and use progressive JPEG
quality 86 with metadata stripped and 300 DPI density restored afterward.
Remaining differences on both tiles are tracked in #1735, #1738, #1745,
#1746 and, for the text layer only, #1736. The comparison images for the two
issues this audit filed are `../issue-1745/compare.jpg` (page 2, the
`Aug..Sep` zero-value run) and `../issue-1746/compare.jpg` (page 3, the title
band's right edge).

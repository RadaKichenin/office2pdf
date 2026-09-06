# Worksheet chart marker placement (#1577)

Excel prints a worksheet chart marker as a bitmap sprite placed on whole
sheet points, not on the plotted vertex. The converter now snaps each plotted
line, scatter, column/bar and radar marker of a worksheet-anchored chart to
that grid (`WorksheetMarkerPlacement` in
`crates/office2pdf/src/render/typst_gen_diagrams.rs`); legend samples keep
their independent placement. Chart values, sparse/error entries, marker size,
fill and outline paint are unchanged: `exact-marker-only-delta.json` asserts
that only 13 marker translations differ between the before and after traces.

Two public fixtures are compared against fresh native Excel exports. The
baseline is main `295bc540e950d8bdbace56840c3861a55f667778`.

| Fixture | SHA-256 | Marker |
| --- | --- | --- |
| `tests/fixtures/xlsx/issue_1181_fit_to_height.xlsx` | `2b4a2d8dceda58758593c88409875efbda05780559154c02bd13fef4f7a1c65b` | 14pt scatter circle, 0.75pt outline, 0.78 fit |
| `tests/fixtures/xlsx/issue_1603_gift_budget.xlsx` | `25f5dc75dab19ea12042979a61842314ddc226e3e45d447e36b2a2a104112613` | 5pt line circles, no declared outline, 0.82 fit |

## Measured rule

The ideal marker point is the plotted vertex in unscaled sheet coordinates,
including the fitted sheet origin of the chart frame. With an explicit outline
width, round the width to an integer with a minimum of one: an odd result
floors the ideal point, an even result rounds it to the nearest sheet point.
An explicit zero width floors; a suppressed or undeclared outline rounds. An
odd declared marker size then moves the center by minus half a sheet point on
both axes. The rule predicts all 286 outline-control positions
(`native-outline-controls.json`, `outline-placement-analysis.json`) and all 93
value, normalized-anchor and size positions
(`native-size-anchor-value-analysis.json`) within 0.00005pt. Column plots keep
their existing vertical calibration and only add the fitted horizontal origin.

`candidate-marker-centers.json` records the compiled candidate PDFs: the
budget marker and all twelve gift markers match native within 0.0001pt.

## Native controls and probe specs

`budget-outline-probe.json`, `budget-size-probe.json` and
`gift-size-probe.json` change one marker property at a time.
`outline-probe-patch-verification.json` and `native-size-controls.json` show
each patched chart part is byte-identical to the measured native package
whose hash is recorded there, with every other package member equal to the
base fixture. The budget size packages change the declared size of both
selected-period scatter series; only the positive series plots a marker.

| Budget control | Native center (pt) |
| --- | --- |
| base 14pt, 0.75pt outline | (195.00, 311.22) |
| size 13 or 15 | (194.61, 310.83) |
| size 20 | (195.00, 311.22) |
| outline 1.5pt, 2pt, 3.5pt or suppressed | (195.78, 312.00) |
| outline 0pt, 1.49pt or 2.5pt | (195.00, 311.22) |

Gift sizes 4 and 6 share one set of twelve centers and sizes 5, 7 and 13
share another, 0.41pt away on both axes.

## Reproduction

Run from the repository root with Microsoft Excel available on macOS:

```sh
python3 scripts/probe_harness.py assets/validation/issue-1577/budget-outline-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1577/budget-size-probe.json --backend office
python3 scripts/probe_harness.py assets/validation/issue-1577/gift-size-probe.json --backend office
```

Read marker sprites with `mutool draw -F trace`; each `fill_image` center is
the sprite center and its alpha-mask centroid (within 0.000206 pixel).
Repackaging can change ZIP hashes without changing member contents; use the
recorded hashes to identify the measured artifacts.

Build with `cargo build --locked -p office2pdf-cli` and convert both sources
with `--font-path '/Applications/Microsoft Excel.app/Contents/Resources/DFonts'`.
Use `compare_layout.py --json --audit --fine-shift 0.5`,
`compare_text_layer.py --json`, and `compare_render.py --dpi 300 --fine-shift 0.5
--strict-clusters` on the same native/current PDFs. `render-provenance.json`
records the PDF hashes.

## Evidence

The budget fixture is the primary evidence under `assets/bugfixes/issue-1577`.
The gift control keeps `control-gt.jpg`, `control-before.jpg`,
`control-after.jpg`, `control-layout-audit.json` and two strict
`control-render-clusters-page-*.json` reports here, with the disposition
inputs in `base-dispositions-page-*.json` and `control-dispositions-page-*.json`.
`after-crop-equivalence.json` records that 716 of the 719 clusters have
identical decoded crop pixels to the individually reviewed baseline audit;
the three new clusters are the moved markers. `budget-marker-crops.jpg` and
`gift-marker-crops.jpg` show native, before, after and 5% diff rows for all
13 markers. `visual-review.json` and `text-layer-verification.json` record
the checklist findings and the unchanged text extraction.

Both workbooks retain independent defects: #1272, #1543, #1545, #1550, #1551,
#1552, #1564, #1566, #1600, #1604, #1605, #1618, #1620, #1621, #1622 and
#1623. A passing strict cluster report means every reported cluster has an
explicit disposition; it does not mean the workbook has no remaining visual
defect.

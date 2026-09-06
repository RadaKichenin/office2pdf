# Zero baseline shift keeps PowerPoint lines on the story grid

A master's `<a:defRPr baseline="0">` reaches every run of its deck as a zero
displacement. The physical-line seating gate treated any displacement, zero
included, as a raised or lowered run and skipped the story-grid rounding that
#1259, #1583, and #1584 established; a centred or bottom-anchored story lost
its baseline snap entirely. A zero displacement is now no displacement, so
those runs seat exactly like unshifted ones. Typst still chooses wrapping and
line-box height; raised and lowered runs keep their existing path.

## Native probe

`scripts/probes/issue-1071-zero-baseline-shift.json` patches slide 2 of
`tests/golden_mocks/business/sources/pptx/01_startup_pitch_en.pptx`: the
`t-baseline0` variant adds `baseline="0"` to the three wrapped 17pt Arial body
runs, and the `ctr-baseline0` variant adds the same attribute and centres the
frame. Exported with `--backend office` (PowerPoint 16.112.3); the re-zip
control gate passed.

- `t-baseline0` against the control: 7/7 lines matched, mean |dy| 0.00pt,
  worst 0.00pt. PowerPoint places a zero-shift run exactly like an unshifted
  one.
- `ctr-baseline0` against the native centred export from the #1584 probe
  (`target/probes/issue-1584-startup-anchor/pdfs/variant-ctr.pdf`): 7/7 lines
  matched, mean |dy| 0.00pt, worst 0.00pt.

## Measurements

Baselines in points from the page top, from MuPDF paint traces of page 2.

| Line | Native | Before | After |
| --- | ---: | ---: | ---: |
| `t-baseline0`: `Server-side document conversion` | 143.04 | 142.00 | 143.00 |
| `t-baseline0`: `or headless browsers` | 162.00 | 162.40 | 162.00 |
| `t-baseline0`: `Enterprises convert 40M+` | 192.96 | 193.00 | 193.00 |
| `t-baseline0`: `signing, and compliance.` | 212.88 | 213.40 | 213.00 |
| `t-baseline0`: `Fidelity failures` | 244.08 | 244.00 | 244.00 |
| `t-baseline0`: `and brand risk.` | 264.00 | 264.40 | 264.00 |
| `ctr-baseline0`: `Server-side document conversion` | 230.16 | 229.20 | 230.20 |
| `ctr-baseline0`: `or headless browsers` | 249.12 | 249.60 | 249.20 |
| `ctr-baseline0`: `Enterprises convert 40M+` | 280.08 | 280.00 | 280.20 |
| `ctr-baseline0`: `signing, and compliance.` | 300.24 | 300.40 | 300.20 |
| `ctr-baseline0`: `Fidelity failures` | 331.20 | 330.80 | 331.20 |
| `ctr-baseline0`: `and brand risk.` | 351.12 | 351.20 | 351.20 |

Every line now sits within 0.12pt of native, which is the export's own
quantisation. The unpatched control converts identically before and after
(143.00, 162.00, 193.00, 213.00, 244.00, 264.00).

The #841 Contoso deck's slide-18 footer, the frame this issue reports, is
20pt Avenir Next LT Pro at 98% in a centred frame under a master that states
`baseline="0"`. Before: 478.04 and 501.56pt (a fractional 23.52pt advance).
After: 478.04 and 502.04pt, the whole-point advance PowerPoint's grid emits.
Native: 480.00 and 503.04pt. The remaining 2pt is #1629: the line-box metric
lookup takes the bundled Noto Serif over the face supplied on `--font-path`,
so the seat is 17pt where the face's own metrics give 19pt. With those
metrics the story-grid model predicts 480.04 and 503.04pt.

## Evidence collection

Each JPEG stacks two full pages vertically at 150 DPI, 2000 × 1125 pixels
per page.

| Collection page | Package | Slide |
| --- | --- | --- |
| 1 | `t-baseline0` variant | 2 |
| 2 | `ctr-baseline0` variant | 2 |

GT is the fresh native PowerPoint 16.112.3 export of each variant. Before is
`main` at `892191c9` logic (this branch with the fix reverted); after uses
this fix. Conversion uses PowerPoint's auto-discovered `Contents/Resources/DFonts`
directory, so Arial resolves to its own face on both sides.

The 0.5pt layout audit has no missing/extra text, changed wrapping,
visibility, fill-occlusion, rectangle-geometry, or fine-position finding on
either page (worst +0.12pt). Searchable text matches: identical normalised
content and codepoint-class census. Both strict cluster reports contain zero
material clusters; the 5% pixel diff is glyph-edge specks on lines that
moved within the export's quantisation and the chart raster's bar edges.

## Reproduction

Build with `cargo build --locked -p office2pdf-cli`. Run
`python3 scripts/probe_harness.py scripts/probes/issue-1071-zero-baseline-shift.json --backend office`
for the native exports and the variant packages (staged under
`~/Library/Containers/com.microsoft.Powerpoint/Data/probes/`), then convert
each variant with `office2pdf VARIANT.pptx -o OUTPUT.pdf`. Assemble the
collections with `mutool merge -o gt.pdf variant-t-baseline0.pdf 2 variant-ctr-baseline0.pdf 2`
and likewise for the converter outputs.

Run `python3 scripts/compare_layout.py gt.pdf after.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py gt.pdf after.pdf`. For each page,
run `python3 scripts/compare_render.py gt.pdf after.pdf --page N --dpi 150 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`,
inspect the full pages, diff, and crops, then rerun with
`--cluster-dispositions PATH --strict-clusters`. Encode progressive JPEGs at
quality 86; strip metadata before setting the density to 150 DPI.

## SHA-256 provenance

- gt.pdf: `454795703cbbef2f42ba37c220fdf4c08f6df515df5f6f2110e224320218f0aa`
- before.pdf: `29720e47ac14cf29aec54cb542eef4700ed685621b789da0d03815e688198f76`
- after.pdf: `e2a2741ccda0193dc69ce3e180d02e22a0c7e03bd4009393da17639669b44584`
- 01_startup_pitch_en.pptx: `752fdaaf6af89ec232a81b36e95f02f45da13acda5cfe903f605b6ce135de595`
- variant-t-baseline0.pptx: `089ec9ceac9c2ac6d24dbd221c02392cd62564099277f9d2ff516371bfa6b6c5`
- variant-ctr-baseline0.pptx: `f7d0fb40f08843fc0184d9f7a1b5f5e4e8628eb5a6ce73e0fcf4c980a358e615`
- variant-t-baseline0.pdf (native): `5bfa6e4793b6185b043b76da30d7240d3fc89912cc1e90c98a2141ff0ca1d3f4`
- variant-ctr-baseline0.pdf (native): `6813327aff98f6d819390dd75c7970f2a9906a00ffcf1d331d81953778ae3dd1`
- 002.CONTOSO.pptx (#841 attachment): `0f225bb743f382926a4758091ff49e1759520dc396bbdbddf10061921afd056a`

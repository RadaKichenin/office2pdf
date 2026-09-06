# PowerPoint physical-line baseline rounding

Typst previously added fractional line advances after the paragraph's first
baseline had been rounded. Round each physical line's running position within
the text story instead, retaining the unsnapped phase. Mixed-size paragraphs
use the largest font on each physical line to restore that line's raw seat.
The paragraph-end font participates only in the final line.

The completed-frame pass moves text, underlines, and links together. Bullets
follow their first body line. Typst still chooses wrapping and line-box height;
unwrapped centered/bottom text and explicit-break layout retain their behavior.
Portable tests cover two embedded font families, three vertical anchors,
plain/list paragraphs, mixed sizes in both orders, and paragraph-mark fonts.

## Measurements

Baselines are in points from the page top, measured from MuPDF paint traces.

| Text | Native PowerPoint | Before | After |
| --- | ---: | ---: | ---: |
| Startup: `or headless browsers` | 162.00 | 162.40 | 162.00 |
| Startup: `signing, and compliance.` | 212.88 | 213.40 | 213.00 |
| Startup: `and brand risk.` | 264.00 | 264.40 | 264.00 |
| Product: `channel (3.1% CVR).` | 178.56 | 179.00 | 178.60 |
| Product: `creative refresh.` | 223.68 | 223.00 | 223.60 |

A one-factor startup probe changes only the body textbox's vertical anchor.
All first and continuation baselines now match native within 0.121pt for top,
center, and bottom. The re-zip control retains native text geometry.

A separate diagnostic uses `tests/fixtures/pptx/customGeo.pptx`, slide 6:
20pt bold `Standards Statements` wraps above 12pt `by Grade Level`. At 90%
spacing, native baselines are 425.040, 446.880, and 462.000pt; current output
is 424.983, 446.983, and 461.983pt. At 100% spacing, the two changed panels
have native baselines 424.800, 448.800, and 464.880pt; current output is
424.863, 448.863, and 464.863pt. The unchanged panel retains its 90% result.
This probe checks line seating; it is not a claim of full-slide fidelity.

## Evidence collection

Each JPEG stacks thirteen full pages vertically at 150 DPI, preserving
2000 × 1125 pixels per page. Source fixtures are under
`tests/golden_mocks/business/sources/pptx/`.

| Collection pages | Source | Slides |
| --- | --- | --- |
| 1–3 | `01_startup_pitch_en.pptx` | 1–3 |
| 4–6 | `03_product_launch_en.pptx` | 1–3 |
| 7–9 | `08_marketing_report_en.pptx` | 1–3 |
| 10–11 | `09_lecture_ko.pptx` | 1–2 |
| 12 | Startup center-anchor probe | 2 |
| 13 | Startup bottom-anchor probe | 2 |

GT uses native Microsoft PowerPoint 16.112.3 exports. The original eleven
pages reuse the integrity-checked #1583 comparison with unchanged sources;
the two anchor variants have fresh native exports. Conversion uses
PowerPoint's `Contents/Resources/DFonts` directory. Before is the verified
main output at `a45fda788497719c13c43e59c32382e8a6defc82`; after uses this fix.
The original issue-report `compare.jpg` is retained.

The 0.5pt layout audit has no missing/extra text, changed wrapping, visibility,
fill-occlusion, geometry, or fine-position findings. Searchable text matches:
2,184 normalized codepoints and an identical codepoint-class census.

Pages 2, 6, 11, 12, and 13 were inspected as full comparisons, 5% pixel
sweeps, and three matched full-scale bands each. The other eight pages have
identical decoded GT/output/diff/band pixels to the fully inspected #1583
comparison. Their existing inspection and exact cluster dispositions apply
unchanged. Titles, body emphasis, Korean weights, chart axes/ticks, legend
outlines, and bar edges remain intact; no new clipping or missing elements
was observed. The lecture question mark's baseline also improves by 0.2pt.

## Remaining differences

- #1581: page 9, seven horizontal-drift clusters caused by `×` disabling the
  Latin run's advance grid; the final period is displaced by −1.073pt.
- #1582: page 5, seventeen suffix clusters caused by cross-space Arial kerning
  being lost; text starting at `Aug` is displaced by +0.834pt.

The resampled gold legend-key edge on page 6 and regular Korean glyph-edge
rasterization on page 11 are assessed separately from these converter defects.

## Reproduction

Build with `cargo build --locked -p office2pdf-cli`. Convert each source with
`office2pdf SOURCE.pptx -o OUTPUT.pdf --font-path FONT_DIRECTORY`.
For the anchor variants, use `scripts/probe_harness.py` with the following
specification, replacing the base with the source's absolute path:

```json
{
  "name": "issue-1584-startup-anchor",
  "base": "ABSOLUTE_PATH/01_startup_pitch_en.pptx",
  "part": "ppt/slides/slide2.xml",
  "factor": "vertical anchoring of wrapped startup story",
  "page": 2,
  "variants": [
    {
      "value": "ctr",
      "find": "<a:bodyPr wrap=\"square\" rtlCol=\"0\" anchor=\"t\">",
      "replace": "<a:bodyPr wrap=\"square\" rtlCol=\"0\" anchor=\"ctr\">",
      "count": 1
    },
    {
      "value": "b",
      "find": "<a:bodyPr wrap=\"square\" rtlCol=\"0\" anchor=\"t\">",
      "replace": "<a:bodyPr wrap=\"square\" rtlCol=\"0\" anchor=\"b\">",
      "count": 1
    }
  ]
}
```

Run `python3 scripts/probe_harness.py SPEC.json --backend office` for native
exports and `--backend office2pdf --converter BINARY` for converter output.
Append only slide 2 of each variant to each eleven-page collection using
`mutool merge`. The CLI also accepts `--slides 2` to convert just that slide.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`.
For every page, run `python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 150 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`,
inspect the full pages, crops, and diff, then rerun with exact cluster
`--cluster-dispositions PATH --strict-clusters`. Encode progressive JPEGs at
quality 86; strip metadata before setting the density to 150 DPI.

## SHA-256 provenance

- gt.pdf: `9c274d519b76071d2cc223c363bdc67d35bc6f9767138495ab6a6b812e0c0095`
- before.pdf: `41d17ef6fefc2db2079cc5f1b58413701eadbe3e0b5437c5376d6631f0b6f196`
- after.pdf: `cc13c3f0aa83093107cde962e4b656b6ed9feb8b62fc8a8b190f67dd47e62b57`
- after-cli: `a1725fc67a97eb72e32eb47488cdfb70c5a1915e3350a74676939941c06156e0`
- 01_startup_pitch_en.pptx: `752fdaaf6af89ec232a81b36e95f02f45da13acda5cfe903f605b6ce135de595`
- 03_product_launch_en.pptx: `2470f5ca6abd44b1b15113aa3028e625738b73f619da3fb16892f5d561b4d7aa`
- 08_marketing_report_en.pptx: `8fe9aa23f97af98ae077e93b849d29d6559e962c322b959f09295b7802c2acaa`
- 09_lecture_ko.pptx: `c93b32a8413f4959aee38f938d4f34e891aa4829f8cf85b915017cff2a609586`
- variant-ctr.pptx: `b61db91d94347aa127bbd191e26a97fe69b85321df0044e1070dcff19c94bf71`
- variant-b.pptx: `e9cf47fe8a113077103a8eda282d23b64c89697d59968b416492e70ced93ad5e`

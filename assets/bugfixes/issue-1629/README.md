# Line-box metrics follow the face the compiler paints with

The native metric lookups resolved a family by walking its whole alias and
substitute chain against the conversion's in-memory faces first, and only
then against the search paths. Naming Avenir Next LT Pro loads the bundled
Noto Serif into memory as that family's reproducible fallback, so the chain's
tail answered before the exact face supplied on `--font-path` was ever
consulted: Typst painted the run with Avenir Next LT Pro while every line-box
metric came from Noto Serif. The lookups now walk the chain once, taking each
candidate's in-memory face before its on-disk copy, which is the order the
compiler's own font book selects in.

## Measurements

Issue #841 attachment `002.CONTOSO.pptx`, slide 18, footer copy
`Fyll ut undersøkelsen vår etter møtet`: 20pt Avenir Next LT Pro under
`<a:spcPct val="98000"/>` in a centred frame (y=450.72pt, h=67.68pt). The
face comes from
`~/Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts/`,
passed with `--font-path`; the CloudFonts cache is deliberately not
auto-discovered (#1409). Baselines in points from the page top, from MuPDF
paint traces of that slide.

| Line | Native | Before | After |
| --- | ---: | ---: | ---: |
| `Fyll ut undersøkelsen` | 480.00 | 478.04 | 480.04 |
| `vår etter møtet` | 503.04 | 502.04 | 503.04 |

Before, the block seated on Noto Serif's 17pt story-grid seat (`usWin`
1069/389 per 1000 upem). After, it seats on the face's own 19pt seat (`usWin`
1972/512 per 2048 upem): (1.176 − 0.2473) × 20 = 18.57pt rounds to 19pt, so
461.04 + 19 = 480.04pt, and the second baseline rounds to 503.04pt. Both sit
on the export's 0.24pt grid, within 0.04pt of native.

The unit test
`an_exact_face_on_a_search_path_outranks_an_in_memory_chain_tail_for_metrics`
pins the general rule with tracked faces: Selawik on a search path against the
bundled Noto Serif in memory as the chain's last resort.

## Evidence collection

Each JPEG is slide 18 of the deck at 150 DPI, 2000 × 1125 pixels. GT is the
native PowerPoint 16.112 export. Before is `main` at `bb6658bb`; after is this
fix. Both conversions pass the CloudFonts directory with `--font-path`.

The 0.5pt layout audit reports no missing GT text, no changed wrapping, no
visibility, fill-occlusion, rectangle-geometry, or fine-position finding on
the footer or the title (worst +0.14pt on the title). Its one text-flow
finding is the master's `Sensitivity: Internal` label, which the Noto Sans
stand-in for Aptos wraps to two lines where PowerPoint saved one; that is
#1712 and is unchanged by this fix. The strict cluster report contains zero
material clusters; the 5% pixel diff is glyph-edge and stroke-edge specks on
the circle outlines, the dot grid, and the contour lines.

## Reproduction

Build with `cargo build --locked -p office2pdf-cli`, then

```sh
office2pdf 002.CONTOSO.pptx -o after.pdf --font-path "$HOME/Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts/"
mutool draw -F trace -o - after.pdf 18 | grep -A2 'Fyll'
```

Extract slide 18 with `mutool merge -o gt.pdf GT.pdf 18` and likewise for the
converter outputs. Run
`python3 scripts/compare_layout.py gt.pdf after.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py gt.pdf after.pdf`, then
`python3 scripts/compare_render.py gt.pdf after.pdf --page 1 --dpi 150 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`,
inspect the full pages, diff, and crops, and rerun with
`--cluster-dispositions PATH --strict-clusters`. Encode progressive JPEGs at
quality 86; strip metadata before setting the density to 150 DPI.

## SHA-256 provenance

- 002.CONTOSO.pptx (#841 attachment): `0f225bb743f382926a4758091ff49e1759520dc396bbdbddf10061921afd056a`
- native export, all 18 pages: `114470306a33c4b10c91b1395754738f21345b0db2c089ef446084c8fd6ced50`
- gt.pdf (slide 18): `a255ff35c6a6da8ae1f67b7acda78c7f9bd7b2b43f6baf97bd4ec8ed8cdb0ba2`
- before.pdf (slide 18): `9aa911de1fccf4719cfaa8ec1c25e4128fffb99dd927637e4a13b846265a3d7c`
- after.pdf (slide 18): `6564157798b5826ff112289fa52ec099ea3c3a7c37b5c7dd62ab0260ae52c1a5`

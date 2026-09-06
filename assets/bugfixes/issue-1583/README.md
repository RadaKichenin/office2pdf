# Fractional PowerPoint text-story origins

Top-anchored paragraphs previously rounded their baselines against the page.
The lecture frame starts at 144pt with a 3.6pt top inset: rounding against
zero discarded the fractional 147.6pt story origin. Capture that origin once,
round each paragraph's running position within it, then translate into the
slide. Bullets use the same origin. Typst still chooses wrapping and layout
height; centered and bottom-anchored layout seats retain their behavior.

## Baseline measurements

All values are points measured from the page top. The affected paragraph
starts now agree with native PowerPoint within 0.12pt.

| Text | Native | Before | After |
| --- | ---: | ---: | ---: |
| Lecture heading | 162.48 | 163.00 | 162.60 |
| Lecture bullet 1 | 191.52 | 192.00 | 191.60 |
| Lecture bullet 2 | 219.60 | 219.00 | 219.60 |
| Lecture bullet 3 | 246.48 | 246.00 | 246.60 |
| Lecture bullet 4 | 273.60 | 274.00 | 273.60 |
| Product Email paragraph | 161.52 | 162.00 | 161.60 |
| Product Display paragraph | 206.64 | 206.00 | 206.60 |
| Product Search paragraph | 250.56 | 251.00 | 250.60 |

A native one-factor probe translated only the lecture slide 2 learning
textbox vertically by 0.25, 0.50 and 0.75pt. Its re-zip control was
layout-identical. The heading moved from 162.48pt to 162.96, 163.20 and
163.44pt; the second bullet moved from 219.60pt to 219.84, 220.08 and
220.32pt. These exporter-quantized movements retain the frame translation
instead of snapping to the page's integer grid. The portable regression
checks exact quarter-point translations for three plain paragraphs and three
bullets, using two embedded font families and different sizes.

## Evidence collection

Sources are under `tests/golden_mocks/business/sources/pptx/`. Each JPEG
stacks eleven full pages vertically, preserving 2000 × 1125 pixels per page
at 150 DPI. Collection page numbers follow this order:

| Collection pages | Fixture | Source slides |
| --- | --- | --- |
| 1–3 | `01_startup_pitch_en.pptx` | 1–3 |
| 4–6 | `03_product_launch_en.pptx` | 1–3 |
| 7–9 | `08_marketing_report_en.pptx` | 1–3 |
| 10–11 | `09_lecture_ko.pptx` | 1–2 |

GT reuses the native Microsoft PowerPoint 16.112.3 exports validated during
#1177, with unchanged source files and fonts. Conversion uses PowerPoint's
`Contents/Resources/DFonts` directory. Before is the verified main output at
`006bcb35e0ee8ecb83b667a2db9f890cb7fc7d3f`; after is built from this change.
The existing `compare.jpg` remains the original issue-report illustration.

Pages 5, 6 and 11 were freshly inspected as full GT/output pages, pixel diffs
and matched full-scale bands. The other eight pages have identical MuPDF
paint traces and identical decoded GT/output/diff/band pixels to the fully
inspected #1177 merge comparison. Their prior visual inspection therefore
applies unchanged. All 27 current material-cluster crops and both fine-shift
crops were inspected again. Covers, cards, chevrons, chart fills and the
Korean panel retain their geometry and colors. Thin axes, ticks, legend
outlines and bar edges remain present; title/body weights and Korean
emphasis remain intact. Searchable text matches: 1,608 normalized codepoints
and an identical codepoint-class census.

## Remaining differences

| Issue | Evidence and cause |
| --- | --- |
| #1581 | Page 9: `×` disables the Latin run's advance grid; seven horizontal-drift clusters, reaching −1.073pt at the final period. |
| #1582 | Page 5: cross-space Arial kerning is lost; 17 suffix clusters starting at `Aug`, displaced by +0.834pt. |
| #1584 | Page 2: the `signing, and compliance.` continuation remains +0.52pt below native. Page 6: paragraph-origin correction exposes continuation-line rounding errors of +0.44pt for `channel (3.1% CVR).` and −0.68pt for `creative refresh.` (previously −0.16pt and −0.28pt). |

Correcting the story origin does not round each physical continuation line;
that separate cause remains #1584, including the increased product-line
errors above. Three other material clusters show only a resampled gold
legend-key edge or regular Korean glyph-edge rasterization, with matching
geometry and emphasis. All clusters have explicit dispositions in the
strict reports; none remains assigned to #1583.

## Reproduction

Build with `cargo build --locked -p office2pdf-cli`. Convert each source with
`office2pdf SOURCE.pptx -o OUTPUT.pdf --font-path FONT_DIRECTORY`, using the
same PowerPoint font directory. Concatenate each collection with
`mutool merge -o COLLECTION.pdf INPUT1.pdf INPUT2.pdf INPUT3.pdf INPUT4.pdf`.
Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`.
For every page, run `python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 150 --fine-shift 0.5 --cluster-report PATH`,
inspect the full pages, matched crops and diff, then rerun with explicit
`--cluster-dispositions PATH --strict-clusters`. Encode progressive JPEGs
at quality 86, stripping metadata before setting density to 150 DPI.

## SHA-256 provenance

- Gt collection PDF: `ae5e419fb4afdc393fdd6f4a605040b8220533e507fbf135c02753b7d4d59b66`
- Before collection PDF: `01237802e09c82f00fbbadbf05f6517e134ee0e38df45118eba9dfe1b138a01b`
- After collection PDF: `c3aae770df0e3aba3e79afa6afafc61c30fbeb5d480d947eea68d5af99979646`

| Fixture | Source PPTX | After PDF |
| --- | --- | --- |
| `01_startup_pitch_en` | `752fdaaf6af89ec232a81b36e95f02f45da13acda5cfe903f605b6ce135de595` | `d5d56979d6b6a37daf5660c40e4de3f45584ae5ca09092192cf552a7695d2edf` |
| `03_product_launch_en` | `2470f5ca6abd44b1b15113aa3028e625738b73f619da3fb16892f5d561b4d7aa` | `5fdce31faac5ccfae619d4f521d7ee6730233dfe7c4aa26031bbeaeac4ba8e7a` |
| `08_marketing_report_en` | `8fe9aa23f97af98ae077e93b849d29d6559e962c322b959f09295b7802c2acaa` | `3f83a8e491ac6f2c9212bc3fa0d01201a9c50596a3cc911da7205dab19876df9` |
| `09_lecture_ko` | `c93b32a8413f4959aee38f938d4f34e891aa4829f8cf85b915017cff2a609586` | `672e96e72dbb0e433ab153ffa17aecf3d0b15886716347cac31f70deb5770a27` |

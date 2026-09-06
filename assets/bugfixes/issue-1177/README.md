# Final-line paragraph-mark seating

PowerPoint applies the paragraph-end font to the last physical line. Applying
that font to every line put the startup fixture's first wrapped baseline at
142pt instead of the native 143.04pt. The corrected baseline is 143pt, with
its bullet on the same baseline. The final line retains its paragraph mark.
Typst still chooses the line breaks and paragraph height; the completed-frame
pass moves nonfinal text, markers, links and decorations together.

## Evidence collection

The images and reports cover four public fixtures under
`tests/golden_mocks/business/sources/pptx/`. Their PDFs are concatenated in the
order below without changing page content. Each JPEG stacks the same eleven
full pages vertically, at the original 2000 × 1125 pixels per page (150 DPI).
The report page numbers refer to this collection.

| Collection pages | Fixture | Source slides |
| --- | --- | --- |
| 1–3 | `01_startup_pitch_en.pptx` | 1–3 |
| 4–6 | `03_product_launch_en.pptx` | 1–3 |
| 7–9 | `08_marketing_report_en.pptx` | 1–3 |
| 10–11 | `09_lecture_ko.pptx` | 1–2 |

GT is a fresh Microsoft PowerPoint 16.112.3 export. Conversion uses the fonts
in PowerPoint's `Contents/Resources/DFonts` directory. Before is the CLI built
at `3a1d6cd182c3fe43223b649c271aa1e62945985b`; the later base commit
`b79d4c7a1b325e7cc0c670f3bb4f598bdd594c46` changes only cleanup documentation.
After is built from this change. Fresh before/after exports have identical
full MuPDF paint traces to the corresponding individually inspected exports,
excluding only the trace's input filename.

## Native one-factor probes

Use `scripts/probe_harness.py SPEC.json --backend office`. Every probe passed
the layout-identical re-zip control. For the startup source, patch
`ppt/slides/slide2.xml` with `page: 2`:

- Width: replace the unique `<a:ext cx="6583680" cy="4114800"/>` with
  `cx="4572000"` (360pt), `10515600` (828pt), or `12192000` (960pt), retaining
  `cy`. The first paragraph has three, two and one physical lines respectively.
  Native first baseline is 143.04pt when wrapped and 142.08pt when unwrapped.
  The original 518.4pt frame has two lines and the same 143.04pt first baseline.
  The deliberately widened diagnostic frame overlaps the unchanged chart;
  measure the unobscured first-line prefix. It is not the published fix fixture.
- Mark font: add `<a:latin typeface="Arial"/>`, `Verdana`, or `Meiryo` to
  the first paragraph's otherwise unchanged `a:endParaRPr`. Native keeps the
  first baseline fixed and changes only the last line by +0.96pt, +0.96pt or
  −0.96pt relative to the inherited mark. A centered Korean 15pt probe also
  isolates the change to its final physical line.

The width-only and mark-only probes together explain the earlier apparent
change of line height: the final line uses a different font seat, while
independent physical-line rounding remains tracked in #1584.

## Remaining differences

| Issue | Evidence and cause |
| --- | --- |
| #1581 | Collection page 9: `×` disables the Latin run's advance grid; drift reaches −1.073pt at the final period. |
| #1582 | Page 5: separating words and spaces loses Arial's `(space, A)` kerning; the suffix starting `Aug` moves +0.834pt. |
| #1583 | Pages 6 and 11: top-anchored paragraphs round against the page instead of their fractional text-box origin. |
| #1584 | Page 2: continuation lines retain fractional offsets; `signing, and compliance.` is +0.52pt below native. |

Matched full-scale crops account for all 40 material diff clusters: 37 belong
to #1581–#1583; three show only a resampled chart legend-key edge or regular
Korean glyph-edge rasterization. The fine layout audit also catches #1584,
which does not create a material pixel cluster.

Full pages, diff images, all matched regions, every material-cluster crop and
the four fine-shift crops were inspected with model vision. Covers retain
their title weights and fills; growth bars, KPI cards, product chevrons,
marketing charts and the Korean panel retain their shapes and content.
Thin axes, ticks, legend outlines and bar edges remain present. The baseline
and width differences above are tracked as geometry, not antialiasing.

## Reproduction

Build with `cargo build --locked -p office2pdf-cli`, convert each source with
`office2pdf SOURCE.pptx -o OUTPUT.pdf --font-path FONT_DIRECTORY` using the
same PowerPoint font directory, and concatenate each GT/before/after set with
`mutool merge -o COLLECTION.pdf INPUT1.pdf INPUT2.pdf INPUT3.pdf INPUT4.pdf`.
Run `compare_layout.py --json --audit --fine-shift 0.5` and
`compare_text_layer.py --json` on the GT and after collections. For every page,
run `compare_render.py --page N --dpi 150 --fine-shift 0.5 --cluster-report PATH`,
inspect its crops, and rerun with explicit `--cluster-dispositions PATH
--strict-clusters`. JPEG encoding follows the parent evidence README.

Explicit hard breaks retain their existing line-stack handling. Scripted runs
retain their existing seating until physical lines can be distinguished from
superscript/subscript glyph baselines; neither is inferred from Y positions.

## SHA-256 provenance

| Fixture | Source PPTX | Native PDF | Before PDF | After PDF |
| --- | --- | --- | --- | --- |
| `01_startup_pitch_en` | `752fdaaf6af89ec232a81b36e95f02f45da13acda5cfe903f605b6ce135de595` | `38cf4653cd04966e0fe53fed5578bcbd865cf50473e4f2ab860393309cfa51bf` | `2e54e43d10750f8173d245a1af15ad3483cf7733a2733f238609d31d5a4211d5` | `d5d56979d6b6a37daf5660c40e4de3f45584ae5ca09092192cf552a7695d2edf` |
| `03_product_launch_en` | `2470f5ca6abd44b1b15113aa3028e625738b73f619da3fb16892f5d561b4d7aa` | `2ad8715ccb22cf023ed73dc996a156693a6d74907c7a3dd662229d67dbf7843a` | `c3eea3acd29f1fb1c32747dc19af709e65ab8462e33e2a11201feb0c1d227c10` | `5f6c3bf7b1db492f3434df95398f43771f32d17a103dd6c0eb96d802e8bf6585` |
| `08_marketing_report_en` | `8fe9aa23f97af98ae077e93b849d29d6559e962c322b959f09295b7802c2acaa` | `6de56fde33887dc07e97cf06bf745d097d5b05bdc3f5d55cc59bc2bad97d7c22` | `3f83a8e491ac6f2c9212bc3fa0d01201a9c50596a3cc911da7205dab19876df9` | `3f83a8e491ac6f2c9212bc3fa0d01201a9c50596a3cc911da7205dab19876df9` |
| `09_lecture_ko` | `c93b32a8413f4959aee38f938d4f34e891aa4829f8cf85b915017cff2a609586` | `c3b0b949a27383e56a6c8fd1c8c5e41e3de2a09114e8e3603d1ffaefcb170fea` | `bfd9cbfdc4b77b851b921bf2b28eb4728afb0d124a56c5fc950037a04ab9cae0` | `ef09b6b654126794ba36166bfd08523e56e68c1d07526ea05d619676b377cea1` |

# Issue #647 visual audit

The source is the tracked business mock
`tests/golden_mocks/business/sources/docx/05_technical_manual_en.docx`. The
comparison selects page 2, whose `Code / Meaning` header row has navy cell
shading and 0.48 pt black borders. `gt.jpg` is the committed native Microsoft
Word export, `before.jpg` is the output at pre-#724 commit `e301f787`, and
`after.jpg` is the output at merged-main commit `1a1405f6` (PR #983). All three
were rendered at 150 DPI, normalized to `1240x1754`, and stored as progressive
JPEG quality 86 with metadata stripped and a 150-DPI density header.

The lossless PNGs, rather than the re-encoded JPEG evidence, produced the AE
figures below:

```sh
pdftoppm -f 2 -l 2 -singlefile -r 150 -png \
  tests/golden_mocks/business/expected/docx/05_technical_manual_en.pdf gt-p2
pdftoppm -f 2 -l 2 -singlefile -r 150 -png before.pdf before-p2
pdftoppm -f 2 -l 2 -singlefile -r 150 -png after.pdf after-p2
magick before-p2.png -crop 1240x1754+0+0 +repage before-p2-norm.png
magick after-p2.png -crop 1240x1754+0+0 +repage after-p2-norm.png
magick compare -metric AE -fuzz 5% gt-p2.png before-p2-norm.png diff-before.png # 15280 (0.00702542)
magick compare -metric AE -fuzz 5% gt-p2.png after-p2-norm.png diff-after.png   # 12427 (0.00571367)
```

## Before-fix checklist

- Page count and order: both PDFs have the same two pages; the compared page is
  page 2 in both.
- Element presence: the running header, horizontal rule, appendix heading,
  four-row table, and footer are all present. The seven unique text lines on
  page 2 match with no missing, extra, or re-wrapped line.
- Position: the first navy cell fill starts at `x=70.85`, the cell boundary,
  while Word's outer fill starts at the border's inner edge, `x=71.28`. The
  output's centred 0.50 pt border covers the fill only to approximately
  `x=71.10`; the exposed shading therefore begins about 0.18 pt left of Word.
  Vertically, the fill starts at `y=84.30`, and the centred top rule covers it
  only to about `y=84.55`, roughly 0.17 pt above Word's `y=84.72` start.
- Size: the old first-cell fill is `75.00x19.92` pt against Word's outer
  `74.64x19.68` pt primitive. Page, table tracks, and row sizing otherwise have
  no material size difference specific to this issue.
- Rotation and flip: no compared element is rotated or flipped.
- Fill: both header cells have the correct navy `#1E2761` fill, but the fill is
  exposed beneath the centred borders instead of beginning at their inner
  edges.
- Stroke, border, and dash style: all table rules are present, solid, and at or
  below 0.50 pt. Before #724 they are centred strokes, which is the shared root
  cause of the exposed shading bleed.
- Text content: normalized page-2 text is identical at 179 characters,
  including 19 spaces and one control character on each side.
- Font family, weight, and style: the Arial-family text is present. The appendix
  heading and both header labels remain bold; body rows, running header, and
  footer remain regular. There are no italic or underlined runs on this page.
- Text color: navy, white, black, and gray text preserve their intended colors.
- Alignment: page, table, and cell alignment remain matched. The independent
  table-cell text inset and baseline displacement is tracked in #649.
- Line and paragraph spacing: no wrap or reflow differs. The remaining
  table-specific baseline difference is tracked in #649.
- Clipping and overflow: no element is clipped or overflows.

## Before-fix fine-detail pass

- The full page, matched table crop, and nearest-neighbor 8x top-left cell crop
  were inspected from 150-DPI renders.
- The 5%-fuzz pixel-difference sweep is `AE 15280` (`0.00702542`). The
  continuous table-grid clusters include #724's border placement and this
  issue's exposed fill edge; glyph clusters include the separately tracked
  #649 text displacement and accepted font rasterization differences.
- Hairline inventory: one light-gray running-header rule plus five horizontal
  and three vertical black table boundary locations are visible on this page.
  Repeated cell-edge operations overlap at shared boundaries, but every
  resulting rule is present, solid, and no thicker than 0.50 pt.
- Weight and emphasis inventory: the appendix heading and `Code` / `Meaning`
  labels are bold in both renders. The body rows, running header, and footer are
  regular; no italic or underline is added or lost.

## Acceptance target

The visible navy shading must begin at the Word border's inner edge without
changing page count, table geometry, row sizing, text, or the non-DOCX table
renderers. Because the opaque border and fill have the same final composite
whether the fill is clipped or covered, #724's edge-aligned filled border bands
can meet this target without duplicating every cell background as a manually
inset shape.

## After-fix checklist

- Page count and order: unchanged at two pages, with the same page 2 selected.
- Element presence: all page and table elements remain present; all seven text
  lines still match with no missing, extra, or re-wrapped line.
- Position: the new positive-axis left border band covers the navy fill from
  `x=70.85` through `x=71.33`, so the visible fill begins only 0.05 pt from
  Word's `x=71.28`. The top band analogously covers through `y=84.78`, 0.06 pt
  from Word's `y=84.72`. On the corroborating
  `07_product_spec_en.docx` page 1, the visible first-cell fill begins at
  `x=57.13` (`56.65 + 0.48`), 0.01 pt from Word's `x=57.12`.
- Size: the underlying Typst cell fill and all table tracks remain unchanged;
  the visible composite is trimmed by the new 0.48 pt border bands. Page and
  row sizes remain unchanged.
- Rotation and flip: unchanged; no element is rotated or flipped.
- Fill: both navy fills remain present with the same color. The border bands
  now fully cover the boundary-to-inner-edge area, removing the visible bleed
  that #647 reports.
- Stroke, border, and dash style: all rules remain present and solid. DOCX table
  borders are now 1/300-inch-quantized filled rectangles rather than centred
  strokes, as audited in #724.
- Text content: page-2 content and codepoint classes are unchanged between the
  before and after PDFs.
- Font family, weight, and style: Arial-family text and all bold/regular runs in
  the inventory are unchanged; no italic or underline appears.
- Text color and alignment: colors and page/cell alignment are unchanged. The
  remaining text inset and baseline deviation remains tracked in #649.
- Line and paragraph spacing: before/after layout comparison is exactly 0.00 pt
  for line position, pitch, width, wrapping, and reflow.
- Clipping and overflow: no element is clipped or overflows.

## After-fix fine-detail pass

- The fresh full page, matched table crop, and 8x edge crop were re-inspected at
  150 DPI. The navy fill meets each black band without a boundary-side leak or
  a newly introduced gap.
- The lossless 5%-fuzz sweep improves from `AE 15280` (`0.00702542`) to
  `AE 12427` (`0.00571367`), an 18.7% reduction. Remaining text-edge clusters
  are #649 or accepted font rasterization; no #647 fill-edge cluster remains.
- Hairline inventory: the running-header rule and every horizontal and vertical
  table rule remain present, solid, continuous, and at or below 0.48 pt. The
  new filled bands cover the old shading bleed through the inner edge.
- Weight and emphasis inventory: the bold appendix/header runs and every
  regular run remain unchanged at full crop scale; no italic or underline is
  added or lost.

The same #724 change was already regression-tested across all 30 business
fixtures: all PPTX and XLSX outputs were byte-identical, all DOCX page/text/
layout comparisons were unchanged, and only bordered DOCX drawing operations
changed.

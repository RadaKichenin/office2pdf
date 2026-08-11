# Issue #724 visual audit

The source is the tracked business mock
`tests/golden_mocks/business/sources/docx/01_invoice_en.docx`. The GT is its
committed one-page native Microsoft Word export; `pdfinfo` identifies macOS
Quartz as the producer. `gt.jpg` and `before.jpg` select page 1 and were
rendered at 150 DPI. The output's one-pixel PDF-width difference was normalized
to the GT's `1240x1754` raster before storing both as progressive JPEG quality
86 with metadata stripped and 150-DPI density restored.

All AE figures below come from the lossless 150-DPI PNG rasters before JPEG
encoding; the committed JPEGs are the visual evidence, not the metric inputs.
Given the baseline `before.pdf` and candidate `after.pdf`, the exact page-1
normalization and comparison were:

```sh
pdftoppm -f 1 -singlefile -r 150 -png tests/golden_mocks/business/expected/docx/01_invoice_en.pdf gt-p1
pdftoppm -f 1 -singlefile -r 150 -png before.pdf before-p1
pdftoppm -f 1 -singlefile -r 150 -png after.pdf after-p1
magick before-p1.png -crop 1240x1754+0+0 +repage before-p1-norm.png
magick after-p1.png -crop 1240x1754+0+0 +repage after-p1-norm.png
magick compare -metric AE -fuzz 5% gt-p1.png before-p1-norm.png diff-before.png  # 39129 (0.0179907)
magick compare -metric AE -fuzz 5% gt-p1.png after-p1-norm.png diff-after.png    # 31377 (0.0144265)
```

Re-encoding those rasters as the checked-in JPEGs changes the AE values, so a
JPEG-to-JPEG comparison is not interchangeable with this lossless sweep.

## Before-fix checklist

- Page count and order: both PDFs contain the same single page.
- Element presence: the logo, invoice heading and metadata, billing line,
  item table, payment terms, and footer are all present. All 14 unique text
  lines match with no missing, extra, or re-wrapped line.
- Position: Word paints the table's 0.48 pt left border from 70.80 to 71.28 pt,
  with its outer edge on the 70.80 pt table boundary. The output instead
  centres a 0.50 pt stroke at 70.85 pt, painting approximately 70.60 to
  71.10 pt. Interior boundaries repeat the same centreline model.
- Size: the page, logo, table tracks, rows, fills, and text extents have no
  material size difference specific to this issue.
- Rotation and flip: no compared element is rotated or flipped.
- Fill: the navy logo and header cells and the light-blue total row are all
  present. Their remaining cell-background extent is tracked in #647.
- Stroke, border, and dash style: every table rule is present, solid, and at
  the declared half-point weight. The defect is that the rules are centred
  strokes rather than Word's edge-aligned filled rectangles.
- Text content: normalized text and the codepoint census are identical at
  500 characters, including 42 spaces and one control character on each side.
- Font family, weight, and style: the Arial-family text is present. `INVOICE`,
  `Bill To:`, the header labels, `Subtotal`, and both total-row labels/amounts
  retain their bold emphasis; the payment terms remain italic. No underlined
  run is visible.
- Text color: navy, gray, black, and white text retain their expected colors;
  the 150-DPI distribution shift is 0.0003.
- Alignment: page-level, cell, numeric, and footer alignment remain visually
  matched. The table-cell inset difference remains tracked in #649.
- Line and paragraph spacing: the table baselines remain 0.26-0.51 pt high and
  the payment-terms baseline 0.58 pt high; the comparison tool classifies the
  page-wide remainder as non-material rasterization/antialiasing, while the
  table-specific inset and baseline are tracked in #649.
- Clipping and overflow: no element is clipped or overflows.

## Before-fix fine-detail pass

- The full page and matched table and left-border crops were inspected side by
  side at full 150-DPI scale, with the border crop additionally enlarged by
  nearest-neighbor scaling.
- A 5%-fuzz full-page pixel sweep was inspected (`AE 39129`, normalized ratio
  `0.0179907`). Residual glyph clusters are font rasterization and the tracked
  table text placement in #649; fill-edge clusters remain #647; the continuous
  grid clusters are this issue.
- Hairline inventory: the table has six vertical rules and eight horizontal
  rules, all solid and at or below 0.50 pt. All are present. Word emits them as
  black `fill_path` rectangles and no `stroke_path`; the output emits fourteen
  centred `stroke_path` rules.
- Weight and emphasis inventory: the bold and italic runs listed above match
  at full crop scale. Regular metadata, item descriptions, quantities, prices,
  VAT, and footer text remain regular; no underline is added or lost.

## Acceptance target

Paint DOCX table borders as 1/300-inch-quantized filled bands whose leading
edge is the grid boundary and whose ink extends on the positive x/y side,
without changing table tracks, row sizing, cell text, or PPTX/XLSX border
behavior. Re-audit #647 and #649 after the border model changes.

## After-fix checklist

- Page count and order: unchanged at one page.
- Element presence: all page and table elements remain present; all 14 unique
  text lines still match with no missing, extra, or re-wrapped line.
- Position: the left rule is now a filled rectangle from 70.85 to 71.33 pt,
  with its leading edge on office2pdf's 70.85 pt grid boundary. This is 0.05
  pt from Word's 70.80 to 71.28 pt rectangle and no longer paints 0.25 pt
  outside the boundary as the centred stroke did. Shared rules are owned by
  the following right/bottom cell and paint on the same positive-axis side as
  the Word trace. Repeating Word headers keep that rule on the body row;
  Excel's separately measured repeating-header exception remains unchanged.
- Size: each nominal 0.50 pt rule is quantized to Word's two 1/300-inch grid
  units, 0.48 pt. Page, logo, tracks, rows, fills, and text extents do not
  move.
- Rotation and flip: no element gains or loses rotation or flipping.
- Fill: the navy and light-blue fills remain present. The new border bands
  cover the old visible shading bleed up to the border's inner edge; #647 is
  re-measured separately after this prerequisite lands.
- Stroke, border, and dash style: all six vertical and eight horizontal solid
  rules remain present. The output changes from 14 table `stroke_path`
  operations to zero and emits black 0.48 pt `fill_path` rectangles instead,
  matching Word's primitive and width.
- Text content: normalized content and codepoint classes remain identical.
- Font family, weight, and style: Arial-family text and every bold/italic run
  listed in the before inventory remain unchanged; no underline appears.
- Text color and alignment: colors and all page/cell alignments are unchanged.
  The remaining table-text inset and baseline deviation is tracked in #649.
- Line and paragraph spacing: before/after layout comparison is exactly 0.00
  pt on every line, including pitch and width, so the border paint change does
  not alter line layout.
- Clipping and overflow: no element is clipped or overflows.

## After-fix fine-detail pass

- The full page, table crop, and nearest-neighbor 6x left-border crop were
  re-inspected at 150 DPI. Every rule is continuous through its intersections;
  no vertical band protrudes into the preceding row.
- The normalized 5%-fuzz pixel sweep falls from `AE 39129` (`0.0179907`) to
  `AE 31377` (`0.0144265`), a 19.8% reduction. The remaining table glyph
  clusters are #649; the logo and glyph-edge clusters are accepted font/image
  antialiasing differences.
- Primitive census: Word is 139 fills / 0 strokes, before is 7 fills / 14
  strokes, and after is 101 fills / 0 strokes. The count difference is
  operation splitting, not missing ink; all 14 hairlines were re-enumerated
  in the full-scale crop.
- Bold `INVOICE`, `Bill To:`, header labels, `Subtotal`, and total labels and
  amounts remain bold; payment terms remain italic; regular runs remain
  regular.

## Corpus regression pass

The merge-base and candidate release binaries converted all 30 business
fixtures. All ten PPTX and all ten XLSX PDFs are byte-identical. Across all ten
DOCX fixtures, every page has identical text, line positions, pitch, width,
wrapping, and page count. Seven bordered DOCX outputs change only their draw-op
census; the three without affected borders remain byte-identical. The 5%-fuzz
GT metric also improves on the additional bordered samples checked at 150 DPI:
`05_technical_manual_en` page 2 (`15280` to `12427`),
`07_product_spec_en` (`62692` to `47907`), and `08_newsletter_en` (`63093` to
`61204`).

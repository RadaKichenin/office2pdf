# Issue #638 visual audit

The source is the tracked business mock
`tests/golden_mocks/business/sources/docx/05_technical_manual_en.docx`. The GT
is its committed two-page native Microsoft Word export; `pdfinfo` identifies
macOS Quartz as the producer, and `scripts/check_gt_integrity.py` found no
structural corruption. `gt.jpg` and `before.jpg` select page 1 and were rendered
with `pdftoppm` at 150 DPI. The one-pixel PDF-width difference was normalized to
the GT's `1240x1754` raster before storing both as progressive JPEG quality 86
with metadata stripped and 150-DPI density restored.

## Before-fix checklist

- Page count and order: both PDFs contain the same two pages in the same order.
- Element presence: all page-1 headings, paragraphs, code blocks, header, rule,
  and footer are present. The text layer matches with no missing, extra, or
  re-wrapped line.
- Position: the mixed Arial/Courier line starts within 0.07 pt of GT, but its
  next baseline is 0.65 pt high. Seven page-1 lines exceed the 0.12 pt Word
  noise floor, with worst vertical drift -0.74 pt lower on the page.
- Size: no issue-specific frame or element-size difference is visible.
- Rotation and flip: no rotated or flipped page-1 element is present.
- Fill: the four light-gray command backgrounds and all text fills are present.
- Stroke, border, and dash style: the solid header hairline is present; page 1
  has no other border or dash-style deviation.
- Text content: normalized text is identical.
- Font family, weight, and style: Arial body text and Courier New inline/code
  runs are present. Font rasterization differs slightly between renderers.
- Text color: navy headings, gray header/footer/code, and black body text are
  present with no material color shift.
- Alignment: headings, paragraphs, header, rule, and footer retain their
  expected horizontal alignment.
- Line and paragraph spacing: defective. The line containing `--font-path`
  starts at 328.80 pt in GT and 328.74 pt before the fix; the following line is
  at 341.52 pt in GT and 340.81 pt before the fix. The resulting advance is
  12.72 pt versus 12.07 pt.
- Clipping and overflow: no page-1 element is clipped or overflows.

## Fine-detail pass

- Both full pages and the matched page-1 troubleshooting crop were inspected
  side by side at full 150-DPI scale.
- A 5%-fuzz full-page pixel sweep was inspected. The page-1 residual clusters
  cover font rasterization plus the accumulated vertical shift described
  above; page 2 contains the table deviations already tracked in #647, #649,
  and #724.
- Hairline inventory: the page-1 header rule and page-2 table borders are the
  elements at or below 1 pt. All are present; page-2 edge alignment remains
  tracked in #724.
- Weight and emphasis inventory: the numbered section headings and both
  troubleshooting subheadings are bold; body and code text are regular. No
  italic or underlined run is visible. The before image preserves that
  inventory.

## After-fix checklist

- Page count and order: unchanged at two pages in the same order.
- Element presence: all 21 page-1 lines and all 7 page-2 lines match, with no
  missing, extra, or re-wrapped text. The normalized text and codepoint census
  are identical to GT (`1069` characters on both sides).
- Position: the mixed line remains within 0.06 pt of GT. Its following baseline
  moves from 340.81 pt to 341.44 pt, against Word's 341.52 pt. The specific
  advance is therefore 12.70 pt versus 12.72 pt, within 0.02 pt. Page 1 improves
  from 7 to 3 lines beyond the conservative 0.12 pt trace floor; its mean
  absolute baseline delta falls from 0.20 pt to 0.08 pt and its worst pitch
  delta from 0.75 pt to 0.18 pt.
- Size: page size and all issue-specific block and text extents remain intact.
- Rotation and flip: no rotated or flipped element was introduced.
- Fill: all command backgrounds, heading fills, and table fills remain present.
- Stroke, border, and dash style: the page-1 solid header hairline remains
  present. The page-2 table's already-known fill/border deviations remain
  tracked in #647 and #724.
- Text content: unchanged and selectable; the normalized text comparison
  reports `content_matches: true` with no codepoint-class delta.
- Font family, weight, and style: Arial and Courier New remain the declared
  faces. The numbered headings and troubleshooting subheadings remain bold;
  body and code text remain regular, with no italic or underlined run.
- Text color: no material shift (`0.0001` distribution shift at 150 DPI).
- Alignment: horizontal MAD is 0.08 pt; paragraph, header, rule, and footer
  alignment remain visually matched.
- Line and paragraph spacing: the issue-specific 0.65 pt deficit is gone. The
  after crop shows the second line and all following blocks seated at the GT
  positions. The remaining page-2 table text offset is tracked in #649.
- Clipping and overflow: neither page contains a clipped or overflowing
  element.

## After-fix fine-detail pass

- Both after pages and matched page-1 text and page-2 table crops were inspected
  side by side with GT at full 150-DPI scale.
- The 5%-fuzz pixel-difference sweep was inspected. The page-1 residuals are
  glyph rasterization/antialiasing around otherwise matching text; the three
  trace deltas just beyond 0.12 pt are sub-pixel and not visually distinct.
  `compare_render.py`, run directly on the GT and after PDFs at 150 DPI,
  reports no material geometry, colour, or coverage difference. Page-2
  residuals are the existing table issues #647, #649, and #724. The committed
  JPEGs are lossy review previews and are not the inputs to that numeric PDF
  comparison.
- Hairline inventory: the header rule and every page-2 table border remain
  present. The known table border edge alignment remains #724.
- Weight and emphasis inventory: every bold run listed above remains bold;
  every regular run remains regular; no italic or underline was added or lost.

## Corpus regression sweep

The same debug binaries built from pre-fix `main` and this worktree converted
all 30 tracked business mocks (10 DOCX, 10 PPTX, and 10 XLSX) with identical
inputs and four workers. Exactly one PDF changed:
`05_technical_manual_en.pdf`. The other 29 PDFs are byte-identical before and
after, so the per-run metric boxes do not alter uniform-family DOCX paragraphs
or any PPTX/XLSX output.

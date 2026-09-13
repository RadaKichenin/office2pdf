# docx-rs 0.4.22: themed run colours and tracked moves

This directory anchors the visual audit of moving our docx-rs fork from 0.4.19
to 0.4.22. The upgrade changes two things on this page:

- A run whose `w:color` lists `w:themeColor` before `w:val` rendered black,
  because the 0.4.19 reader took the element's first attribute as the colour
  (#1698).
- A tracked move rendered its text at both its origin and its destination
  (#1699). 0.4.22 reads `w:moveTo` and `w:moveFrom` as their own paragraph
  children, and office2pdf now keeps the destination and drops the origin.

## Fixture

`tests/fixtures/docx/theme_color_and_tracked_move.docx` (SHA-256
`65ca62fc37e15a3a521a6dcc3be711ff24ae88a50ac6836e0e43523113e5a0f6`):
- Page setup: Arial 11pt defaults, A4, 1in margins, and no paragraph spacing.
- Themed runs: the 16pt bold heading is `accent1`/`2E75B6` and the bold "At
  risk" run is `accent2`/`C55A11`. The theme defines both scheme colours with
  those same values, so the theme and the literal colour agree.
- Tracked move: the second paragraph is a `w:moveTo` destination, and the last
  paragraph ends with the matching `w:moveFrom` origin.

The GT is its native Word for Mac 16.112.4 export (macOS 26.6.2 build 25G83,
Quartz PDFContext, SHA-256
`f1da2b06ffcf83b7dc3082b9bcc32c3ec90e82af1b09d400fe1bd756e3046d0b`), made by
`scripts/macos/export_word_pdfs.applescript` staged in Word's container. The
script deletes comments and accepts all revisions before saving, so the GT is
Word's final view. `scripts/check_gt_integrity.py` reports no structural
corruption.

## Measured

Text colours come from `mutool draw -F trace`, and baselines from
`mutool draw -F stext`, in points from the page top:

| | Heading colour | "At risk" colour | Text lines | Last baseline |
| --- | --- | --- | --- | --- |
| Word | `#2E75B6` | `#C55A11` | 5 | 138.48 |
| Before (docx-rs 0.4.19) | `#000000` | `#000000` | 6: the moved sentence also ends the last paragraph and wraps | 151.31 |
| After (docx-rs 0.4.22) | `#2E75B6` | `#C55A11` | 5 | 138.66 |

After lands every line within 0.18pt of Word's baseline.

## Evidence

- `gt.jpg`, `before.jpg`, `after.jpg`: page 1 at 150 DPI with Poppler
  `pdftoppm`.
- `compare.jpg`: the defect image filed with #1698, with Word on the left and
  `main` on the right, from the same page at the same DPI.
- `layout-audit.json`: `compare_layout.py --json --audit --fine-shift 0.25`
  over the GT and after PDFs. Five of five lines match, the worst dy is
  +0.18pt, and there is no finding.
- `render-clusters-page-1.json`: `compare_render.py --dpi 300 --fine-shift 0.25
  --strict-clusters`. It finds no material diff cluster and records one
  inspected glyph-edge observation.

# Business Golden Mock Visual Audit

This ledger records the full visual audit of the repository-owned business
golden corpus on `main` commit
`775b492760cc7ecadf149584c9cd429702edc35b` with the office2pdf `0.6.4`
release binary. All 30 source documents converted successfully, and all 54
generated pages matched the native Microsoft Office PDF page count and order.

## Audit method

Every native Office PDF page was compared with the corresponding office2pdf
page using the same process:

1. Render both PDFs with `pdftoppm -r 150`.
2. Normalize the output canvas to the GT page size without scaling content.
3. Inspect a whole-page side-by-side comparison.
4. Divide the page into three or four matched regions, according to page
   geometry, and inspect the GT/output crops at 2x and 4x magnification.
5. Run `magick compare -metric AE -fuzz 5%` and inspect every highlighted
   cluster.
6. Inventory page count/order, element presence, position, size,
   rotation/flip, fill, stroke/border and dash style, text content, font
   family/weight/style, text color, alignment, line/paragraph spacing,
   clipping/overflow, elements at or below 1 pt, and bold/italic/underlined
   runs.

The AE ratio is a triage signal, not a pass/fail threshold. A page is complete
only when each visible deviation is assigned to an issue or documented below
as an accepted renderer/environment difference.

## Root-cause disposition

| Issue | Status at audit | Root cause | Evidence |
| --- | --- | --- | --- |
| #355 | Reopened | DOCX explicit table column widths are not honored | Existing issue evidence |
| #377 | Reopened | XLSX icon-set glyphs are thin text arrows instead of Excel solid icons | Existing issue evidence |
| #390 | Reopened | PPTX outer shadows are crisp offset duplicates without blur | Existing issue evidence |
| #402 | Open, reconfirmed | DOCX large title under `docGrid` renders short and leaves too little body gap | Existing issue evidence |
| #404 | Open, reconfirmed | DOCX table-cell rows render about 4.3 pt shorter than Word | Existing issue evidence |
| #417 | Open, reconfirmed | XLSX print-title range `$3:$3` repeats rows 1-3 on later pages | Existing issue evidence |
| #418 | Open, reconfirmed | XLSX data bars render 31.8% taller than Excel | Existing issue evidence |
| #420 | Open, reconfirmed | DOCX first-paragraph `space-before` collapses at page top | Existing issue evidence |
| #436 | New | DOCX page-break carrier adds a blank line before next-page content | [GT/output comparison](https://raw.githubusercontent.com/developer0hye/office2pdf/63d609fdd66e3b2a4332416ad17d8752940ce8c1/assets/bugfixes/issue-436/compare.jpg) |
| #437 | New | DOCX paragraph borders in headers are dropped | [GT/output comparison](https://raw.githubusercontent.com/developer0hye/office2pdf/575a2bf1878c66e4c115a4a7855618c52cda1f4c/assets/bugfixes/issue-437/compare.jpg) |
| #438 | New | PPTX Korean text wraps earlier than PowerPoint in the same text box | [GT/output comparison](https://raw.githubusercontent.com/developer0hye/office2pdf/90d527edf24cca4efe51a2b8304eac2bf18523ed/assets/bugfixes/issue-438/compare.jpg) |
| #439 | New | XLSX `cellIs` text equality drops the matching differential style | [GT/output comparison](https://raw.githubusercontent.com/developer0hye/office2pdf/4ff0b7f6836ce9d4db70f2d5c433e3c506942dda/assets/bugfixes/issue-439/compare.jpg) |
| #419 | Closed after re-audit | Escaped currency prefixes now render the reported values with exactly two decimals | Fresh four-page inventory comparison |

## Page-by-page ledger

`Text` reports whitespace-normalized PDF text extraction. `Exact` does not mean
pixel-identical; `Review` means extraction order or a tracked visual glyph
differs, while manual inspection confirmed that no untracked source text is
missing.

### DOCX

| Case | Page | AE pixels at 5% | AE ratio | Text | Disposition |
| --- | ---: | ---: | ---: | --- | --- |
| `docx-invoice-en` | 1 | 143,340 | 6.590% | Review | #355, #404, #420 |
| `docx-contract-ko` | 1 | 198,719 | 9.137% | Review | #402, #420 |
| `docx-meeting-minutes-ko` | 1 | 188,803 | 8.681% | Review | #404, #420 |
| `docx-resume-en` | 1 | 112,342 | 5.165% | Exact | Accepted font rasterization/metrics only |
| `docx-technical-manual-en` | 1 | 104,428 | 4.801% | Exact | #420, #437 |
| `docx-technical-manual-en` | 2 | 97,923 | 4.502% | Exact | #404, #436, #437 |
| `docx-official-letter-ko` | 1 | 89,487 | 4.114% | Review | #420 |
| `docx-product-spec-en` | 1 | 201,298 | 9.255% | Exact | #355, #404, #420 |
| `docx-newsletter-en` | 1 | 154,241 | 7.092% | Exact | #420 |
| `docx-offer-letter-en` | 1 | 84,158 | 3.869% | Exact | Accepted font rasterization/metrics only |
| `docx-research-report-ko` | 1 | 257,637 | 11.846% | Review | #404, #420 |
| `docx-research-report-ko` | 2 | 53,797 | 2.473% | Review | #404 |

### PPTX

| Case | Page | AE pixels at 5% | AE ratio | Text | Disposition |
| --- | ---: | ---: | ---: | --- | --- |
| `pptx-startup-pitch-en` | 1 | 4,945 | 0.220% | Exact | No untracked deviation |
| `pptx-startup-pitch-en` | 2 | 67,946 | 3.020% | Exact | No untracked deviation |
| `pptx-startup-pitch-en` | 3 | 18,515 | 0.823% | Exact | #390 |
| `pptx-quarterly-review-ko` | 1 | 8,900 | 0.396% | Exact | No untracked deviation |
| `pptx-quarterly-review-ko` | 2 | 9,666 | 0.430% | Exact | #390 |
| `pptx-quarterly-review-ko` | 3 | 8,608 | 0.383% | Exact | No untracked deviation |
| `pptx-product-launch-en` | 1 | 5,199 | 0.231% | Exact | No untracked deviation |
| `pptx-product-launch-en` | 2 | 36,391 | 1.617% | Exact | No untracked deviation |
| `pptx-product-launch-en` | 3 | 29,107 | 1.294% | Exact | No untracked deviation |
| `pptx-training-deck-ko` | 1 | 8,429 | 0.375% | Exact | No untracked deviation |
| `pptx-training-deck-ko` | 2 | 43,614 | 1.938% | Exact | No untracked deviation |
| `pptx-training-deck-ko` | 3 | 29,767 | 1.323% | Exact | #390 |
| `pptx-company-intro-en` | 1 | 3,969 | 0.176% | Exact | No untracked deviation |
| `pptx-company-intro-en` | 2 | 13,393 | 0.595% | Exact | No untracked deviation |
| `pptx-company-intro-en` | 3 | 14,747 | 0.655% | Exact | #390 |
| `pptx-project-status-ko` | 1 | 8,600 | 0.382% | Exact | No untracked deviation |
| `pptx-project-status-ko` | 2 | 15,196 | 0.675% | Exact | No untracked deviation |
| `pptx-conference-talk-en` | 1 | 7,562 | 0.336% | Exact | No untracked deviation |
| `pptx-conference-talk-en` | 2 | 74,421 | 3.308% | Exact | No untracked deviation |
| `pptx-marketing-report-en` | 1 | 8,651 | 0.384% | Exact | No untracked deviation |
| `pptx-marketing-report-en` | 2 | 17,545 | 0.780% | Exact | #390 |
| `pptx-marketing-report-en` | 3 | 62,456 | 2.776% | Exact | No untracked deviation |
| `pptx-lecture-ko` | 1 | 7,282 | 0.324% | Exact | No untracked deviation |
| `pptx-lecture-ko` | 2 | 56,627 | 2.517% | Review | #438 |
| `pptx-sales-proposal-en` | 1 | 8,924 | 0.397% | Exact | No untracked deviation |
| `pptx-sales-proposal-en` | 2 | 45,926 | 2.041% | Exact | #390 |

### XLSX

| Case | Page | AE pixels at 5% | AE ratio | Text | Disposition |
| --- | ---: | ---: | ---: | --- | --- |
| `xlsx-quotation-ko` | 1 | 78,510 | 3.608% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-quotation-ko` | 2 | 17,971 | 0.826% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-financial-model-en` | 1 | 29,672 | 1.363% | Exact | Accepted font rasterization/metrics only |
| `xlsx-financial-model-en` | 2 | 99,785 | 4.585% | Exact | Accepted font rasterization/metrics only |
| `xlsx-inventory-en` | 1 | 364,391 | 16.744% | Exact | #418 |
| `xlsx-inventory-en` | 2 | 246,852 | 11.343% | Review | #417, #418 |
| `xlsx-inventory-en` | 3 | 65,504 | 3.010% | Exact | #439 |
| `xlsx-inventory-en` | 4 | 34,939 | 1.606% | Exact | #439 |
| `xlsx-payroll-ko` | 1 | 79,214 | 3.640% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-payroll-ko` | 2 | 12,866 | 0.591% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-project-schedule-en` | 1 | 79,941 | 3.673% | Exact | No untracked deviation |
| `xlsx-sales-dashboard-en` | 1 | 66,466 | 3.054% | Exact | #418 |
| `xlsx-attendance-ko` | 1 | 54,860 | 2.521% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-budget-ko` | 1 | 85,243 | 3.917% | Exact | Accepted CJK font substitution/rasterization only |
| `xlsx-expense-report-en` | 1 | 75,307 | 3.460% | Exact | No untracked deviation |
| `xlsx-kpi-tracker-en` | 1 | 60,438 | 2.777% | Review | #377 |

## Accepted differences

The following clusters were reviewed and are not filed as office2pdf defects:

- Minor glyph antialiasing and rasterization differences where the extracted
  text, geometry, fill, border, color, alignment, and emphasis match.
- CJK XLSX glyph-shape differences caused by the available
  `Noto Sans CJK SC` fallback versus the native Excel rendering environment.
  High-DPI crop review confirmed that source bold runs remain bold and that no
  text or decoration is missing.

All other visible deviations found in this audit are represented by an open
issue in the root-cause table.

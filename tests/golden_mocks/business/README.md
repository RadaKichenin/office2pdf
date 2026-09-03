# Business Golden Mocks

This corpus adds synthetic, print-ready business documents that combine the Office features people use together in normal work. Every source file is paired with a PDF exported by the corresponding native Microsoft Office application.

The corpus contains no third-party or confidential source material. Names, organizations, addresses, account data, and metrics are fictional.

## Why this is separate from `tests/fixtures`

The existing fixture set is strong at parser breadth and isolated regressions:

- focused samples for tables, images, lists, fields, charts, shapes, conditional formatting, and page setup;
- large LibreOffice and Apache POI corpora for compatibility and crash resistance;
- contributor acceptance fixtures for narrowly scoped behavior.

Those samples do not provide a balanced, repository-owned set of realistic business documents where several common features interact on the same printed page. This corpus fills that gap.

| Format | Existing samples already cover | Business combinations added here |
| --- | --- | --- |
| DOCX | isolated tables, images, lists, fields, headers/footers, equations, and styles | invoices, contracts, meeting minutes, resumes, manuals, official letters, product specs, newsletters, offer letters, and research reports |
| PPTX | isolated charts, SmartArt, masters/layouts, shapes, media, themes, and tables | pitches, quarterly reviews, launches, training, company intros, status reports, talks, marketing reports, lectures, and proposals |
| XLSX | isolated values, formatting, merged cells, charts, conditional formatting, drawings, and page setup | quotations, financial models, inventory, payroll, schedules, dashboards, attendance, budgets, expenses, and KPI trackers |

The cases also balance English and Korean content, formulas and calculated displays, multi-page print geometry, nested structures, and feature interactions that have already exposed real office2pdf regressions.

## Layout

```text
tests/golden_mocks/business/
├── manifest.json
├── VISUAL_AUDIT.md  # Current full-corpus visual disposition and issue ledger
├── sources/
│   ├── docx/    # 10 synthetic Word documents
│   ├── pptx/    # 10 synthetic PowerPoint decks
│   └── xlsx/    # 10 synthetic Excel workbooks
├── expected/
│   ├── docx/    # Native Microsoft Word PDF exports
│   ├── pptx/    # Native Microsoft PowerPoint PDF exports
│   └── xlsx/    # Native Microsoft Excel PDF exports
├── baselines/   # layout.json: per-page layout deviation baseline (regenerated);
│                # main-3e788d0.json: historical hand-recorded conversion status
└── audits/      # Historical per-run visual metrics and review snapshots
```

`manifest.json` is the source of truth for case IDs, paths, locales, expected
page counts, feature coverage, and related regression issues.
[`VISUAL_AUDIT.md`](VISUAL_AUDIT.md) is the source of truth for the latest
full-corpus visual disposition, per-page review results, and current root-cause
issue mapping. Files under `audits/` retain historical per-run snapshots.

## DOCX cases

| Case | Scenario | Main coverage |
| --- | --- | --- |
| `docx-invoice-en` | customer invoice | logo images, billing metadata, fixed-width item table, currency totals, preserved spaces |
| `docx-contract-ko` | Korean software contract | document grid, large title, numbered clauses, suffix tabs, Korean typography, signature table |
| `docx-meeting-minutes-ko` | Korean meeting minutes | metadata/action tables, nested bullets, contextual list spacing, Korean table text |
| `docx-resume-en` | professional resume | paragraph borders, compact hierarchy, dates, inline emphasis, bulleted achievements |
| `docx-technical-manual-en` | CLI technical manual | shaded code blocks, literal symbols, page break, commands, troubleshooting table |
| `docx-official-letter-ko` | Korean official letter | paragraph borders, aligned labels, preserved spaces, numbered body, formal closing |
| `docx-product-spec-en` | product comparison sheet | explicit unequal table widths, merged headers, fills, borders, numeric alignment |
| `docx-newsletter-en` | company newsletter | images, callout table, borders, lists, section hierarchy, mixed visual/text content |
| `docx-offer-letter-en` | employment offer | formal letter rhythm, compensation values, benefits list, signature area |
| `docx-research-report-ko` | Korean research report | long multi-page table, repeating header, percentages, notes, Korean typography |

## PPTX cases

| Case | Scenario | Main coverage |
| --- | --- | --- |
| `pptx-startup-pitch-en` | startup pitch | hero imagery, KPI shapes, rounded cards, bullet copy, shadows |
| `pptx-quarterly-review-ko` | Korean quarterly review | metric cards, native table, rounded corners, picture shadow, Korean text |
| `pptx-product-launch-en` | launch plan | chevron timeline, image, bullets, annotations, compact metrics |
| `pptx-training-deck-ko` | Korean security training | nested bullets, paragraph spacing, rounded callouts, shadows, literal quotes |
| `pptx-company-intro-en` | company introduction | image composition, value markers, KPI cards, rounded shapes |
| `pptx-project-status-ko` | Korean project status | status table, progress visuals, risk/decision callout, Korean text |
| `pptx-conference-talk-en` | technical conference talk | code-like text, literal OOXML tokens, bullets, spacing preservation |
| `pptx-marketing-report-en` | marketing performance report | KPI cards, image, bullets, percentage/currency copy, shadows |
| `pptx-lecture-ko` | Korean lecture | question callout, definition text, bullets, Korean typography |
| `pptx-sales-proposal-en` | commercial sales proposal | three-tier pricing, bullet lists, recommendation highlight, image |

## XLSX cases

| Case | Scenario | Main coverage |
| --- | --- | --- |
| `xlsx-quotation-ko` | Korean quotation | formulas, merged headers, currency, borders, portrait fit-to-page |
| `xlsx-financial-model-en` | SaaS financial model | assumptions sheet, cross-sheet formulas, projections, percentages, rounding |
| `xlsx-inventory-en` | warehouse inventory | 60-row dataset, status formulas, conditional fill, data bars, print pagination |
| `xlsx-payroll-ko` | Korean payroll | payroll formulas, currency, totals, wide-table pagination, Korean text |
| `xlsx-project-schedule-en` | project schedule | typed dates, duration formulas, phase fills, borders, fit-to-page |
| `xlsx-sales-dashboard-en` | regional sales dashboard | monthly totals, color scale, data bars, row/column print geometry |
| `xlsx-attendance-ko` | Korean attendance sheet | symbols, `COUNTIF` formulas, percentages, merged title/footer |
| `xlsx-budget-ko` | Korean departmental budget | vertically merged departments, subtotals, ratios, grand total, currency |
| `xlsx-expense-report-en` | travel expense report | print area, typed dates, decimal currency, literal currency suffix, totals |
| `xlsx-kpi-tracker-en` | product KPI tracker | achievement formulas, icon set thresholds, exception fill, percentages |

## Validation contract

1. A case is complete only when both its source and expected PDF exist.
2. The source stem and expected PDF stem must match.
3. DOCX/PPTX/XLSX sources must pass ZIP integrity checks.
4. Expected PDFs must open with Poppler and match `expected_pages` in the manifest.
5. PPTX slide counts must match native PDF page counts.
6. Any source change requires a fresh PDF export from native Microsoft Word, PowerPoint, or Excel. LibreOffice output is not accepted as ground truth.
7. Visual comparisons use at least 150 DPI and review every page for element presence, geometry, fill, stroke, text, font emphasis, alignment, spacing, and clipping.

The repository CLI can be exercised manually with:

```sh
office2pdf tests/golden_mocks/business/sources/docx/01_invoice_en.docx \
  -o /tmp/01_invoice_en.pdf
```

Do not overwrite files under `expected/` with office2pdf output; they are the native Office reference.

## Re-exporting the native Office PDFs

On macOS with Microsoft Word, PowerPoint, and Excel installed, stage a fresh export without touching the tracked goldens:

```sh
scripts/macos/export_business_golden_pdfs.sh /tmp/office2pdf-business-goldens
```

Word and PowerPoint export whole documents in source order. Excel exports every visible worksheet with its native print settings, then `pdfunite` losslessly combines the sheet PDFs in workbook order. The staging directory includes `provenance.txt` with Office versions, macOS build, export time, and SHA-256 hashes. Review the staged PDFs visually before replacing any tracked file and then update the matching hashes and metadata in `manifest.json`.

The three apps are sandboxed, so the run copies each format's sources into `~/Library/Containers/<bundle id>/Data/business-golden-export/` — Word, PowerPoint, and Excel each in their own container — exports there, copies the PDFs back to the staging directory, and removes the container stage. Files anywhere else raise a per-file "Grant Access" dialog that stalls the run (#1128), so the staging directory argument is free to sit wherever is convenient.

Run both validator layers after any source or golden update:

```sh
python3 -m unittest scripts.tests.test_validate_business_golden_mocks
python3 scripts/validate_business_golden_mocks.py
```

The `Business Golden Contract` CI job runs both checks on every push and pull request.

## Layout deviation baseline

`baselines/layout.json` stores the `compare_layout.py` deviation vector for
every page of every case (matched/missing/extra lines, baseline dy statistics,
line-width drift, pitch deltas, wrap and reflow counts, large instance shifts,
painted-text visibility mismatches, visible-fill occlusions, and path census
plus axis-aligned rectangle/line geometry deltas), measured against the native
GT with the per-format noise floor (0.12pt Word/PowerPoint, 0.5pt Excel).
Regenerate it with a local build:

```sh
cargo build -p office2pdf-cli
python3 scripts/generate_layout_baselines.py --office2pdf target/debug/office2pdf
```

The producer refuses to record anything when a GT fails the
`check_gt_integrity.py` gate or when a trace parses to zero pages — both
states would bake a silently wrong baseline in.

A PR that changes conversion output regenerates the file; the git diff is the
quantitative before/after. To classify the change under the measurement-noise
tolerance policy (±0.24pt for Word/PowerPoint cases, ±1.0pt for Excel cases,
±0.5% width, counts exact):

```sh
python3 scripts/generate_layout_baselines.py \
  --office2pdf target/debug/office2pdf --output /tmp/fresh-layout.json
python3 scripts/check_layout_baselines.py --fresh /tmp/fresh-layout.json
```

`check_layout_baselines.py` exits nonzero only on a regression, and its report
names every case, page, and metric that moved materially in either direction.
CI validates the baseline structurally (schema, case set, page coverage
against the manifest); the numeric comparison needs a local converter build,
so it is a review-time tool, not a CI gate. `main-3e788d0.json` is the
historical hand-recorded predecessor, kept for provenance.

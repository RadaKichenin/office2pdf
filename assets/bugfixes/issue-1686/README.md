# typst 0.15: a list marker sits on its item's first baseline

This directory anchors the visual audit of the typst 0.14.2 to 0.15.1 upgrade
(#1686). Across the 30 business golden fixtures, the upgrade changes no
rendered page beyond Poppler edge rasterisation on one rule per page in two
documents; MuPDF renders those pages identically. The construct it visibly
changes is a native list whose first line is taller than its marker.

## Fixture

`tests/fixtures/docx/list_marker_taller_first_line.docx` (SHA-256
`806dd40f18ce6e6c8eacf7bac10d9c6ab1948e9161caf0e8359d360af903efe8`): Arial
11pt defaults, a one-level bullet list, and no stated line spacing. Item 1
opens with a 28pt run; item 2 is plain 11pt.

The GT is its native Word for Mac 16.112.4 export (macOS 26.6.2, Quartz
PDFContext, SHA-256
`cecb128b340863b8de9621912d79d3746cce038de09273faff2f199d44731b04`), made by
`scripts/probe_harness.py --backend office` with a layout-identical re-zip
control.

The fixture leaves `w:spacing w:line` unset on purpose: a stated line spacing
triggers #1685, an extra line between list items that the upgrade does not
touch.

## Measured

Baseline origins from `mutool draw -F stext`, in points from the page top:

| | Bullet 1 | `Quarterly` | Bullet 2 and `Second` |
| --- | --- | --- | --- |
| Word | 110.88 | 110.88 | 127.20 |
| Before (typst 0.14.2) | 94.97 | 110.91 | 127.16 |
| After (typst 0.15.1) | 110.91 | 110.91 | 127.16 |

typst 0.14 top-aligned a list marker, so the 11pt bullet hung 15.94pt above
its 28pt line. typst 0.15 aligns the marker with the item's first baseline
(typst#7895).

## Evidence

- `gt.jpg`, `before.jpg`, `after.jpg`: page 1 at 150 DPI with Poppler
  `pdftoppm`.
- `layout-audit.json`: `compare_layout.py --json --audit --fine-shift 0.25`
  over the GT and after PDFs; no deviation past the noise floor.
- `render-clusters-page-1.json`: `compare_render.py --strict-clusters`; no
  material diff cluster, and one inspected glyph-edge observation.

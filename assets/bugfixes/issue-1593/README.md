# Inherited chart-series stroke geometry

A series that omits its local cap or join now inherits that property from the
chart's related `dataPointLine/spPr/ln`. An explicit local cap or join retains
precedence. The package loader is shared by DOCX, PPTX and XLSX; missing,
external or malformed style parts leave the existing fallback unchanged.

The [Office chart-style definition](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-odrawxml/4019845e-6b38-47b0-88b9-c6d739e7292f)
assigns `dataPointLine` to 2-D line, scatter and unfilled radar charts. Defaults
are applied while the original XML family is known, so 3-D line, filled radar,
bar, area and pie charts do not accidentally inherit that role. This fix reads
direct stroke geometry; it does not add theme `lnRef`, width or color inheritance.

## Native controls

Original: `tests/fixtures/xlsx/issue_1181_fit_to_height.xlsx`, SHA-256
`2b4a2d8dceda58758593c88409875efbda05780559154c02bd13fef4f7a1c65b`.
The derived `issue_1593_inherited_series_cap.xlsx` removes only the Cash Flow
series' local `cap="rnd"` from `xl/charts/chart2.xml`; every other ZIP member
has identical bytes. Its SHA-256 is
`0dd078dc817e442312dc7e743f1bf80f06983e7f79954eea7a381b2fb5a65ea2`.

The references are verified Excel 16.112.3 exports retained from the #1590
investigation. Both converters use Excel's DFonts. Before is
`6f122389a7416879e4626e2936ec6d32d58c3cef`; the subsequent #1592 merge changes
only the native exporter, its tests and documentation. All native exports have
two ordered sheets and pass the integrity checks; re-zip controls retain
identical page paint traces.

Fifteen cases (eleven unique source packages) match native cap/join geometry
after the fix, including round, flat and square inherited caps and the omitted
local join. Miter controls retain the measured ratio 8; miter limits are
irrelevant for round/bevel joins. Every other paint primitive is unchanged,
and all six direct-property controls have wholly unchanged page traces.
Values, widths, colors, marker paint and text/layout findings are preserved.

## Collection

| Page | Source and native page |
| --- | --- |
| 1 | Original workbook, first printed sheet |
| 2 | Derived fixture, second sheet: inherited round cap |
| 3 | Related-style flat-cap variant, second sheet |
| 4 | Related-style square-cap variant, second sheet |
| 5 | Original with local join omitted, second sheet: inherited round join |

The fine-detail threshold is 0.5pt; full-page evidence uses pdftoppm at 300 DPI.
All 2,053 material clusters have explicit passing dispositions in the five
accompanying reports. Full contexts, original-scale endpoint/join/marker crops
and final JPEG typography/rule crops were visually inspected. The progressive
JPEGs use quality 86, metadata stripped before restoring 300-DPI density.
Remaining independent findings: chart frame #1272; drawing origin #1542;
column width #1543; fixed-row text #1545; title-row height #1550; wrapped-note
pitch #1551; footer baseline #1552; worksheet rules #1564; missing chart
separators #1566; cached marker value #1577. Native marker and sparkline bitmap
edges differ from the vector output. Non-whitespace text matches under
`pdftotext -layout`; default extraction retains baseline table reading-order
and whitespace-census differences.

## Reproduction

Run each portable spec with `python3 scripts/probe_harness.py SPEC --backend office`:

- `scripts/probes/issue-1593-related-series-cap.json`
- `scripts/probes/issue-1593-local-series-properties.json`

Both specs recreate the measured native input packages byte for byte. Native
staging defaults to Excel's sandbox container; PDF reports are copied to
`target/probes/<spec-name>/`. Preserve native outputs before rerunning the same
spec with another backend. Check each native export with
`python3 scripts/check_gt_integrity.py NATIVE.pdf --source SOURCE.xlsx --json`.
Build each version using `cargo build --locked -p office2pdf-cli`, preserve its
binary, and convert with `office2pdf SOURCE.xlsx -o OUTPUT.pdf --font-path FONT_DIRECTORY`.
Use Microsoft Excel.app's `Contents/Resources/DFonts`. Assemble the five pages
in the table's order using `mutool merge`.

Run `python3 scripts/compare_layout.py GT.pdf AFTER.pdf --json --audit --fine-shift 0.5`
and `python3 scripts/compare_text_layer.py GT.pdf AFTER.pdf --json`. Their
remaining findings are described above. For every page run
`python3 scripts/compare_render.py GT.pdf AFTER.pdf --page N --dpi 300 --fine-shift 0.5 --artifacts-dir DIRECTORY --cluster-report PATH`.
Inspect the full pages, matched crops and pixel sweeps, then rerun with
`--cluster-dispositions PATH --strict-clusters` using reviewed exact IDs.

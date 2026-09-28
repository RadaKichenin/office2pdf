# Explicit zero worksheet margin

`source.xlsx` is a synthetic one-page workbook with three labelled alignment
samples in Malgun Gothic 11pt, four 18-character columns, fixed 15pt rows, and
`pageMargins left="0" right="0.7" top="1.15" bottom="1"`. It carries no private
data.

`sheet_print_margins` read a margin's *value* rather than its presence, so the
declared zero fell through to Excel's 0.7in default and the first glyph landed
at x53 instead of on the paper edge. A controlled left-margin sweep gave native
x22 for 0-0.25in and x24/31/53 for 0.3/0.4/0.7in; the converter matched only the
last three. Preserving attribute presence — `has_left()` and its siblings, added
to the fork in MathNya/umya-spreadsheet#373 and developer0hye/umya-spreadsheet#15
— puts the declared zero on the paper and leaves a worksheet with no
`<pageMargins>` at all on the defaults.

After the fix the first glyph sits at x3 against the native x22. The residual
19pt is the native export's own 20pt floor on the printed left origin, which
this converter deliberately does not model: the floor may belong to the printer
the export went through rather than to Excel, and one machine's sweep cannot
tell. That is tracked separately in issue #1929.

`compare_layout.py --audit --fine-shift 0.5` reports 3/3 matched lines with no
missing, extra or re-wrapped text, no visibility or fill mismatch, and the three
remaining instance shifts all `dx -19.00pt` (#1929) with `dy` within +-1.00pt
and a 2.00pt worst pitch delta — the native baselines dither 93/127/158 against
our uniform 94/126/158, which stays under the materiality bar and is
dispositioned to the tracker #1874. The strict 150-DPI render census finds five
clusters, every one a fragment of those same three labels
(`cluster-dispositions-page-1.json`), and the text layer is intact.

# Prefixed worksheet reproduction

Evidence for #1803. `control.xlsx` uses the default SpreadsheetML namespace;
`prefixed.xlsx` binds the same URI to `s` and prefixes the worksheet elements.
Expanded element names, text and attributes are identical, and every other ZIP
part is byte-identical. Native Excel renders the two sources with zero decoded-
pixel difference at 150 DPI.

Before the fix the converter rendered the control normally but reported success
and produced a blank page for the prefixed source: its dependency worksheet
reader matched literal `e.name()` values such as `row` and `headerFooter`, which
do not match `s:row` and `s:headerFooter`. After the fix both sources convert to
a byte-identical PDF, so the namespace serialization no longer reaches the
output at all.

`cluster-dispositions-page-1.json` maps every diff cluster of the fixed prefixed
output against its native export to an open issue; all of them are the
default-namespace path's own pre-existing deviations, unchanged by this fix.

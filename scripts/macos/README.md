# Native Excel PDF export

`export_excel_pdfs.applescript` exports each visible sheet in workbook order:

```sh
osascript scripts/macos/export_excel_pdfs.applescript OUTPUT_DIR case-id INPUT.xlsx
```

Additional `case-id INPUT.xlsx` pairs form one batch. Inputs and outputs must be
inside Excel's sandbox container (`~/Library/Containers/com.microsoft.Excel/Data/`).
`python3 scripts/probe_harness.py SPEC.json --backend office` handles staging.

The exporter opens inputs read-only, closes each batch workbook without saving,
and restores Excel's previous display-alert setting. Excel stays running and
unrelated open workbooks remain open. Failed inputs are reported after the
remaining jobs have been attempted; successful sheet PDFs remain available.

Run the native lifecycle regressions on a Mac with Excel, Poppler and MuPDF:

```sh
OFFICE2PDF_TEST_EXCEL=1 python3 -m unittest discover -s scripts/tests -p test_export_excel_lifecycle.py -v
```

Tests use temporary sandbox copies, retain a separate edited workbook during
export, and restore the prior alert setting. A legacy application-wide quit is
replaced with a controlled cancellation in the test copy; the fixed exporter
runs unchanged. Set `OFFICE2PDF_EXCEL_TEST_ARTIFACTS` to retain PDFs and state logs.

# office2pdf

[![CI](https://github.com/developer0hye/office2pdf/actions/workflows/ci.yml/badge.svg)](https://github.com/developer0hye/office2pdf/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/office2pdf.svg)](https://crates.io/crates/office2pdf)
[![docs.rs](https://docs.rs/office2pdf/badge.svg)](https://docs.rs/office2pdf)
[![License](https://img.shields.io/crates/l/office2pdf.svg)](LICENSE)

Pure-Rust library and CLI for converting DOCX, XLSX, and PPTX files to PDF.

No LibreOffice, no Chromium, no Docker — just a single binary powered by [Typst](https://github.com/typst/typst).

## Features

- **DOCX** — paragraphs, inline formatting (bold/italic/underline/color), tables, images, drawing shapes, ordered/nested lists, syntax-highlighted code, headers/footers, page setup
- **PPTX** — slides, text boxes, shapes, tables (with theme-based table styles), images, slide masters, speaker notes, gradient backgrounds, shadow/reflection effects
- **XLSX** — sheets, cell formatting, merged cells, column widths, row heights, conditional formatting (DataBar, IconSet, and formula rules)
- **PDF/A-2b** — archival-compliant output via `--pdf-a`
- **Embedded font extraction** — fonts embedded in PPTX/DOCX are automatically extracted, deobfuscated, and used during conversion
- **macOS Office font auto-discovery** — PowerPoint/Word/Excel bundled fonts and Office cloud font caches are searched automatically
- **WASM** — runs in browsers and Node.js via WebAssembly (optional `wasm` feature)
- **Zero external dependencies** — runs as a standalone executable

## Installation

### Library

```toml
[dependencies]
office2pdf = "0.6.5"
```

### CLI

```sh
cargo install office2pdf-cli
```

#### Prebuilt binaries

Every [GitHub release](https://github.com/developer0hye/office2pdf/releases) ships standalone CLI binaries — no Rust toolchain needed:

| Platform | Asset |
|----------|-------|
| Linux x86_64 (glibc) | `office2pdf-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux x86_64 (static musl) | `office2pdf-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `office2pdf-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `office2pdf-<version>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `office2pdf-<version>-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `office2pdf-<version>-x86_64-pc-windows-msvc.zip` |

On Linux and macOS, download, extract, and place the binary on your `PATH`:

```sh
VERSION=v0.6.5
TARGET=x86_64-unknown-linux-gnu  # pick your platform's target from the table above
curl -L "https://github.com/developer0hye/office2pdf/releases/download/${VERSION}/office2pdf-${VERSION}-${TARGET}.tar.gz" | tar xz
sudo install "office2pdf-${VERSION}-${TARGET}/office2pdf" /usr/local/bin/
```

On Windows, unzip the archive and add `office2pdf.exe` to your `PATH`.

The macOS binaries are not notarized. Binaries downloaded with a browser are quarantined by Gatekeeper; clear the flag with `xattr -d com.apple.quarantine office2pdf` (downloads via `curl` are unaffected).

## Quick Start

### As a library

```rust
// Simple one-liner
let result = office2pdf::convert("report.docx").unwrap();
std::fs::write("report.pdf", &result.pdf).unwrap();

// With options
use office2pdf::config::{ConvertOptions, PaperSize};

let options = ConvertOptions {
    paper_size: Some(PaperSize::A4),
    ..Default::default()
};
let result = office2pdf::convert_with_options("slides.pptx", &options).unwrap();
std::fs::write("slides.pdf", &result.pdf).unwrap();

// In-memory conversion
use office2pdf::config::Format;

let docx_bytes = std::fs::read("report.docx").unwrap();
let result = office2pdf::convert_bytes(
    &docx_bytes,
    Format::Docx,
    &ConvertOptions::default(),
).unwrap();
std::fs::write("report.pdf", &result.pdf).unwrap();
```

### CLI

```sh
# Single file
office2pdf report.docx

# Explicit output path
office2pdf report.docx -o output.pdf

# Batch conversion
office2pdf *.docx --outdir pdfs/

# With options
office2pdf slides.pptx --paper a4 --landscape
office2pdf spreadsheet.xlsx --sheets "Sheet1,Summary"
office2pdf document.docx --pdf-a
office2pdf report.docx --font-path /usr/share/fonts/custom
```

On macOS, `office2pdf` automatically searches Microsoft Office app fonts and local Office font caches before falling back to regular system fonts. `--font-path` is only needed as an override for custom local fonts.

### WASM (Browser / Node.js)

Build with `wasm-pack`:

```sh
wasm-pack build crates/office2pdf --target web --features wasm
```

Use from JavaScript:

```js
import init, {
  Office2PdfConverter,
  convertDocxToPdf,
  convertToPdf,
} from './pkg/office2pdf.js';

await init();

const docxBytes = new Uint8Array(await file.arrayBuffer());
const pdfBytes = convertDocxToPdf(docxBytes);

// Or use the generic API with a format string
const pdfBytes2 = convertToPdf(xlsxBytes, "xlsx");

// Register a font for this converter and make it the final fallback for every
// emitted font chain. The result-bearing API preserves structured warnings.
const fontBytes = new Uint8Array(await fontFile.arrayBuffer());
const converter = new Office2PdfConverter();
converter.registerFont(fontBytes);
converter.setLastResortFontFamily("Noto Sans SC");

const result = converter.convertDocxToPdf(docxBytes);
for (let index = 0; index < result.warningCount; index += 1) {
  const warning = result.warningAt(index);
  console.warn(warning.kind, warning.from, warning.to, warning.message);
}
const pdfBytes3 = result.pdf;
```

The compatibility functions return PDF bytes directly:
`convertToPdf(data, format)`, `convertDocxToPdf(data)`,
`convertPptxToPdf(data)`, and `convertXlsxToPdf(data)`. Their
`*WithResult` counterparts return `ConversionResult`, which exposes `pdf`,
`warningCount`, and `warningAt(index)`. `Office2PdfConverter` provides the same
result-bearing methods together with `registerFont`, `clearFonts`,
`setLastResortFontFamily`, and `clearLastResortFontFamily`.

Browser and Node.js builds use the bundled Typst fallback fonts and also honor
font faces embedded inside DOCX and PPTX files. Filesystem font paths remain
unavailable in WASM, but callers can supply standalone TTF, OTF, or TTC bytes
through `Office2PdfConverter`. When no registered or embedded face covers a
CJK run, the result-bearing API emits a `fallback-used` warning whose `to`
value is `.notdef`.

Native Rust callers can use the same per-conversion path through
`ConvertOptions::font_bytes` and `ConvertOptions::last_resort_font_family`.

## CLI Options

| Flag | Description |
|------|-------------|
| `-o, --output <PATH>` | Output file path (single input only) |
| `--outdir <DIR>` | Output directory for batch conversion |
| `--paper <SIZE>` | Paper size: `a4`, `letter`, `legal` |
| `--landscape` | Force landscape orientation |
| `--pdf-a` | Produce PDF/A-2b compliant output |
| `--sheets <NAMES>` | XLSX sheet filter (comma-separated) |
| `--slides <RANGE>` | PPTX slide range (e.g. `1-5` or `3`) |
| `--font-path <DIR>` | Additional font directory override (repeatable) |

## Supported Formats

| Format | Status | Key Features |
|--------|--------|-------------|
| DOCX | Supported | Text, tables, images, drawing shapes, lists, code highlighting, headers/footers, page setup |
| PPTX | Supported | Slides, text boxes, shapes, tables, images, masters, gradients, effects |
| XLSX | Supported | Sheets, formatting, merged cells, column/row sizing, conditional formatting |

## License

Licensed under [Apache License, Version 2.0](LICENSE).

# plotters

- **URL:** https://github.com/plotters-rs/plotters
- **Stars:** ~4.5k
- **Language:** Rust
- **License:** MIT

## What it does

Pure Rust drawing library for data visualization. Supports SVG, PNG, and bitmap backends. WASM compatible. Supports line, bar, scatter, histogram, and area charts.

## Why relevant

- Most mature Rust charting library. Could render OOXML chart data (bar, line, pie, scatter) into SVG/PNG for embedding in Typst output.
- office2pdf renders supported chart families with Typst primitives; unsupported families retain fallback data tables. Plotters is an alternative backend reference.
- Pure Rust, WASM-compatible = aligns with our no-external-dependency philosophy.

## When to consult

- Implementing chart rendering from OOXML chart data
- Choosing chart rendering approach (SVG embed vs. Typst native)

# tpt-barcode-render

> Output renderers for the `tpt-barcode` workspace: self-contained SVG
> strings, PNG bytes (via the `image` crate), and ANSI/ASCII terminal
> previews.

Part of the [tpt-barcode](https://github.com/tpt-solutions/tpt-barcode) workspace.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode-render.svg)](https://crates.io/crates/tpt-barcode-render)
[![Docs.rs](https://docs.rs/tpt-barcode-render/badge.svg)](https://docs.rs/tpt-barcode-render)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## Renderers

| Format | Module | Notes |
|--------|--------|-------|
| SVG    | `svg`  | Builder API with quiet zone; one rect per dark module (2D) and run-length-merged bars (1D) |
| PNG    | `png`  | Via the `image` crate (`png` feature) |
| ANSI   | `ansi` | Unicode half-blocks for 2:1 aspect terminal previews |

## Quick start

```rust
use tpt_barcode_render::SvgBuilder;

// Square module grids (QR, Data Matrix)
let svg = SvgBuilder::new(&matrix, size)
    .module_size(4)
    .quiet_zone(4)
    .build();

// 1D barcodes
let svg = tpt_barcode_render::svg::render_1d(&widths, 2, 80);
```

Most applications should use the `tpt-barcode` facade's extension traits
(`qr.to_svg_string(4)`, `qr.to_png_bytes(8, 4)`) instead of calling this
crate directly.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Standard library support |
| `alloc` | via std | Heap-using APIs |
| `png`   | no      | PNG output via the `image` crate |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.

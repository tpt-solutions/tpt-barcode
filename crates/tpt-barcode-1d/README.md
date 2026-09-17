# tpt-barcode-1d

> One-dimensional barcode symbologies for the `tpt-barcode` workspace:
> Code 128, EAN-13, UPC-A, and Code 39 — encoding and decoding, with
> run-length input support for scanline scanners. `no_std` compatible.

Part of the [tpt-barcode](https://github.com/tpt-solutions/tpt-barcode) workspace.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode-1d.svg)](https://crates.io/crates/tpt-barcode-1d)
[![Docs.rs](https://docs.rs/tpt-barcode-1d/badge.svg)](https://docs.rs/tpt-barcode-1d)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## Symbologies

| Symbology | Encode | Decode | Notes                          |
|-----------|:------:|:------:|--------------------------------|
| Code 128  | ✅     | ✅     | Subset B; printable ASCII      |
| EAN-13    | ✅     | ✅     | 12 digits + auto check digit   |
| UPC-A     | ✅     | ✅     | Leading 0 + 11 digits          |
| Code 39   | ✅     | ✅     | Printable ASCII subset         |

## Quick start

```rust
use tpt_barcode_1d::{code128, ean13, runs};

// Encode (widths are alternating bar/space, starting with a bar)
let code = code128::encode_b(b"HELLO-128")?;

// Decode from module widths
let text = code128::decode_b(&code.modules)?;

// Decode from RUN LENGTHS — what an optical scanner or the image
// pipeline's scanlines actually produce
let runs = runs::modules_to_runs(&code.modules);
assert_eq!(runs::decode_code128_runs(&runs)?, "HELLO-128");

// EAN-13: digits are binary values 0-9, checksum auto-computed
let ean = ean13::encode(&[4, 0, 0, 6, 3, 8, 1, 3, 3, 3, 9, 3])?;
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Standard library support |
| `alloc` | via std | Heap-using APIs (String-based decoders) |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.

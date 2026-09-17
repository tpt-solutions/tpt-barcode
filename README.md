# tpt-barcode

> Zero-allocation barcode generation and scanning for Rust — `no_std` compatible, perspective-corrected QR scanning, pure MIT dependency chain.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode.svg)](https://crates.io/crates/tpt-barcode)
[![Docs.rs](https://docs.rs/tpt-barcode/badge.svg)](https://docs.rs/tpt-barcode)
[![CI](https://github.com/tpt-solutions/tpt-barcode/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-barcode/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

## Features

- **True zero-allocation scanning** — perspective correction uses `tpt-math-linalg-fixed` for compile-time-known 3×3 matrix inversion with no heap allocation.
- **`no_std` throughout** — every crate supports `no_std + alloc`; `std` is an opt-in feature.
- **Symbology coverage** — QR Code (encode + decode, v1–40), DataMatrix ECC 200 (encode + decode), Code 128, EAN-13, UPC-A, Code 39. PDF417 is scaffolding only (see the [changelog](CHANGELOG.md)).
- **Conformance-anchored** — QR and DataMatrix output verified matrix-for-matrix against reference implementations (`qrcode`, libdmtx); QR decoding verified against ISO-conformant foreign symbols.
- **Dual license** — `MIT OR Apache-2.0`.

## Quick Start

```toml
[dependencies]
tpt-barcode = "0.1"
```

### Generate a QR code

```rust
use tpt_barcode::prelude::*;

let qr = tpt_barcode::qr::encode("https://github.com/tpt-solutions", EcLevel::M)?;
let svg = tpt_barcode::render::svg::SvgBuilder::new(&qr.matrix, qr.size)
    .module_size(4)
    .build();
```

### Scan an image

```rust
use tpt_barcode::core::Format;

let img = image::open("skewed_qr.jpg")?.into_luma8();

let results = tpt_barcode::scan(img.as_raw(), img.width() as usize, img.height() as usize)
    .formats(&[Format::QrCode])
    .try_harder(true)
    .execute()?;

for result in results {
    println!("Decoded: {} ({:?})", result.text(), result.format());
    let corners = result.bounding_box();
    println!("Corners: {:?}", corners);
}
```

## Workspace Crates

| Crate | Description | Docs | `no_std` |
|---|---|---|---|
| [`tpt-barcode-core`](crates/tpt-barcode-core) | GF(256) (0x11D + 0x12D), Reed-Solomon error correction, shared traits | [README](crates/tpt-barcode-core/README.md) | yes |
| [`tpt-barcode-1d`](crates/tpt-barcode-1d) | Code 128, EAN-13, UPC-A, Code 39 + run-length scanning | [README](crates/tpt-barcode-1d/README.md) | yes |
| [`tpt-barcode-2d`](crates/tpt-barcode-2d) | QR Code, DataMatrix, PDF417 (encode + decode) | [README](crates/tpt-barcode-2d/README.md) | yes |
| [`tpt-barcode-image`](crates/tpt-barcode-image) | Binarization, finder detection, homography | [README](crates/tpt-barcode-image/README.md) | yes |
| [`tpt-barcode-render`](crates/tpt-barcode-render) | SVG, PNG, ANSI terminal output | [README](crates/tpt-barcode-render/README.md) | std |
| [`tpt-barcode`](crates/tpt-barcode) | Facade — re-exports all crates via feature flags | [README](README.md) | opt-in |

## Feature Flags

```toml
[dependencies]
tpt-barcode = { version = "0.1", default-features = false, features = ["alloc", "2d"] }
```

| Flag | Default | Description |
|---|---|---|
| `std` | yes | Enable standard library support |
| `alloc` | via std | Enable heap allocation (`no_std + alloc`) |
| `1d` | yes | Include 1D symbologies |
| `2d` | yes | Include 2D symbologies (QR, DataMatrix, PDF417) |
| `scan` | yes | Include image scanning pipeline |
| `render` | yes | Include output renderers |
| `simd` | no | SIMD-accelerated binarization (SSE2 threshold stage) |
| `png` | no | PNG output via `image` crate |

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

Copyright 2025 TPT Solutions.

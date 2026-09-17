# tpt-barcode-2d

> Two-dimensional barcode symbologies for the `tpt-barcode` workspace:
> QR Code (versions 1–40), Data Matrix (ECC 200), and PDF417 — encoding
> *and* decoding, all `no_std` compatible.

Part of the [tpt-barcode](https://github.com/tpt-solutions/tpt-barcode) workspace.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode-2d.svg)](https://crates.io/crates/tpt-barcode-2d)
[![Docs.rs](https://docs.rs/tpt-barcode-2d/badge.svg)](https://docs.rs/tpt-barcode-2d)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## Symbologies

| Symbology   | Encode | Decode | Notes                                            |
|-------------|:------:|:------:|--------------------------------------------------|
| QR Code     | ✅     | ✅     | v1–40, numeric/alphanumeric/byte, all EC levels  |
| Data Matrix | ✅     | ✅     | ECC 200, 10×10–26×26, ASCII encodation           |
| PDF417      | ✅     | ✅     | Text/byte/numeric compaction, EC levels 0–8      |

## Quick start

```rust
use tpt_barcode_2d::qr;
use tpt_barcode_core::EcLevel;

// Encode — auto-selects version and mask
let qr = qr::encode("https://example.com", EcLevel::M)?;

// ...or pin version/mask for reproducible output
let qr = qr::QrBuilder::new()
    .ec_level(EcLevel::Q)
    .version(3)
    .mask(2)
    .build("pinned")?;

// Decode a module grid (e.g. sampled from a camera image)
let payload = qr::decode_grid(&qr.matrix, qr.size)?;
```

Data Matrix and PDF417 live in their own modules with the same
encode → matrix → decode shape (`datamatrix::encode`/`decode`,
`pdf417::encode`/`decode`).

## Conformance

Correctness is cross-checked against independent reference implementations:
QR symbols match the Python `qrcode` library matrix-for-matrix; Data Matrix
symbols match libdmtx; PDF417 symbols are decoded by `zxing-cpp`.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Standard library support |
| `alloc` | via std | Heap-using APIs |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.

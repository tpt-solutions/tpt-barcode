# tpt-barcode-core

> Mathematical heart of the `tpt-barcode` workspace: shared symbology traits,
> GF(256) field arithmetic for two primitive polynomials, and a Reed–Solomon
> error-correction engine. `no_std` compatible with zero runtime table init.

Part of the [tpt-barcode](https://github.com/tpt-solutions/tpt-barcode) workspace.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode-core.svg)](https://crates.io/crates/tpt-barcode-core)
[![Docs.rs](https://docs.rs/tpt-barcode-core/badge.svg)](https://docs.rs/tpt-barcode-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## What's inside

- **GF(256) arithmetic** (`gf256`) — const-built log/antilog tables for both
  the QR Code field (**0x11D**) and the Data Matrix field (**0x12D**),
  exposed through the zero-sized `QrField` / `DmField` descriptors and the
  `Field` trait. Zero runtime initialization cost.
- **Reed–Solomon engine** (`reed_solomon`) — polynomial-long-division encoder
  and a full decoder (Berlekamp–Massey → Chien search → Forney magnitudes).
  Generic over the field and the generator-root / syndrome start exponent,
  covering the QR convention (roots α⁰…αⁿ⁻¹) *and* the ECC 200 convention
  (roots α¹…αⁿ).
- **Shared traits** (`traits`) — `Format`, `EcLevel`, `EncodeError`,
  `DecodeError` used across the workspace, all with `Display` +
  `std::error::Error` impls.

## Quick start

```rust
use tpt_barcode_core::{gf256::{QrField, Field}, reed_solomon};

// 10 EC codewords for the ISO worked example
let data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17];
let mut ec = [0u8; 10];
reed_solomon::encode(&data, 10, &mut ec);
assert_eq!(ec, [196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
```

Correctness is anchored to published vectors: the ISO/IEC 18004 worked
example above, the Data Matrix "123456" example, and a full-table
cross-check against a shift-and-reduce GF multiplier.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Standard library support (`Error` impls) |
| `alloc` | via std | Heap-using APIs |

`no_std` works without either feature; the RS engine is allocation-free.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.

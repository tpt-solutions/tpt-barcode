# Changelog — tpt-barcode-core

All notable changes to this crate are documented here. Workspace-wide
changes are in the [root CHANGELOG](../../CHANGELOG.md).

## [0.1.0] — unreleased

### Added
- GF(256) field arithmetic with const-built log/antilog tables for the QR
  Code field (0x11D) and Data Matrix field (0x12D), exposed via the
  zero-sized `QrField` / `DmField` descriptors and the `Field` trait.
- Reed-Solomon encoder (polynomial long division) and full decoder
  (Berlekamp-Massey → Chien search → Forney), generic over the field and
  the generator-root/syndrome start exponent.
- Shared `Format`, `EcLevel`, `EncodeError`, `DecodeError` types with
  `Display` and `std::error::Error` impls.
- Tests anchored to the ISO/IEC 18004 worked example and a full-table
  GF multiplier cross-check.

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

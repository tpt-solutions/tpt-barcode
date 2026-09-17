# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — unreleased

### Added

#### tpt-barcode-core
- GF(256) field arithmetic with `const`-built log/antilog tables for both the
  QR Code field (0x11D) and the Data Matrix field (0x12D), exposed through the
  zero-sized `QrField` / `DmField` descriptors (`gf256::Field` trait).
- Reed-Solomon encoder (polynomial long division) and decoder
  (Berlekamp–Massey → Chien search → Forney), generic over the field and the
  generator-root / syndrome start exponent, covering QR Code and ECC 200
  conventions. Zero-allocation, `no_std`.
- Shared `Symbology` traits, `Format`, `EcLevel`, `EncodeError`, `DecodeError`.

#### tpt-barcode-1d
- Code 128 encoder and decoder (subsets A/B/C), Code 39, EAN-13 (with checksum
  validation) and UPC-A.

#### tpt-barcode-2d
- QR Code generation for versions 1–40 with automatic version/mode selection
  (numeric, alphanumeric, byte), ISO 18004 zigzag data placement, BCH(15,5)
  format information, and penalty-optimal mask selection.
- Error traits implement `Display` and `std::error::Error`; `Format`/`EcLevel`
  implement `Display`.
- QR Code decoding from module grids: format-info read with BCH validation and
  3-bit-error correction, unmasking, codeword extraction, block
  de-interleaving, Reed-Solomon error correction, and mode-stream parsing
  (numeric / alphanumeric / byte / kanji / ECI / structured-append).
- GS1 Application Identifier parsing (`two_d::gs1::parse` /
  `parse_text`) for FNC1-carrying QR Code and Data Matrix symbols.
- Data Matrix (ECC 200) generation and decoding: ASCII encodation with digit
  pairing and upper shift, Reed-Solomon ECC over GF(256)/0x12D, ISO/IEC 16022
  Annex M symbol placement (utah shapes + corner patterns), solid-L finder and
  timing borders, and the matching inverse pipeline. Nine single-block symbol
  sizes from 10×10 to 26×26.
- PDF417 encoding and decoding (ISO/IEC 15438): text, byte, and numeric
  compaction; the 3×929 codeword cluster pattern table; Reed-Solomon over
  GF(929) (prime-field Berlekamp–Massey / Chien / Forney with the
  odd-characteristic Forney sign and full σ′ scalar factors); symbol length
  descriptor and pad rules; row/column layout with row-indicator codewords;
  start/stop rendering; and the inverse decode path with row-indicator
  geometry decoding. Auto-compaction picks the densest mode per payload.

#### tpt-barcode-image
- Bradley adaptive local-mean binarization with an integral image, Otsu global
  threshold, and an SSE2 vectorized threshold stage behind the `simd` feature
  (bit-identical to the scalar path).
- Sobel edge detection.
- QR finder-pattern detection with 1:1:3:1:1 run-ratio scanning, horizontal +
  vertical cross-checking, centroid merging, and isosceles-right-triple
  selection.
- Perspective correction: DLT homography from 4 point pairs, closed-form 3×3
  inversion via `tpt-math-linalg-fixed`, and nearest-neighbour grid sampling.

#### tpt-barcode-render
- SVG renderer (builder API, one `<rect>` per dark module, quiet zone), PNG
  renderer behind the `png` feature, and ANSI/ASCII terminal renderers.

#### tpt-barcode (facade)
- Feature-gated re-exports (`1d`, `2d`, `scan`, `render`, `png`, `simd`,
  `std`/`alloc`) and a `prelude`.
- `scan()` builder API (`.formats()`, `.try_harder()`, `.execute()`) running
  the full pipeline: binarize → finder detection → homography perspective
  correction → module-grid sampling → QR decode, returning `ScanResult`s with
  text, format, and bounding box.

### Changed
- Initial public state; everything is new.

### Verified against reference implementations
- QR Reed-Solomon vectors from the thonky.com QR tutorial.
- QR symbol matrices cross-checked against the Python `qrcode` library.
- Data Matrix symbols cross-checked against libdmtx (via `pylibdmtx`) and the
  `zxing-cpp` reference decoder.
- PDF417 symbols cross-checked in both directions: our encoder's output is
  decoded by `zxing-cpp`, and our decoder reads symbols produced by
  `pdf417gen` (permanent fixture test).

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

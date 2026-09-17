# Changelog — tpt-barcode-2d

All notable changes to this crate are documented here. Workspace-wide
changes are in the [root CHANGELOG](../../CHANGELOG.md).

## [0.1.0] — unreleased

### Added
- QR Code: encode (versions 1-40, numeric/alphanumeric/byte, auto
  version/mask selection, ISO 18004 penalty scoring) and decode (format
  info with BCH validation and 3-bit correction, unmask, de-interleave,
  Reed-Solomon, mode-stream parsing). `QrBuilder` for forced version/mask.
- Data Matrix (ECC 200): encode (ASCII encodation, digit pairing, upper
  shift, GF(256)/0x12D Reed-Solomon, Annex M placement) and decode.
- PDF417: text/byte/numeric compaction, GF(929) Reed-Solomon
  (Berlekamp-Massey / Chien / Forney with odd-characteristic Forney sign),
  row/column layout with row-indicator codewords, and decode with
  row-indicator geometry recovery.
- QR Code Kanji mode: `encode_sjis` interleaves Shift-JIS Kanji segments
  (base-192 13-bit packing) with byte segments; decode maps the 13-bit
  values back to Shift-JIS via the base-192 scheme for both JIS X 0208
  ranges.
- GS1: `qr::encode_gs1` and `datamatrix::encode_gs1` emit FNC1-carrying
  symbols (ISO 18004 FNC1 first position; ECC 200 codeword 232 after the
  SLD); `gs1::parse` / `gs1::parse_text` split a decoded element string
  into typed `(AI, value)` pairs using length-aware AI rules
  (fixed-length values, FNC1-terminated variable values).
- Conformance anchors: QR symbols match the Python `qrcode` library; Data
  Matrix symbols match libdmtx; PDF417 output decoded by `zxing-cpp`.

### Fixed
- QR data placement zigzag skipped column 0 and mis-paired strips.
- QR format information was XOR-masked twice and placed in a non-standard
  bit order.
- QR mask selection now scores with format information written, per ISO
  18004.
- QR version table completed to all 40 versions (several block structures
  were wrong).

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

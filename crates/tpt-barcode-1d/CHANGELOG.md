# Changelog — tpt-barcode-1d

All notable changes to this crate are documented here. Workspace-wide
changes are in the [root CHANGELOG](../../CHANGELOG.md).

## [0.1.0] — unreleased

### Added
- Code 128 (Subset B) encoder and decoder.
- EAN-13 encoder with auto check digit and decoder with checksum
  validation.
- UPC-A encoder and decoder (derived from EAN-13).
- Code 39 encoder and decoder.
- `runs` module: run-length input for scanline scanners — run-length to
  module conversion, width normalization, and run-length decode entry
  points for every symbology above.

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

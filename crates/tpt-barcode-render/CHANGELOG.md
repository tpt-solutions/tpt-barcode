# Changelog — tpt-barcode-render

All notable changes to this crate are documented here. Workspace-wide
changes are in the [root CHANGELOG](../../CHANGELOG.md).

## [0.1.0] — unreleased

### Added
- SVG renderer with builder API (module size, quiet zone, colors) for
  square module grids, plus a rectangular-matrix variant with
  run-length-merged bars for PDF417.
- 1D SVG renderer for alternating bar/space width vectors.
- PNG renderer for square grids and 1D barcodes (`png` feature).
- ANSI terminal renderer using Unicode half-blocks, and an ASCII
  fallback.

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

# Changelog — tpt-barcode-image

All notable changes to this crate are documented here. Workspace-wide
changes are in the [root CHANGELOG](../../CHANGELOG.md).

## [0.1.0] — unreleased

### Added
- Bradley adaptive local-mean binarization with an integral image; Otsu
  global threshold; global binarization.
- SSE2 SIMD threshold stage (`simd` feature, x86_64, runtime-detected) —
  bit-identical with the scalar path, verified by test.
- Sobel edge detection.
- QR finder-pattern detection: 1:1:3:1:1 run-ratio scan with
  horizontal/vertical cross-checking, order-independent centroid merging,
  and isosceles-right-triple selection with rotation-tolerant corner
  assignment.
- DLT homography from 4 point pairs, closed-form 3x3 inversion via
  `tpt-math-linalg-fixed`, nearest-neighbour grid sampling.

### Fixed
- Finder merge biased the centroid toward the last scan-row hit.
- Integral-image inclusion-exclusion could underflow `u32` on
  mostly-light windows (now computed in signed arithmetic).
- Spurious finder candidates from data regions are now rejected by the
  cross-check.

[0.1.0]: https://github.com/tpt-solutions/tpt-barcode

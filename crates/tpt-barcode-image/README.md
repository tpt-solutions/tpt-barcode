# tpt-barcode-image

> The computer-vision pipeline of the `tpt-barcode` workspace: adaptive
> binarization, QR finder-pattern detection, and perspective-corrected
> module-grid sampling via homography — `no_std` compatible.

Part of the [tpt-barcode](https://github.com/tpt-solutions/tpt-barcode) workspace.

[![Crates.io](https://img.shields.io/crates/v/tpt-barcode-image.svg)](https://crates.io/crates/tpt-barcode-image)
[![Docs.rs](https://docs.rs/tpt-barcode-image/badge.svg)](https://docs.rs/tpt-barcode-image)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## Pipeline stages

| Stage | Module | Notes |
|-------|--------|-------|
| Binarization | `binarize` | Bradley adaptive local-mean; Otsu global; SSE2 SIMD path (`simd` feature) bit-identical to scalar |
| Edge detection | `edge` | Sobel operator |
| Finder detection | `finder` | 1:1:3:1:1 run-ratio scan, horizontal + vertical cross-check, centroid merge, isosceles-right triple selection |
| Perspective correction | `homography` | DLT homography from 4 points, closed-form 3×3 inversion, nearest-neighbour grid sampling |

The finder → homography → sampling path performs **zero heap
allocations**, enforced by a CI test with a counting allocator.

## Quick start

```rust
use tpt_barcode_image::{binarize, finder, homography};

// 1. Binarize a grayscale image
let mut binary = vec![0u8; pixels.len()];
let t = binarize::global_threshold(&pixels);
binarize::binarize_global(&pixels, &mut binary, t);

// 2. Locate QR finder patterns
let candidates = finder::find_candidates(&binary, width, height);
let (tl, tr, bl) = finder::select_finder_triple(&candidates)?;

// 3. Sample the module grid through a homography
let h = homography::compute_homography(&src_points, &dst_points)?;
let mut grid = vec![0u8; size * size];
homography::sample_grid(&binary, width, height, &h, &mut grid, size, size);
```

Most applications should use the `tpt-barcode` facade's `scan()` API,
which wires these stages together.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Standard library support + `image` crate |
| `alloc` | via std | Heap-using APIs |
| `simd`  | no      | SSE2-accelerated binarization (x86_64) |

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.

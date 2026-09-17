//! Image processing pipeline for barcode scanning.
//!
//! - Adaptive binarization (Bradley local mean; SIMD-accelerated behind `simd` feature)
//! - Sobel edge detection
//! - QR finder pattern detection using `tpt-math-geometry`
//! - Zero-allocation homography inversion via `tpt-math-linalg-fixed::Matrix3`
//! - Perspective-corrected pixel grid sampling
//!
//! `no_std` compatible. Enable `alloc` for heap-using APIs; `std` (default)
//! also enables the `image` crate for pixel buffer I/O.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod binarize;
pub mod edge;
pub mod finder;
pub mod homography;
mod neon;

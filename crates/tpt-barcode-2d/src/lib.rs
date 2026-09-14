//! 2D barcode symbologies: QR Code, DataMatrix, and PDF417.
//!
//! Leverages `tpt-math-linalg-fixed` for zero-allocation 3×3 homography inversion
//! and `tpt-math-geometry` for perspective-corrected pixel sampling during scanning.
//!
//! `no_std` compatible. Enable `alloc` for heap-using APIs; `std` (default)
//! for full standard-library support.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod datamatrix;
pub mod pdf417;
pub mod qr;

#[cfg(feature = "alloc")]
pub use qr::QrCode;

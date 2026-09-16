//! 1D barcode symbologies: Code 128, EAN-13, UPC-A, and Code 39.
//!
//! `no_std` compatible. Enable `alloc` for heap-using APIs; `std` (default)
//! for full standard-library support.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod code128;
pub mod code39;
pub mod ean13;
pub mod runs;
pub mod upca;

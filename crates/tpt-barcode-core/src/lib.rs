//! Core barcode primitives: GF(256) arithmetic, Reed-Solomon error correction,
//! and shared symbology traits.
//!
//! This crate is `no_std` compatible. Enable the `alloc` feature for heap-using
//! APIs and `std` (default) for full standard-library support.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod gf256;
pub mod reed_solomon;
pub mod traits;

pub use traits::{DecodeError, EcLevel, EncodeError, Format};

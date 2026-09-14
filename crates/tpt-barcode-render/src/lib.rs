//! Barcode output renderers: SVG string builder, PNG (behind `png` feature),
//! and ANSI terminal block-character renderer.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod ansi;
pub mod png;
pub mod svg;

#[cfg(feature = "alloc")]
pub use svg::SvgBuilder;

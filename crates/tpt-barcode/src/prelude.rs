//! Convenience re-exports for the most commonly used types.
//!
//! ```rust
//! use tpt_barcode::prelude::*;
//! ```

pub use tpt_barcode_core::{DecodeError, EcLevel, EncodeError, Format};

#[cfg(all(feature = "2d", feature = "alloc"))]
pub use tpt_barcode_2d::qr::QrCode;

#[cfg(feature = "render")]
pub use tpt_barcode_render::SvgBuilder;

#[cfg(feature = "scan")]
pub use crate::{ScanResult, Scanner};

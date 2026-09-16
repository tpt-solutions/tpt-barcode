//! Convenience re-exports for the most commonly used types.
//!
//! ```rust
//! use tpt_barcode::prelude::*;
//! ```

pub use tpt_barcode_core::{DecodeError, EcLevel, EncodeError, Format};

#[cfg(all(feature = "2d", feature = "alloc"))]
pub use tpt_barcode_2d::qr::{QrBuilder, QrCode};

#[cfg(feature = "render")]
pub use tpt_barcode_render::SvgBuilder;

#[cfg(all(feature = "2d", feature = "render"))]
pub use crate::{DataMatrixRenderExt, Pdf417RenderExt, QrRenderExt};

#[cfg(all(feature = "scan", feature = "alloc"))]
pub use crate::{ScanResult, Scanner, SymbolMetadata};

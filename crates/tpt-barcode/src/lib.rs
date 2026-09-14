//! `tpt-barcode` — zero-allocation barcode generation and scanning for Rust.
//!
//! This is the facade crate. It re-exports the individual sub-crates behind
//! feature flags so users can depend on a single crate and pay only for what
//! they enable.
//!
//! # Quick Start
//!
//! ```rust
//! use tpt_barcode::prelude::*;
//!
//! // Generate a QR code
//! let qr = tpt_barcode::qr::encode("https://github.com/tpt-solutions", EcLevel::M)?;
//! let svg = SvgBuilder::new(&qr.matrix, qr.size).module_size(4).build();
//!
//! // Scan an image (requires `scan` feature)
//! # Ok::<(), EncodeError>(())
//! ```
//!
//! # Feature Flags
//!
//! | Flag     | Default | Description                                      |
//! |----------|---------|--------------------------------------------------|
//! | `std`    | yes     | Standard library support                         |
//! | `alloc`  | via std | Heap allocation (`no_std + alloc`)               |
//! | `1d`     | yes     | Code 128, EAN-13, UPC-A, Code 39                 |
//! | `2d`     | yes     | QR Code, DataMatrix, PDF417                      |
//! | `scan`   | yes     | Image scanning pipeline                          |
//! | `render` | yes     | SVG, PNG, ANSI output                            |
//! | `simd`   | no      | SIMD-accelerated binarization                    |
//! | `png`    | no      | PNG output via `image` crate                     |

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub use tpt_barcode_core as core;

#[cfg(feature = "1d")]
pub use tpt_barcode_1d as one_d;

#[cfg(feature = "2d")]
pub use tpt_barcode_2d as two_d;

#[cfg(feature = "scan")]
pub use tpt_barcode_image as image_scan;

#[cfg(feature = "render")]
pub use tpt_barcode_render as render;

pub mod prelude;

// Convenience re-export of the QR encode function
#[cfg(all(feature = "2d", feature = "alloc"))]
pub mod qr {
    //! QR Code encoding convenience module.
    pub use tpt_barcode_2d::qr::{encode, QrCode};
}

// ── Scanner API ──────────────────────────────────────────────────────────────

/// The result of scanning a barcode from an image.
#[cfg(all(feature = "scan", feature = "alloc"))]
pub struct ScanResult {
    /// Decoded text (UTF-8).
    pub text: alloc::string::String,
    /// Barcode format.
    pub format: tpt_barcode_core::Format,
    /// The four corner points of the barcode in the source image (pixels).
    pub bounding_box: [tpt_math_geometry::Point2<f32>; 4],
}

#[cfg(all(feature = "scan", feature = "alloc"))]
impl ScanResult {
    /// Decoded text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Barcode format.
    pub fn format(&self) -> tpt_barcode_core::Format {
        self.format
    }

    /// Four corner points of the barcode in the source image.
    pub fn bounding_box(&self) -> [tpt_math_geometry::Point2<f32>; 4] {
        self.bounding_box
    }
}

/// Builder for configuring and executing a barcode scan.
#[cfg(all(feature = "scan", feature = "alloc"))]
pub struct Scanner<'a> {
    pixels: &'a [u8],
    width: usize,
    height: usize,
    formats: alloc::vec::Vec<tpt_barcode_core::Format>,
    try_harder: bool,
}

#[cfg(all(feature = "scan", feature = "alloc"))]
impl<'a> Scanner<'a> {
    /// Create a scanner for a grayscale (luma8) image.
    pub fn new(pixels: &'a [u8], width: usize, height: usize) -> Self {
        Self {
            pixels,
            width,
            height,
            formats: alloc::vec![tpt_barcode_core::Format::QrCode],
            try_harder: false,
        }
    }

    /// Restrict scanning to the given set of formats.
    pub fn formats(mut self, formats: &[tpt_barcode_core::Format]) -> Self {
        self.formats = formats.to_vec();
        self
    }

    /// Enable adaptive binarization plus perspective correction (homography
    /// un-skewing) for QR codes.
    ///
    /// Uses `tpt-math-linalg-fixed::Matrix3::inverse` for zero-allocation 3×3
    /// matrix inversion.
    pub fn try_harder(mut self, enabled: bool) -> Self {
        self.try_harder = enabled;
        self
    }

    /// Execute the scan and return all found barcodes.
    pub fn execute(self) -> Result<alloc::vec::Vec<ScanResult>, tpt_barcode_core::DecodeError> {
        use tpt_barcode_image::{binarize, finder};

        // ── 1. Binarize ──────────────────────────────────────────────────────
        let mut binary = alloc::vec![0u8; self.pixels.len()];
        if self.try_harder {
            let window = (self.width / 8).max(1);
            binarize::binarize_adaptive(
                self.pixels,
                self.width,
                self.height,
                &mut binary,
                window,
                0.15,
            );
        } else {
            let t = binarize::global_threshold(self.pixels);
            binarize::binarize_global(self.pixels, &mut binary, t);
        }

        // ── 2. Locate finder patterns ────────────────────────────────────────
        let candidates = finder::find_candidates(&binary, self.width, self.height);
        if candidates.len() < 3 {
            return Ok(alloc::vec::Vec::new());
        }

        let mut results = alloc::vec::Vec::new();
        let want_qr = self.formats.contains(&tpt_barcode_core::Format::QrCode);

        if want_qr {
            if let Some(result) = self.scan_qr(&binary, &candidates) {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Decode a QR symbol given finder candidates and a binarized image.
    fn scan_qr(
        &self,
        binary: &[u8],
        candidates: &[tpt_barcode_image::finder::FinderCandidate],
    ) -> Option<ScanResult> {
        use tpt_barcode_image::{finder, homography};

        let (tl, tr, bl) = finder::select_finder_triple(candidates)?;
        let module_px = (tl.module_size + tr.module_size + bl.module_size) / 3.0;
        if module_px < 1.0 {
            return None;
        }

        // Centre-to-centre distance equals (size − 7) modules.
        let dist_tr = ((tr.cx - tl.cx).powi(2) + (tr.cy - tl.cy).powi(2)).sqrt();
        let dist_bl = ((bl.cx - tl.cx).powi(2) + (bl.cy - tl.cy).powi(2)).sqrt();
        let est = ((dist_tr + dist_bl) / 2.0 / module_px + 7.0).round() as i32;
        // Snap to the nearest valid QR symbol size (21 + 4k modules).
        let size = {
            let v = ((est - 21) as f32 / 4.0).round().max(0.0) as usize;
            let v = v.min(39);
            21 + 4 * v
        };

        // Homography from module coordinates to image coordinates. The fourth
        // corner is extrapolated assuming a parallelogram, which the DLT fit
        // then refines for mild perspective.
        let module_pt = |x: f64, y: f64| tpt_math_geometry::Point2::from_array([x, y]);
        let src = [
            module_pt(3.5, 3.5),
            module_pt(size as f64 - 3.5, 3.5),
            module_pt(3.5, size as f64 - 3.5),
            module_pt(size as f64 - 3.5, size as f64 - 3.5),
        ];
        let (tlx, tly) = (tl.cx as f64, tl.cy as f64);
        let (trx, tryy) = (tr.cx as f64, tr.cy as f64);
        let (blx, bly) = (bl.cx as f64, bl.cy as f64);
        let img_pt = |x: f64, y: f64| tpt_math_geometry::Point2::from_array([x, y]);
        let dst = [
            img_pt(tlx, tly),
            img_pt(trx, tryy),
            img_pt(blx, bly),
            img_pt(
                tlx + (trx - tlx) + (blx - tlx),
                tly + (tryy - tly) + (bly - tly),
            ),
        ];

        let h = homography::compute_homography(&src, &dst)?;

        // Perspective-corrected sampling of the module grid. `sample_grid`
        // takes the transform mapping output (module) pixels → image pixels,
        // which is exactly `h` (module → image).
        let mut grid = alloc::vec![0u8; size * size];
        homography::sample_grid(binary, self.width, self.height, &h, &mut grid, size, size);

        // Decode: format info → unmask → RS → payload
        let payload = tpt_barcode_2d::qr::decode_grid(&grid, size).ok()?;

        // Bounding box: the four symbol corners in image coordinates
        let corner = |col: f64, row: f64| {
            let (x, y) = homography::map_point(&h, col, row);
            tpt_math_geometry::Point2::from_array([x as f32, y as f32])
        };
        let bounding_box = [
            corner(0.0, 0.0),
            corner(size as f64, 0.0),
            corner(size as f64, size as f64),
            corner(0.0, size as f64),
        ];

        Some(ScanResult {
            text: alloc::string::String::from_utf8_lossy(&payload).into_owned(),
            format: tpt_barcode_core::Format::QrCode,
            bounding_box,
        })
    }
}

/// Begin scanning a grayscale (luma8) pixel buffer.
///
/// # Example
///
/// ```rust,no_run
/// let pixels = vec![0u8; 200 * 200];
/// let results = tpt_barcode::scan(&pixels, 200, 200)
///     .formats(&[tpt_barcode::core::Format::QrCode])
///     .try_harder(true)
///     .execute()?;
/// # Ok::<(), tpt_barcode::core::DecodeError>(())
/// ```
#[cfg(all(feature = "scan", feature = "alloc"))]
pub fn scan(pixels: &[u8], width: usize, height: usize) -> Scanner<'_> {
    Scanner::new(pixels, width, height)
}

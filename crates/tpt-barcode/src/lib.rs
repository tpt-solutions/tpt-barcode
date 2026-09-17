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
//! let svg = qr.to_svg_string(4);
//!
//! // Scan an image (requires `scan` feature)
//! # Ok::<(), EncodeError>(())
//! ```
//!
//! # Feature Flags
//!
//! | Flag          | Default | Description                                      |
//! |---------------|---------|--------------------------------------------------|
//! | `std`         | yes     | Standard library support                         |
//! | `alloc`       | via std | Heap allocation (`no_std + alloc`)               |
//! | `1d`          | yes     | Code 128, EAN-13, UPC-A, Code 39                 |
//! | `2d`          | yes     | QR Code, DataMatrix, PDF417                      |
//! | `scan`        | yes     | Image scanning pipeline                          |
//! | `render`      | yes     | SVG, PNG, ANSI output                            |
//! | `image-input` | no      | `scan_image` helper for `image` crate buffers    |
//! | `simd`        | no      | SIMD-accelerated binarization                    |
//! | `png`         | no      | PNG output via `image` crate                     |

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
    pub use tpt_barcode_2d::qr::{
        decode_grid_detailed, encode, encode_gs1, encode_sjis, DecodedQr, QrBuilder, QrCode,
    };
}

// ── Extension traits: one-line rendering ─────────────────────────────────────

/// One-line rendering for QR Codes.
///
/// ```rust
/// use tpt_barcode::prelude::*;
///
/// let qr = tpt_barcode::qr::encode("hello", EcLevel::L).unwrap();
/// let svg = qr.to_svg_string(4);
/// assert!(svg.starts_with("<svg"));
/// ```
#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
pub trait QrRenderExt {
    /// Render to a self-contained SVG string (`module_size` px per module).
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String;

    /// Render to ANSI terminal half-blocks for quick terminal preview.
    fn to_ansi(&self) -> alloc::string::String;

    /// Render to PNG bytes (`module_px` px per module, `quiet_zone` modules).
    #[cfg(feature = "png")]
    fn to_png_bytes(&self, module_px: u32, quiet_zone: u32) -> alloc::vec::Vec<u8>;
}

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
impl QrRenderExt for tpt_barcode_2d::qr::QrCode {
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String {
        tpt_barcode_render::SvgBuilder::new(&self.matrix, self.size)
            .module_size(module_size)
            .build()
    }

    fn to_ansi(&self) -> alloc::string::String {
        tpt_barcode_render::ansi::render(&self.matrix, self.size, 2)
    }

    #[cfg(feature = "png")]
    fn to_png_bytes(&self, module_px: u32, quiet_zone: u32) -> alloc::vec::Vec<u8> {
        tpt_barcode_render::png::render_to_png(&self.matrix, self.size, module_px, quiet_zone)
    }
}

/// One-line rendering for Data Matrix symbols.
#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
pub trait DataMatrixRenderExt {
    /// Render to a self-contained SVG string (`module_size` px per module).
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String;

    /// Render to PNG bytes (`module_px` px per module, `quiet_zone` modules).
    #[cfg(feature = "png")]
    fn to_png_bytes(&self, module_px: u32, quiet_zone: u32) -> alloc::vec::Vec<u8>;
}

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
impl DataMatrixRenderExt for tpt_barcode_2d::datamatrix::DataMatrix {
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String {
        tpt_barcode_render::SvgBuilder::new(&self.matrix, self.size)
            .module_size(module_size)
            .build()
    }

    #[cfg(feature = "png")]
    fn to_png_bytes(&self, module_px: u32, quiet_zone: u32) -> alloc::vec::Vec<u8> {
        tpt_barcode_render::png::render_to_png(&self.matrix, self.size, module_px, quiet_zone)
    }
}

/// One-line rendering for PDF417 symbols (rectangular).
#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
pub trait Pdf417RenderExt {
    /// Render to a self-contained SVG string (`module_size` px per module).
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String;
}

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
impl Pdf417RenderExt for tpt_barcode_2d::pdf417::Pdf417 {
    fn to_svg_string(&self, module_size: u32) -> alloc::string::String {
        tpt_barcode_render::svg::render_matrix_svg(
            &self.matrix,
            self.width,
            self.height,
            module_size,
            2,
        )
    }
}

// ── Scanner API ──────────────────────────────────────────────────────────────

/// Symbology-specific detail for a scanned symbol.
#[cfg(all(feature = "scan", feature = "alloc"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMetadata {
    /// No structural detail available (linear symbols).
    None,
    /// QR Code structure.
    Qr {
        /// Version (1–40; side length = 21 + 4·(version−1)).
        version: u8,
        /// Mask pattern id (0–7).
        mask: u8,
        /// Error-correction level.
        ec_level: Option<tpt_barcode_core::EcLevel>,
    },
    /// Data Matrix structure.
    DataMatrix {
        /// Side length in modules.
        size: usize,
    },
    /// PDF417 structure.
    Pdf417 {
        /// Data columns.
        cols: usize,
        /// Row count.
        rows: usize,
        /// Error-correction level (0–8).
        ec_level: u8,
    },
}

/// The result of scanning a barcode from an image.
#[cfg(all(feature = "scan", feature = "alloc"))]
pub struct ScanResult {
    /// Decoded text (UTF-8).
    pub text: alloc::string::String,
    /// Barcode format.
    pub format: tpt_barcode_core::Format,
    /// The four corner points of the barcode in the source image (pixels).
    pub bounding_box: [tpt_math_geometry::Point2<f32>; 4],
    /// Symbology-specific structure.
    pub metadata: SymbolMetadata,
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

    /// Symbology-specific structure.
    pub fn metadata(&self) -> SymbolMetadata {
        self.metadata
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
    bilinear: bool,
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
            bilinear: false,
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

    /// Use bilinear interpolation when sampling the module grid (softer
    /// edges on low-resolution or perspective-skewed images; the
    /// nearest-neighbour default is sharper on clean sources).
    pub fn bilinear(mut self, enabled: bool) -> Self {
        self.bilinear = enabled;
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

        let mut results = alloc::vec::Vec::new();

        // ── 2. QR symbols (possibly several) ─────────────────────────────────
        if self.formats.contains(&tpt_barcode_core::Format::QrCode) {
            let candidates = finder::find_candidates(&binary, self.width, self.height);
            if candidates.len() >= 3 {
                self.scan_all_qr(&binary, candidates, &mut results);
            }
        }

        // ── 3. Linear (1D) symbols via sampled scanlines ─────────────────────
        type LinearDecoder =
            fn(&[u32]) -> Result<alloc::string::String, tpt_barcode_core::DecodeError>;
        const LINEAR: [(tpt_barcode_core::Format, LinearDecoder); 4] = [
            (tpt_barcode_core::Format::Code128, |runs| {
                tpt_barcode_1d::runs::decode_code128_runs(runs)
                    .map_err(|_| tpt_barcode_core::DecodeError::InvalidFormat)
            }),
            (tpt_barcode_core::Format::Code39, |runs| {
                tpt_barcode_1d::runs::decode_code39_runs(runs)
                    .map_err(|_| tpt_barcode_core::DecodeError::InvalidFormat)
            }),
            (tpt_barcode_core::Format::Ean13, |runs| {
                tpt_barcode_1d::runs::decode_ean13_runs(runs).map(|digits| {
                    // digit values 0-9 → ASCII text
                    let ascii: alloc::vec::Vec<u8> = digits.iter().map(|&d| d + b'0').collect();
                    alloc::string::String::from_utf8(ascii).expect("digits 0-9 are ASCII")
                })
            }),
            (tpt_barcode_core::Format::UpcA, |runs| {
                tpt_barcode_1d::runs::decode_upca_runs(runs)
                    .map(|digits| {
                        // digit values 0-9 → ASCII text
                        let ascii: alloc::vec::Vec<u8> = digits.iter().map(|&d| d + b'0').collect();
                        alloc::string::String::from_utf8(ascii).expect("digits 0-9 are ASCII")
                    })
                    .map_err(|_| tpt_barcode_core::DecodeError::InvalidFormat)
            }),
        ];
        let enabled_1d: alloc::vec::Vec<(tpt_barcode_core::Format, LinearDecoder)> = LINEAR
            .iter()
            .filter(|(fmt, _)| self.formats.contains(fmt))
            .copied()
            .collect();

        if !enabled_1d.is_empty() {
            let stride = (self.height / 128).max(1);
            let mut seen: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();
            for y in (0..self.height).step_by(stride) {
                let row = &binary[y * self.width..(y + 1) * self.width];
                let runs = match row_runs(row) {
                    Some(r) if r.len() >= 6 => r,
                    _ => continue,
                };
                for (fmt, decode) in enabled_1d.iter() {
                    if let Ok(text) = decode(&runs) {
                        // The same symbol is hit on many adjacent scanlines;
                        // report each distinct payload once.
                        if seen.contains(&text) {
                            continue;
                        }
                        seen.push(text.clone());

                        let x0 = row.iter().position(|&p| p > 128).unwrap_or(0) as f32;
                        let x1 = row
                            .iter()
                            .rposition(|&p| p > 128)
                            .unwrap_or(self.width.saturating_sub(1))
                            as f32
                            + 1.0;
                        let y0 = y as f32;
                        results.push(ScanResult {
                            text,
                            format: *fmt,
                            bounding_box: [
                                tpt_math_geometry::Point2::from_array([x0, y0]),
                                tpt_math_geometry::Point2::from_array([x1, y0]),
                                tpt_math_geometry::Point2::from_array([x1, y0 + 1.0]),
                                tpt_math_geometry::Point2::from_array([x0, y0 + 1.0]),
                            ],
                            metadata: SymbolMetadata::None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Alternating dark/light run lengths of a scanline, starting with the first
/// dark pixel (leading quiet-zone light run dropped). `None` if the row is
/// entirely light.
#[cfg(all(feature = "scan", feature = "alloc"))]
fn row_runs(row: &[u8]) -> Option<alloc::vec::Vec<u32>> {
    let first_dark = row.iter().position(|&p| p > 128)?;
    let mut runs = alloc::vec::Vec::new();
    let mut count = 0u32;
    let mut dark = true;
    for &p in &row[first_dark..] {
        if (p > 128) == dark {
            count += 1;
        } else {
            runs.push(count);
            count = 1;
            dark = !dark;
        }
    }
    runs.push(count);
    Some(runs)
}

#[cfg(all(feature = "scan", feature = "alloc"))]
impl<'a> Scanner<'a> {
    /// Greedily decode as many distinct QR symbols as the finder candidates
    /// support: decode the best triple, remove its candidates on success and
    /// stop on failure.
    fn scan_all_qr(
        &self,
        binary: &[u8],
        mut candidates: alloc::vec::Vec<tpt_barcode_image::finder::FinderCandidate>,
        results: &mut alloc::vec::Vec<ScanResult>,
    ) {
        while candidates.len() >= 3 {
            let before = candidates.len();
            match self.scan_qr(binary, &candidates) {
                Some(result) => {
                    // Drop the three candidates nearest the decoded symbol so
                    // the same code is not reported twice.
                    let cx = result.bounding_box.iter().map(|p| p.x()).sum::<f32>() / 4.0;
                    let cy = result.bounding_box.iter().map(|p| p.y()).sum::<f32>() / 4.0;
                    candidates.sort_by_key(|c| {
                        let dx = c.cx - cx;
                        let dy = c.cy - cy;
                        ((dx * dx + dy * dy) * 4096.0) as i64
                    });
                    candidates.drain(0..3.min(candidates.len()));
                    if candidates.len() == before {
                        break;
                    }
                    results.push(result);
                }
                None => break,
            }
        }
    }

    /// Decode one QR symbol given finder candidates and a binarized image.
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

        // Centre-to-centre distance equals (size − 7) modules; snap to the
        // nearest valid QR symbol size (21 + 4k modules).
        let dist_tr =
            sqrt_f32((tr.cx - tl.cx) * (tr.cx - tl.cx) + (tr.cy - tl.cy) * (tr.cy - tl.cy));
        let dist_bl =
            sqrt_f32((bl.cx - tl.cx) * (bl.cx - tl.cx) + (bl.cy - tl.cy) * (bl.cy - tl.cy));
        // round-half-up in integer math (no_std-safe)
        let est_units = (dist_tr + dist_bl) / 2.0 / module_px + 7.0;
        let est = (est_units + 0.5) as i32;
        let size = {
            // round-half-up in integer math; est − 21 is ≥ −4 in practice
            let v = (((est - 21) as f32 / 4.0) + 0.5) as i32;
            let v = v.clamp(0, 39) as usize;
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
        if self.bilinear {
            homography::sample_grid_bilinear(
                binary,
                self.width,
                self.height,
                &h,
                &mut grid,
                size,
                size,
            );
        } else {
            homography::sample_grid(binary, self.width, self.height, &h, &mut grid, size, size);
        }

        // Decode: format info → unmask → RS → payload (+ metadata)
        let detailed = tpt_barcode_2d::qr::decode_grid_detailed(&grid, size).ok()?;

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
            text: alloc::string::String::from_utf8_lossy(&detailed.payload).into_owned(),
            format: tpt_barcode_core::Format::QrCode,
            bounding_box,
            metadata: SymbolMetadata::Qr {
                version: detailed.version,
                mask: detailed.mask,
                ec_level: detailed.ec_level,
            },
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

/// Begin scanning an `image`-crate grayscale buffer.
///
/// Requires the `image-input` feature.
///
/// # Example
///
/// ```rust,no_run
/// let img = image::open("qr.png")?.to_luma8();
/// let results = tpt_barcode::scan_image(&img).execute()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(all(feature = "scan", feature = "image-input"))]
pub fn scan_image(img: &image::ImageBuffer<image::Luma<u8>, alloc::vec::Vec<u8>>) -> Scanner<'_> {
    Scanner::new(img.as_raw(), img.width() as usize, img.height() as usize)
}

/// `sqrt` for positive finite f32 via Newton–Raphson (no_std-safe; used only
/// for distance estimates where ~1e-5 relative error is plenty).
#[cfg(all(feature = "scan", feature = "alloc"))]
fn sqrt_f32(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = if x < 1.0 { 1.0 } else { x / 2.0 };
    for _ in 0..8 {
        g = 0.5 * (g + x / g);
    }
    g
}

//! `tpt-barcode-wasm` — WASM/npm bindings for `tpt-barcode`.
//!
//! This crate wraps the [`tpt-barcode`](https://github.com/tpt-solutions/tpt-barcode)
//! facade in a `#[wasm_bindgen]` surface so it can be built with `wasm-pack`
//! into a browser-ready npm package. It intentionally exposes a small,
//! JS-friendly API rather than re-exporting the Rust types directly:
//!
//! - [`encode_qr_svg`] / [`encode_qr_png`] — QR Code generation.
//! - [`encode_code128_svg`] — Code 128 (Subset B) generation.
//! - [`scan_gray`] / [`scan_rgba`] — decode a still frame (grayscale or
//!   RGBA, e.g. straight from a `<canvas>` `ImageData`) into zero or more
//!   [`WasmScanResult`] values.
//!
//! See `crates/tpt-barcode-wasm/README.md` for build instructions and a
//! usage snippet, and `templates/web/` for a full camera-scanning demo page
//! built on top of this crate.

use tpt_barcode::core::{DecodeError, EcLevel, EncodeError, Format};
use tpt_barcode::prelude::*;
use wasm_bindgen::prelude::*;

fn js_err<E: core::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn parse_ec_level(level: &str) -> Result<EcLevel, JsValue> {
    match level.to_ascii_uppercase().as_str() {
        "L" => Ok(EcLevel::L),
        "M" => Ok(EcLevel::M),
        "Q" => Ok(EcLevel::Q),
        "H" => Ok(EcLevel::H),
        other => Err(JsValue::from_str(&format!(
            "invalid EC level '{other}': expected one of L, M, Q, H"
        ))),
    }
}

/// Encode `text` as a QR Code and render it to a self-contained SVG string.
///
/// `ec_level` is one of `"L"`, `"M"`, `"Q"`, `"H"` (case-insensitive).
/// `module_size` is the pixel size of a single QR module.
#[wasm_bindgen]
pub fn encode_qr_svg(text: &str, ec_level: &str, module_size: u32) -> Result<String, JsValue> {
    let ec = parse_ec_level(ec_level)?;
    let qr = tpt_barcode::qr::encode(text, ec).map_err(js_err)?;
    Ok(qr.to_svg_string(module_size))
}

/// Encode `text` as a QR Code and render it to PNG bytes.
///
/// `ec_level` is one of `"L"`, `"M"`, `"Q"`, `"H"` (case-insensitive).
/// `module_px` is the pixel size of a single QR module and `quiet_zone` is
/// the number of blank modules to pad around the symbol.
#[wasm_bindgen]
pub fn encode_qr_png(
    text: &str,
    ec_level: &str,
    module_px: u32,
    quiet_zone: u32,
) -> Result<Vec<u8>, JsValue> {
    let ec = parse_ec_level(ec_level)?;
    let qr = tpt_barcode::qr::encode(text, ec).map_err(js_err)?;
    Ok(qr.to_png_bytes(module_px, quiet_zone))
}

/// Encode `text` as a Code 128 (Subset B) barcode and render it to SVG.
///
/// Subset B supports printable ASCII (`0x20..=0x7e`) only.
#[wasm_bindgen]
pub fn encode_code128_svg(text: &str, module_size: u32, height_px: u32) -> Result<String, JsValue> {
    if !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(js_err(EncodeError::InvalidCharacter));
    }
    let code = tpt_barcode::one_d::code128::encode_b(text.as_bytes()).map_err(js_err)?;
    Ok(tpt_barcode::render::svg::render_1d(
        &code.modules,
        module_size,
        height_px,
    ))
}

/// A single decoded barcode, returned from [`scan_gray`] / [`scan_rgba`].
#[wasm_bindgen]
pub struct WasmScanResult {
    text: String,
    format: &'static str,
    corners: [f32; 8],
}

#[wasm_bindgen]
impl WasmScanResult {
    /// The decoded text payload (UTF-8).
    #[wasm_bindgen(getter)]
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// The barcode symbology, e.g. `"QR Code"`, `"Code 128"`, `"EAN-13"`.
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format.to_string()
    }

    /// The four corner points of the symbol in the source image, flattened
    /// as `[x0, y0, x1, y1, x2, y2, x3, y3]` (pixel coordinates).
    #[wasm_bindgen(getter)]
    pub fn corners(&self) -> Vec<f32> {
        self.corners.to_vec()
    }
}

fn format_name(format: Format) -> &'static str {
    match format {
        Format::QrCode => "QR Code",
        Format::DataMatrix => "Data Matrix",
        Format::Pdf417 => "PDF417",
        Format::Code128 => "Code 128",
        Format::Ean13 => "EAN-13",
        Format::UpcA => "UPC-A",
        Format::Code39 => "Code 39",
    }
}

fn all_formats() -> [Format; 7] {
    [
        Format::QrCode,
        Format::DataMatrix,
        Format::Pdf417,
        Format::Code128,
        Format::Ean13,
        Format::UpcA,
        Format::Code39,
    ]
}

fn run_scan(pixels: &[u8], width: u32, height: u32) -> Result<Vec<WasmScanResult>, JsValue> {
    let formats = all_formats();
    let results = tpt_barcode::scan(pixels, width as usize, height as usize)
        .formats(&formats)
        .try_harder(true)
        .execute();

    match results {
        Ok(hits) => Ok(hits
            .into_iter()
            .map(|r| {
                let bb = r.bounding_box();
                WasmScanResult {
                    text: r.text().to_string(),
                    format: format_name(r.format()),
                    corners: [
                        bb[0].x(),
                        bb[0].y(),
                        bb[1].x(),
                        bb[1].y(),
                        bb[2].x(),
                        bb[2].y(),
                        bb[3].x(),
                        bb[3].y(),
                    ],
                }
            })
            .collect()),
        // `DecodeError::NotFound` (or similar "nothing here") just means an
        // empty result set for a live camera feed, not a hard failure.
        Err(DecodeError::NotFound) => Ok(Vec::new()),
        Err(other) => Err(js_err(other)),
    }
}

/// Scan a single-channel grayscale (luma8) image buffer for barcodes.
///
/// `pixels.len()` must equal `width * height`. Returns an array of
/// [`WasmScanResult`] (empty if nothing was found).
#[wasm_bindgen]
pub fn scan_gray(pixels: &[u8], width: u32, height: u32) -> Result<Vec<WasmScanResult>, JsValue> {
    if pixels.len() != (width as usize) * (height as usize) {
        return Err(JsValue::from_str(
            "scan_gray: pixels.len() must equal width * height",
        ));
    }
    run_scan(pixels, width, height)
}

/// Scan an RGBA image buffer (e.g. straight from a `<canvas>` `ImageData`)
/// for barcodes. Internally converts to grayscale via a standard luminance
/// weighting before scanning.
///
/// `pixels.len()` must equal `width * height * 4`.
#[wasm_bindgen]
pub fn scan_rgba(pixels: &[u8], width: u32, height: u32) -> Result<Vec<WasmScanResult>, JsValue> {
    let expected = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if pixels.len() != expected {
        return Err(JsValue::from_str(
            "scan_rgba: pixels.len() must equal width * height * 4",
        ));
    }

    let mut gray = Vec::with_capacity((width as usize) * (height as usize));
    for chunk in pixels.chunks_exact(4) {
        let (r, g, b) = (chunk[0] as u32, chunk[1] as u32, chunk[2] as u32);
        // ITU-R BT.601 luma weighting.
        let y = (r * 299 + g * 587 + b * 114) / 1000;
        gray.push(y as u8);
    }
    run_scan(&gray, width, height)
}

/// Called once by the JS glue on module init; wires up panic messages to the
/// browser console so failures are easier to diagnose than a bare "unreachable".
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

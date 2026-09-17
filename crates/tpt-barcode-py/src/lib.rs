//! Python bindings for `tpt-barcode`, built with [`pyo3`].
//!
//! This crate is compiled as a `cdylib` and loaded from Python as the
//! `tpt_barcode` extension module (see `pyproject.toml` / `maturin`).
//! It is intentionally thin: it converts between Python-friendly types
//! (`str`, `bytes`, `list[dict]`-like objects) and the zero-allocation
//! Rust API exposed by the `tpt-barcode` facade crate.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

// This crate's own `cdylib`/`#[pymodule]` is also named `tpt_barcode` (see
// `Cargo.toml` `[lib] name` and the `#[pymodule] fn tpt_barcode` below), so
// references to the `tpt-barcode` *dependency* must use the leading `::` to
// disambiguate from the local module path.
use ::tpt_barcode::core::EcLevel;
use ::tpt_barcode::{QrRenderExt, Scanner};

/// Parse an error-correction level string (`"L"`, `"M"`, `"Q"`, `"H"`,
/// case-insensitive) into an [`EcLevel`].
fn parse_ec_level(ec_level: &str) -> PyResult<EcLevel> {
    match ec_level.to_ascii_uppercase().as_str() {
        "L" => Ok(EcLevel::L),
        "M" => Ok(EcLevel::M),
        "Q" => Ok(EcLevel::Q),
        "H" => Ok(EcLevel::H),
        other => Err(PyValueError::new_err(format!(
            "invalid EC level {other:?}: expected one of \"L\", \"M\", \"Q\", \"H\""
        ))),
    }
}

/// Encode `data` as a QR Code and return a self-contained SVG string.
///
/// `ec_level` is one of `"L"`, `"M"`, `"Q"`, `"H"` (case-insensitive).
/// `module_size` is the pixel size of one QR module (defaults to 4).
#[pyfunction]
#[pyo3(signature = (data, ec_level, module_size=4))]
fn encode_qr_svg(data: &str, ec_level: &str, module_size: u32) -> PyResult<String> {
    let ec = parse_ec_level(ec_level)?;
    let qr = ::tpt_barcode::qr::encode(data, ec)
        .map_err(|e| PyValueError::new_err(format!("QR encode failed: {e}")))?;
    Ok(qr.to_svg_string(module_size))
}

/// Encode `data` as a QR Code and return PNG-encoded bytes.
///
/// `ec_level` is one of `"L"`, `"M"`, `"Q"`, `"H"` (case-insensitive).
/// `module_px` is the pixel size of one QR module, `quiet_zone` is the
/// border width in modules.
#[pyfunction]
#[pyo3(signature = (data, ec_level, module_px=8, quiet_zone=4))]
fn encode_qr_png<'py>(
    py: Python<'py>,
    data: &str,
    ec_level: &str,
    module_px: u32,
    quiet_zone: u32,
) -> PyResult<Bound<'py, PyBytes>> {
    let ec = parse_ec_level(ec_level)?;
    let qr = ::tpt_barcode::qr::encode(data, ec)
        .map_err(|e| PyValueError::new_err(format!("QR encode failed: {e}")))?;
    let bytes = qr.to_png_bytes(module_px, quiet_zone);
    Ok(PyBytes::new(py, &bytes))
}

/// Encode `data` (ASCII text) as a Code 128 (subset B) barcode and return a
/// self-contained SVG string.
///
/// `module_size` is the pixel width of the narrowest bar, `height_px` is the
/// bar height in pixels.
#[pyfunction]
#[pyo3(signature = (data, module_size=2, height_px=80))]
fn encode_code128_svg(data: &str, module_size: u32, height_px: u32) -> PyResult<String> {
    let code = ::tpt_barcode::one_d::code128::encode_b(data.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("Code 128 encode failed: {e}")))?;
    Ok(::tpt_barcode::render::svg::render_1d(
        &code.modules,
        module_size,
        height_px,
    ))
}

/// One scanned barcode symbol.
///
/// Returned from [`scan`]. `bounding_box` is a list of four `(x, y)` pixel
/// coordinate tuples for the symbol's corners, in image space.
#[pyclass(get_all)]
struct ScanResult {
    /// Decoded text content (UTF-8).
    text: String,
    /// Barcode format name, e.g. `"QR Code"`, `"Code 128"`.
    format: String,
    /// Four `(x, y)` corner points of the symbol in the source image.
    bounding_box: [(f32, f32); 4],
}

#[pymethods]
impl ScanResult {
    fn __repr__(&self) -> String {
        format!(
            "ScanResult(text={:?}, format={:?}, bounding_box={:?})",
            self.text, self.format, self.bounding_box
        )
    }
}

/// Scan a grayscale (8-bit luma) image buffer for barcodes.
///
/// `image_bytes` must be exactly `width * height` bytes, one byte per pixel
/// (0 = black, 255 = white). `try_harder` enables adaptive binarization and
/// perspective correction for QR codes at the cost of scan time.
///
/// Returns a list of [`ScanResult`] objects, one per barcode found.
#[pyfunction]
#[pyo3(signature = (image_bytes, width, height, try_harder=false))]
fn scan(
    image_bytes: &[u8],
    width: usize,
    height: usize,
    try_harder: bool,
) -> PyResult<Vec<ScanResult>> {
    if image_bytes.len() != width * height {
        return Err(PyValueError::new_err(format!(
            "image_bytes has {} bytes, expected width*height = {}",
            image_bytes.len(),
            width * height
        )));
    }

    let results = Scanner::new(image_bytes, width, height)
        .try_harder(try_harder)
        .execute()
        .map_err(|e| PyValueError::new_err(format!("scan failed: {e}")))?;

    Ok(results
        .into_iter()
        .map(|r| ScanResult {
            text: r.text,
            format: r.format.to_string(),
            bounding_box: [
                (r.bounding_box[0].x(), r.bounding_box[0].y()),
                (r.bounding_box[1].x(), r.bounding_box[1].y()),
                (r.bounding_box[2].x(), r.bounding_box[2].y()),
                (r.bounding_box[3].x(), r.bounding_box[3].y()),
            ],
        })
        .collect())
}

/// Python extension module `tpt_barcode`.
#[pymodule]
fn tpt_barcode(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode_qr_svg, m)?)?;
    m.add_function(wrap_pyfunction!(encode_qr_png, m)?)?;
    m.add_function(wrap_pyfunction!(encode_code128_svg, m)?)?;
    m.add_function(wrap_pyfunction!(scan, m)?)?;
    m.add_class::<ScanResult>()?;
    Ok(())
}

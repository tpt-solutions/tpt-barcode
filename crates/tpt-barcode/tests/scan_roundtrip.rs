//! Full-pipeline integration test: encode a QR Code → render to PNG → binarize
//! and scan the rendered image → recover the original payload.

#![cfg(all(feature = "scan", feature = "render", feature = "2d"))]

use tpt_barcode::core::{EcLevel, Format};

/// Render a QR matrix to a grayscale (luma8) pixel buffer, as a camera would
/// capture it (quiet zone included).
fn render_luma(
    qr: &tpt_barcode::qr::QrCode,
    module_px: usize,
    quiet_modules: usize,
) -> (Vec<u8>, usize) {
    let total = (qr.size + 2 * quiet_modules) * module_px;
    let mut img = vec![255u8; total * total];
    for row in 0..qr.size {
        for col in 0..qr.size {
            if qr.matrix[row * qr.size + col] != 0 {
                for dy in 0..module_px {
                    for dx in 0..module_px {
                        let x = (quiet_modules * module_px) + col * module_px + dx;
                        let y = (quiet_modules * module_px) + row * module_px + dy;
                        img[y * total + x] = 0;
                    }
                }
            }
        }
    }
    (img, total)
}

fn scan(
    pixels: &[u8],
    width: usize,
    height: usize,
    try_harder: bool,
) -> Vec<tpt_barcode::ScanResult> {
    tpt_barcode::scan(pixels, width, height)
        .formats(&[Format::QrCode])
        .try_harder(try_harder)
        .execute()
        .expect("scan should not error")
}

#[test]
fn encode_render_scan_round_trip() {
    let payload = "https://example.com/tpt-barcode-roundtrip";
    let qr = tpt_barcode::qr::encode(payload, EcLevel::M).expect("encode");
    let (pixels, dim) = render_luma(&qr, 8, 4);

    let results = scan(&pixels, dim, dim, false);
    assert_eq!(results.len(), 1, "expected exactly one scan result");
    assert_eq!(results[0].text(), payload);
    assert_eq!(results[0].format(), Format::QrCode);

    // The bounding box covers the symbol's outer module corners (not the
    // quiet zone): a square of `size` modules × 8 px.
    let bb = results[0].bounding_box();
    let w = bb[1].x() - bb[0].x();
    let h = bb[3].y() - bb[0].y();
    let expected = (qr.size * 8) as f32;
    assert!(
        (w - expected).abs() < 2.0,
        "width {w} vs expected {expected}"
    );
    assert!(
        (h - expected).abs() < 2.0,
        "height {h} vs expected {expected}"
    );
}

#[test]
fn encode_render_scan_round_trip_adaptive() {
    // Same pipeline with try_harder (adaptive thresholding + same homography path)
    let payload = "HELLO SCAN 42";
    let qr = tpt_barcode::qr::encode(payload, EcLevel::H).expect("encode");
    let (pixels, dim) = render_luma(&qr, 6, 4);

    let results = scan(&pixels, dim, dim, true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].text(), payload);
}

#[test]
fn scan_image_without_barcode_returns_empty() {
    // Uniform mid-gray image — no finder patterns to find
    let pixels = vec![128u8; 120 * 120];
    let results = scan(&pixels, 120, 120, false);
    assert!(results.is_empty());
}

#[test]
fn png_render_scan_round_trip() {
    // Through the actual PNG codec: render → encode PNG → decode PNG → scan
    let payload = "PNG pipeline check";
    let qr = tpt_barcode::qr::encode(payload, EcLevel::M).expect("encode");
    let png_bytes = tpt_barcode::render::png::render_to_png(&qr.matrix, qr.size, 8, 4);

    let img = image::load_from_memory(&png_bytes)
        .expect("valid PNG")
        .to_luma8();
    let results = scan(
        img.as_raw(),
        img.width() as usize,
        img.height() as usize,
        false,
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].text(), payload);
}

#[test]
fn rotated_qr_scans_via_homography() {
    // Rotate the rendered image by a small angle; perspective correction must
    // recover the symbol.
    let payload = "ROTATED";
    let qr = tpt_barcode::qr::encode(payload, EcLevel::M).expect("encode");
    // Generous quiet zone so the rotated symbol never clips at the image
    // edges (clipping would destroy the corner finder patterns).
    let (src, dim) = render_luma(&qr, 8, 10);

    let angle = 10.0f64.to_radians();
    let (cx, cy) = (dim as f64 / 2.0, dim as f64 / 2.0);
    let (sin, cos) = angle.sin_cos();
    let mut dst = vec![255u8; src.len()];
    for y in 0..dim {
        for x in 0..dim {
            // Inverse-rotate destination pixel into source coordinates
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let sx = (cos * dx + sin * dy + cx).round() as i64;
            let sy = (-sin * dx + cos * dy + cy).round() as i64;
            if sx >= 0 && sy >= 0 && (sx as usize) < dim && (sy as usize) < dim {
                dst[y * dim + x] = src[sy as usize * dim + sx as usize];
            }
        }
    }

    let results = scan(&dst, dim, dim, false);
    assert_eq!(results.len(), 1, "rotated QR should be found");
    assert_eq!(results[0].text(), payload);
}

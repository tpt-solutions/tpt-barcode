//! Encode text to a QR Code SVG file.
//!
//! Run: `cargo run --example qr_svg -- "https://example.com" out.svg`

use tpt_barcode::prelude::*;

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
fn main() -> Result<(), tpt_barcode::core::EncodeError> {
    let mut args = std::env::args().skip(1);
    let text = args.next().unwrap_or_else(|| "https://example.com".into());
    let out = args.next().unwrap_or_else(|| "qr.svg".into());

    let qr = tpt_barcode::qr::encode(&text, tpt_barcode::core::EcLevel::M)?;
    let svg = qr.to_svg_string(4);

    std::fs::write(&out, svg).expect("write svg");
    println!(
        "wrote {out} (version {}, mask {}, {}×{} modules)",
        qr.version, qr.mask_id, qr.size, qr.size
    );
    Ok(())
}

#[cfg(not(all(feature = "2d", feature = "render", feature = "alloc")))]
fn main() {
    eprintln!("build with --features 2d,render");
}

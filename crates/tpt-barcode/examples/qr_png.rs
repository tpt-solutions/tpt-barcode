//! Encode text to a QR Code PNG file (requires `png`).
//!
//! Run: `cargo run --example qr_png --features png -- "hello" out.png`

use tpt_barcode::prelude::*;

#[cfg(all(feature = "2d", feature = "render", feature = "png", feature = "alloc"))]
fn main() -> Result<(), tpt_barcode::core::EncodeError> {
    let mut args = std::env::args().skip(1);
    let text = args.next().unwrap_or_else(|| "hello".into());
    let out = args.next().unwrap_or_else(|| "qr.png".into());

    let qr = tpt_barcode::qr::encode(&text, tpt_barcode::core::EcLevel::M)?;
    let png = qr.to_png_bytes(8, 4);

    std::fs::write(&out, png).expect("write png");
    println!("wrote {out}");
    Ok(())
}

#[cfg(not(all(feature = "2d", feature = "render", feature = "png", feature = "alloc")))]
fn main() {
    eprintln!("build with --features 2d,render,png");
}

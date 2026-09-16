//! Encode bytes to a PDF417 SVG file.
//!
//! Run: `cargo run --example pdf417_svg -- "boarding pass data" out.svg`

use tpt_barcode::prelude::*;

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
fn main() -> Result<(), tpt_barcode::core::EncodeError> {
    use tpt_barcode::two_d::pdf417::{self, Compaction, EcLevel};

    let mut args = std::env::args().skip(1);
    let text = args.next().unwrap_or_else(|| "PDF417 demo".into());
    let out = args.next().unwrap_or_else(|| "pdf417.svg".into());

    let symbol = pdf417::encode(
        text.as_bytes(),
        EcLevel(pdf417::recommended_ec_level(text.len())),
        Compaction::Byte,
    )?;
    let svg = symbol.to_svg_string(3);

    std::fs::write(&out, svg).expect("write svg");
    println!("wrote {out} ({}×{} modules)", symbol.width, symbol.height);
    Ok(())
}

#[cfg(not(all(feature = "2d", feature = "render", feature = "alloc")))]
fn main() {
    eprintln!("build with --features 2d,render");
}

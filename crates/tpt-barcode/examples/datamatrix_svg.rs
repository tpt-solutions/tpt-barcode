//! Encode text to a Data Matrix SVG file.
//!
//! Run: `cargo run --example datamatrix_svg -- "batch-42" out.svg`

use tpt_barcode::prelude::*;

#[cfg(all(feature = "2d", feature = "render", feature = "alloc"))]
fn main() -> Result<(), tpt_barcode::core::EncodeError> {
    let mut args = std::env::args().skip(1);
    let text = args.next().unwrap_or_else(|| "batch-42".into());
    let out = args.next().unwrap_or_else(|| "datamatrix.svg".into());

    let dm = tpt_barcode::two_d::datamatrix::encode(text.as_bytes())?;
    let svg = dm.to_svg_string(6);

    std::fs::write(&out, svg).expect("write svg");
    println!("wrote {out} ({}×{} modules)", dm.size, dm.size);
    Ok(())
}

#[cfg(not(all(feature = "2d", feature = "render", feature = "alloc")))]
fn main() {
    eprintln!("build with --features 2d,render");
}

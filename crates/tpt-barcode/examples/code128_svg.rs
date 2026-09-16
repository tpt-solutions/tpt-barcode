//! Encode text to a Code 128 SVG file.
//!
//! Run: `cargo run --example code128_svg -- "HELLO-128" out.svg`

#[cfg(all(feature = "1d", feature = "render", feature = "alloc"))]
fn main() -> Result<(), tpt_barcode::core::EncodeError> {
    let mut args = std::env::args().skip(1);
    let text = args.next().unwrap_or_else(|| "HELLO-128".into());
    let out = args.next().unwrap_or_else(|| "code128.svg".into());

    if !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        eprintln!("Code 128 Subset B supports printable ASCII only");
        return Err(tpt_barcode::core::EncodeError::InvalidCharacter);
    }
    let code = tpt_barcode::one_d::code128::encode_b(text.as_bytes())?;
    let svg = tpt_barcode::render::svg::render_1d(&code.modules, 2, 80);

    std::fs::write(&out, svg).expect("write svg");
    println!("wrote {out} ({} modules wide)", code.modules.len());
    Ok(())
}

#[cfg(not(all(feature = "1d", feature = "render", feature = "alloc")))]
fn main() {
    eprintln!("build with --features 1d,render");
}

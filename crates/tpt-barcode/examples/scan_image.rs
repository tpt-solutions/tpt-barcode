//! Scan an image file for barcodes and print the results.
//!
//! Run: `cargo run --example scan_image --features image-input -- qr.png`

#[cfg(all(feature = "scan", feature = "image-input"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "qr.png".into());
    let img = image::open(&path)?.to_luma8();

    let formats = [
        tpt_barcode::core::Format::QrCode,
        tpt_barcode::core::Format::DataMatrix,
        tpt_barcode::core::Format::Pdf417,
        tpt_barcode::core::Format::Code128,
        tpt_barcode::core::Format::Ean13,
        tpt_barcode::core::Format::UpcA,
        tpt_barcode::core::Format::Code39,
    ];
    let results = tpt_barcode::scan_image(&img)
        .formats(&formats)
        .try_harder(true)
        .execute()?;

    if results.is_empty() {
        println!("no barcodes found in {path}");
        return Ok(());
    }
    for r in &results {
        println!("{}: {:?} {:?}", r.format(), r.text(), r.metadata());
    }
    Ok(())
}

#[cfg(not(all(feature = "scan", feature = "image-input")))]
fn main() {
    eprintln!("build with --features scan,image-input");
}

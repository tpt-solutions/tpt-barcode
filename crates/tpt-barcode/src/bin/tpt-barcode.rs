//! `tpt-barcode` — command-line barcode encoder and scanner.
//!
//! ```text
//! tpt-barcode encode <format> [--ec LEVEL] [--out FILE] [--module N] <text>
//! tpt-barcode scan <image-file> [--try-harder]
//! ```
//!
//! Formats: `qr`, `datamatrix`, `pdf417`, `code128`, `ean13`.
//! Output is written as SVG; choose the file name with `--out`.

// The binary needs the full feature set.
#[cfg(all(
    feature = "2d",
    feature = "1d",
    feature = "render",
    feature = "image-input",
    feature = "alloc",
    feature = "std"
))]
mod cli {
    use tpt_barcode::core::{EcLevel, EncodeError};
    // one-line renderers
    use tpt_barcode::prelude::*;

    pub fn run(args: Vec<String>) -> Result<(), String> {
        let mut it = args.into_iter();
        let cmd = it.next().ok_or_else(usage)?;
        match cmd.as_str() {
            "encode" => {
                let format = it.next().ok_or_else(usage)?;
                let mut ec: Option<String> = None;
                let mut out: Option<String> = None;
                let mut module = 4u32;
                let mut positional = Vec::new();
                while let Some(a) = it.next() {
                    match a.as_str() {
                        "--ec" => {
                            ec = Some(it.next().ok_or_else(usage)?);
                        }
                        "--out" => {
                            out = Some(it.next().ok_or_else(usage)?);
                        }
                        "--module" => {
                            module = it
                                .next()
                                .ok_or_else(usage)?
                                .parse()
                                .map_err(|_| "--module expects a number".to_string())?;
                        }
                        other => positional.push(other.to_string()),
                    }
                }
                let text = positional.join(" ");
                let ec_level = parse_ec(ec.as_deref().unwrap_or("m"))?;
                match format.as_str() {
                    "qr" => {
                        let qr =
                            tpt_barcode::qr::encode(&text, ec_level).map_err(|e| e.to_string())?;
                        let o = out.unwrap_or_else(|| "qr.svg".into());
                        std::fs::write(&o, qr.to_svg_string(module)).map_err(|e| e.to_string())?;
                        println!("wrote {o} (version {}, mask {})", qr.version, qr.mask_id);
                        Ok(())
                    }
                    "datamatrix" => {
                        let dm = tpt_barcode::two_d::datamatrix::encode(text.as_bytes())
                            .map_err(|e| e.to_string())?;
                        let o = out.unwrap_or_else(|| "datamatrix.svg".into());
                        std::fs::write(&o, dm.to_svg_string(module)).map_err(|e| e.to_string())?;
                        println!("wrote {o} ({}×{} modules)", dm.size, dm.size);
                        Ok(())
                    }
                    "pdf417" => {
                        use tpt_barcode::two_d::pdf417::{self, Compaction};
                        let symbol = pdf417::encode(
                            text.as_bytes(),
                            pdf417::EcLevel(pdf417::recommended_ec_level(text.len())),
                            Compaction::Byte,
                        )
                        .map_err(|e| e.to_string())?;
                        let o = out.unwrap_or_else(|| "pdf417.svg".into());
                        std::fs::write(
                            &o,
                            tpt_barcode::render::svg::render_matrix_svg(
                                &symbol.matrix,
                                symbol.width,
                                symbol.height,
                                module,
                                2,
                            ),
                        )
                        .map_err(|e| e.to_string())?;
                        println!("wrote {o} ({}×{} modules)", symbol.width, symbol.height);
                        Ok(())
                    }
                    "code128" => {
                        let code = tpt_barcode::one_d::code128::encode_b(text.as_bytes())
                            .map_err(encode_err)?;
                        let o = out.unwrap_or_else(|| "code128.svg".into());
                        std::fs::write(
                            &o,
                            tpt_barcode::render::svg::render_1d(&code.modules, 2, 80),
                        )
                        .map_err(|e| e.to_string())?;
                        println!("wrote {o}");
                        Ok(())
                    }
                    "ean13" => {
                        let digits: Vec<u8> = text
                            .bytes()
                            .filter(|b| b.is_ascii_digit())
                            .map(|b| b - b'0')
                            .collect();
                        let code =
                            tpt_barcode::one_d::ean13::encode(&digits).map_err(encode_err)?;
                        let o = out.unwrap_or_else(|| "ean13.svg".into());
                        std::fs::write(
                            &o,
                            tpt_barcode::render::svg::render_1d(&code.modules, 2, 80),
                        )
                        .map_err(|e| e.to_string())?;
                        println!("wrote {o}");
                        Ok(())
                    }
                    other => Err(format!(
                        "unknown format '{other}' (use qr|datamatrix|pdf417|code128|ean13)"
                    )),
                }
            }
            "scan" => {
                let mut path = None;
                let mut try_harder = false;
                for a in it {
                    match a.as_str() {
                        "--try-harder" => try_harder = true,
                        other if !other.starts_with("--") => path = Some(other.to_string()),
                        _ => {}
                    }
                }
                let path = path.ok_or_else(usage)?;
                let img = image::open(&path)
                    .map_err(|e| format!("{path}: {e}"))?
                    .to_luma8();
                let results = tpt_barcode::scan_image(&img)
                    .formats(&[
                        tpt_barcode::core::Format::QrCode,
                        tpt_barcode::core::Format::DataMatrix,
                        tpt_barcode::core::Format::Pdf417,
                        tpt_barcode::core::Format::Code128,
                        tpt_barcode::core::Format::Ean13,
                        tpt_barcode::core::Format::UpcA,
                        tpt_barcode::core::Format::Code39,
                    ])
                    .try_harder(try_harder)
                    .execute()
                    .map_err(|e| e.to_string())?;
                if results.is_empty() {
                    println!("no barcodes found in {path}");
                }
                for r in &results {
                    println!("{:?}\t{}", r.format(), r.text());
                }
                Ok(())
            }
            _ => Err(usage()),
        }
    }

    fn parse_ec(s: &str) -> Result<EcLevel, String> {
        match s.to_ascii_lowercase().as_str() {
            "l" => Ok(EcLevel::L),
            "m" => Ok(EcLevel::M),
            "q" => Ok(EcLevel::Q),
            "h" => Ok(EcLevel::H),
            other => Err(format!("unknown EC level '{other}' (use l|m|q|h)")),
        }
    }

    fn encode_err(e: EncodeError) -> String {
        e.to_string()
    }

    fn usage() -> String {
        let nl = '\n';
        format!(
            "usage:{nl}  tpt-barcode encode <qr|datamatrix|pdf417|code128|ean13> \
             [--ec l|m|q|h] [--out FILE] [--module N] <text>{nl}  \
             tpt-barcode scan <image> [--try-harder]"
        )
    }
}

#[cfg(all(
    feature = "2d",
    feature = "1d",
    feature = "render",
    feature = "image-input",
    feature = "alloc",
    feature = "std"
))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(match cli::run(args) {
        Ok(()) => 0,
        Err(msg) => {
            eprintln!("{msg}");
            1
        }
    });
}

#[cfg(not(all(
    feature = "2d",
    feature = "1d",
    feature = "render",
    feature = "image-input",
    feature = "alloc",
    feature = "std"
)))]
fn main() {
    eprintln!("build with --features 2d,1d,render,image-input");
}

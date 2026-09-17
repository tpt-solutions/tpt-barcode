//! Sketch: generate a QR Code on a `no_std` target.
//! Copy this into your firmware crate (RTIC/embassy/bare metal).
#![no_std]

extern crate alloc;

use alloc::string::String;

/// Returns the QR module grid as (side, dark-module iterator data).
fn qr_modules(text: &str) -> Result<(usize, alloc::vec::Vec<u8>), ()> {
    let qr = tpt_barcode::qr::encode(text, tpt_barcode::core::EcLevel::M).map_err(|_| ())?;
    Ok((qr.size, qr.matrix))
}

/// Example: render into a monochrome framebuffer of `fb_w × fb_h` pixels,
/// `scale` framebuffer pixels per module, centred.
fn draw_to_fb(
    text: &str,
    fb: &mut [u8],
    fb_w: usize,
    fb_h: usize,
    scale: usize,
) -> Result<(), ()> {
    let (size, matrix) = qr_modules(text)?;
    let total = size * scale;
    if total > fb_w || total > fb_h {
        return Err(());
    }
    let off_x = (fb_w - total) / 2;
    let off_y = (fb_h - total) / 2;

    // white background
    fb.fill(0xFF);

    for row in 0..size {
        for col in 0..size {
            if matrix[row * size + col] != 0 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = off_x + col * scale + dx;
                        let y = off_y + row * scale + dy;
                        fb[y * fb_w + x] = 0x00; // dark module
                    }
                }
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn demo() {
    let mut fb = alloc::vec![0xFFu8; 128 * 128];
    let _ = draw_to_fb("embedded qr", &mut fb, 128, 128, 3);
}

fn main() {
    // firmware entry point — your RTIC/embassy app goes here
    demo();
}

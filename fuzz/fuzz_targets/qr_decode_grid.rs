#![no_main]
//! Fuzz the QR module-grid decoder with arbitrary grids.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Interpret as a square grid whose side is a valid QR size (21 + 4k).
    for &size in &[21usize, 25, 29] {
        if data.len() < size * size {
            continue;
        }
        let grid: Vec<u8> = data[..size * size].iter().map(|&b| (b > 127) as u8).collect();
        // Must not panic; decode failures are fine.
        let _ = tpt_barcode::two_d::qr::decode_grid(&grid, size);
    }
});

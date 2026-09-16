#![no_main]
//! Fuzz the Data Matrix decoder with arbitrary module grids.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for &size in &[10usize, 12, 14, 16, 18, 20, 22, 24, 26] {
        if data.len() < size * size {
            continue;
        }
        let grid: Vec<u8> = data[..size * size].iter().map(|&b| (b > 127) as u8).collect();
        let _ = tpt_barcode::two_d::datamatrix::decode(&grid, size);
    }
});

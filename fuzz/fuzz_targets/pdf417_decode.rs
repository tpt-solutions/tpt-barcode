#![no_main]
//! Fuzz the PDF417 decoder with arbitrary module grids.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Structured input: two size bytes then a module grid whose width is a
    // valid PDF417 width (17·cols + 69).
    if data.len() < 3 {
        return;
    }
    let cols = 1 + (data[0] as usize % 30);
    let width = 17 * cols + 69;
    let height = 3 + (data[1] as usize % 20);
    let need = width * height;
    if data.len() < 2 + need {
        return;
    }
    let grid: Vec<u8> = data[2..2 + need]
        .iter()
        .map(|&b| (b > 127) as u8)
        .collect();
    let _ = tpt_barcode::two_d::pdf417::decode(&grid, width, height);
});

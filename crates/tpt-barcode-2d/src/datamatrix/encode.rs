//! Data Matrix ECC 200 encoding: ASCII encodation, Reed-Solomon ECC (0x12D
//! field, generator roots α^1…α^n), Annex M placement, and finder borders.

use alloc::vec::Vec;

use tpt_barcode_core::gf256::DmField;
use tpt_barcode_core::reed_solomon::encode_with;

use super::placement::Placement;
use super::DmSize;

/// ASCII-priority encodation of `data` (without padding).
///
/// Digit pairs pack two-per-codeword (130 + value); bytes 0–127 encode as
/// value + 1; bytes ≥ 128 use the upper shift (235, value − 127). A byte ≥ 128
/// therefore consumes two codewords.
pub fn encode_data(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0usize;
    while i < data.len() {
        if i + 1 < data.len() && data[i].is_ascii_digit() && data[i + 1].is_ascii_digit() {
            let v = (data[i] - b'0') as usize * 10 + (data[i + 1] - b'0') as usize;
            out.push((130 + v) as u8);
            i += 2;
        } else {
            push_byte(&mut out, data[i]);
            i += 1;
        }
    }
    out
}

fn push_byte(out: &mut Vec<u8>, b: u8) {
    if b < 128 {
        out.push(b + 1);
    } else {
        out.push(235); // upper shift
        out.push(b - 127);
    }
}

/// Append ECC 200 padding to reach `capacity` codewords: a single gap takes
/// 129; longer gaps alternate 129, 130, 129, …
pub fn pad(mut codewords: Vec<u8>, capacity: usize) -> Vec<u8> {
    if codewords.len() == capacity - 1 {
        codewords.push(129);
        return codewords;
    }
    let mut pad = 129u8;
    while codewords.len() < capacity {
        codewords.push(pad);
        pad = if pad == 129 { 130 } else { 129 };
    }
    codewords
}

/// Convenience: encodation + padding for a known symbol size.
pub fn encode_codewords(data: &[u8], size: DmSize) -> Vec<u8> {
    let encoded = encode_data(data);
    pad(encoded, size.data_codewords())
}

/// Append the ECC 200 Reed-Solomon error-correction codewords (0x12D field,
/// generator roots α^1…α^n) to the padded data codewords.
pub fn with_ec(mut codewords: Vec<u8>, size: DmSize) -> Vec<u8> {
    let n_ec = size.ec_codewords();
    let mut ec = alloc::vec![0u8; n_ec];
    encode_with::<DmField>(&codewords, n_ec, 1, &mut ec);
    codewords.extend_from_slice(&ec);
    codewords
}

/// Build the full symbol matrix (finders + placed data) for `size`.
pub fn build_symbol(codewords: &[u8], size: DmSize, matrix: &mut [u8]) {
    let s = size.modules();
    assert_eq!(matrix.len(), s * s);
    let side = s - 2;

    let placement = Placement::place(codewords, side, side);

    // Data region at rows/cols 1..s-1
    for r in 0..side {
        for c in 0..side {
            let cell = placement.bits[r * side + c];
            let v = match cell {
                1 => 1,
                super::placement::FIXED => 1, // fixed pattern modules are dark
                // Unset cells carry no data bit (the codeword count determines
                // the payload); they render light. The decoder skips them via
                // its placement map.
                _ => 0,
            };
            matrix[(r + 1) * s + (c + 1)] = v;
        }
    }

    // Solid L-finder: left column and bottom row
    for r in 0..s {
        matrix[r * s] = 1;
    }
    for c in 0..s {
        matrix[(s - 1) * s + c] = 1;
    }

    // Timing patterns: top row alternates dark starting at column 0; right
    // column alternates dark starting at row 1 (the top-right corner is light)
    for c in (0..s).step_by(2) {
        matrix[c] = 1;
    }
    for r in (1..s).step_by(2) {
        matrix[r * s + (s - 1)] = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_pair_packing() {
        // "123456" → three codewords 130+12, 130+34, 130+56
        let cw = encode_codewords(b"123456", DmSize::S10x10);
        assert_eq!(&cw[..3], &[142, 164, 186]);
        assert_eq!(cw.len(), 3); // exactly fills the 3-codeword capacity
    }

    #[test]
    fn ascii_shift_and_upper_shift() {
        // 'A' = 65 → 66; 0x80 → 235, 1
        let cw = encode_codewords(&[b'A', 0x80], DmSize::S10x10);
        assert_eq!(&cw[..3], &[66, 235, 1]);
    }

    #[test]
    fn trailing_single_digit_is_ascii() {
        // "12345" → two pairs + single ASCII digit '5' → 54
        let cw = encode_codewords(b"12345", DmSize::S12x12);
        assert_eq!(&cw[..3], &[142, 164, 54]);
    }

    #[test]
    fn padding_alternation() {
        // Two ASCII bytes in a 3-codeword symbol → one pad codeword 129
        let cw = encode_codewords(b"Hi", DmSize::S10x10);
        assert_eq!(cw, alloc::vec![b'H' + 1, b'i' + 1, 129]);
        // One byte in a 3-codeword symbol → two pads 129, 130
        let cw = encode_codewords(b"H", DmSize::S10x10);
        assert_eq!(cw, alloc::vec![b'H' + 1, 129, 130]);
    }
}

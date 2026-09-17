//! Code 39 barcode encoder and decoder.
//!
//! Encodes uppercase letters A–Z, digits 0–9, and the symbols `-.$/+% ` (space).
//! Each character is represented by 5 bars and 4 spaces (9 elements),
//! with the pattern determined by which 3 of the 9 elements are wide (W=2 units)
//! vs narrow (N=1 unit). Inter-character gap is 1 narrow space.

use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// Encoded Code 39 barcode: module widths (alternating bar/space, starting with a bar).
#[cfg(feature = "alloc")]
pub struct Code39 {
    /// Module widths: alternating bar (N=1 or W=2) and space widths, starting with a bar.
    pub modules: alloc::vec::Vec<u8>,
}

/// The Code 39 character set in order.
const CHARSET: &[u8; 43] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

/// Encoding for each character: 9 bits representing W(1)/N(0), positions 0..8
/// are bar0,sp0,bar1,sp1,bar2,sp2,bar3,sp3,bar4 — 5 bars and 4 spaces.
/// 3 of the 9 are always W.
#[rustfmt::skip]
const PATTERNS: [u16; 43] = [
    // 0–9
    0b000110100, 0b100100001, 0b001100001, 0b101100000,
    0b000110001, 0b100110000, 0b001110000, 0b000100101,
    0b100100100, 0b001100100,
    // A–Z (26 entries)
    0b100001001, 0b001001001, 0b101001000, 0b000011001,
    0b100011000, 0b001011000, 0b000001101, 0b100001100,
    0b001001100, 0b000011100, 0b100000011, 0b001000011,
    0b101000010, 0b000010011, 0b100010010, 0b001010010,
    0b000000111, 0b100000110, 0b001000110, 0b000010110,
    0b110000001, 0b011000001, 0b111000000, 0b010010001,
    0b110010000, 0b011010000,
    // - . SPACE $ / + % (7 entries)
    0b010000011, 0b110000010, 0b011000010, 0b010101000,
    0b010100010, 0b010001010, 0b000101010,
];

const START_STOP: u16 = 0b010010100; // '*' — same as '$'

const NARROW: u8 = 1;
const WIDE: u8 = 2;

fn char_index(b: u8) -> Option<usize> {
    CHARSET.iter().position(|&c| c == b.to_ascii_uppercase())
}

fn encode_pattern(pat: u16, modules: &mut impl Extend<u8>) {
    for bit in (0..9).rev() {
        let w = if (pat >> bit) & 1 == 1 { WIDE } else { NARROW };
        modules.extend(core::iter::once(w));
    }
}

/// Encode `data` as a Code 39 barcode.
#[cfg(feature = "alloc")]
pub fn encode(data: &[u8]) -> Result<Code39, EncodeError> {
    if data.iter().any(|b| char_index(*b).is_none()) {
        return Err(EncodeError::InvalidCharacter);
    }

    let mut modules: alloc::vec::Vec<u8> = alloc::vec::Vec::new();

    // Start '*'
    encode_pattern(START_STOP, &mut modules);
    modules.push(NARROW); // inter-character gap

    for &b in data {
        encode_pattern(PATTERNS[char_index(b).unwrap()], &mut modules);
        modules.push(NARROW); // inter-character gap
    }

    // Stop '*'
    encode_pattern(START_STOP, &mut modules);

    Ok(Code39 { modules })
}

/// Decode Code 39 module widths to a string.
#[cfg(feature = "alloc")]
pub fn decode(modules: &[u8]) -> Result<alloc::string::String, DecodeError> {
    // Threshold: anything ≥ 2 is wide, < 2 is narrow
    let threshold = 2u8;
    let bits: alloc::vec::Vec<u16> = modules
        .iter()
        .map(|&w| if w >= threshold { 1 } else { 0 })
        .collect();

    // Strip start/stop (9 modules each) and inter-character gaps (1 module)
    if bits.len() < 9 {
        return Err(DecodeError::InvalidFormat);
    }

    let start_pat: u16 = bits[..9]
        .iter()
        .enumerate()
        .fold(0, |acc, (i, &b)| acc | (b << (8 - i)));
    if start_pat != START_STOP {
        return Err(DecodeError::InvalidFormat);
    }

    let mut out = alloc::string::String::new();
    let mut pos = 10; // skip start (9) + gap (1)

    while pos + 9 <= bits.len() {
        let chunk = &bits[pos..pos + 9];
        let pat: u16 = chunk
            .iter()
            .enumerate()
            .fold(0, |acc, (i, &b)| acc | (b << (8 - i)));

        if pat == START_STOP {
            break; // stop character
        }

        let idx = PATTERNS
            .iter()
            .position(|&p| p == pat)
            .ok_or(DecodeError::InvalidFormat)?;
        out.push(CHARSET[idx] as char);
        pos += 10; // 9 modules + 1 gap
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let bc = encode(b"HELLO").unwrap();
        let decoded = decode(&bc.modules).unwrap();
        assert_eq!(decoded, "HELLO");
    }

    #[test]
    fn encode_digits() {
        let bc = encode(b"12345").unwrap();
        let decoded = decode(&bc.modules).unwrap();
        assert_eq!(decoded, "12345");
    }

    #[test]
    fn invalid_char_rejected() {
        assert!(encode(b"|").is_err());
    }
}

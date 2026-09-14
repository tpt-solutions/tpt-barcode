//! EAN-13 barcode encoder and decoder with checksum verification.

use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// EAN-13 module encoding for left-side digits.
/// Two sets: parity set L (no parity inversion) and G (inverted).
const L: [[u8; 7]; 10] = [
    [0, 0, 0, 1, 1, 0, 1],
    [0, 0, 1, 1, 0, 0, 1],
    [0, 0, 1, 0, 0, 1, 1],
    [0, 1, 1, 1, 1, 0, 1],
    [0, 1, 0, 0, 0, 1, 1],
    [0, 1, 1, 0, 0, 0, 1],
    [0, 1, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 0, 1, 1],
    [0, 1, 1, 0, 1, 1, 1],
    [0, 0, 0, 1, 0, 1, 1],
];

const G: [[u8; 7]; 10] = [
    [0, 1, 0, 0, 1, 1, 1],
    [0, 1, 1, 0, 0, 1, 1],
    [0, 0, 1, 1, 0, 1, 1],
    [0, 1, 0, 0, 0, 0, 1],
    [0, 0, 1, 1, 1, 0, 1],
    [0, 1, 1, 1, 0, 0, 1],
    [0, 0, 0, 0, 1, 0, 1],
    [0, 0, 1, 0, 0, 0, 1],
    [0, 0, 0, 1, 0, 0, 1],
    [0, 0, 1, 0, 1, 1, 1],
];

const R: [[u8; 7]; 10] = [
    [1, 1, 1, 0, 0, 1, 0],
    [1, 1, 0, 0, 1, 1, 0],
    [1, 1, 0, 1, 1, 0, 0],
    [1, 0, 0, 0, 0, 1, 0],
    [1, 0, 1, 1, 1, 0, 0],
    [1, 0, 0, 1, 1, 1, 0],
    [1, 0, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 1, 0, 0],
    [1, 0, 0, 1, 0, 0, 0],
    [1, 1, 1, 0, 1, 0, 0],
];

/// Parity pattern for the first digit (determines L/G encoding of digits 2–7).
const FIRST_DIGIT_PARITY: [[u8; 6]; 10] = [
    [0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 1, 1],
    [0, 0, 1, 1, 0, 1],
    [0, 0, 1, 1, 1, 0],
    [0, 1, 0, 0, 1, 1],
    [0, 1, 1, 0, 0, 1],
    [0, 1, 1, 1, 0, 0],
    [0, 1, 0, 1, 0, 1],
    [0, 1, 0, 1, 1, 0],
    [0, 1, 1, 0, 1, 0],
];

/// Compute the EAN-13 checksum digit for 12-digit payload.
pub fn checksum(digits: &[u8; 12]) -> u8 {
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| d as u32 * if i % 2 == 0 { 1 } else { 3 })
        .sum();
    ((10 - (sum % 10)) % 10) as u8
}

/// Encoded EAN-13 barcode as a flat array of 0 (light) / 1 (dark) modules.
/// Standard EAN-13 has 95 modules total.
pub struct Ean13 {
    /// 95 modules: 0 = light (space), 1 = dark (bar).
    pub modules: [u8; 95],
    /// The 13 digits (including check digit).
    pub digits: [u8; 13],
}

/// Encode a 13-digit (or 12-digit, auto-checksum) EAN-13 barcode.
pub fn encode(digits: &[u8]) -> Result<Ean13, EncodeError> {
    if digits.len() != 12 && digits.len() != 13 {
        return Err(EncodeError::DataTooLong);
    }
    if digits.iter().any(|&d| d > 9) {
        return Err(EncodeError::InvalidCharacter);
    }

    let mut d13 = [0u8; 13];
    d13[..digits.len()].copy_from_slice(digits);
    if digits.len() == 12 {
        let d12: &[u8; 12] = (&d13[..12]).try_into().unwrap();
        d13[12] = checksum(d12);
    } else {
        // Verify the provided check digit
        let d12: &[u8; 12] = (&d13[..12]).try_into().unwrap();
        if d13[12] != checksum(d12) {
            return Err(EncodeError::InvalidCharacter);
        }
    }

    let mut m = [0u8; 95];
    let mut pos = 0;

    // Guard: 1 0 1
    m[pos] = 1;
    m[pos + 1] = 0;
    m[pos + 2] = 1;
    pos += 3;

    let first = d13[0] as usize;
    let parity = &FIRST_DIGIT_PARITY[first];

    // Left group (digits 2–7, i.e., d13[1..7])
    for i in 0..6 {
        let d = d13[i + 1] as usize;
        let enc = if parity[i] == 0 { L[d] } else { G[d] };
        m[pos..pos + 7].copy_from_slice(&enc);
        pos += 7;
    }

    // Centre guard: 0 1 0 1 0
    m[pos] = 0;
    m[pos + 1] = 1;
    m[pos + 2] = 0;
    m[pos + 3] = 1;
    m[pos + 4] = 0;
    pos += 5;

    // Right group (digits 8–13, i.e., d13[7..13])
    for &d in &d13[7..13] {
        m[pos..pos + 7].copy_from_slice(&R[d as usize]);
        pos += 7;
    }

    // Guard: 1 0 1
    m[pos] = 1;
    m[pos + 1] = 0;
    m[pos + 2] = 1;

    Ok(Ean13 {
        modules: m,
        digits: d13,
    })
}

/// Decode an EAN-13 barcode from 95 modules. Returns the 13 digits.
pub fn decode(modules: &[u8; 95]) -> Result<[u8; 13], DecodeError> {
    if modules[0] != 1 || modules[1] != 0 || modules[2] != 1 {
        return Err(DecodeError::InvalidFormat);
    }
    if modules[92] != 1 || modules[93] != 0 || modules[94] != 1 {
        return Err(DecodeError::InvalidFormat);
    }
    if modules[45] != 0
        || modules[46] != 1
        || modules[47] != 0
        || modules[48] != 1
        || modules[49] != 0
    {
        return Err(DecodeError::InvalidFormat);
    }

    let mut digits = [0u8; 13];
    let mut parity_pattern = [0u8; 6];

    // Decode left group
    for i in 0..6 {
        let pos = 3 + i * 7;
        let chunk = modules[pos..pos + 7].try_into().unwrap();
        if let Some((d, p)) = decode_left(chunk) {
            digits[i + 1] = d;
            parity_pattern[i] = p;
        } else {
            return Err(DecodeError::InvalidFormat);
        }
    }

    // Recover first digit from parity pattern
    digits[0] = (0..10u8)
        .find(|&d| FIRST_DIGIT_PARITY[d as usize] == parity_pattern)
        .ok_or(DecodeError::InvalidFormat)?;

    // Decode right group
    for i in 0..6 {
        let pos = 50 + i * 7;
        let chunk = modules[pos..pos + 7].try_into().unwrap();
        digits[i + 7] = decode_right(chunk).ok_or(DecodeError::InvalidFormat)?;
    }

    // Verify checksum
    let d12: &[u8; 12] = (&digits[..12]).try_into().unwrap();
    if checksum(d12) != digits[12] {
        return Err(DecodeError::TooManyErrors);
    }

    Ok(digits)
}

fn decode_left(chunk: &[u8; 7]) -> Option<(u8, u8)> {
    for (d, pat) in L.iter().enumerate() {
        if pat == chunk {
            return Some((d as u8, 0));
        }
    }
    for (d, pat) in G.iter().enumerate() {
        if pat == chunk {
            return Some((d as u8, 1));
        }
    }
    None
}

fn decode_right(chunk: &[u8; 7]) -> Option<u8> {
    R.iter().position(|pat| pat == chunk).map(|d| d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_known() {
        // EAN-13: 5901234123457
        let digits: [u8; 12] = [5, 9, 0, 1, 2, 3, 4, 1, 2, 3, 4, 5];
        assert_eq!(checksum(&digits), 7);
    }

    #[test]
    fn encode_decode_round_trip() {
        let input: [u8; 12] = [5, 9, 0, 1, 2, 3, 4, 1, 2, 3, 4, 5];
        let bc = encode(&input).unwrap();
        assert_eq!(bc.digits[12], 7);
        let decoded = decode(&bc.modules).unwrap();
        assert_eq!(decoded, bc.digits);
    }

    #[test]
    fn bad_check_digit_rejected() {
        let bad: [u8; 13] = [5, 9, 0, 1, 2, 3, 4, 1, 2, 3, 4, 5, 0]; // wrong check
        assert!(encode(&bad).is_err());
    }
}

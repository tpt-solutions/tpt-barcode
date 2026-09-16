//! PDF417 Numeric Compaction (ISO/IEC 15438 §5.4.4): up to 44 digits per
//! group, prefixed with a leading "1", packed base-10 → base-900 into
//! exactly 15 codewords per group (2.93 digits per codeword).

use alloc::vec::Vec;

use tpt_barcode_core::traits::DecodeError;

/// Latch: Numeric Compaction mode.
pub const LATCH_NUMERIC: u16 = 902;

/// Digits per group (the "1" prefix makes each group a 45-digit number,
/// which always packs into exactly 15 base-900 codewords).
const GROUP_DIGITS: usize = 44;
/// Codewords per group.
const GROUP_CODEWORDS: usize = 15;

/// Whether every byte of `data` is an ASCII digit.
pub fn is_numeric_encodable(data: &[u8]) -> bool {
    data.iter().all(|b| b.is_ascii_digit())
}

/// Divide a little-endian base-10 digit vector by `divisor` (≤ 30); returns
/// the remainder and leaves the quotient in place. Single-decimal-digit long
/// division is only valid while rem·10 + d stays below the divisor × 10, so
/// division by 900 is performed as two passes of division by 30.
fn div_mod(digits: &mut Vec<u8>, divisor: u32) -> u32 {
    let mut rem = 0u32;
    for d in digits.iter_mut().rev() {
        let cur = rem * 10 + u32::from(*d);
        *d = (cur / divisor) as u8;
        rem = cur % divisor;
    }
    while digits.len() > 1 && *digits.last().unwrap() == 0 {
        digits.pop();
    }
    rem
}

/// Divide a little-endian base-10 digit vector by 900; returns the remainder
/// and leaves the quotient in place (leading zeros trimmed).
fn div_mod_900(digits: &mut Vec<u8>) -> u16 {
    let r1 = div_mod(digits, 30);
    let r2 = div_mod(digits, 30);
    // N = Q2·900 + r2·30 + r1
    (r2 * 30 + r1) as u16
}

/// Encode `data` (ASCII digits) into numeric-compaction codewords: latch 902
/// followed by 44-digit groups, each prefixed with "1" and packed base-10 →
/// base-900 (most significant codeword first).
pub fn encode_numeric(data: &[u8]) -> Result<Vec<u16>, DecodeError> {
    if !is_numeric_encodable(data) || data.is_empty() {
        return Err(DecodeError::InvalidFormat);
    }
    let mut out = alloc::vec![LATCH_NUMERIC];
    for group in data.chunks(GROUP_DIGITS) {
        // decimal digit VALUES, little-endian, prefixed with a leading 1
        let mut digits: Vec<u8> = group.iter().rev().map(|&b| b - b'0').collect();
        digits.push(1);

        let mut rems = Vec::with_capacity(GROUP_CODEWORDS);
        while digits.len() > 1 || digits[0] != 0 {
            rems.push(div_mod_900(&mut digits));
        }
        out.extend(rems.iter().rev());
    }
    Ok(out)
}

/// Multiply a little-endian base-10 digit vector by `factor` and add `add`.
fn mul_add(digits: &mut Vec<u8>, factor: u32, add: u32) {
    let mut carry = add;
    for d in digits.iter_mut() {
        let cur = *d as u32 * factor + carry;
        *d = (cur % 10) as u8;
        carry = cur / 10;
    }
    while carry > 0 {
        digits.push((carry % 10) as u8);
        carry /= 10;
    }
}

/// Decode one 15-codeword numeric group into its digits (leading "1"
/// stripped).
fn decode_group(codewords: &[u16]) -> Result<Vec<u8>, DecodeError> {
    let mut digits: Vec<u8> = alloc::vec![0];
    for &cw in codewords {
        if cw >= 900 {
            return Err(DecodeError::InvalidFormat);
        }
        mul_add(&mut digits, 900, cw as u32);
    }
    // Strip the leading "1" (most significant digit of the value). The
    // remaining digits — including any leading zeros — are exactly the
    // group's payload, so no trimming is permitted here.
    if digits.pop() != Some(1) {
        return Err(DecodeError::InvalidFormat);
    }
    digits.reverse(); // little-endian → ASCII order
    Ok(digits.iter().map(|&d| b'0' + d).collect())
}

/// Decode a numeric-compaction segment: `codewords` are the codewords after
/// the 902 latch (all of which belong to this segment). Groups of 15
/// codewords are converted independently and concatenated.
pub fn decode_numeric(codewords: &[u16]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    for chunk in codewords.chunks(GROUP_CODEWORDS) {
        out.extend(decode_group(chunk)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_example_15_digits() {
        // ISO/IEC 15438 §5.4.4 example
        let encoded = encode_numeric(b"000213298174000").unwrap();
        assert_eq!(encoded, alloc::vec![902, 1, 624, 434, 632, 282, 200]);
        assert_eq!(decode_numeric(&encoded[1..]).unwrap(), b"000213298174000");
    }

    #[test]
    fn group_boundary_44_vs_45() {
        // 44 digits → 1 group of 15 codewords
        let e44 = encode_numeric(&[b'9'; 44]).unwrap();
        assert_eq!(e44.len(), 16); // latch + 15
        assert_eq!(decode_numeric(&e44[1..]).unwrap(), alloc::vec![b'9'; 44]);

        // 45 digits → 2 groups (44 + 1); the single-digit group packs into
        // one codeword ("1" + "9" = 19 < 900)
        let e45 = encode_numeric(&[b'9'; 45]).unwrap();
        assert_eq!(e45.len(), 1 + 15 + 1);
        assert_eq!(decode_numeric(&e45[1..]).unwrap(), alloc::vec![b'9'; 45]);

        // 89 digits → 2 groups (44 + 45)
        let e89 = encode_numeric(&[b'9'; 89]).unwrap();
        assert_eq!(e89.len(), 1 + 31);
        assert_eq!(decode_numeric(&e89[1..]).unwrap(), alloc::vec![b'9'; 89]);
    }

    #[test]
    fn single_digit() {
        let e = encode_numeric(b"7").unwrap();
        assert_eq!(decode_numeric(&e[1..]).unwrap(), b"7");
    }
}

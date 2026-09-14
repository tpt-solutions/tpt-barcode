//! UPC-A barcode encoder and decoder.
//!
//! UPC-A is a strict subset of EAN-13 with an implicit leading `0` digit.
//! All logic delegates to the EAN-13 implementation.

use crate::ean13;
use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// Encoded UPC-A barcode (95 modules, identical layout to EAN-13 with first digit = 0).
pub struct UpcA {
    /// 95 modules: 0 = light, 1 = dark.
    pub modules: [u8; 95],
    /// The 12 visible UPC-A digits (the 13th digit of the underlying EAN-13 is the check digit).
    pub digits: [u8; 12],
}

/// Encode an 11-digit (auto-checksum) or 12-digit UPC-A barcode.
pub fn encode(digits: &[u8]) -> Result<UpcA, EncodeError> {
    if digits.len() != 11 && digits.len() != 12 {
        return Err(EncodeError::DataTooLong);
    }
    if digits.iter().any(|&d| d > 9) {
        return Err(EncodeError::InvalidCharacter);
    }

    // Prepend a 0 to make EAN-13
    let mut ean_input = [0u8; 12];
    ean_input[0] = 0;
    ean_input[1..1 + digits.len()].copy_from_slice(digits);

    // If only 11 digits provided, leave ean_input[12] as 0 and let ean13 auto-compute
    let ean_bc = if digits.len() == 11 {
        ean13::encode(&ean_input[..12])?
    } else {
        let mut ean13_digits = [0u8; 13];
        ean13_digits[..13].copy_from_slice(&{
            let mut tmp = [0u8; 13];
            tmp[0] = 0;
            tmp[1..13].copy_from_slice(digits);
            tmp
        });
        ean13::encode(&ean13_digits)?
    };

    let mut upca_digits = [0u8; 12];
    upca_digits.copy_from_slice(&ean_bc.digits[1..13]);

    Ok(UpcA {
        modules: ean_bc.modules,
        digits: upca_digits,
    })
}

/// Decode a UPC-A barcode from 95 modules. Returns the 12 UPC-A digits.
pub fn decode(modules: &[u8; 95]) -> Result<[u8; 12], DecodeError> {
    let ean_digits = ean13::decode(modules)?;
    if ean_digits[0] != 0 {
        return Err(DecodeError::InvalidFormat);
    }
    let mut out = [0u8; 12];
    out.copy_from_slice(&ean_digits[1..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        // 11-digit input, auto-checksum
        let digits: [u8; 11] = [0, 3, 6, 0, 0, 0, 2, 9, 1, 4, 5];
        let bc = encode(&digits).unwrap();
        let decoded = decode(&bc.modules).unwrap();
        assert_eq!(&decoded[..11], &digits);
    }
}

//! Data Matrix (ISO/IEC 16022) barcode symbology.
//!
//! Data Matrix uses an L-shaped solid finder border (left column + bottom row)
//! and alternating timing patterns on the top row and right column. Data lives
//! in the interior, encoded with Reed-Solomon over GF(256) with the ECC 200
//! primitive polynomial 0x12D and generator roots α^1…α^n (note: shifted one
//! exponent versus QR Code's 0x11D field).

mod decode;
mod encode;
mod placement;

pub use decode::decode_grid;
pub use encode::encode_codewords;

use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// Supported Data Matrix sizes (ECC 200 square symbols, single RS block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmSize {
    /// 10×10 modules (3 data codewords, 5 EC codewords).
    S10x10,
    /// 12×12 modules.
    S12x12,
    /// 14×14 modules.
    S14x14,
    /// 16×16 modules.
    S16x16,
    /// 18×18 modules.
    S18x18,
    /// 20×20 modules.
    S20x20,
    /// 22×22 modules.
    S22x22,
    /// 24×24 modules.
    S24x24,
    /// 26×26 modules.
    S26x26,
}

impl DmSize {
    /// All supported sizes, smallest first.
    pub const ALL: [DmSize; 9] = [
        DmSize::S10x10,
        DmSize::S12x12,
        DmSize::S14x14,
        DmSize::S16x16,
        DmSize::S18x18,
        DmSize::S20x20,
        DmSize::S22x22,
        DmSize::S24x24,
        DmSize::S26x26,
    ];

    /// Side length in modules.
    pub fn modules(self) -> usize {
        match self {
            DmSize::S10x10 => 10,
            DmSize::S12x12 => 12,
            DmSize::S14x14 => 14,
            DmSize::S16x16 => 16,
            DmSize::S18x18 => 18,
            DmSize::S20x20 => 20,
            DmSize::S22x22 => 22,
            DmSize::S24x24 => 24,
            DmSize::S26x26 => 26,
        }
    }

    /// Data codewords capacity.
    pub fn data_codewords(self) -> usize {
        match self {
            DmSize::S10x10 => 3,
            DmSize::S12x12 => 5,
            DmSize::S14x14 => 8,
            DmSize::S16x16 => 12,
            DmSize::S18x18 => 18,
            DmSize::S20x20 => 22,
            DmSize::S22x22 => 30,
            DmSize::S24x24 => 36,
            DmSize::S26x26 => 44,
        }
    }

    /// Error correction codewords.
    pub fn ec_codewords(self) -> usize {
        match self {
            DmSize::S10x10 => 5,
            DmSize::S12x12 => 7,
            DmSize::S14x14 => 10,
            DmSize::S16x16 => 12,
            DmSize::S18x18 => 14,
            DmSize::S20x20 => 18,
            DmSize::S22x22 => 20,
            DmSize::S24x24 => 24,
            DmSize::S26x26 => 28,
        }
    }

    /// Select the smallest size that fits `n` data bytes.
    pub fn select(n: usize) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.data_codewords() >= n)
    }
}

/// An encoded Data Matrix symbol.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataMatrix {
    /// Flat row-major module array (0 = light, 1 = dark), `size × size`.
    pub matrix: alloc::vec::Vec<u8>,
    /// Side length in modules.
    pub size: usize,
}

/// Encode `data` as a Data Matrix ECC 200 symbol.
///
/// Payload bytes are encodable in full: values ≥ 128 use the ASCII upper
/// shift. Returns [`EncodeError::DataTooLong`] when no single-block size fits.
#[cfg(feature = "alloc")]
pub fn encode(data: &[u8]) -> Result<DataMatrix, EncodeError> {
    let encoded = encode::encode_data(data);
    let size = DmSize::select(encoded.len()).ok_or(EncodeError::DataTooLong)?;
    let codewords = encode::with_ec(encode::pad(encoded, size.data_codewords()), size);
    let mut matrix = alloc::vec![0u8; size.modules() * size.modules()];
    encode::build_symbol(&codewords, size, &mut matrix);
    Ok(DataMatrix {
        matrix,
        size: size.modules(),
    })
}

/// Encode `data` as a GS1 Data Matrix: the FNC1 codeword (232) is emitted
/// as the first data codeword, marking the payload as GS1-formatted
/// (Application Identifier strings).
#[cfg(feature = "alloc")]
pub fn encode_gs1(data: &[u8]) -> Result<DataMatrix, EncodeError> {
    let encoded = encode::encode_data(data);
    let size = DmSize::select(encoded.len() + 1).ok_or(EncodeError::DataTooLong)?;
    let codewords = encode::with_ec(encode::pad(encoded, size.data_codewords()), size);
    let mut with_fnc1 = codewords;
    with_fnc1[0] = 232; // FNC1 (first position) — codeword after the SLD

    let mut matrix = alloc::vec![0u8; size.modules() * size.modules()];
    encode::build_symbol(&with_fnc1, size, &mut matrix);

    Ok(DataMatrix {
        matrix,
        size: size.modules(),
    })
}

/// Decode a Data Matrix symbol from a flat row-major module array.
///
/// `matrix` is `size × size`, values 0 (light) / 1 (dark).
pub fn decode(matrix: &[u8], size: usize) -> Result<alloc::vec::Vec<u8>, DecodeError> {
    decode::decode_grid(matrix, size)
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use tpt_barcode_core::traits::DecodeError;

    /// Reference 10×10 symbol for "123456", generated by the `pylibdmtx`
    /// (libdmtx) reference encoder and verified decodable by the `zxing-cpp`
    /// reference decoder.
    const REFERENCE_123456_10X10: [&str; 10] = [
        "1010101010",
        "1100101101",
        "1100000100",
        "1100011101",
        "1100001000",
        "1000001111",
        "1110110000",
        "1111011001",
        "1001110100",
        "1111111111",
    ];

    fn reference_grid() -> (alloc::vec::Vec<u8>, usize) {
        let size = REFERENCE_123456_10X10.len();
        let grid = REFERENCE_123456_10X10
            .iter()
            .flat_map(|row| row.bytes().map(|b| b - b'0'))
            .collect();
        (grid, size)
    }

    #[test]
    fn encode_matches_reference_matrix() {
        let dm = encode(b"123456").unwrap();
        let (grid, size) = reference_grid();
        assert_eq!(dm.size, size);
        assert_eq!(dm.matrix, grid, "encoder must match the reference symbol");
    }

    #[test]
    fn decodes_reference_matrix() {
        let (grid, size) = reference_grid();
        let payload = decode(&grid, size).expect("reference symbol must decode");
        assert_eq!(payload, b"123456");
    }

    #[test]
    fn round_trip_ascii() {
        for payload in ["tpt-barcode", "Hello, DataMatrix!", "ABC123"] {
            let dm = encode(payload.as_bytes()).unwrap();
            assert_eq!(decode(&dm.matrix, dm.size).unwrap(), payload.as_bytes());
        }
    }

    #[test]
    fn round_trip_digit_heavy() {
        // Digit pairs pack two per codeword in ASCII encodation
        let payload = "0123456789";
        let dm = encode(payload.as_bytes()).unwrap();
        // 5 codewords exceed 10×10's capacity of 3 → 12×12
        assert_eq!(dm.size, 12);
        assert_eq!(decode(&dm.matrix, dm.size).unwrap(), payload.as_bytes());
    }

    #[test]
    fn round_trip_high_bytes_use_upper_shift() {
        let payload: &[u8] = &[b'H', b'i', 0x80, 0xFF, 0xC3];
        let dm = encode(payload).unwrap();
        assert_eq!(decode(&dm.matrix, dm.size).unwrap(), payload);
    }

    #[test]
    fn round_trip_all_sizes() {
        // Walk sizes by growing the payload until each size is selected
        let mut seen = alloc::collections::BTreeSet::new();
        for len in [3usize, 5, 8, 12, 18, 22, 30, 36, 44] {
            let payload: alloc::vec::Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
            let dm = encode(&payload).unwrap();
            seen.insert(dm.size);
            assert_eq!(decode(&dm.matrix, dm.size).unwrap(), payload);
        }
        assert_eq!(seen.len(), 9, "all nine sizes exercised, got {seen:?}");
    }

    #[test]
    fn corrects_module_errors() {
        // Flip a handful of data modules (within the EC capacity) and decode.
        let payload = b"payload8";
        let dm = encode(payload).unwrap();
        assert_eq!(dm.size, 14);
        let mut corrupted = dm.matrix.clone();
        let mut flipped = 0;
        'outer: for r in 0..dm.size {
            for c in 0..dm.size {
                // Only flip interior (data region) modules
                if r > 0 && r < dm.size - 1 && c > 0 && c < dm.size - 1 {
                    corrupted[r * dm.size + c] ^= 1;
                    flipped += 1;
                }
                if flipped == 4 {
                    break 'outer;
                }
            }
        }
        assert_eq!(flipped, 4);
        assert_eq!(decode(&corrupted, dm.size).unwrap(), payload);
    }

    #[test]
    fn rejects_bad_finder() {
        let (mut grid, size) = reference_grid();
        // Break the solid L-finder
        grid[size * size - 1] = 0;
        assert_eq!(decode(&grid, size), Err(DecodeError::InvalidFormat));
    }

    #[test]
    fn gs1_round_trip() {
        // FNC1 (232) as first data codeword; payload decodes without the
        // FNC1 marker itself
        let payload = b"0109501101020917";
        let dm = encode_gs1(payload).unwrap();
        let decoded = decode(&dm.matrix, dm.size).unwrap();
        // the decoder returns the raw payload; GS1 AIs remain in the text
        assert_eq!(decoded, payload);
    }

    #[test]
    fn rejects_data_too_long() {
        let long = [b'A'; 45];
        assert!(matches!(encode(&long), Err(EncodeError::DataTooLong)));
    }
}

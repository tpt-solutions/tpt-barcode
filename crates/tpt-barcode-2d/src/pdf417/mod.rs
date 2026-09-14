//! PDF417 (ISO/IEC 15438) stacked linear barcode symbology.
//!
//! PDF417 arranges up to 928 data codewords in rows of 1-30 codewords, each
//! row bracketed by start/stop patterns and row-indicator codewords drawn
//! from one of three interleaved 17-module pattern clusters. Error correction
//! is Reed-Solomon over GF(929) (a prime field - see [`gf929`]), structurally
//! different from the GF(2^8) engines in [`crate::qr`] and [`crate::datamatrix`].
//!
//! # Status
//!
//! Byte compaction is implemented end to end (encode, render, decode, EC
//! correction). Text and numeric compaction are not implemented yet; see
//! [`Compaction`].

mod decode;
mod encode;
mod gf929;
mod tables;

pub use decode::decode_grid;
pub use encode::recommended_ec_level;

use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// PDF417 compaction modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compaction {
    /// Binary/byte compaction - encodes arbitrary bytes. **Implemented.**
    Byte,
    /// Text compaction - encodes printable ASCII more efficiently. Not
    /// implemented yet.
    Text,
    /// Numeric compaction - encodes long digit strings very compactly. Not
    /// implemented yet.
    Numeric,
}

/// PDF417 error correction level (0-8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EcLevel(pub u8);

impl EcLevel {
    /// Number of EC codewords for this level: 2^(level+1).
    pub fn codeword_count(self) -> usize {
        2usize.pow(self.0 as u32 + 1)
    }
}

/// An encoded PDF417 symbol.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdf417 {
    /// Flat row-major module array (0 = light, 1 = dark).
    pub matrix: alloc::vec::Vec<u8>,
    /// Width in modules (`17*columns + 69`).
    pub width: usize,
    /// Height in modules (one module per row).
    pub height: usize,
}

/// Encode `data` as a PDF417 barcode using byte compaction.
///
/// Byte compaction encodes arbitrary binary payloads; `ec` must be 0-8 (see
/// [`recommended_ec_level`] for the ISO-recommended choice).
#[cfg(feature = "alloc")]
pub fn encode(data: &[u8], ec: EcLevel, compaction: Compaction) -> Result<Pdf417, EncodeError> {
    match compaction {
        Compaction::Byte => {}
        Compaction::Text | Compaction::Numeric => return Err(EncodeError::Unsupported),
    }
    if data.is_empty() {
        return Err(EncodeError::InvalidCharacter);
    }

    let codewords = encode::encode_codewords(data, ec)?;
    let m = encode::encode_byte_compaction(data).len();
    let k = ec.codeword_count();
    let (cols, rows) = encode::determine_dimensions(m, k).ok_or(EncodeError::DataTooLong)?;

    let width = 17 * cols + 69;
    let matrix = encode::render(&codewords, cols, rows, ec.0);
    debug_assert_eq!(matrix.len(), width * rows);

    Ok(Pdf417 {
        matrix,
        width,
        height: rows,
    })
}

/// Decode a PDF417 symbol from a flat row-major module array.
///
/// `matrix` is `width x height` with 0 = light, 1 = dark, where the symbol
/// width is `17*cols + 69` modules.
#[cfg(feature = "alloc")]
pub fn decode(
    matrix: &[u8],
    width: usize,
    height: usize,
) -> Result<alloc::vec::Vec<u8>, DecodeError> {
    decode::decode_grid(matrix, width, height)
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use tpt_barcode_core::traits::DecodeError;

    #[test]
    fn round_trip_byte_compaction() {
        for payload in [
            &b"PDF417"[..],
            b"Hello, PDF417 symbol!",
            b"exact-sixpack-of-bytes!",
            &[0x00u8, 0x01, 0xFE, 0xFF, 0x80, 0x7F, 0x42][..],
        ] {
            let level = EcLevel(recommended_ec_level(payload.len()));
            let symbol = encode(payload, level, Compaction::Byte).unwrap();
            let decoded = decode(&symbol.matrix, symbol.width, symbol.height).unwrap();
            assert_eq!(decoded, payload, "round trip failed for {payload:?}");
        }
    }

    #[test]
    fn round_trip_multi_row() {
        // Enough codewords to force several rows
        let payload: alloc::vec::Vec<u8> = (0..120u32).map(|i| (i % 251) as u8).collect();
        let level = EcLevel(recommended_ec_level(payload.len()));
        let symbol = encode(&payload, level, Compaction::Byte).unwrap();
        assert!(symbol.height >= 3, "expected multiple rows");
        assert_eq!(
            decode(&symbol.matrix, symbol.width, symbol.height).unwrap(),
            payload
        );
    }

    #[test]
    fn corrects_codeword_errors() {
        use super::tables::CODEWORD_TABLE;

        // Long enough to produce several columns
        let payload: alloc::vec::Vec<u8> = b"error correction payload. "
            .iter()
            .copied()
            .cycle()
            .take(100)
            .collect();
        let symbol = encode(&payload, EcLevel(4), Compaction::Byte).unwrap();
        assert!((symbol.width - 69) / 17 >= 2, "need multiple columns");

        // Substitute valid-but-wrong codeword patterns (what a scanner reads
        // after misgrading bars): level 4 corrects up to 32 EC codewords.
        let mut corrupted = symbol.matrix.clone();
        for row in 0..2usize {
            let cluster = row % 3;
            for data_col in [0usize, 1] {
                let x = 34 + data_col * 17;
                let current =
                    decode::lookup(cluster, slice(&corrupted, row * symbol.width + x)).unwrap();
                let wrong = (current + 137) % 929;
                let mask = CODEWORD_TABLE[cluster][wrong as usize];
                for dx in 0..17 {
                    let bit = ((mask >> (16 - dx)) & 1) as u8;
                    corrupted[row * symbol.width + x + dx] = bit;
                }
            }
        }
        match decode(&corrupted, symbol.width, symbol.height) {
            Ok(d) if d == payload.as_slice() => {}
            other => panic!("decode failed: {:?}", other.err()),
        }
    }

    fn slice(m: &[u8], start: usize) -> u32 {
        m[start..start + 17]
            .iter()
            .fold(0u32, |acc, &b| (acc << 1) | b as u32)
    }

    #[test]
    fn rejects_bad_start_pattern() {
        let symbol = encode(b"PDF", EcLevel(2), Compaction::Byte).unwrap();
        let mut broken = symbol.matrix.clone();
        broken[0] ^= 1;
        assert_eq!(
            decode(&broken, symbol.width, symbol.height),
            Err(DecodeError::InvalidFormat)
        );
    }

    #[test]
    fn rejects_unsupported_modes() {
        assert!(matches!(
            encode(b"PDF", EcLevel(2), Compaction::Text),
            Err(EncodeError::Unsupported)
        ));
        assert!(matches!(
            encode(b"PDF", EcLevel(9), Compaction::Byte),
            Err(EncodeError::Unsupported)
        ));
        assert!(matches!(
            encode(b"", EcLevel(2), Compaction::Byte),
            Err(EncodeError::InvalidCharacter)
        ));
    }
}

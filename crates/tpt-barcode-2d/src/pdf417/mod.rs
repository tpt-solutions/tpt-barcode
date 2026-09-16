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
mod numeric;
mod tables;
mod text;

#[cfg(feature = "alloc")]
pub use numeric::{decode_numeric, encode_numeric};
#[cfg(feature = "alloc")]
pub use text::{decode_text, encode_text, is_text_encodable as text_encodable};

pub use decode::decode_grid;
pub use encode::recommended_ec_level;
pub use text::is_text_encodable;

use tpt_barcode_core::traits::{DecodeError, EncodeError};

/// PDF417 compaction modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compaction {
    /// Pick the most compact mode per payload (numeric for ≥11 digits, text
    /// for printable ASCII, byte otherwise).
    Auto,
    /// Binary/byte compaction — encodes arbitrary bytes.
    Byte,
    /// Text compaction — encodes printable ASCII (2 chars per codeword).
    Text,
    /// Numeric compaction — encodes digit strings (~2.93 digits per codeword).
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

/// Encode `data` as a PDF417 barcode.
///
/// `ec` must be 0-8 (see [`recommended_ec_level`] for the ISO-recommended
/// choice). With [`Compaction::Auto`] the most compact mode is chosen:
/// numeric for ≥11 digits, text for printable ASCII, byte otherwise.
#[cfg(feature = "alloc")]
pub fn encode(data: &[u8], ec: EcLevel, compaction: Compaction) -> Result<Pdf417, EncodeError> {
    if data.is_empty() {
        return Err(EncodeError::InvalidCharacter);
    }

    // Auto-select the most compact mode unless explicitly overridden.
    let compaction = match compaction {
        c @ (Compaction::Byte | Compaction::Text | Compaction::Numeric) => c,
        Compaction::Auto => {
            if data.iter().all(u8::is_ascii_digit) && data.len() >= 11 {
                Compaction::Numeric
            } else if text::is_text_encodable(data) {
                Compaction::Text
            } else {
                Compaction::Byte
            }
        }
    };

    // m = source codewords of the chosen mode (drives row/column layout)
    let (codewords, m) = match compaction {
        Compaction::Byte => {
            let cw = encode::encode_codewords_byte(data, ec)?;
            let m = encode::encode_byte_compaction(data).len();
            (cw, m)
        }
        Compaction::Text => {
            let cw = encode::encode_codewords_text(data, ec)?;
            let m = text::encode_text(data)
                .map_err(|_| EncodeError::InvalidCharacter)?
                .len();
            (cw, m)
        }
        Compaction::Numeric => {
            let cw = encode::encode_codewords_numeric(data, ec)?;
            let m = numeric::encode_numeric(data)
                .map_err(|_| EncodeError::InvalidCharacter)?
                .len();
            (cw, m)
        }
        // The shadowed binding above is never Auto; the arm satisfies the
        // exhaustive match.
        Compaction::Auto => unreachable!("compaction resolved above"),
    };
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
    fn rejects_unsupported_modes_and_levels() {
        assert!(matches!(
            encode(b"PDF", EcLevel(9), Compaction::Byte),
            Err(EncodeError::Unsupported)
        ));
        assert!(matches!(
            encode(b"", EcLevel(2), Compaction::Byte),
            Err(EncodeError::InvalidCharacter)
        ));
    }

    #[test]
    fn text_compaction_round_trip() {
        for payload in [
            &b"Hello, PDF417 text compaction!"[..],
            b"Mixed CASE 123 with punctuation?!",
            b"tab	newline
return
 chars",
        ] {
            let symbol = encode(payload, EcLevel(2), Compaction::Text).unwrap();
            let decoded = decode(&symbol.matrix, symbol.width, symbol.height).unwrap();
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn numeric_compaction_round_trip() {
        for len in [11usize, 15, 44, 45, 89, 133] {
            let payload: alloc::vec::Vec<u8> =
                (0..len).map(|i| b'0' + (i * 7 % 10) as u8).collect();
            let symbol = encode(&payload, EcLevel(2), Compaction::Numeric).unwrap();
            let decoded = decode(&symbol.matrix, symbol.width, symbol.height).unwrap();
            assert_eq!(decoded, payload, "numeric len={len}");
        }
    }

    #[test]
    fn auto_selects_numeric_for_digit_payloads() {
        // 60 digits: numeric uses ~2.93 digits/codeword vs 1 for byte
        let payload: alloc::vec::Vec<u8> =
            (0..60usize).map(|i| b'0' + (i * 7 % 10) as u8).collect();
        let auto = encode(&payload, EcLevel(2), Compaction::Auto).unwrap();
        let byte = encode(&payload, EcLevel(2), Compaction::Byte).unwrap();
        assert!(
            auto.width * auto.height <= byte.width * byte.height,
            "numeric should pack at least as tight"
        );
        assert_eq!(
            decode(&auto.matrix, auto.width, auto.height).unwrap(),
            payload
        );
    }
}

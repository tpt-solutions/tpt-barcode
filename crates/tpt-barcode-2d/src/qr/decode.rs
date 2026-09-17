//! QR Code decoding: module grid → codewords → error-corrected payload bytes.
//!
//! Reads format information (BCH-validated, both copies), unmasks the data
//! region, extracts codewords in the ISO 18004 zigzag order, de-interleaves the
//! EC blocks, runs the Reed-Solomon decoder from `tpt-barcode-core`, and parses
//! the mode/bit stream back into payload bytes.

use alloc::format;
use alloc::vec::Vec;

use tpt_barcode_core::reed_solomon;
use tpt_barcode_core::traits::{DecodeError, EcLevel};

use super::mask::is_masked;
use super::matrix::{for_each_data_position, format_bits, function_patterns, FORMAT_MASK};
use super::mode::{char_count_bits_of, ALPHANUMERIC_CHARSET};
use super::version::version_info;

/// A decoded QR Code plus its structural metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedQr {
    /// Decoded payload bytes.
    pub payload: Vec<u8>,
    /// QR version (1–40).
    pub version: u8,
    /// Mask pattern id (0–7) from the format information.
    pub mask: u8,
    /// Error-correction level from the format information.
    pub ec_level: Option<EcLevel>,
}

/// Decode a QR Code from a sampled module grid, returning payload + metadata.
///
/// `grid` is a flat row-major size×size array of 0 (light) / 1 (dark) modules.
pub fn decode_grid_detailed(grid: &[u8], size: usize) -> Result<DecodedQr, DecodeError> {
    if size < 21 || grid.len() != size * size || (size - 21) % 4 != 0 {
        return Err(DecodeError::InvalidFormat);
    }
    let version = ((size - 21) / 4 + 1) as u8;

    let (ec_bits, mask_id) = read_format(grid, size).ok_or(DecodeError::InvalidFormat)?;
    let ec_level = ec_from_bits(ec_bits);
    let info = version_info(version, ec_level).ok_or(DecodeError::InvalidFormat)?;

    // Unmask data modules in place on a copy
    let mut data = grid.to_vec();
    let (_, is_function) = function_patterns(version);
    for i in 0..size * size {
        if !is_function[i] && is_masked(mask_id, i / size, i % size) {
            data[i] ^= 1;
        }
    }

    // Extract codewords in zigzag order
    let total = info.total_codewords as usize;
    let mut codewords = alloc::vec![0u8; total];
    let mut bit_idx = 0usize;
    for_each_data_position(size, |r, c| {
        let idx = r * size + c;
        if is_function[idx] {
            return;
        }
        if bit_idx / 8 < total {
            codewords[bit_idx / 8] |= (data[idx] & 1) << (7 - bit_idx % 8);
        }
        bit_idx += 1;
    });

    let data_bytes = deinterleave_and_correct(&codewords, &info)?;

    let payload = decode_payload(&data_bytes, version)?;
    Ok(DecodedQr {
        payload,
        version,
        mask: mask_id,
        ec_level: Some(ec_level),
    })
}

/// Decode a QR Code from a sampled module grid; returns payload bytes only.
pub fn decode_grid(grid: &[u8], size: usize) -> Result<Vec<u8>, DecodeError> {
    decode_grid_detailed(grid, size).map(|d| d.payload)
}

/// Read and BCH-validate the 15-bit format information.
///
/// Tries the first copy (top-left), then the second copy. If neither is valid,
/// picks the candidate (ec, mask) codeword with the smallest Hamming distance
/// (corrects up to 3 bit errors, the BCH(15,5) guarantee).
pub fn read_format(grid: &[u8], size: usize) -> Option<(u8, u8)> {
    let read = |r: usize, c: usize| grid[r * size + c] as u16;

    // First copy (around the top-left finder): bits 0–5 up column 8, bit 6 at
    // (7,8), bit 7 at (8,8), bit 8 at (8,7), bits 9–14 leftward along row 8.
    let mut bits1: u16 = 0;
    for i in 0..6usize {
        bits1 |= read(i, 8) << i;
    }
    bits1 |= read(7, 8) << 6;
    bits1 |= read(8, 8) << 7;
    bits1 |= read(8, 7) << 8;
    for i in 9..15usize {
        bits1 |= read(8, 14 - i) << i;
    }

    // Second copy: bits 0–7 along row 8 from the right edge, bits 8–14 up
    // column 8 from the bottom.
    let mut bits2: u16 = 0;
    for i in 0..8usize {
        bits2 |= read(8, size - 1 - i) << i;
    }
    for i in 8..15usize {
        bits2 |= read(size - 15 + i, 8) << i;
    }

    for bits in [bits1, bits2] {
        if let Some(payload) = super::matrix::validate_format(bits ^ FORMAT_MASK) {
            return Some((payload >> 3 & 0b11, payload & 0b111));
        }
    }

    // Error correction: find the valid codeword closest to either copy
    let mut best: Option<(u32, u8, u8)> = None;
    for ec in 0u8..4 {
        for mask in 0u8..8 {
            let codeword = format_bits(ec, mask);
            let d1 = (codeword ^ bits1).count_ones();
            let d2 = (codeword ^ bits2).count_ones();
            let d = d1.min(d2);
            if d <= 3 && best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, ec, mask));
            }
        }
    }
    best.map(|(_, ec, mask)| (ec, mask))
}

fn ec_from_bits(ec_bits: u8) -> EcLevel {
    match ec_bits {
        0b01 => EcLevel::L,
        0b00 => EcLevel::M,
        0b11 => EcLevel::Q,
        _ => EcLevel::H,
    }
}

/// Split the interleaved codeword stream into blocks, RS-correct each block,
/// and return the reassembled data codewords.
fn deinterleave_and_correct(
    codewords: &[u8],
    info: &super::version::VersionInfo,
) -> Result<Vec<u8>, DecodeError> {
    let g1 = &info.group1;
    let g2 = &info.group2;
    let n_blocks = g1.count as usize + g2.count as usize;
    let n_ec = g1.ec_codewords as usize;
    let data_len = info.data_codewords();

    // Block shapes in order
    let mut block_data_lens = Vec::new();
    for _ in 0..g1.count {
        block_data_lens.push(g1.data_codewords as usize);
    }
    for _ in 0..g2.count {
        block_data_lens.push(g2.data_codewords as usize);
    }

    // Undo the interleaving (same column-major order the encoder used)
    let mut blocks = alloc::vec![alloc::vec![0u8; 0]; n_blocks];
    for b in blocks.iter_mut() {
        b.reserve(n_ec);
    }
    let mut pos = 0usize;
    let max_data = block_data_lens.iter().copied().max().unwrap_or(0);
    for j in 0..max_data {
        for (b, &len) in blocks.iter_mut().zip(block_data_lens.iter()) {
            if j < len {
                b.push(codewords[pos]);
                pos += 1;
            }
        }
    }
    for _ in 0..n_ec {
        for b in blocks.iter_mut() {
            b.push(codewords[pos]);
            pos += 1;
        }
    }
    debug_assert_eq!(pos, codewords.len());

    // RS-correct each block, then concatenate data sections
    let mut out = alloc::vec![0u8; 0];
    out.reserve(data_len);
    for b in blocks.iter_mut() {
        reed_solomon::decode(b, n_ec).map_err(|_| DecodeError::TooManyErrors)?;
        out.extend_from_slice(&b[..b.len() - n_ec]);
    }
    debug_assert_eq!(out.len(), data_len);
    Ok(out)
}

/// Parse the mode/bit stream of `data` codewords into payload bytes.
pub fn decode_payload(data: &[u8], version: u8) -> Result<Vec<u8>, DecodeError> {
    struct BitReader<'a> {
        data: &'a [u8],
        pos: usize,
    }
    impl<'a> BitReader<'a> {
        fn read(&mut self, n: usize) -> Option<u32> {
            if self.pos + n > self.data.len() * 8 {
                return None;
            }
            let mut v = 0u32;
            for _ in 0..n {
                let bit = (self.data[self.pos / 8] >> (7 - self.pos % 8)) & 1;
                v = (v << 1) | bit as u32;
                self.pos += 1;
            }
            Some(v)
        }
        fn remaining_bits(&self) -> usize {
            self.data.len() * 8 - self.pos.min(self.data.len() * 8)
        }
    }

    let mut r = BitReader { data, pos: 0 };
    let mut out = Vec::new();

    while r.remaining_bits() >= 4 {
        let mode = r.read(4).unwrap();
        if mode == 0 {
            break; // terminator
        }
        // Character-count header: present for the four data modes only.
        // FNC1 first (0101), structured append (0011) and ECI (0111) have
        // their own header shapes handled inside the arms below.
        let has_count = matches!(mode, 0b0001 | 0b0010 | 0b0100 | 0b1000);
        let count = if has_count {
            let ccb = char_count_bits_of(mode as u8, version) as usize;
            match r.read(ccb) {
                Some(c) => c as usize,
                None => return Err(DecodeError::InvalidFormat),
            }
        } else {
            0
        };
        match mode {
            // Numeric
            0b0001 => {
                let mut left = count;
                while left >= 3 {
                    let v = r.read(10).ok_or(DecodeError::InvalidFormat)?;
                    out.extend_from_slice(format!("{:03}", v).as_bytes());
                    left -= 3;
                }
                if left == 2 {
                    let v = r.read(7).ok_or(DecodeError::InvalidFormat)?;
                    out.extend_from_slice(format!("{:02}", v).as_bytes());
                } else if left == 1 {
                    let v = r.read(4).ok_or(DecodeError::InvalidFormat)?;
                    out.push(b'0' + v as u8);
                }
            }
            // Alphanumeric
            0b0010 => {
                let mut left = count;
                while left >= 2 {
                    let v = r.read(11).ok_or(DecodeError::InvalidFormat)?;
                    out.push(ALPHANUMERIC_CHARSET[(v / 45) as usize]);
                    out.push(ALPHANUMERIC_CHARSET[(v % 45) as usize]);
                    left -= 2;
                }
                if left == 1 {
                    let v = r.read(6).ok_or(DecodeError::InvalidFormat)?;
                    out.push(ALPHANUMERIC_CHARSET[v as usize]);
                }
            }
            // Byte
            0b0100 => {
                for _ in 0..count {
                    let v = r.read(8).ok_or(DecodeError::InvalidFormat)?;
                    out.push(v as u8);
                }
            }
            // Kanji (13-bit Shift-JIS, ISO 18004 §8.3.5): the 13-bit value
            // packs the two SJIS bytes as base-192 digits — q = v/192,
            // r = v%192; high = q + base, low = r + 0x40, where base is
            // 0x81 for the first range and 0xC1 for the second.
            0b1000 => {
                for _ in 0..count {
                    let v = r.read(13).ok_or(DecodeError::InvalidFormat)?;
                    let q = v / 0xC0;
                    let rr = v % 0xC0;
                    let sjis = if q + 0x81 <= 0x9F {
                        ((q + 0x81) << 8) | (rr + 0x40)
                    } else {
                        ((q + 0xC1) << 8) | (rr + 0x40)
                    };
                    out.push((sjis >> 8) as u8);
                    out.push((sjis & 0xFF) as u8);
                }
            }
            // FNC1 first position (0101): GS1 payloads — no data bits, the
            // segment that follows carries the Application Identifier data.
            0b0101 => {}
            // ECI (8-bit assignment), then continue with next segment
            0b0111 => {
                if r.read(8).is_none() {
                    return Err(DecodeError::InvalidFormat);
                }
            }
            // FNC1 second position: 8-bit application indicator. FNC1 first
            // position (0101) has no additional header bits.
            0b1001 => {
                if r.read(8).is_none() {
                    return Err(DecodeError::InvalidFormat);
                }
            }
            _ => return Err(DecodeError::InvalidFormat),
        }
    }
    Ok(out)
}

//! PDF417 decoding: row parsing, codeword lookup, row-indicator decoding,
//! GF(929) error correction, and byte-compaction payload recovery.

use alloc::vec::Vec;

use tpt_barcode_core::traits::DecodeError;

use super::gf929::rs_decode;
use super::tables::{CODEWORD_TABLE, START_PATTERN, STOP_PATTERN};

/// Symbol geometry extracted from the row indicators.
struct Geometry {
    rows: usize,
    cols: usize,
    ec_level: u8,
}

/// Decode a PDF417 symbol from a flat row-major module matrix.
///
/// `matrix` is `width × height` with 0 = light, 1 = dark, where
/// `width = 17·cols + 69` and `height` is the row count.
pub fn decode_grid(matrix: &[u8], width: usize, height: usize) -> Result<Vec<u8>, DecodeError> {
    if width < 17 + 17 + 17 + 17 + 18 || matrix.len() != width * height || height == 0 {
        return Err(DecodeError::InvalidFormat);
    }
    if (width - 69) % 17 != 0 {
        return Err(DecodeError::InvalidFormat);
    }
    let cols = (width - 69) / 17;
    if cols == 0 || cols > 30 {
        return Err(DecodeError::InvalidFormat);
    }

    // Parse every row: verify start/stop patterns, read indicator + data
    // codewords by cluster.
    let mut codewords = Vec::with_capacity(cols * height);
    for y in 0..height {
        let cluster = y % 3;
        let row = &matrix[y * width..(y + 1) * width];

        let start = slice_mask(row, 0, 17);
        if start != START_PATTERN {
            return Err(DecodeError::InvalidFormat);
        }
        let stop = slice_mask(row, width - 18, 18);
        if stop != STOP_PATTERN {
            return Err(DecodeError::InvalidFormat);
        }

        let left = lookup(cluster, slice_mask(row, 17, 17))?;
        let right = lookup(cluster, slice_mask(row, 17 + 17 + cols * 17, 17))?;
        codewords.push(left);
        for i in 0..cols {
            codewords.push(lookup(cluster, slice_mask(row, 34 + i * 17, 17))?);
        }
        codewords.push(right);
    }

    let geo = decode_geometry(&codewords, cols, height)?;

    // Flatten data codewords (drop the left/right indicators interleaved per
    // row): layout is [start][left][d0..d_{c-1}][right][stop] per row.
    let mut data = Vec::with_capacity(geo.rows * geo.cols);
    for y in 0..geo.rows {
        let base = y * (cols + 2);
        data.extend_from_slice(&codewords[base + 1..base + 1 + cols]);
    }
    if data.len() != geo.rows * geo.cols {
        return Err(DecodeError::InvalidFormat);
    }

    // Error correction: last k codewords are EC
    let k = 1usize << (geo.ec_level + 1);
    if data.len() < k + 1 {
        return Err(DecodeError::InvalidFormat);
    }
    rs_decode(&mut data, k).map_err(|_| DecodeError::TooManyErrors)?;

    // Strip the symbol length descriptor and trailing PAD codewords
    let payload = &data[1..data.len() - k];
    let end = payload
        .iter()
        .rposition(|&cw| cw != 900)
        .map_or(0, |p| p + 1);
    decode_byte_compaction(&payload[..end])
}

/// Read `len` modules starting at `x` as an MSB-first bitmask.
fn slice_mask(row: &[u8], x: usize, len: usize) -> u32 {
    let mut mask = 0u32;
    for i in 0..len {
        mask = (mask << 1) | (row[x + i] & 1) as u32;
    }
    mask
}

/// Reverse-lookup a 17-module pattern within a cluster.
pub(crate) fn lookup(cluster: usize, mask: u32) -> Result<u16, DecodeError> {
    CODEWORD_TABLE[cluster]
        .iter()
        .position(|&m| m == mask)
        .map(|p| p as u16)
        .ok_or(DecodeError::InvalidFormat)
}

/// Decode total rows / columns / EC level from the per-row indicators and
/// cross-check consistency.
fn decode_geometry(codewords: &[u16], cols: usize, rows: usize) -> Result<Geometry, DecodeError> {
    let stride = cols + 2;
    let mut ec_level: Option<u8> = None;

    for y in 0..rows {
        let cluster = y % 3;
        let left = codewords[y * stride];
        let right = codewords[y * stride + cols + 1];
        let base = (30 * (y / 3)) as u16;
        let l = left.checked_sub(base).ok_or(DecodeError::InvalidFormat)? as usize;
        let r = right.checked_sub(base).ok_or(DecodeError::InvalidFormat)? as usize;

        match cluster {
            0 => {
                // left = (rows−1)/3; right = (cols−1)
                if l != (rows - 1) / 3 || r + 1 != cols {
                    return Err(DecodeError::InvalidFormat);
                }
            }
            1 => {
                // left = level·3 + (rows−1)%3; right = (rows−1)/3
                if l < 3 {
                    return Err(DecodeError::InvalidFormat);
                }
                let level = (l / 3) as u8;
                if level > 8 || l % 3 != (rows - 1) % 3 || r != (rows - 1) / 3 {
                    return Err(DecodeError::InvalidFormat);
                }
                ec_level = Some(level);
            }
            _ => {
                // left = (cols−1); right = level·3 + (rows−1)%3
                if l + 1 != cols || r < 3 {
                    return Err(DecodeError::InvalidFormat);
                }
                let level = (r / 3) as u8;
                if level > 8 || r % 3 != (rows - 1) % 3 {
                    return Err(DecodeError::InvalidFormat);
                }
                ec_level = Some(level);
            }
        }
    }

    let ec_level = ec_level.ok_or(DecodeError::InvalidFormat)?;
    Ok(Geometry {
        rows,
        cols,
        ec_level,
    })
}

/// Decode a byte-compaction codeword stream (latch already consumed by the
/// caller's data section): 5-codeword groups → 6 bytes, tail bytes direct.
/// Encounters of mode latches / PAD terminate the payload.
fn decode_byte_compaction(codewords: &[u16]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut latch: Option<u16> = None;

    while i < codewords.len() {
        let cw = codewords[i];
        if cw == 901 || cw == 924 {
            latch = Some(cw);
            i += 1;
            continue;
        }
        if cw >= 900 {
            break; // PAD or a mode we do not decode
        }
        match latch {
            None => {
                // Payload began without a byte latch — invalid for our encoder
                return Err(DecodeError::InvalidFormat);
            }
            Some(mode) => {
                let remaining = codewords[i..].iter().take_while(|&&c| c < 900).count();
                // Latch 924 promises whole sixpacks; 901 carries a trailing
                // partial group of 1..5 bytes — its size falls out of the
                // codeword count modulo 5.
                let tail = if mode == 901 {
                    ((remaining - 1) % 5) + 1
                } else {
                    0
                };
                let groups = (remaining - tail) / 5;
                for _ in 0..groups {
                    let mut t: u64 = 0;
                    for cw in &codewords[i..i + 5] {
                        t = t * 900 + *cw as u64;
                    }
                    let mut six = [0u8; 6];
                    for slot in six.iter_mut().rev() {
                        *slot = (t & 0xFF) as u8;
                        t >>= 8;
                    }
                    out.extend_from_slice(&six);
                    i += 5;
                }
                for cw in &codewords[i..i + tail] {
                    out.push(*cw as u8);
                }
                i += tail;
            }
        }
    }
    Ok(out)
}

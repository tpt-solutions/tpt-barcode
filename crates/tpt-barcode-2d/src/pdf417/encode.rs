//! PDF417 encoding: byte compaction, symbol length descriptor and padding,
//! GF(929) error correction, row/column layout, and matrix rendering.

use alloc::vec::Vec;

use tpt_barcode_core::traits::EncodeError;

use super::gf929::rs_encode;
use super::tables::{CODEWORD_TABLE, START_PATTERN, STOP_PATTERN};
use super::EcLevel;

/// Recommended minimum EC level for `n` data codewords (ISO/IEC 15438 Annex E).
pub fn recommended_ec_level(n: usize) -> u8 {
    match n {
        0..=40 => 2,
        41..=160 => 3,
        161..=320 => 4,
        _ => 5,
    }
}

/// Encode the payload with byte compaction (ISO/IEC 15438 §4.4.3).
///
/// The whole payload is one byte-compaction segment: latch 924 when the byte
/// count is a multiple of 6, 901 otherwise; 6-byte groups pack into five
/// base-900 codewords, a trailing partial group emits one codeword per byte.
pub fn encode_byte_compaction(data: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(data.len() * 2);
    if data.len() % 6 == 0 {
        out.push(924);
    } else {
        out.push(901);
    }

    let mut i = 0usize;
    while data.len() - i >= 6 {
        let mut t: u64 = 0;
        for b in &data[i..i + 6] {
            t = (t << 8) | *b as u64;
        }
        let mut group = [0u16; 5];
        for slot in group.iter_mut().rev() {
            *slot = (t % 900) as u16;
            t /= 900;
        }
        out.extend_from_slice(&group);
        i += 6;
    }
    // Trailing partial group: one codeword per byte
    for b in &data[i..] {
        out.push(*b as u16);
    }
    out
}

/// Number of rows for `m` source codewords, `k` EC codewords, `c` columns
/// (ISO/IEC 15438 §4.9.1).
fn calculate_rows(m: usize, k: usize, c: usize) -> usize {
    let mut r = (m + 1 + k) / c + 1;
    if c * r >= m + 1 + k + c {
        r -= 1;
    }
    r
}

/// Number of pad codewords (ISO/IEC 15438 §4.9.2).
fn pad_count(m: usize, k: usize, c: usize, r: usize) -> usize {
    let n = c * r - k;
    if n > m + 1 {
        n - m - 1
    } else {
        0
    }
}

/// Choose (columns, rows) for `m` source codewords and `k` EC codewords.
///
/// The chosen grid must fill exactly: `c·r = (m + 1 + pads) + k`, with the
/// symbol length descriptor accounting for the +1.
pub(crate) fn determine_dimensions(m: usize, k: usize) -> Option<(usize, usize)> {
    for c in 1..=30usize {
        let r = calculate_rows(m, k, c);
        if !(3..=90).contains(&r) {
            continue;
        }
        // Grid must hold the SLD + source + pads + EC; the pad rule then
        // guarantees an exact fill
        let n_grid = c * r;
        if n_grid < m + 1 + k || n_grid - k > 928 {
            continue;
        }
        return Some((c, r));
    }
    None
}

/// Encode `data` (byte compaction) at `ec` level into a full codeword stream:
/// symbol length descriptor, compacted data, pads, then EC codewords.
pub fn encode_codewords_byte(data: &[u8], ec: EcLevel) -> Result<Vec<u16>, EncodeError> {
    let level = ec.0;
    if level > 8 {
        return Err(EncodeError::Unsupported);
    }
    let k = 1usize << (level + 1);

    let compacted = encode_byte_compaction(data);
    let m = compacted.len();

    let (c, r) = determine_dimensions(m, k).ok_or(EncodeError::DataTooLong)?;
    let pads = pad_count(m, k, c, r);

    let n = m + pads + 1; // symbol length descriptor value
    if n > 928 {
        return Err(EncodeError::DataTooLong);
    }

    let mut codewords = Vec::with_capacity(n + k);
    codewords.push(n as u16);
    codewords.extend_from_slice(&compacted);
    codewords.resize(n, 900); // PAD

    let mut ecw = alloc::vec![0u16; k];
    rs_encode(&codewords, k, &mut ecw);
    codewords.extend_from_slice(&ecw);

    Ok(codewords)
}

/// Write a pattern bitmask (`len` modules, MSB first) at `(x, y)`.
fn write_pattern(matrix: &mut [u8], width: usize, y: usize, x: usize, mask: u32, len: usize) {
    for i in 0..len {
        let bit = ((mask >> (len - 1 - i)) & 1) as u8;
        matrix[y * width + x + i] = bit;
    }
}

/// Row-indicator codeword values for row `y` of `rows` rows, `cols` columns.
fn row_indicators(y: usize, rows: usize, cols: usize, level: u8) -> (u16, u16) {
    let cluster = y % 3;
    let base = 30 * (y / 3);
    match cluster {
        0 => ((base + (rows - 1) / 3) as u16, (base + (cols - 1)) as u16),
        1 => (
            (base + level as usize * 3 + (rows - 1) % 3) as u16,
            (base + (rows - 1) / 3) as u16,
        ),
        _ => (
            (base + (cols - 1)) as u16,
            (base + level as usize * 3 + (rows - 1) % 3) as u16,
        ),
    }
}

/// Render `codewords` (data + EC, `rows × cols`) into a module matrix of
/// `width = 17·cols + 69` by `height = rows` modules.
pub fn render(codewords: &[u16], cols: usize, rows: usize, level: u8) -> Vec<u8> {
    let width = 17 * cols + 69;
    let mut matrix = alloc::vec![0u8; width * rows];

    for y in 0..rows {
        let cluster = y % 3;
        write_pattern(&mut matrix, width, y, 0, START_PATTERN, 17);

        let (left, right) = row_indicators(y, rows, cols, level);
        write_pattern(
            &mut matrix,
            width,
            y,
            17,
            CODEWORD_TABLE[cluster][left as usize],
            17,
        );
        for i in 0..cols {
            let cw = codewords[y * cols + i] as usize;
            write_pattern(
                &mut matrix,
                width,
                y,
                34 + i * 17,
                CODEWORD_TABLE[cluster][cw],
                17,
            );
        }
        write_pattern(
            &mut matrix,
            width,
            y,
            34 + cols * 17,
            CODEWORD_TABLE[cluster][right as usize],
            17,
        );
        write_pattern(&mut matrix, width, y, 51 + cols * 17, STOP_PATTERN, 18);
    }
    matrix
}

/// Encode `data` (text compaction) at `ec` level into a full codeword stream.
pub fn encode_codewords_text(data: &[u8], ec: EcLevel) -> Result<Vec<u16>, EncodeError> {
    let level = ec.0;
    if level > 8 {
        return Err(EncodeError::Unsupported);
    }
    let k = 1usize << (level + 1);
    let compacted = super::text::encode_text(data).map_err(|_| EncodeError::InvalidCharacter)?;
    let m = compacted.len();
    let (c, r) = determine_dimensions(m, k).ok_or(EncodeError::DataTooLong)?;
    let pads = pad_count(m, k, c, r);
    let n = m + pads + 1;
    if n > 928 {
        return Err(EncodeError::DataTooLong);
    }
    let mut codewords = Vec::with_capacity(n + k);
    codewords.push(n as u16);
    codewords.extend_from_slice(&compacted);
    codewords.resize(n, 900);
    let mut ecw = alloc::vec![0u16; k];
    rs_encode(&codewords, k, &mut ecw);
    codewords.extend_from_slice(&ecw);
    Ok(codewords)
}

/// Encode `data` (numeric compaction) at `ec` level into a full codeword stream.
pub fn encode_codewords_numeric(data: &[u8], ec: EcLevel) -> Result<Vec<u16>, EncodeError> {
    let level = ec.0;
    if level > 8 {
        return Err(EncodeError::Unsupported);
    }
    let k = 1usize << (level + 1);
    let compacted =
        super::numeric::encode_numeric(data).map_err(|_| EncodeError::InvalidCharacter)?;
    let m = compacted.len();
    let (c, r) = determine_dimensions(m, k).ok_or(EncodeError::DataTooLong)?;
    let pads = pad_count(m, k, c, r);
    let n = m + pads + 1;
    if n > 928 {
        return Err(EncodeError::DataTooLong);
    }
    let mut codewords = Vec::with_capacity(n + k);
    codewords.push(n as u16);
    codewords.extend_from_slice(&compacted);
    codewords.resize(n, 900);
    let mut ecw = alloc::vec![0u16; k];
    rs_encode(&codewords, k, &mut ecw);
    codewords.extend_from_slice(&ecw);
    Ok(codewords)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_compaction_sixpack() {
        // 6 bytes → 5 codewords
        let cw = encode_byte_compaction(b"ABCDEF");
        assert_eq!(cw[0], 924); // multiple of 6 → 924 latch
        assert_eq!(cw.len(), 6); // latch + 5 codewords
        let mut t: u64 = 0;
        for b in b"ABCDEF" {
            t = (t << 8) | *b as u64;
        }
        for slot in cw[1..].iter().rev() {
            assert_eq!(*slot, (t % 900) as u16);
            t /= 900;
        }
    }

    #[test]
    fn byte_compaction_partial_tail() {
        // 8 bytes: 901 latch + 5 (one sixpack) + 2 tail codewords
        let cw = encode_byte_compaction(b"ABCDEFGH");
        assert_eq!(cw[0], 901);
        assert_eq!(cw.len(), 1 + 5 + 2);
        assert_eq!(&cw[6..], &b"GH".map(u16::from)[..]);
    }

    #[test]
    fn rows_formula_matches_reference() {
        // ISO §4.9.1 example semantics: enough cells, minimal overshoot
        assert_eq!(calculate_rows(5, 8, 3), 5);
    }
}

//! Data Matrix ECC 200 decoding: finder validation, inverse placement,
//! Reed-Solomon error correction, and ASCII decodation.

use alloc::vec::Vec;

use tpt_barcode_core::gf256::DmField;
use tpt_barcode_core::reed_solomon::decode_with;
use tpt_barcode_core::traits::DecodeError;

use super::placement::{for_each_placed_bit, Placement, FIXED, UNSET};
use super::DmSize;

/// Decode a Data Matrix symbol from a flat row-major module array
/// (`size × size`, 0 = light, 1 = dark).
pub fn decode_grid(matrix: &[u8], size: usize) -> Result<Vec<u8>, DecodeError> {
    let dm_size = DmSize::ALL
        .iter()
        .copied()
        .find(|s| s.modules() == size)
        .ok_or(DecodeError::InvalidFormat)?;

    if matrix.len() != size * size {
        return Err(DecodeError::InvalidFormat);
    }

    validate_finders(matrix, size)?;

    // Re-run the placement to obtain the codeword-bit ↔ cell mapping, then
    // read each codeword's bits back out of the module grid.
    let side = size - 2;
    let n_cw = dm_size.data_codewords() + dm_size.ec_codewords();
    let placement = Placement::place(&alloc::vec![0u8; n_cw], side, side);

    // Map placement cells to (codeword, bit) by replaying the walk with the
    // same occupancy dynamic (an empty grid that fills as bits land).
    let owner = core::cell::RefCell::new(alloc::vec![None; side * side]);
    let occupied = |r: isize, c: isize| owner.borrow()[r as usize * side + c as usize].is_some();
    for_each_placed_bit(n_cw, side, side, &occupied, &mut |pos, bit, r, c| {
        let idx = r as usize * side + c as usize;
        let mut o = owner.borrow_mut();
        if o[idx].is_none() {
            o[idx] = Some((pos, bit));
        }
    });
    let cell_owner = owner.into_inner();

    let at = |r: usize, c: usize| matrix[(r + 1) * size + (c + 1)];

    let mut codewords = alloc::vec![0u8; n_cw];
    for r in 0..side {
        for c in 0..side {
            // UNSET cells are placement gaps (no data bit); FIXED cells are
            // the dark corner pattern. Neither carries payload.
            let cell = placement.bits[r * side + c];
            if cell == UNSET || cell == FIXED {
                continue;
            }
            let (pos, bit) = cell_owner[r * side + c].ok_or(DecodeError::InvalidFormat)?;
            codewords[pos] |= (at(r, c) & 1) << (8 - bit);
        }
    }

    // RS error correction (ECC 200: 0x12D field, syndromes start at α^1)
    let n_ec = dm_size.ec_codewords();
    decode_with::<DmField>(&mut codewords, n_ec, 1).map_err(|_| DecodeError::TooManyErrors)?;

    codewords.truncate(dm_size.data_codewords());
    decode_ascii(&codewords)
}

/// Verify the solid L-finder and the alternating timing patterns.
fn validate_finders(matrix: &[u8], size: usize) -> Result<(), DecodeError> {
    let at = |r: usize, c: usize| matrix[r * size + c];
    // Left column + bottom row solid
    for r in 0..size {
        if at(r, 0) != 1 {
            return Err(DecodeError::InvalidFormat);
        }
    }
    for c in 0..size {
        if at(size - 1, c) != 1 {
            return Err(DecodeError::InvalidFormat);
        }
    }
    // Top row alternates dark on even columns; right column alternates dark
    // on odd rows (the top-right corner module is light)
    for c in 0..size {
        let want = if c % 2 == 0 { 1 } else { 0 };
        if at(0, c) != want {
            return Err(DecodeError::InvalidFormat);
        }
    }
    for r in 0..size {
        let want = if r % 2 == 1 { 1 } else { 0 };
        if at(r, size - 1) != want {
            return Err(DecodeError::InvalidFormat);
        }
    }
    Ok(())
}

/// ASCII decodation of error-corrected data codewords.
fn decode_ascii(codewords: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < codewords.len() {
        let cw = codewords[i];
        match cw {
            1..=128 => out.push(cw - 1),
            130..=229 => {
                let v = cw as usize - 130;
                out.push(b'0' + (v / 10) as u8);
                out.push(b'0' + (v % 10) as u8);
            }
            235 => {
                i += 1;
                if i >= codewords.len() {
                    return Err(DecodeError::InvalidFormat);
                }
                out.push(codewords[i].wrapping_add(127));
            }
            // 129 = pad, 230+ = C40/Text/X12/Edifact latches — unsupported
            // encodations terminate the payload here.
            _ => break,
        }
        i += 1;
    }
    Ok(out)
}

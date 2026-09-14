//! ECC 200 symbol character placement (ISO/IEC 16022 Annex M).
//!
//! Places the 8 bits of each codeword into the (nrows × ncols) data region in
//! the standard diagonal zigzag with "utah" bit shapes and the four special
//! corner patterns. Cells that end up untouched by the walk are fixed pattern
//! modules, not data.
//!
//! The walk itself is exposed through [`for_each_placed_bit`] so the decoder
//! can replay it and recover the codeword-bit ↔ cell mapping.

use alloc::vec::Vec;

/// Cell state in the placement grid: unset, a data bit, or a fixed pattern.
pub const UNSET: i8 = -1;
/// Fixed-pattern cell (always participates as a placed module, carries no data).
pub const FIXED: i8 = 2;

/// A placement map for one symbol size.
pub struct Placement {
    /// Row-major `ncols × nrows` grid of [`UNSET`] / bit value / [`FIXED`].
    pub bits: Vec<i8>,
    #[allow(dead_code)] // kept for symmetry/debuggability of the map
    pub ncols: usize,
    #[allow(dead_code)]
    pub nrows: usize,
}

impl Placement {
    /// Run the Annex M placement walk over `codewords`.
    #[cfg(feature = "alloc")]
    pub fn place(codewords: &[u8], ncols: usize, nrows: usize) -> Placement {
        let bits = core::cell::RefCell::new(alloc::vec![UNSET; ncols * nrows]);
        let is_set = |r: isize, c: isize| bits.borrow()[r as usize * ncols + c as usize] != UNSET;
        for_each_placed_bit(
            codewords.len(),
            ncols,
            nrows,
            &is_set,
            &mut |pos, bit, row, col| {
                let idx = row as usize * ncols + col as usize;
                let value = ((codewords[pos] >> (8 - bit)) & 1) as i8;
                let mut b = bits.borrow_mut();
                if b[idx] == UNSET {
                    b[idx] = value;
                }
            },
        );
        let mut bits = bits.into_inner();

        // Lastly, if the lower right-hand corner is untouched, it is a fixed
        // pattern module (not data)
        let last = (nrows - 1) * ncols + (ncols - 1);
        if bits[last] == UNSET {
            bits[last] = FIXED;
            bits[last - ncols - 1] = FIXED;
        }

        Placement { bits, ncols, nrows }
    }
}

/// Replay the Annex M walk, invoking `visit(pos, bit, row, col)` for every
/// codeword bit at its final (wrap-adjusted) in-bounds cell, in placement
/// order. `pos` runs over `0..n_codewords` and `bit` over `1..=8`.
///
/// `is_set` reports whether a sweep-anchor cell has already received a bit
/// (ISO: "noBit"); anchors that are set skip their whole utah shape. Corners
/// fire unconditionally on their geometric conditions.
pub fn for_each_placed_bit(
    n_codewords: usize,
    ncols: usize,
    nrows: usize,
    is_set: &dyn Fn(isize, isize) -> bool,
    visit: &mut dyn FnMut(usize, u32, isize, isize),
) {
    let (nc, nr) = (ncols as isize, nrows as isize);
    let mut pos = 0usize;
    let mut row = 4isize;
    let mut col = 0isize;

    // Corner-1 cells: bits 1–3 at the bottom-left of the last row, bits 4–8 up
    // the right edge. (Absolute positions, spelled out per Annex M.)
    macro_rules! place_shape {
        ($shape:expr) => {{
            if pos < n_codewords {
                for (bit, r, c) in $shape {
                    visit_wrapped(pos, bit, r, c, nc, nr, visit);
                }
                pos += 1;
            }
        }};
    }

    loop {
        // Repeatedly first check for one of the special corner cases
        if row == nr && col == 0 {
            place_shape!([
                (1, nr - 1, 0),
                (2, nr - 1, 1),
                (3, nr - 1, 2),
                (4, 0, nc - 2),
                (5, 0, nc - 1),
                (6, 1, nc - 1),
                (7, 2, nc - 1),
                (8, 3, nc - 1),
            ]);
        }
        if row == nr - 2 && col == 0 && ncols % 4 != 0 {
            place_shape!([
                (1, nr - 3, 0),
                (2, nr - 2, 0),
                (3, nr - 1, 0),
                (4, 0, nc - 4),
                (5, 0, nc - 3),
                (6, 0, nc - 2),
                (7, 0, nc - 1),
                (8, 1, nc - 1),
            ]);
        }
        if row == nr - 2 && col == 0 && ncols % 8 == 4 {
            place_shape!([
                (1, nr - 3, 0),
                (2, nr - 2, 0),
                (3, nr - 1, 0),
                (4, 0, nc - 2),
                (5, 0, nc - 1),
                (6, 1, nc - 1),
                (7, 2, nc - 1),
                (8, 3, nc - 1),
            ]);
        }
        if row == nr + 4 && col == 2 && ncols % 8 == 0 {
            place_shape!([
                (1, nr - 1, 0),
                (2, nr - 1, nc - 1),
                (3, 0, nc - 3),
                (4, 0, nc - 2),
                (5, 0, nc - 1),
                (6, 1, nc - 3),
                (7, 1, nc - 2),
                (8, 1, nc - 1),
            ]);
        }

        // Sweep upward diagonally
        loop {
            if row < nr && col >= 0 && !is_set(row, col) && pos < n_codewords {
                for (i, (dr, dc)) in UTAH.iter().enumerate() {
                    visit_wrapped(pos, i as u32 + 1, row + dr, col + dc, nc, nr, visit);
                }
                pos += 1;
            }
            row -= 2;
            col += 2;
            if !(row >= 0 && col < nc) {
                break;
            }
        }
        row += 1;
        col += 3;

        // Sweep downward diagonally
        loop {
            if row >= 0 && col < nc && !is_set(row, col) && pos < n_codewords {
                for (i, (dr, dc)) in UTAH.iter().enumerate() {
                    visit_wrapped(pos, i as u32 + 1, row + dr, col + dc, nc, nr, visit);
                }
                pos += 1;
            }
            row += 2;
            col -= 2;
            if !(row < nr && col >= 0) {
                break;
            }
        }
        row += 3;
        col += 1;

        // …until the entire array is scanned
        if !(row < nr || col < nc) {
            break;
        }
    }
}

/// The utah shape: bit b anchors at (row + dr, col + dc) for b = 1..=8.
const UTAH: [(isize, isize); 8] = [
    (-2, -2),
    (-2, -1),
    (-1, -2),
    (-1, -1),
    (-1, 0),
    (0, -2),
    (0, -1),
    (0, 0),
];

/// Apply the Annex M wrap when a shape pokes out of the low edges.
fn visit_wrapped(
    pos: usize,
    bit: u32,
    mut row: isize,
    mut col: isize,
    nc: isize,
    nr: isize,
    visit: &mut dyn FnMut(usize, u32, isize, isize),
) {
    if row < 0 {
        row += nr;
        col += 4 - (nr + 4) % 8;
    }
    if col < 0 {
        col += nc;
        row += 4 - (nc + 4) % 8;
    }
    visit(pos, bit, row, col);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamatrix::DmSize;

    #[test]
    fn placement_covers_all_data_cells() {
        // Every codeword bit must land in a distinct cell; only the (rare)
        // fixed-pattern cells stay data-free.
        for size in DmSize::ALL {
            let n_cw = size.data_codewords() + size.ec_codewords();
            let codewords = alloc::vec![0xA5u8; n_cw];
            let side = size.modules() - 2;
            let p = Placement::place(&codewords, side, side);

            let data_cells = p.bits.iter().filter(|&&b| b != UNSET && b != FIXED).count();
            assert_eq!(
                data_cells,
                n_cw * 8,
                "size {size:?}: data cells must hold exactly all codeword bits"
            );
            // Some sizes leave a few placement-gap cells (no data bit); they
            // render light and are skipped by the decoder's placement map.
            let gaps = p.bits.iter().filter(|&&b| b == UNSET).count();
            assert!(gaps < 8, "size {size:?}: excessive placement gaps ({gaps})");
        }
    }

    #[test]
    fn walk_replay_matches_place() {
        // Replaying the walk with an identical occupancy dynamic (empty grid,
        // cells fill as they are written) must reproduce the same decisions —
        // the decoder relies on this to recover the bit ↔ cell mapping.
        for size in DmSize::ALL {
            let n_cw = size.data_codewords() + size.ec_codewords();
            let side = size.modules() - 2;

            let reference = Placement::place(&alloc::vec![0xA5u8; n_cw], side, side);

            let owner = core::cell::RefCell::new(alloc::vec![None; side * side]);
            let occupied =
                |r: isize, c: isize| owner.borrow()[r as usize * side + c as usize].is_some();
            for_each_placed_bit(n_cw, side, side, &occupied, &mut |pos, bit, r, c| {
                let idx = r as usize * side + c as usize;
                let mut o = owner.borrow_mut();
                if o[idx].is_none() {
                    o[idx] = Some((pos, bit));
                }
            });

            let owner = owner.into_inner();
            for r in 0..side {
                for c in 0..side {
                    let cell = reference.bits[r * side + c];
                    match (cell, owner[r * side + c]) {
                        (UNSET, None) | (FIXED, None) => {}
                        (v, Some((_, bit))) => {
                            assert_eq!(v, ((0xA5u16 >> (8 - bit)) & 1) as i8);
                        }
                        (_, None) => {
                            panic!("size {size:?}: replay diverged at ({r},{c})")
                        }
                    }
                }
            }
        }
    }
}

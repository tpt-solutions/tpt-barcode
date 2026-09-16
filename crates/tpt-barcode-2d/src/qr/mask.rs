//! QR Code mask patterns and ISO 18004 penalty rule evaluation.

/// Returns `true` if the module at (row, col) is dark under mask pattern `mask_id`.
#[inline(always)]
pub fn is_masked(mask_id: u8, row: usize, col: usize) -> bool {
    let r = row as u32;
    let c = col as u32;
    match mask_id {
        0 => (r + c) % 2 == 0,
        1 => r % 2 == 0,
        2 => c % 3 == 0,
        3 => (r + c) % 3 == 0,
        4 => (r / 2 + c / 3) % 2 == 0,
        5 => (r * c) % 2 + (r * c) % 3 == 0,
        6 => ((r * c) % 2 + (r * c) % 3) % 2 == 0,
        7 => ((r + c) % 2 + (r * c) % 3) % 2 == 0,
        _ => false,
    }
}

/// Compute the ISO 18004 penalty score for a matrix (all 4 rules).
pub fn penalty(matrix: &[u8], size: usize) -> u32 {
    penalty_rule1(matrix, size)
        + penalty_rule2(matrix, size)
        + penalty_rule3(matrix, size)
        + penalty_rule4(matrix, size)
}

/// Rule 1: 5+ consecutive same-colour modules in a row/column.
pub fn penalty_rule1(matrix: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    for r in 0..size {
        score += run_penalty(&matrix[r * size..(r + 1) * size]);
    }
    for c in 0..size {
        let col = ColIter {
            matrix,
            size,
            col: c,
        };
        score += run_penalty_iter(col.iter());
    }
    score
}

fn run_penalty(row: &[u8]) -> u32 {
    let mut score = 0u32;
    let mut run = 1usize;
    for i in 1..row.len() {
        if row[i] == row[i - 1] {
            run += 1;
        } else {
            if run >= 5 {
                score += (run - 5) as u32 + 3;
            }
            run = 1;
        }
    }
    if run >= 5 {
        score += (run - 5) as u32 + 3;
    }
    score
}

fn run_penalty_iter(mut iter: impl Iterator<Item = u8>) -> u32 {
    let mut score = 0u32;
    let first = match iter.next() {
        Some(v) => v,
        None => return 0,
    };
    let mut prev = first;
    let mut run = 1usize;
    for v in iter {
        if v == prev {
            run += 1;
        } else {
            if run >= 5 {
                score += (run - 5) as u32 + 3;
            }
            run = 1;
            prev = v;
        }
    }
    if run >= 5 {
        score += (run - 5) as u32 + 3;
    }
    score
}

struct ColIter<'a> {
    matrix: &'a [u8],
    size: usize,
    col: usize,
}

impl<'a> ColIter<'a> {
    fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.size).map(|r| self.matrix[r * self.size + self.col])
    }
}

/// Rule 2: 2×2 blocks of same colour.
fn penalty_rule2(matrix: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    for r in 0..size - 1 {
        for c in 0..size - 1 {
            let v = matrix[r * size + c];
            if matrix[r * size + c + 1] == v
                && matrix[(r + 1) * size + c] == v
                && matrix[(r + 1) * size + c + 1] == v
            {
                score += 3;
            }
        }
    }
    score
}

/// Rule 3: Finder-like patterns.
const PATTERN_A: [u8; 11] = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
const PATTERN_B: [u8; 11] = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];

fn penalty_rule3(matrix: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    for r in 0..size {
        for c in 0..size.saturating_sub(10) {
            let row = &matrix[r * size..][..size];
            if matches_pattern(&row[c..c + 11], &PATTERN_A)
                || matches_pattern(&row[c..c + 11], &PATTERN_B)
            {
                score += 40;
            }
        }
    }
    for c in 0..size {
        for r in 0..size.saturating_sub(10) {
            let col: [u8; 11] = core::array::from_fn(|i| matrix[(r + i) * size + c]);
            if matches_pattern(&col, &PATTERN_A) || matches_pattern(&col, &PATTERN_B) {
                score += 40;
            }
        }
    }
    score
}

fn matches_pattern(data: &[u8], pat: &[u8]) -> bool {
    data.iter().zip(pat.iter()).all(|(&d, &p)| d == p)
}

/// Rule 4: Proportion of dark modules.
fn penalty_rule4(matrix: &[u8], size: usize) -> u32 {
    let total = (size * size) as u32;
    let dark = matrix.iter().filter(|&&b| b != 0).count() as u32;
    let pct = dark * 100 / total;
    let prev5 = (pct / 5) * 5;
    let next5 = prev5 + 5;
    let a = ((prev5 as i32 - 50).unsigned_abs() / 5).min((next5 as i32 - 50).unsigned_abs() / 5);
    a * 10
}

/// Choose the best mask (0–7) for a matrix by ISO 18004 penalty evaluation.
///
/// `matrix` must be a flat row-major array of size×size with data already placed
/// (unmasked). `is_function[i]` is `true` for reserved modules (finders, timing,
/// format areas), which are never masked but do participate in penalty scoring.
/// For each candidate the format information of (ec_bits, candidate mask) is
/// written before scoring, so function modules hold their final values — the
/// ISO-conformant evaluation. The winning mask (with its format info) is
/// applied in place and returned.
pub fn select_mask(matrix: &mut [u8], is_function: &[bool], size: usize, ec_bits: u8) -> u8 {
    let pristine = matrix.to_vec();
    let mut best_mask = 0u8;
    let mut best_score = u32::MAX;

    for mask_id in 0u8..8 {
        matrix.copy_from_slice(&pristine);
        super::matrix::apply_mask(matrix, is_function, size, mask_id);
        super::matrix::write_format(matrix, size, ec_bits, mask_id);

        let score = penalty(matrix, size);
        if score < best_score {
            best_score = score;
            best_mask = mask_id;
        }
    }

    // Re-apply the winning mask (with its format info) permanently
    matrix.copy_from_slice(&pristine);
    super::matrix::apply_mask(matrix, is_function, size, best_mask);
    super::matrix::write_format(matrix, size, ec_bits, best_mask);

    best_mask
}

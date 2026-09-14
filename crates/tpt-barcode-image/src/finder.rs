//! QR Code finder pattern detection.
//!
//! Scans rows for the 1:1:3:1:1 dark-light-dark-light-dark bar ratio that
//! identifies a QR finder pattern centre, then cross-checks in columns.
//! Uses `tpt-math-geometry` for distance and ratio verification once
//! candidate centres are collected.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// A candidate finder pattern location with estimated module size.
#[derive(Clone, Copy, Debug)]
pub struct FinderCandidate {
    /// Estimated centre X in image coordinates (pixels).
    pub cx: f32,
    /// Estimated centre Y in image coordinates (pixels).
    pub cy: f32,
    /// Estimated module size (pixels per module).
    pub module_size: f32,
}

/// Scan a binarized image for QR finder pattern candidates.
///
/// `binary` must be a flat row-major `width × height` image where dark pixels
/// are 255 and light pixels are 0.
#[cfg(feature = "alloc")]
pub fn find_candidates(binary: &[u8], width: usize, height: usize) -> Vec<FinderCandidate> {
    debug_assert_eq!(binary.len(), width * height);

    let mut row_hits: Vec<FinderCandidate> = Vec::new();
    for y in 0..height {
        let row = &binary[y * width..(y + 1) * width];
        row_hits.append(&mut scan_row_for_ratio(row, y));
    }

    // Cross-check: a genuine finder produces ratio hits from both horizontal
    // and vertical run scans at (nearly) the same centre; noise patterns in
    // data regions rarely do. Keep row hits that a column hit confirms.
    let mut col_hits: Vec<FinderCandidate> = Vec::new();
    let mut column = alloc::vec::Vec::with_capacity(height);
    for x in 0..width {
        column.clear();
        for y in 0..height {
            column.push(binary[y * width + x]);
        }
        for mut hit in scan_row_for_ratio(&column, x) {
            // Swap the scan axes back: (line index, offset) → (x, y)
            core::mem::swap(&mut hit.cx, &mut hit.cy);
            col_hits.push(hit);
        }
    }

    let confirmed: Vec<FinderCandidate> = row_hits
        .into_iter()
        .filter(|r| {
            col_hits.iter().any(|c| {
                let dx = r.cx - c.cx;
                let dy = r.cy - c.cy;
                let tol = 2.0 * r.module_size;
                dx * dx + dy * dy < tol * tol
            })
        })
        .collect();

    // Merge nearby candidates (within 3× module size of each other)
    merge_candidates(confirmed)
}

/// Scan a single row for 1:1:3:1:1 patterns. Returns centre candidates.
#[allow(clippy::needless_range_loop)] // run-length state machine indexes by pixel offset
fn scan_row_for_ratio(row: &[u8], y: usize) -> alloc::vec::Vec<FinderCandidate> {
    // State machine: track 5 consecutive run lengths [d, l, d, l, d]
    let mut runs = [0usize; 5];
    let mut run_idx = 0usize;
    let mut is_dark = row[0] > 128;
    let mut run_len = 1usize;
    let mut candidates = alloc::vec::Vec::new();

    for x in 1..row.len() {
        let dark = row[x] > 128;
        if dark == is_dark {
            run_len += 1;
        } else {
            runs[run_idx % 5] = run_len;
            run_idx += 1;

            if run_idx >= 5 {
                let base = run_idx - 5;
                let r: [usize; 5] = [
                    runs[base % 5],
                    runs[(base + 1) % 5],
                    runs[(base + 2) % 5],
                    runs[(base + 3) % 5],
                    runs[(base + 4) % 5],
                ];
                if is_finder_ratio(&r) && is_dark {
                    // Centre is at x - (r[3] + r[4]) - r[2]/2
                    let total: usize = r.iter().sum();
                    let unit = total as f32 / 7.0;
                    let cx = (x as f32) - (r[3] + r[4]) as f32 - r[2] as f32 * 0.5;
                    candidates.push(FinderCandidate {
                        cx,
                        cy: y as f32,
                        module_size: unit,
                    });
                }
            }

            run_len = 1;
            is_dark = dark;
        }
    }
    candidates
}

/// Check whether run lengths approximately satisfy the 1:1:3:1:1 ratio.
fn is_finder_ratio(r: &[usize; 5]) -> bool {
    let total: usize = r.iter().sum();
    if total < 7 {
        return false;
    }
    let unit = total as f32 / 7.0;
    let tolerance = unit * 0.5;

    let check = |actual: usize, expected_units: f32| -> bool {
        let expected = expected_units * unit;
        (actual as f32 - expected).abs() <= tolerance
    };

    check(r[0], 1.0) && check(r[1], 1.0) && check(r[2], 3.0) && check(r[3], 1.0) && check(r[4], 1.0)
}

/// Merge candidates within 3× module size of each other, keeping the centroid.
#[cfg(feature = "alloc")]
fn merge_candidates(mut candidates: Vec<FinderCandidate>) -> Vec<FinderCandidate> {
    #[derive(Clone, Copy)]
    struct Acc {
        cx: f32,
        cy: f32,
        module_size: f32,
        count: f32,
    }

    let mut merged: Vec<Acc> = Vec::new();

    'outer: for cand in candidates.drain(..) {
        for existing in &mut merged {
            let dx = cand.cx - existing.cx;
            let dy = cand.cy - existing.cy;
            let radius = existing.module_size * 3.0;
            if dx * dx + dy * dy < radius * radius {
                // Running mean over all members (order-independent centroid)
                let n = existing.count;
                existing.cx = (existing.cx * n + cand.cx) / (n + 1.0);
                existing.cy = (existing.cy * n + cand.cy) / (n + 1.0);
                existing.module_size = (existing.module_size * n + cand.module_size) / (n + 1.0);
                existing.count = n + 1.0;
                continue 'outer;
            }
        }
        merged.push(Acc {
            cx: cand.cx,
            cy: cand.cy,
            module_size: cand.module_size,
            count: 1.0,
        });
    }

    merged
        .into_iter()
        .map(|a| FinderCandidate {
            cx: a.cx,
            cy: a.cy,
            module_size: a.module_size,
        })
        .collect()
}

/// From a list of merged candidates, select the best 3 that form a QR Code
/// finder pattern triangle: a near-isosceles right triangle (right angle at the
/// top-left finder, equal legs).
///
/// Returns `Some((top_left, top_right, bottom_left))` or `None`.
#[cfg(feature = "alloc")]
pub fn select_finder_triple(
    candidates: &[FinderCandidate],
) -> Option<(FinderCandidate, FinderCandidate, FinderCandidate)> {
    if candidates.len() < 3 {
        return None;
    }

    // Try all triples; pick the one most like an isosceles right triangle.
    let n = candidates.len();
    let mut best: Option<(usize, usize, usize, f32)> = None;

    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                let pi = candidates[i];
                let pj = candidates[j];
                let pk = candidates[k];

                let score = best_right_angle_score(pi, pj, pk);
                if best.is_none_or(|(_, _, _, s)| score < s) {
                    best = Some((i, j, k, score));
                }
            }
        }
    }

    let (i, j, k, _) = best?;
    let (a, b, c) = (candidates[i], candidates[j], candidates[k]);

    // Identify the right-angle vertex — that is the top-left finder.
    let right_angle_idx = find_right_angle_vertex(a, b, c);
    let (tl, other1, other2) = match right_angle_idx {
        0 => (a, b, c),
        1 => (b, a, c),
        _ => (c, a, b),
    };

    // Decide which of the remaining two is top-right (+x direction from tl)
    // versus bottom-left (+y direction), robust to moderate rotation.
    let tr_bl = assign_tr_bl(tl, other1, other2)?;
    Some((tl, tr_bl.0, tr_bl.1))
}

/// Assign (top_right, bottom_left) given the top-left vertex: the candidate
/// whose leg is more x-dominant (`dx² − dy²` larger) is top-right. Robust for
/// rotations within ±45° and free of trigonometry (no_std friendly).
#[cfg(feature = "alloc")]
fn assign_tr_bl(
    _tl: FinderCandidate,
    a: FinderCandidate,
    b: FinderCandidate,
) -> Option<(FinderCandidate, FinderCandidate)> {
    let x_dominance = |p: FinderCandidate| {
        let dx = p.cx - _tl.cx;
        let dy = p.cy - _tl.cy;
        dx * dx - dy * dy
    };
    if x_dominance(a) >= x_dominance(b) {
        Some((a, b))
    } else {
        Some((b, a))
    }
}

fn best_right_angle_score(a: FinderCandidate, b: FinderCandidate, c: FinderCandidate) -> f32 {
    let mut best = f32::INFINITY;
    for (vertex, p1, p2) in [(a, b, c), (b, a, c), (c, a, b)] {
        let score = right_angle_deviation(vertex, p1, p2);
        if score == f32::INFINITY {
            continue;
        }
        if score < best {
            best = score;
        }
    }
    best
}

/// Deviation from an isosceles right triangle at `vertex`, computed entirely
/// with squared lengths (no `sqrt`): `cos²` of the vertex angle plus the
/// leg-ratio excess. 0 = perfect isosceles right angle.
fn right_angle_deviation(vertex: FinderCandidate, p1: FinderCandidate, p2: FinderCandidate) -> f32 {
    let v1x = p1.cx - vertex.cx;
    let v1y = p1.cy - vertex.cy;
    let v2x = p2.cx - vertex.cx;
    let v2y = p2.cy - vertex.cy;
    let l1sq = v1x * v1x + v1y * v1y;
    let l2sq = v2x * v2x + v2y * v2y;
    if l1sq < 1e-12 || l2sq < 1e-12 {
        return f32::INFINITY;
    }
    let dot = v1x * v2x + v1y * v2y;
    // cos²θ = dot² / (l1² · l2²): 0 at a right angle, 1 at 0°/180°
    let cos_sq = dot * dot / (l1sq * l2sq);
    // Squared leg ratio ≥ 1: 1 for equal legs
    let ratio_sq = l1sq.max(l2sq) / l1sq.min(l2sq);
    cos_sq + (ratio_sq - 1.0)
}

fn find_right_angle_vertex(a: FinderCandidate, b: FinderCandidate, c: FinderCandidate) -> usize {
    let scores = [
        right_angle_deviation(a, b, c),
        right_angle_deviation(b, a, c),
        right_angle_deviation(c, a, b),
    ];
    scores
        .iter()
        .enumerate()
        .min_by(|x, y| x.1.partial_cmp(y.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_check_valid() {
        assert!(is_finder_ratio(&[1, 1, 3, 1, 1]));
        assert!(is_finder_ratio(&[2, 2, 6, 2, 2]));
    }

    #[test]
    fn ratio_check_invalid() {
        assert!(!is_finder_ratio(&[1, 1, 1, 1, 1]));
        assert!(!is_finder_ratio(&[1, 2, 3, 1, 1]));
    }
}

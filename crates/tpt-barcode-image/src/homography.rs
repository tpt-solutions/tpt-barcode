//! Perspective correction via homography.
//!
//! Constructs a 3×3 homography matrix from 4 source→destination point pairs,
//! inverts it with `tpt_math_linalg_fixed::Matrix3` (closed-form, zero heap
//! allocation), then samples the corrected pixel grid via projective division.

use tpt_math_geometry::Point2;
use tpt_math_linalg_fixed::{Matrix3, Vector3};

/// Compute a 3×3 homography matrix H such that dst_i ≅ H * src_i for 4 point pairs.
///
/// Uses a Direct Linear Transform (DLT) approach solved via the 8-equation linear
/// system in the homography coefficients (h00..h22, with h22=1 normalised).
///
/// Returns `None` if the system is singular (degenerate configuration).
pub fn compute_homography(src: &[Point2<f64>; 4], dst: &[Point2<f64>; 4]) -> Option<Matrix3<f64>> {
    // DLT: for each point pair (x,y) → (x',y'), two equations:
    //   -x*h00 - y*h01 - h02 + x'*x*h20 + x'*y*h21 = -x'
    //    x*h10 + y*h11 + h12 - y'*x*h20 - y'*y*h21 = -y'
    // With h22 = 1 (normalised), 8 unknowns, 8 equations from 4 points.

    let mut a_data = [0f64; 64]; // 8×8 matrix (flat row-major)
    let mut b_data = [0f64; 8];

    for (i, (s, d)) in src.iter().zip(dst.iter()).enumerate() {
        let sx = s.x();
        let sy = s.y();
        let dx = d.x();
        let dy = d.y();

        // Row 2i: x equation
        let r0 = 2 * i;
        a_data[r0 * 8] = -sx;
        a_data[r0 * 8 + 1] = -sy;
        a_data[r0 * 8 + 2] = -1.0;
        a_data[r0 * 8 + 3] = 0.0;
        a_data[r0 * 8 + 4] = 0.0;
        a_data[r0 * 8 + 5] = 0.0;
        a_data[r0 * 8 + 6] = dx * sx;
        a_data[r0 * 8 + 7] = dx * sy;
        b_data[r0] = -dx;

        // Row 2i+1: y equation
        let r1 = 2 * i + 1;
        a_data[r1 * 8] = 0.0;
        a_data[r1 * 8 + 1] = 0.0;
        a_data[r1 * 8 + 2] = 0.0;
        a_data[r1 * 8 + 3] = -sx;
        a_data[r1 * 8 + 4] = -sy;
        a_data[r1 * 8 + 5] = -1.0;
        a_data[r1 * 8 + 6] = dy * sx;
        a_data[r1 * 8 + 7] = dy * sy;
        b_data[r1] = -dy;
    }

    // Solve Ax = b via Gaussian elimination with partial pivoting
    let h = solve_8x8(&a_data, &b_data)?;
    // h = [h00, h01, h02, h10, h11, h12, h20, h21] with h22 = 1

    Some(Matrix3::new([
        [h[0], h[1], h[2]],
        [h[3], h[4], h[5]],
        [h[6], h[7], 1.0],
    ]))
}

/// Invert a homography matrix using `tpt_math_linalg_fixed::Matrix3::inverse`.
///
/// Returns `None` if the matrix is singular.
pub fn invert_homography(h: &Matrix3<f64>) -> Option<Matrix3<f64>> {
    h.inverse()
}

/// Map a destination pixel (col, row) back to source image coordinates using
/// the inverse homography.
///
/// Returns the (x, y) source coordinates, which may be non-integer and outside
/// the source image bounds.
#[inline]
pub fn map_point(h_inv: &Matrix3<f64>, col: f64, row: f64) -> (f64, f64) {
    // Apply homography: [x'; y'; w'] = H_inv * [col; row; 1]
    let p = h_inv.mul_vec(&Vector3::new([col, row, 1.0]));
    let w = p.z();
    debug_assert!(w.abs() > 1e-12, "homography produced a point at infinity");
    (p.x() / w, p.y() / w)
}

/// Sample a perspective-corrected grid from a source image.
///
/// `binary`: flat row-major binarized source image (0 or 255), size `src_w × src_h`.
/// `h_inv`: inverse homography mapping destination pixel → source pixel.
/// `out`: output grid, `dst_w × dst_h` bytes (0 = light module, 1 = dark module).
pub fn sample_grid(
    binary: &[u8],
    src_w: usize,
    src_h: usize,
    h_inv: &Matrix3<f64>,
    out: &mut [u8],
    dst_w: usize,
    dst_h: usize,
) {
    debug_assert_eq!(out.len(), dst_w * dst_h);
    debug_assert_eq!(binary.len(), src_w * src_h);

    for row in 0..dst_h {
        for col in 0..dst_w {
            let (sx, sy) = map_point(h_inv, col as f64 + 0.5, row as f64 + 0.5);
            // Nearest-neighbour sampling (round half away from zero; core-only)
            let ix = round_half_away(sx) as i64;
            let iy = round_half_away(sy) as i64;
            let val = if ix >= 0 && iy >= 0 && (ix as usize) < src_w && (iy as usize) < src_h {
                binary[iy as usize * src_w + ix as usize]
            } else {
                0 // out-of-bounds → light
            };
            out[row * dst_w + col] = if val > 128 { 1 } else { 0 };
        }
    }
}

/// `floor` for f64 without `std` float methods (truncation + compare).
fn floor_f64(x: f64) -> f64 {
    let t = x as i64 as f64;
    if x < t {
        t - 1.0
    } else {
        t
    }
}

/// Bilinear variant of [`sample_grid`]: instead of nearest-neighbour, the
/// four surrounding source pixels are weighted by their distance to the
/// sample point. Reduces aliasing on low-resolution or perspective-skewed
/// images at the cost of four pixel reads per module.
pub fn sample_grid_bilinear(
    binary: &[u8],
    src_w: usize,
    src_h: usize,
    h_inv: &Matrix3<f64>,
    out: &mut [u8],
    dst_w: usize,
    dst_h: usize,
) {
    debug_assert_eq!(out.len(), dst_w * dst_h);
    debug_assert_eq!(binary.len(), src_w * src_h);

    let px = |x: i64, y: i64| -> f64 {
        if x >= 0 && y >= 0 && (x as usize) < src_w && (y as usize) < src_h {
            f64::from(binary[y as usize * src_w + x as usize])
        } else {
            0.0
        }
    };

    for row in 0..dst_h {
        for col in 0..dst_w {
            let (sx, sy) = map_point(h_inv, col as f64 + 0.5, row as f64 + 0.5);
            // Bilerp over the 4 pixels around the (sx-0.5, sy-0.5) sample grid
            let fx = sx - 0.5;
            let fy = sy - 0.5;
            let x0 = floor_f64(fx);
            let y0 = floor_f64(fy);
            let dx = fx - x0;
            let dy = fy - y0;
            let (ix, iy) = (x0 as i64, y0 as i64);
            let v = (1.0 - dx) * (1.0 - dy) * px(ix, iy)
                + dx * (1.0 - dy) * px(ix + 1, iy)
                + (1.0 - dx) * dy * px(ix, iy + 1)
                + dx * dy * px(ix + 1, iy + 1);
            out[row * dst_w + col] = if v > 128.0 { 1 } else { 0 };
        }
    }
}

/// Round half away from zero without `std` float methods. Pixel coordinates
/// are small enough that truncation via integer casts is safe; NaN maps to 0.
fn round_half_away(x: f64) -> f64 {
    let truncated = if x < 0.0 {
        -((-x) as i64 as f64)
    } else {
        x as i64 as f64
    };
    let frac = x - truncated;
    if frac.abs() >= 0.5 {
        truncated + if x < 0.0 { -1.0 } else { 1.0 }
    } else {
        truncated
    }
}

// ── 8×8 Gaussian elimination ─────────────────────────────────────────────────

#[allow(clippy::needless_range_loop)] // pivot/elimination loops are index-centric
/// Solve an 8×8 linear system Ax = b using Gaussian elimination with partial
/// pivoting. Returns the 8-element solution vector, or `None` if singular.
fn solve_8x8(a: &[f64; 64], b: &[f64; 8]) -> Option<[f64; 8]> {
    const N: usize = 8;
    let mut m = [[0f64; N + 1]; N]; // augmented [A | b]
    for i in 0..N {
        for j in 0..N {
            m[i][j] = a[i * N + j];
        }
        m[i][N] = b[i];
    }

    for col in 0..N {
        // Partial pivot
        let mut max_row = col;
        let mut max_val = m[col][col].abs();
        for row in col + 1..N {
            if m[row][col].abs() > max_val {
                max_val = m[row][col].abs();
                max_row = row;
            }
        }
        if max_val < 1e-12 {
            return None; // singular
        }
        m.swap(col, max_row);

        let pivot = m[col][col];
        for row in col + 1..N {
            let factor = m[row][col] / pivot;
            for k in col..=N {
                let v = m[col][k] * factor;
                m[row][k] -= v;
            }
        }
    }

    // Back substitution
    let mut x = [0f64; N];
    for i in (0..N).rev() {
        x[i] = m[i][N];
        for j in i + 1..N {
            x[i] -= m[i][j] * x[j];
        }
        x[i] /= m[i][i];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_math_geometry::Point2;

    fn pt(x: f64, y: f64) -> Point2<f64> {
        Point2::from_array([x, y])
    }

    #[test]
    fn identity_homography() {
        // Map a unit square to itself — homography should be identity
        let src = [pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)];
        let dst = src;
        let h = compute_homography(&src, &dst).unwrap();
        let h_inv = invert_homography(&h).unwrap();
        let (mx, my) = map_point(&h_inv, 0.5, 0.5);
        assert!((mx - 0.5).abs() < 1e-6);
        assert!((my - 0.5).abs() < 1e-6);
    }

    #[test]
    fn inverse_of_identity_is_identity() {
        let id = Matrix3::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        let inv = invert_homography(&id).unwrap();
        for r in 0..3 {
            for c in 0..3 {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((inv.get(r, c) - expected).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn translation_homography() {
        // Translate by (+10, +20)
        let src = [pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)];
        let dst = [
            pt(10.0, 20.0),
            pt(11.0, 20.0),
            pt(11.0, 21.0),
            pt(10.0, 21.0),
        ];
        let h = compute_homography(&src, &dst).unwrap();
        let h_inv = invert_homography(&h).unwrap();
        // Sampling dst pixel (10.5, 20.5) should map back to src (0.5, 0.5)
        let (sx, sy) = map_point(&h_inv, 10.5, 20.5);
        assert!((sx - 0.5).abs() < 1e-5, "sx={sx}");
        assert!((sy - 0.5).abs() < 1e-5, "sy={sy}");
    }
}

#[cfg(test)]
mod bilinear_tests {
    use super::*;
    extern crate alloc;
    use alloc::vec;

    #[test]
    fn bilinear_uniform_interior_matches() {
        // Bilinear interpolation of a uniform image must stay uniform in the
        // interior (edges blend with the out-of-bounds light border, which is
        // expected darkening).
        let w = 40usize;
        let binary = vec![255u8; w * w];
        let id = Matrix3::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        let mut bilerp = vec![0u8; w * w];
        sample_grid_bilinear(&binary, w, w, &id, &mut bilerp, w, w);
        for r in 1..w - 1 {
            for c in 1..w - 1 {
                assert_eq!(bilerp[r * w + c], 1, "interior ({r},{c}) not dark");
            }
        }
    }

    #[test]
    fn bilinear_centre_matches_nearest_midpoints() {
        // A half-dark / half-light image: bilinear must classify the dark
        // half dark and the light half light away from the boundary.
        let w = 40usize;
        let mut binary = vec![0u8; w * w];
        for r in 0..w {
            for c in 0..w / 2 {
                binary[r * w + c] = 255;
            }
        }
        let id = Matrix3::new([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        let mut out = vec![0u8; w * w];
        sample_grid_bilinear(&binary, w, w, &id, &mut out, w, w);
        // column 10 is well inside the dark half; column 30 in the light half
        for r in 1..w - 1 {
            assert_eq!(out[r * w + 10], 1, "row {r} col 10 should be dark");
            assert_eq!(out[r * w + 30], 0, "row {r} col 30 should be light");
        }
    }
}

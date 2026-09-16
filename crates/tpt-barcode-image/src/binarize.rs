//! Adaptive thresholding (Bradley local-mean algorithm).
//!
//! The scalar path is always available. When the `simd` feature is enabled,
//! an accelerated path is compiled for targets that support it.

/// Binarize a grayscale image using Bradley's adaptive local-mean thresholding.
///
/// `pixels` is a flat row-major slice of `width × height` luma values (0–255).
/// `out` receives 0 (light) or 255 (dark) for each pixel.
/// `window` is the half-size of the local averaging window (e.g. `width / 8`).
/// `k` is the sensitivity constant (typical: 0.15).
pub fn binarize_adaptive(
    pixels: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
    window: usize,
    k: f32,
) {
    debug_assert_eq!(pixels.len(), width * height);
    debug_assert_eq!(out.len(), width * height);

    #[cfg(feature = "simd")]
    {
        binarize_simd(pixels, width, height, out, window, k);
    }

    #[cfg(not(feature = "simd"))]
    binarize_scalar(pixels, width, height, out, window, k);
}

/// Pure scalar Bradley adaptive threshold.
fn binarize_scalar(
    pixels: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
    window: usize,
    k: f32,
) {
    // Build integral image for O(1) rectangle sum queries.
    // integral[y * (width+1) + x] = sum of pixels in [0..y)[0..x)
    let iw = width + 1;
    let ih = height + 1;
    let mut integral = alloc_or_stack_integral(iw * ih);

    for y in 1..=height {
        let mut row_sum: u32 = 0;
        for x in 1..=width {
            row_sum += pixels[(y - 1) * width + (x - 1)] as u32;
            integral[y * iw + x] = integral[(y - 1) * iw + x] + row_sum;
        }
    }

    // Compare in the multiply form `p * area <= sum * (1-k)` — the same
    // expression the SIMD path evaluates, keeping both bit-identical.
    let scale = 1.0f32 - k;
    for y in 0..height {
        let y1 = y.saturating_sub(window);
        let y2 = (y + window + 1).min(height);
        for x in 0..width {
            let x1 = x.saturating_sub(window);
            let x2 = (x + window + 1).min(width);

            let area = ((y2 - y1) * (x2 - x1)) as u32;
            // Signed arithmetic: the intermediate terms of the inclusion–
            // exclusion can individually exceed the running difference.
            let sum = (integral[y2 * iw + x2] as i64)
                - (integral[y1 * iw + x2] as i64)
                - (integral[y2 * iw + x1] as i64)
                + (integral[y1 * iw + x1] as i64);
            let lhs = pixels[y * width + x] as f32 * area as f32;
            out[y * width + x] = if lhs <= sum as f32 * scale { 255 } else { 0 };
        }
    }
}

#[cfg(feature = "simd")]
fn binarize_simd(
    pixels: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
    window: usize,
    k: f32,
) {
    // The integral image is inherently sequential, so it stays scalar. The
    // per-pixel threshold/compare stage — the bulk of the arithmetic — is
    // vectorized: per row we precompute `sum(x) * (1-k)` into an f32 buffer
    // (one multiply, no division), then SSE2 processes 16 pixels per
    // iteration comparing `p * area <= sum * (1-k)` in float domain
    // (exact for all u8·u32 products involved). Trailing pixels and the
    // border strips where `area` varies per pixel are handled scalar.
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("sse2") {
            return binarize_simd_sse2(pixels, width, height, out, window, k);
        }
    }
    binarize_scalar(pixels, width, height, out, window, k);
}

/// Build the integral image (shared by scalar and SIMD paths).
#[cfg(feature = "simd")]
fn build_integral(pixels: &[u8], width: usize, height: usize) -> alloc::vec::Vec<u32> {
    let iw = width + 1;
    let mut integral = alloc::vec![0u32; iw * (height + 1)];
    for y in 1..=height {
        let mut row_sum: u32 = 0;
        for x in 1..=width {
            row_sum += pixels[(y - 1) * width + (x - 1)] as u32;
            integral[y * iw + x] = integral[(y - 1) * iw + x] + row_sum;
        }
    }
    integral
}

/// SSE2 thresholding: 16 pixels per iteration on the interior where the window
/// area is constant; scalar for borders and the trailing tail.
#[cfg(all(feature = "simd", target_arch = "x86_64"))]
fn binarize_simd_sse2(
    pixels: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
    window: usize,
    k: f32,
) {
    use core::arch::x86_64::*;

    let iw = width + 1;
    let integral = build_integral(pixels, width, height);
    let scale = 1.0f32 - k;

    // Interior region (x and y) where the window area is the constant (2w+1)²
    let x_lo = window;
    let x_hi = width.saturating_sub(window);
    let y_lo = window;
    let y_hi = height.saturating_sub(window);
    let area = ((2 * window + 1) * (2 * window + 1)) as f32;

    let mut sums = alloc::vec![0f32; width];
    for y in 0..height {
        let y1 = y.saturating_sub(window);
        let y2 = (y + window + 1).min(height);

        // Scalar pass: block sums → scaled threshold buffer
        for (x, slot) in sums.iter_mut().enumerate() {
            let x1 = x.saturating_sub(window);
            let x2 = (x + window + 1).min(width);
            let s = (integral[y2 * iw + x2] as i64)
                - (integral[y1 * iw + x2] as i64)
                - (integral[y2 * iw + x1] as i64)
                + (integral[y1 * iw + x1] as i64);
            *slot = s as f32 * scale;
        }

        let row_px = &pixels[y * width..(y + 1) * width];
        let row_out = &mut out[y * width..(y + 1) * width];

        // Vectorized interior (only for rows whose y-window is unclipped)
        if y < y_lo || y >= y_hi {
            for x in 0..width {
                let x1 = x.saturating_sub(window);
                let x2 = (x + window + 1).min(width);
                let a = ((y2 - y1) * (x2 - x1)) as f32;
                row_out[x] = if row_px[x] as f32 * a <= sums[x] {
                    255
                } else {
                    0
                };
            }
            continue;
        }

        let mut x = x_lo;
        while x + 16 <= x_hi {
            unsafe {
                let p = _mm_loadu_si128(row_px[x..].as_ptr() as *const __m128i);
                // u8 → u32 (two zero-extended unpacks) → f32
                let p16lo = _mm_unpacklo_epi8(p, _mm_setzero_si128());
                let p16hi = _mm_unpackhi_epi8(p, _mm_setzero_si128());
                let p32_0 = _mm_unpacklo_epi16(p16lo, _mm_setzero_si128());
                let p32_1 = _mm_unpackhi_epi16(p16lo, _mm_setzero_si128());
                let p32_2 = _mm_unpacklo_epi16(p16hi, _mm_setzero_si128());
                let p32_3 = _mm_unpackhi_epi16(p16hi, _mm_setzero_si128());
                let pf_0 = _mm_cvtepi32_ps(p32_0);
                let pf_1 = _mm_cvtepi32_ps(p32_1);
                let pf_2 = _mm_cvtepi32_ps(p32_2);
                let pf_3 = _mm_cvtepi32_ps(p32_3);

                let area_v = _mm_set1_ps(area);
                let lhs = _mm_mul_ps(pf_0, area_v);
                let t = _mm_loadu_ps(sums[x..].as_ptr());
                let mask0 = _mm_cmple_ps(lhs, t);
                let lhs = _mm_mul_ps(pf_1, area_v);
                let t = _mm_loadu_ps(sums[x + 4..].as_ptr());
                let mask1 = _mm_cmple_ps(lhs, t);
                let lhs = _mm_mul_ps(pf_2, area_v);
                let t = _mm_loadu_ps(sums[x + 8..].as_ptr());
                let mask2 = _mm_cmple_ps(lhs, t);
                let lhs = _mm_mul_ps(pf_3, area_v);
                let t = _mm_loadu_ps(sums[x + 12..].as_ptr());
                let mask3 = _mm_cmple_ps(lhs, t);

                // Each f32 mask lane is 0xFFFFFFFF (dark) or 0; shift the top
                // byte down to 0x000000FF so the saturating narrow yields a
                // clean 0xFF / 0x00 output byte.
                let dark0 = _mm_srli_epi32(_mm_castps_si128(mask0), 24);
                let dark1 = _mm_srli_epi32(_mm_castps_si128(mask1), 24);
                let dark2 = _mm_srli_epi32(_mm_castps_si128(mask2), 24);
                let dark3 = _mm_srli_epi32(_mm_castps_si128(mask3), 24);

                // Narrow 4×(4×u32) → 16 bytes: i32→i16 saturating, i16→u8 saturating
                let i16_lo = _mm_packs_epi32(dark0, dark1);
                let i16_hi = _mm_packs_epi32(dark2, dark3);
                let bytes = _mm_packus_epi16(i16_lo, i16_hi);
                _mm_storeu_si128(row_out[x..].as_mut_ptr() as *mut __m128i, bytes);
            }
            x += 16;
        }

        // Scalar tail (border strips + trailing < 16 pixels)
        for x in x..x_hi {
            row_out[x] = if row_px[x] as f32 * area <= sums[x] {
                255
            } else {
                0
            };
        }
        for x in 0..x_lo {
            let x1 = x.saturating_sub(window);
            let x2 = (x + window + 1).min(width);
            let a = ((y2 - y1) * (x2 - x1)) as f32;
            row_out[x] = if row_px[x] as f32 * a <= sums[x] {
                255
            } else {
                0
            };
        }
        for x in x_hi..width {
            let x1 = x.saturating_sub(window);
            let x2 = (x + window + 1).min(width);
            let a = ((y2 - y1) * (x2 - x1)) as f32;
            row_out[x] = if row_px[x] as f32 * a <= sums[x] {
                255
            } else {
                0
            };
        }
    }
}

// The integral image needs heap allocation. For `no_alloc` targets a
// caller-supplied-buffer API is the intended design (tracked in todo.md);
// the combination currently fails to compile rather than silently blowing
// a 64 MiB stack array.
#[cfg(feature = "alloc")]
fn alloc_or_stack_integral(len: usize) -> alloc::vec::Vec<u32> {
    alloc::vec![0u32; len]
}

#[cfg(not(feature = "alloc"))]
compile_error!(
    "tpt-barcode-image requires the `alloc` feature (or `std`).      A caller-supplied-buffer binarization API for `no_alloc` targets is tracked in todo.md."
);

/// Global threshold via Otsu's method (maximizes between-class variance).
///
/// Returns a threshold value 0–255; pixels ≤ `t` are treated as dark.
pub fn global_threshold(pixels: &[u8]) -> u8 {
    if pixels.is_empty() {
        return 128;
    }
    let mut hist = [0u32; 256];
    for &p in pixels {
        hist[p as usize] += 1;
    }
    let total = pixels.len() as f64;
    let sum_all: f64 = hist
        .iter()
        .enumerate()
        .map(|(v, &c)| v as f64 * c as f64)
        .sum();

    let mut sum_b = 0f64;
    let mut w_b = 0f64;
    let mut best_var = -1f64;
    let mut best_t = 128u8;
    for (v, &count) in hist.iter().enumerate() {
        w_b += count as f64;
        if w_b <= 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f <= 0.0 {
            break;
        }
        sum_b += v as f64 * count as f64;
        let mean_b = sum_b / w_b;
        let mean_f = (sum_all - sum_b) / w_f;
        let between = w_b * w_f * (mean_b - mean_f) * (mean_b - mean_f);
        if between > best_var {
            best_var = between;
            best_t = v as u8;
        }
    }
    best_t
}

/// Apply a global threshold: pixels ≤ `t` become 255 (dark), others become 0.
pub fn binarize_global(pixels: &[u8], out: &mut [u8], t: u8) {
    for (o, &p) in out.iter_mut().zip(pixels.iter()) {
        *o = if p <= t { 255 } else { 0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_threshold_midpoint() {
        let pixels: [u8; 4] = [0, 100, 155, 255];
        let t = global_threshold(&pixels);
        assert!(t >= 100 && t <= 155);
    }

    #[test]
    fn binarize_global_splits_correctly() {
        let pixels: [u8; 4] = [50, 100, 150, 200];
        let mut out = [0u8; 4];
        binarize_global(&pixels, &mut out, 120);
        assert_eq!(out, [255, 255, 0, 0]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn binarize_adaptive_uniform_dark() {
        // A fully dark (0-value) image should produce all-dark output.
        let pixels = alloc::vec![0u8; 16 * 16];
        let mut out = alloc::vec![0u8; 16 * 16];
        binarize_adaptive(&pixels, 16, 16, &mut out, 4, 0.15);
        assert!(out.iter().all(|&v| v == 255));
    }
}

#[cfg(all(test, feature = "simd", target_arch = "x86_64"))]
mod simd_micro {
    use super::*;

    #[test]
    fn widen_mul_cmp_narrow_matches_scalar() {
        use core::arch::x86_64::*;
        let px: alloc::vec::Vec<u8> = (0..16).map(|i| (i * 17) as u8).collect();
        let sums: alloc::vec::Vec<f32> = (0..16).map(|i| i as f32 * 1000.0).collect();
        let area = 529.0f32;

        let expected: alloc::vec::Vec<u8> = (0..16)
            .map(|i| {
                if px[i] as f32 * area <= sums[i] {
                    255
                } else {
                    0
                }
            })
            .collect();

        let got: alloc::vec::Vec<u8> = unsafe {
            let p = _mm_loadu_si128(px.as_ptr() as *const __m128i);
            let p16lo = _mm_unpacklo_epi8(p, _mm_setzero_si128());
            let p16hi = _mm_unpackhi_epi8(p, _mm_setzero_si128());
            let p32s = [
                _mm_cvtepi32_ps(_mm_unpacklo_epi16(p16lo, _mm_setzero_si128())),
                _mm_cvtepi32_ps(_mm_unpackhi_epi16(p16lo, _mm_setzero_si128())),
                _mm_cvtepi32_ps(_mm_unpacklo_epi16(p16hi, _mm_setzero_si128())),
                _mm_cvtepi32_ps(_mm_unpackhi_epi16(p16hi, _mm_setzero_si128())),
            ];
            let area_v = _mm_set1_ps(area);
            let masks = [
                _mm_cmple_ps(_mm_mul_ps(p32s[0], area_v), _mm_loadu_ps(sums.as_ptr())),
                _mm_cmple_ps(
                    _mm_mul_ps(p32s[1], area_v),
                    _mm_loadu_ps(sums.as_ptr().add(4)),
                ),
                _mm_cmple_ps(
                    _mm_mul_ps(p32s[2], area_v),
                    _mm_loadu_ps(sums.as_ptr().add(8)),
                ),
                _mm_cmple_ps(
                    _mm_mul_ps(p32s[3], area_v),
                    _mm_loadu_ps(sums.as_ptr().add(12)),
                ),
            ];
            // Move each f32 mask's sign byte to the lane's low byte: lane
            // becomes 0x000000FF (dark) or 0. Saturating narrow then maps it
            // to a 0xFF / 0x00 byte. (An AND with 0xFF bytes would leave
            // 0xFFFFFFFF lanes, which saturate to 0x00.)
            let d = [
                _mm_srli_epi32(_mm_castps_si128(masks[0]), 24),
                _mm_srli_epi32(_mm_castps_si128(masks[1]), 24),
                _mm_srli_epi32(_mm_castps_si128(masks[2]), 24),
                _mm_srli_epi32(_mm_castps_si128(masks[3]), 24),
            ];
            let i16_lo = _mm_packs_epi32(d[0], d[1]);
            let i16_hi = _mm_packs_epi32(d[2], d[3]);
            let bytes = _mm_packus_epi16(i16_lo, i16_hi);
            let mut out = [0u8; 16];
            _mm_storeu_si128(out.as_mut_ptr() as *mut __m128i, bytes);
            out.to_vec()
        };

        assert_eq!(expected, got, "widen/mul/cmp/narrow chain diverges");
    }
}

#[cfg(all(test, feature = "simd"))]
mod simd_tests {
    use super::*;

    #[test]
    fn simd_matches_scalar() {
        // Pseudo-random grayscale content, odd size to exercise tails/borders
        let (width, height) = (131, 67);
        let mut seed = 0x12345678u32;
        let pixels: alloc::vec::Vec<u8> = (0..width * height)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();

        let mut scalar = alloc::vec![0u8; pixels.len()];
        binarize_scalar(&pixels, width, height, &mut scalar, 11, 0.15);

        let mut simd = alloc::vec![0u8; pixels.len()];
        binarize_simd(&pixels, width, height, &mut simd, 11, 0.15);

        let diffs: alloc::vec::Vec<usize> = scalar
            .iter()
            .zip(simd.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        if !diffs.is_empty() {
            let lane_hist: alloc::vec::Vec<(usize, usize)> = {
                let mut h = [0usize; 16];
                for &d in &diffs {
                    h[d % width % 16] += 1;
                }
                h.iter()
                    .copied()
                    .enumerate()
                    .filter(|&(_, c)| c > 0)
                    .collect()
            };
            panic!(
                "SIMD mismatch: {} diffs; first idx {} (x={}, y={}) scalar={} simd={}; lane histogram (x%16): {:?}",
                diffs.len(),
                diffs[0],
                diffs[0] % width,
                diffs[0] / width,
                scalar[diffs[0]],
                simd[diffs[0]],
                lane_hist
            );
        }
    }
}

//! NEON binarization stage — compiles only on aarch64.
#![cfg(target_arch = "aarch64")]

//! NEON binarization threshold stage for aarch64 (mirrors the SSE2 path).
//!
//! The integral image is inherently sequential and stays scalar. The
//! per-pixel threshold/compare stage — the bulk of the arithmetic — is
//! vectorized: per row `sum(x) * (1-k)` is precomputed into an f32 buffer
//! (one multiply, no division), then NEON processes 16 pixels per iteration
//! comparing `p * area <= sum * (1-k)` in float domain. Interior rows are
//! vectorized with a constant window area; border strips and rows whose
//! y-window is clipped are handled scalar, keeping results bit-identical
//! with the scalar path.

#[cfg(feature = "alloc")]
pub(super) fn binarize_simd_neon(
    pixels: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
    window: usize,
    k: f32,
) {
    use crate::binarize::build_integral;
    use core::arch::aarch64::*;

    let iw = width + 1;
    let integral = build_integral(pixels, width, height);
    let scale = 1.0f32 - k;

    // Interior region (x and y) where the window area is the constant (2w+1)^2
    let x_lo = window;
    let x_hi = width.saturating_sub(window);
    let y_lo = window;
    let y_hi = height.saturating_sub(window);
    let area = ((2 * window + 1) * (2 * window + 1)) as f32;

    let mut sums = alloc::vec![0f32; width];
    for y in 0..height {
        let y1 = y.saturating_sub(window);
        let y2 = (y + window + 1).min(height);

        // Scalar pass: block sums -> scaled threshold buffer
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

        // Rows outside the constant-area band (or narrow widths) stay scalar
        if y < y_lo || y >= y_hi || x_hi <= x_lo + 16 {
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

        // Vectorized interior: 16 pixels per iteration
        let mut x = x_lo;
        while x + 16 <= x_hi {
            unsafe {
                let p = vld1q_u8(row_px[x..].as_ptr());
                let zero8 = vdupq_n_u8(0);
                let zero16 = vdupq_n_u16(0);

                // widen 16 u8 -> 4x(4 u32) via interleaved zero-extends
                let p16lo = vzip1q_u8(p, zero8);
                let p16hi = vzip2q_u8(p, zero8);
                let p32 = [
                    vreinterpretq_u32_u16(vzip1q_u16(vreinterpretq_u16_u8(p16lo), zero16)),
                    vreinterpretq_u32_u16(vzip2q_u16(vreinterpretq_u16_u8(p16lo), zero16)),
                    vreinterpretq_u32_u16(vzip1q_u16(vreinterpretq_u16_u8(p16hi), zero16)),
                    vreinterpretq_u32_u16(vzip2q_u16(vreinterpretq_u16_u8(p16hi), zero16)),
                ];

                let area_v = vdupq_n_f32(area);
                // lhs = p * area (f32); dark iff lhs <= t (f32 lanes 0xFF..)
                let masks = [
                    vcleq_f32(
                        vmulq_f32(vcvtq_f32_u32(p32[0]), area_v),
                        vld1q_f32(sums[x..].as_ptr()),
                    ),
                    vcleq_f32(
                        vmulq_f32(vcvtq_f32_u32(p32[1]), area_v),
                        vld1q_f32(sums[x + 4..].as_ptr()),
                    ),
                    vcleq_f32(
                        vmulq_f32(vcvtq_f32_u32(p32[2]), area_v),
                        vld1q_f32(sums[x + 8..].as_ptr()),
                    ),
                    vcleq_f32(
                        vmulq_f32(vcvtq_f32_u32(p32[3]), area_v),
                        vld1q_f32(sums[x + 12..].as_ptr()),
                    ),
                ];

                // masks: 0xFFFFFFFF (dark) / 0 per f32 lane. Narrow each lane
                // right by 16 (0x0000FFFF / 0) then to bytes: one byte per
                // pixel, 0xFF dark / 0x00 light.
                let m16 = [
                    vshrn_n_u32(masks[0], 16),
                    vshrn_n_u32(masks[1], 16),
                    vshrn_n_u32(masks[2], 16),
                    vshrn_n_u32(masks[3], 16),
                ];
                let bytes = [
                    vqmovn_u16(vcombine_u16(m16[0], m16[1])),
                    vqmovn_u16(vcombine_u16(m16[2], m16[3])),
                ];
                vst1q_u8(row_out[x..].as_mut_ptr(), vcombine_u8(bytes[0], bytes[1]));
            }
            x += 16;
        }

        // Scalar borders + trailing pixels (< 16 wide strip)
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

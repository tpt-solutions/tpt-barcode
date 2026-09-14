//! Sobel edge detection for binarized grayscale images.

/// Compute the Sobel gradient magnitude for each pixel of a binarized image.
///
/// `pixels` is a flat row-major slice of `width × height` values (0 or 255).
/// `out` receives 0–255 gradient magnitude for each pixel.
pub fn sobel(pixels: &[u8], width: usize, height: usize, out: &mut [u8]) {
    debug_assert_eq!(pixels.len(), width * height);
    debug_assert_eq!(out.len(), width * height);

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let get = |dy: i32, dx: i32| -> i32 {
                pixels[((y as i32 + dy) as usize) * width + ((x as i32 + dx) as usize)] as i32
            };

            // Sobel Gx kernel: [-1 0 1; -2 0 2; -1 0 1]
            let gx =
                -get(-1, -1) + get(-1, 1) - 2 * get(0, -1) + 2 * get(0, 1) - get(1, -1) + get(1, 1);

            // Sobel Gy kernel: [-1 -2 -1; 0 0 0; 1 2 1]
            let gy =
                -get(-1, -1) - 2 * get(-1, 0) - get(-1, 1) + get(1, -1) + 2 * get(1, 0) + get(1, 1);

            // Approximate magnitude (avoids sqrt): |Gx| + |Gy| scaled to 0..255
            let mag = (gx.unsigned_abs() + gy.unsigned_abs()) / 8;
            out[y * width + x] = mag.min(255) as u8;
        }
    }

    // Zero out the border
    for x in 0..width {
        out[x] = 0;
        out[(height - 1) * width + x] = 0;
    }
    for y in 0..height {
        out[y * width] = 0;
        out[y * width + width - 1] = 0;
    }
}

/// Thin edges using a simple non-maximum suppression pass.
///
/// Keeps a pixel only if it is a local maximum along the gradient direction.
pub fn non_max_suppress(magnitude: &[u8], width: usize, height: usize, out: &mut [u8]) {
    debug_assert_eq!(magnitude.len(), width * height);
    debug_assert_eq!(out.len(), width * height);

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let m = magnitude[y * width + x];
            // Compare to horizontal and vertical neighbours (simplified 4-direction NMS)
            let horiz = magnitude[y * width + x - 1].max(magnitude[y * width + x + 1]);
            let vert = magnitude[(y - 1) * width + x].max(magnitude[(y + 1) * width + x]);
            out[y * width + x] = if m > horiz && m > vert { m } else { 0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sobel_flat_image_has_no_edges() {
        let pixels = [128u8; 9];
        let mut out = [0u8; 9];
        sobel(&pixels, 3, 3, &mut out);
        assert!(out.iter().all(|&v| v == 0));
    }

    #[test]
    fn sobel_vertical_edge() {
        // Left half dark, right half light — strong vertical edge
        let mut pixels = [255u8; 6 * 6];
        for r in 0..6 {
            for c in 0..3 {
                pixels[r * 6 + c] = 0;
            }
        }
        let mut out = [0u8; 6 * 6];
        sobel(&pixels, 6, 6, &mut out);
        // Centre column should have non-zero gradient
        assert!(out[2 * 6 + 3] > 0 || out[2 * 6 + 2] > 0);
    }
}

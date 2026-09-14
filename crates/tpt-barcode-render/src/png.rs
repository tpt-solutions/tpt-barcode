//! PNG renderer via the `image` crate (requires `png` feature).

/// Render a QR Code matrix to a PNG byte buffer.
///
/// `matrix`: flat row-major module array (0 = light, 1 = dark), `size × size`.
/// Returns raw PNG bytes suitable for writing to a file or network response.
#[cfg(all(feature = "std", feature = "png"))]
pub fn render_to_png(
    matrix: &[u8],
    size: usize,
    module_px: u32,
    quiet_zone: u32,
) -> alloc::vec::Vec<u8> {
    use image::{ImageBuffer, Luma};

    let padded = size as u32 + 2 * quiet_zone;
    let image_px = padded * module_px;
    let mut img: ImageBuffer<Luma<u8>, _> = ImageBuffer::new(image_px, image_px);

    // Fill with white
    for pixel in img.pixels_mut() {
        *pixel = Luma([255u8]);
    }

    // Draw dark modules
    for row in 0..size as u32 {
        for col in 0..size as u32 {
            if matrix[(row * size as u32 + col) as usize] != 0 {
                let px_x = (quiet_zone + col) * module_px;
                let px_y = (quiet_zone + row) * module_px;
                for dy in 0..module_px {
                    for dx in 0..module_px {
                        img.put_pixel(px_x + dx, px_y + dy, Luma([0u8]));
                    }
                }
            }
        }
    }

    let mut buf = alloc::vec::Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("PNG encoding failed");
    buf
}

/// Render a 1D barcode module array to PNG.
///
/// `modules`: alternating bar/space widths (bar first), starting with a bar.
#[cfg(all(feature = "std", feature = "png"))]
pub fn render_1d_to_png(
    modules: &[u8],
    module_px: u32,
    bar_height_px: u32,
    quiet_zone_px: u32,
) -> alloc::vec::Vec<u8> {
    use image::{ImageBuffer, Luma};

    let total_width: u32 = modules.iter().map(|&w| w as u32 * module_px).sum();
    let image_w = total_width + 2 * quiet_zone_px;
    let image_h = bar_height_px + 2 * quiet_zone_px;

    let mut img: ImageBuffer<Luma<u8>, _> = ImageBuffer::new(image_w, image_h);
    for pixel in img.pixels_mut() {
        *pixel = Luma([255u8]);
    }

    let mut x = quiet_zone_px;
    let mut is_bar = true;
    for &width in modules {
        let px = width as u32 * module_px;
        if is_bar {
            for dy in 0..bar_height_px {
                for dx in 0..px {
                    img.put_pixel(x + dx, quiet_zone_px + dy, Luma([0u8]));
                }
            }
        }
        x += px;
        is_bar = !is_bar;
    }

    let mut buf = alloc::vec::Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("PNG encoding failed");
    buf
}

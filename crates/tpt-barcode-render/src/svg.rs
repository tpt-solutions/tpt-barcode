//! SVG renderer for QR Code and 1D barcodes.
//!
//! Produces a self-contained SVG string with one `<rect>` per dark module.
//! Uses a builder API: `.svg().module_size(4).quiet_zone(4).build()`.

#[cfg(feature = "alloc")]
use alloc::{format, string::String};

/// SVG render builder.
#[cfg(feature = "alloc")]
pub struct SvgBuilder<'a> {
    matrix: &'a [u8],
    size: usize,
    module_size: u32,
    quiet_zone: u32,
    dark_color: &'static str,
    light_color: &'static str,
}

#[cfg(feature = "alloc")]
impl<'a> SvgBuilder<'a> {
    /// Create a new SVG builder for a QR Code or 1D barcode matrix.
    ///
    /// `matrix`: flat row-major module array (0 = light, 1 = dark).
    /// `size`: number of modules per row/column.
    pub fn new(matrix: &'a [u8], size: usize) -> Self {
        Self {
            matrix,
            size,
            module_size: 4,
            quiet_zone: 4,
            dark_color: "#000000",
            light_color: "#ffffff",
        }
    }

    /// Set the pixel size of each module (default: 4).
    pub fn module_size(mut self, px: u32) -> Self {
        self.module_size = px;
        self
    }

    /// Set the quiet zone width in modules (default: 4).
    pub fn quiet_zone(mut self, modules: u32) -> Self {
        self.quiet_zone = modules;
        self
    }

    /// Set the dark module colour (default: `#000000`).
    pub fn dark_color(mut self, color: &'static str) -> Self {
        self.dark_color = color;
        self
    }

    /// Set the light (background) module colour (default: `#ffffff`).
    pub fn light_color(mut self, color: &'static str) -> Self {
        self.light_color = color;
        self
    }

    /// Render and return the SVG string.
    pub fn build(self) -> String {
        let ms = self.module_size;
        let qz = self.quiet_zone * ms;
        let total = self.size as u32 * ms + 2 * qz;

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total} {total}" width="{total}" height="{total}">"#,
            total = total
        );

        // Background
        svg.push_str(&format!(
            r#"<rect width="{t}" height="{t}" fill="{lc}"/>"#,
            t = total,
            lc = self.light_color
        ));

        // Dark modules
        for row in 0..self.size {
            for col in 0..self.size {
                if self.matrix[row * self.size + col] != 0 {
                    let x = qz + col as u32 * ms;
                    let y = qz + row as u32 * ms;
                    svg.push_str(&format!(
                        r#"<rect x="{x}" y="{y}" width="{ms}" height="{ms}" fill="{dc}"/>"#,
                        x = x,
                        y = y,
                        ms = ms,
                        dc = self.dark_color
                    ));
                }
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

/// Render a 1D barcode module array as an SVG string.
///
/// `modules`: alternating bar/space widths starting with a bar.
/// `height_px`: bar height in pixels.
#[cfg(feature = "alloc")]
pub fn render_1d(modules: &[u8], module_size: u32, height_px: u32) -> String {
    let total_width: u32 = modules.iter().map(|&w| w as u32 * module_size).sum();
    let qz = 10u32;
    let vw = total_width + 2 * qz;
    let vh = height_px + 2 * qz;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {vw} {vh}" width="{vw}" height="{vh}">"#
    );
    svg.push_str(&format!(
        r##"<rect width="{vw}" height="{vh}" fill="#ffffff"/>"##
    ));

    let mut x = qz;
    let mut is_bar = true;
    for &width in modules {
        let px = width as u32 * module_size;
        if is_bar {
            svg.push_str(&format!(
                r##"<rect x="{x}" y="{qz}" width="{px}" height="{height_px}" fill="#000000"/>"##
            ));
        }
        x += px;
        is_bar = !is_bar;
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_contains_svg_tag() {
        let matrix = [1u8, 0, 0, 1];
        let svg = SvgBuilder::new(&matrix, 2).module_size(2).build();
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn dark_modules_produce_rects() {
        let matrix = [1u8, 0, 0, 1];
        let svg = SvgBuilder::new(&matrix, 2).build();
        assert_eq!(svg.matches("<rect").count(), 3); // 1 background + 2 dark modules
    }
}

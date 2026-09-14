//! ANSI terminal renderer using Unicode half-block characters.
//!
//! Renders two rows of modules per terminal line using `▀` (U+2580 UPPER HALF BLOCK)
//! with foreground = dark colour and background = light colour, enabling a 2:1
//! aspect ratio that looks square on most monospace terminals.

#[cfg(feature = "alloc")]
use alloc::string::String;

/// Render a QR Code matrix to an ANSI terminal string.
///
/// `matrix`: flat row-major module array (0 = light, 1 = dark), `size × size`.
/// `quiet_zone`: number of quiet-zone module rows/columns to add on each side.
#[cfg(feature = "alloc")]
pub fn render(matrix: &[u8], size: usize, quiet_zone: usize) -> String {
    let _padded_size = size + 2 * quiet_zone;
    let mut out = String::new();

    // Process two rows at a time
    let mut row = 0i32 - quiet_zone as i32;
    while row < (size + quiet_zone) as i32 {
        for col_raw in -(quiet_zone as i32)..(size + quiet_zone) as i32 {
            let top = module_at(matrix, size, row, col_raw);
            let bot = module_at(matrix, size, row + 1, col_raw);

            match (top, bot) {
                (true, true) => out.push_str("\x1b[30;47m█\x1b[0m"), // both dark → full block
                (true, false) => out.push_str("\x1b[30;47m▀\x1b[0m"), // top dark
                (false, true) => out.push_str("\x1b[30;47m▄\x1b[0m"), // bottom dark
                (false, false) => out.push_str("\x1b[47m \x1b[0m"),  // both light
            }
        }
        out.push('\n');
        row += 2;
    }

    out
}

/// Render using simple ASCII characters (■ for dark, space for light).
/// Useful for environments without ANSI escape support.
#[cfg(feature = "alloc")]
pub fn render_ascii(matrix: &[u8], size: usize, quiet_zone: usize) -> String {
    let mut out = String::new();
    let total = size + 2 * quiet_zone;

    // Top quiet zone
    for _ in 0..quiet_zone {
        for _ in 0..total {
            out.push(' ');
        }
        out.push('\n');
    }

    for row in 0..size {
        for _ in 0..quiet_zone {
            out.push(' ');
        }
        for col in 0..size {
            out.push(if matrix[row * size + col] != 0 {
                '█'
            } else {
                ' '
            });
        }
        for _ in 0..quiet_zone {
            out.push(' ');
        }
        out.push('\n');
    }

    // Bottom quiet zone
    for _ in 0..quiet_zone {
        for _ in 0..total {
            out.push(' ');
        }
        out.push('\n');
    }

    out
}

fn module_at(matrix: &[u8], size: usize, row: i32, col: i32) -> bool {
    if row < 0 || col < 0 || row as usize >= size || col as usize >= size {
        return false; // quiet zone — light
    }
    matrix[row as usize * size + col as usize] != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_render_has_correct_line_count() {
        let matrix = [1u8, 0, 0, 1];
        let rendered = render_ascii(&matrix, 2, 1);
        // 1 top quiet + 2 data rows + 1 bottom quiet = 4 lines
        assert_eq!(rendered.lines().count(), 4);
    }

    #[test]
    fn ascii_render_correct_width() {
        let matrix = [0u8; 4];
        let rendered = render_ascii(&matrix, 2, 2);
        // Each line: 2+2+2 = 6 characters + newline
        for line in rendered.lines() {
            assert_eq!(line.chars().count(), 6, "line: {line:?}");
        }
    }
}

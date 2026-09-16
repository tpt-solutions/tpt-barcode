//! QR Code matrix construction: function patterns (finder, timing, alignment,
//! format), data codeword placement, and masking.
//!
//! Module layout follows ISO/IEC 18004. Data placement zigzags up two-module
//! column strips from the bottom-right corner, skipping the vertical timing
//! column; format information is a BCH(15,5) code placed around the finder
//! patterns and XORed with the mask `0b101010000010010`.

use alloc::vec;
use alloc::vec::Vec;

use super::mask::{is_masked, select_mask};
use super::version::{alignment_positions, modules};

/// Module state sentinel for function-pattern cells (finder, timing, alignment).
pub const RESERVED: u8 = 0x80;
/// Dark module value (1).
pub const DARK: u8 = 1;
/// Light module value (0).
pub const LIGHT: u8 = 0;

/// BCH format-info XOR mask (ISO 18004 §8.9).
pub const FORMAT_MASK: u16 = 0b101010000010010;
/// Format BCH generator polynomial x^10 + x^8 + x^5 + x^4 + x^2 + x + 1.
const FORMAT_GENERATOR: u16 = 0b10100110111;

/// Compute the 15-bit format information for (ec_bits, mask_id), mask included.
pub fn format_bits(ec_bits: u8, mask_id: u8) -> u16 {
    let data = (((ec_bits & 0b11) as u16) << 3) | (mask_id & 0b111) as u16;
    (data << 10 | bch_remainder(data)) ^ FORMAT_MASK
}

/// 10-bit BCH remainder of the 5-bit format payload.
fn bch_remainder(data: u16) -> u16 {
    let mut rem = data; // degree < 5
    for _ in 0..10 {
        // shift left one coefficient; fold the overflowing bit into the generator
        let overflow = rem >> 9 & 1;
        rem = (rem << 1) & 0x3FF;
        if overflow != 0 {
            rem ^= FORMAT_GENERATOR & 0x3FF; // generator without its x^10 term
        }
    }
    rem
}

/// Validate an unmasked 15-bit format value (must be divisible by the BCH
/// generator). Returns the 5-bit payload `(ec_bits << 3) | mask_id`.
pub fn validate_format(unmasked: u16) -> Option<u8> {
    format_payload_checked(unmasked)
}

/// Validate a 15-bit format codeword (mask already removed): the value must be
/// divisible by the BCH generator. Returns the 5-bit payload (ec<<3 | mask).
fn format_payload_checked(bits: u16) -> Option<u8> {
    let mut v = bits;
    for i in (10..15).rev() {
        if v >> i & 1 == 1 {
            v ^= FORMAT_GENERATOR << (i - 10);
        }
    }
    if v == 0 {
        Some((bits >> 10) as u8 & 0b1_1111)
    } else {
        None
    }
}

/// Build a complete QR Code matrix from interleaved codewords.
///
/// Returns a flat row-major `Vec<u8>` of size×size with values 0 (light) / 1 (dark),
/// and the selected mask pattern id. `ec_bits` is the 2-bit EC-level indicator
/// (L=01, M=00, Q=11, H=10).
#[cfg(feature = "alloc")]
pub fn build(
    version: u8,
    codewords: &[u8],
    ec_bits: u8,
    mask_id_hint: Option<u8>,
) -> (alloc::vec::Vec<u8>, u8) {
    let size = modules(version);
    let (mut matrix, is_function) = function_patterns(version);

    // ── Place data codewords ─────────────────────────────────────────────────
    place_data(&mut matrix, &is_function, size, codewords);

    // ── Mask selection ───────────────────────────────────────────────────────
    let mask_id = if let Some(m) = mask_id_hint {
        apply_mask(&mut matrix, &is_function, size, m);
        m
    } else {
        select_mask(&mut matrix, &is_function, size, ec_bits)
    };
    // ── Write format information ─────────────────────────────────────────────
    write_format(&mut matrix, size, ec_bits, mask_id);

    (matrix, mask_id)
}

/// Mark all fixed function patterns and return (initial values, function flags).
///
/// The value matrix has finders/alignment/timing/dark-module set; separators and
/// format areas are light. The flag matrix marks every non-data module.
#[cfg(feature = "alloc")]
pub fn function_patterns(version: u8) -> (Vec<u8>, Vec<bool>) {
    let size = modules(version);
    let mut matrix = vec![LIGHT; size * size];
    let mut is_function = vec![false; size * size];

    macro_rules! mark {
        ($r:expr, $c:expr, $v:expr) => {{
            let idx = ($r) * size + ($c);
            matrix[idx] = $v;
            is_function[idx] = true;
        }};
    }

    // Finder patterns (top-left, top-right, bottom-left)
    place_finder(&mut matrix, &mut is_function, size, 0, 0);
    place_finder(&mut matrix, &mut is_function, size, 0, size - 7);
    place_finder(&mut matrix, &mut is_function, size, size - 7, 0);

    // Separators (1-module light border around each finder)
    for i in 0..8 {
        // Top-left: row 7 and column 7
        mark!(7, i, LIGHT);
        mark!(i, 7, LIGHT);
        // Top-right: row 7 and column size-8
        mark!(7, size - 8 + i, LIGHT);
        mark!(i, size - 8, LIGHT);
        // Bottom-left: row size-8 and column 7
        mark!(size - 8, i, LIGHT);
        mark!(size - 8 + i, 7, LIGHT);
    }

    // Timing patterns (row 6 and column 6, alternating from the finders)
    for i in 8..size - 8 {
        let v = if i % 2 == 0 { DARK } else { LIGHT };
        mark!(6, i, v);
        mark!(i, 6, v);
    }

    // Alignment patterns
    if version >= 2 {
        let positions = alignment_positions(version);
        for &r in positions {
            for &c in positions {
                // Skip centres whose 5×5 pattern would overlap a finder area
                if (r <= 8 && c <= 8)
                    || (r <= 8 && c >= size as u8 - 8)
                    || (r >= size as u8 - 8 && c <= 8)
                {
                    continue;
                }
                place_alignment(&mut matrix, &mut is_function, size, r as usize, c as usize);
            }
        }
    }

    // Dark module at (4·version + 9, 8)
    mark!(4 * version as usize + 9, 8, DARK);

    // Reserve format information areas
    // Top-left: row 8 cols 0–8 and col 8 rows 0–8 (timing cell (8,6) already marked)
    for i in 0..9 {
        if i != 6 {
            mark!(8, i, LIGHT);
            mark!(i, 8, LIGHT);
        }
    }
    // Top-right: row 8, cols size-8 .. size-1 (8 cells of the second format copy)
    for c in size - 8..size {
        mark!(8, c, LIGHT);
    }
    // Bottom-left: col 8, rows size-7 .. size-1 (7 cells; dark module is size-8)
    for r in size - 7..size {
        mark!(r, 8, LIGHT);
    }

    (matrix, is_function)
}

/// Apply mask `m` (XOR) to every data module. `is_function[i]` is `true` for
/// reserved (non-data) modules, which are left untouched.
#[cfg(feature = "alloc")]
pub fn apply_mask(matrix: &mut [u8], is_function: &[bool], size: usize, m: u8) {
    for (i, &reserved) in is_function.iter().enumerate() {
        if !reserved {
            let r = i / size;
            let c = i % size;
            if is_masked(m, r, c) {
                matrix[i] ^= 1;
            }
        }
    }
}

fn place_finder(matrix: &mut [u8], func: &mut [bool], size: usize, row: usize, col: usize) {
    for dr in 0..7usize {
        for dc in 0..7usize {
            let idx = (row + dr) * size + (col + dc);
            let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
            let in_inner = (2..=4).contains(&dr) && (2..=4).contains(&dc);
            matrix[idx] = if on_border || in_inner { DARK } else { LIGHT };
            func[idx] = true;
        }
    }
}

fn place_alignment(matrix: &mut [u8], func: &mut [bool], size: usize, cr: usize, cc: usize) {
    for dr in -2i32..=2 {
        for dc in -2i32..=2 {
            let idx = ((cr as i32 + dr) as usize) * size + (cc as i32 + dc) as usize;
            let on_border = dr.abs() == 2 || dc.abs() == 2;
            let center = dr == 0 && dc == 0;
            matrix[idx] = if on_border || center { DARK } else { LIGHT };
            func[idx] = true;
        }
    }
}

/// Walk data modules in the ISO 18004 zigzag order (bottom-right, upward pairs of
/// columns, vertical timing column skipped) and invoke `visit` with (row, col).
pub(crate) fn for_each_data_position(size: usize, mut visit: impl FnMut(usize, usize)) {
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            // The vertical timing column is never part of a strip; shift this
            // strip left. Mutating `right` keeps the step-by-2 walk intact so
            // the final strip is (1, 0).
            right = 5;
        }
        let upward = (right + 1) & 2 == 0;
        for vert in 0..size {
            for j in 0..2usize {
                let c = (right - j as i32) as usize;
                let r = if upward { size - 1 - vert } else { vert };
                visit(r, c);
            }
        }
        right -= 2;
    }
}

fn place_data(matrix: &mut [u8], is_function: &[bool], size: usize, codewords: &[u8]) {
    let mut bit_idx = 0usize;
    for_each_data_position(size, |r, c| {
        let idx = r * size + c;
        if is_function[idx] {
            return;
        }
        let bit = if bit_idx / 8 < codewords.len() {
            (codewords[bit_idx / 8] >> (7 - bit_idx % 8)) & 1
        } else {
            0
        };
        matrix[idx] = bit;
        bit_idx += 1;
    });
}

/// Write the 15-bit format information (both copies) for (ec_level, mask_id).
///
/// EC level encoding: L=01, M=00, Q=11, H=10. Bit 14 (MSB) sits at (8, 0) in
/// the first copy; the second copy runs bits 0–7 along row 8 from the right
/// edge and bits 8–14 up column 8 from the bottom.
pub fn write_format(matrix: &mut [u8], size: usize, ec_bits: u8, mask_id: u8) {
    let bits = format_bits(ec_bits, mask_id);
    let bit = |i: u16| ((bits >> i) & 1) as u8;

    // First copy (around the top-left finder)
    for i in 0..6usize {
        matrix[i * size + 8] = bit(i as u16); // (row i, col 8)
    }
    matrix[7 * size + 8] = bit(6);
    matrix[8 * size + 8] = bit(7);
    matrix[8 * size + 7] = bit(8);
    for i in 9..15usize {
        matrix[8 * size + (14 - i)] = bit(i as u16); // (row 8, col 14-i)
    }

    // Second copy
    for i in 0..8usize {
        matrix[8 * size + (size - 1 - i)] = bit(i as u16); // (row 8, col size-1-i)
    }
    for i in 8..15usize {
        matrix[(size - 15 + i) * size + 8] = bit(i as u16); // (row size-15+i, col 8)
    }

    // Dark module (constant)
    matrix[(size - 8) * size + 8] = DARK;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bch_matches_known_codewords() {
        // ISO 18004 published format strings. ec_bits: L = 0b01, M = 0b00,
        // Q = 0b11, H = 0b10.
        assert_eq!(format_bits(0b01, 0), 0b111011111000100); // L, mask 0
        assert_eq!(format_bits(0b00, 0), 0b101010000010010); // M, mask 0
        assert_eq!(format_bits(0b11, 0), 0b011010101011111); // Q, mask 0
        assert_eq!(format_bits(0b10, 0), 0b001011010001001); // H, mask 0
        assert_eq!(format_bits(0b01, 1), 0b111001011110011); // L, mask 1
        assert_eq!(format_bits(0b10, 7), 0b000100000111011); // H, mask 7
    }

    #[test]
    fn format_round_trip() {
        for ec in 0u8..4 {
            for mask in 0u8..8 {
                let bits = format_bits(ec, mask);
                let payload = format_payload_checked(bits ^ FORMAT_MASK).unwrap();
                assert_eq!(payload, ((ec & 0b11) << 3) | (mask & 0b111));
            }
        }
    }

    #[test]
    fn data_position_count_matches_capacity() {
        // Every non-function module must be visited exactly once.
        for version in 1u8..=7 {
            let size = modules(version);
            let (_, is_function) = function_patterns(version);
            let capacity = is_function.iter().filter(|&&f| !f).count();
            let mut visited = vec![false; size * size];
            for_each_data_position(size, |r, c| {
                let idx = r * size + c;
                if is_function[idx] {
                    return; // place_data skips these
                }
                assert!(!visited[idx], "v{version}: module ({r},{c}) visited twice");
                visited[idx] = true;
            });
            assert_eq!(
                visited.iter().filter(|&&v| v).count(),
                capacity,
                "v{version}: not all data modules visited"
            );
        }
    }
}

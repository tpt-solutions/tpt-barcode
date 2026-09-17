//! Compile-time QR Code generation: the full encode pipeline as `const fn`.
//!
//! ```ignore
//! use tpt_barcode_2d::qr::const_qr::qr_svg;
//!
//! const TICKET_URL: &str = qr_svg(b"https://example.com/t/1234").as_str();
//! ```
//!
//! The const pipeline mirrors the runtime encoder exactly: byte-mode
//! segmentation, smallest-version selection, alternating pad codewords,
//! Reed-Solomon EC with multi-block interleaving (QR field 0x11D), ISO 18004
//! zigzag placement, penalty-optimal mask selection, format information and
//! version information (versions 7+) — and it is byte-for-byte identical to
//! the alloc-based [`super::QrCode::encode`] output, which the test suite
//! verifies across all 40 versions and all four EC levels.
//!
//! Only byte mode is supported (any payload up to the version-40-L capacity
//! of 2,953 bytes). Symbols are returned in fixed-size buffers; `panic!` at
//! compile time (clear message, no runtime fallthrough) signals a payload or
//! buffer overflow. `const fn` on stable requires `while` loops — they are
//! used throughout.

use tpt_barcode_core::traits::EcLevel;

use super::mask::is_masked;
use super::version::{alignment_positions, modules};

/// Maximum modules per side (version 40).
pub const MAX_SIZE: usize = 177;
/// Flat module-buffer capacity (177 × 177).
pub const MAX_MODULES: usize = MAX_SIZE * MAX_SIZE;
/// Largest EC codeword count per block across all (version, EC) layouts.
const MAX_ECN: usize = 30;
/// Largest block count across all (version, EC) layouts (version 40-H: 81).
const MAX_BLOCKS: usize = 81;
/// Largest data-codeword count (version 40-L).
const MAX_DATA_CW: usize = 2956;
/// Largest total codeword count (all versions top out at 3,706).
const MAX_TOTAL_CW: usize = 3706;

/// A QR Code symbol computed entirely at compile time.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ConstQr {
    /// Modules per side (21 for version 1, +4 per version).
    pub size: usize,
    /// QR Code version (1–40).
    pub version: u8,
    /// Mask pattern that was selected (0–7).
    pub mask: u8,
    /// Flat row-major module data (0 = light, 1 = dark), `size × size`.
    pub modules: [u8; MAX_MODULES],
}

impl ConstQr {
    /// Module at (row, col): `true` when dark.
    pub const fn module(&self, row: usize, col: usize) -> bool {
        self.modules[row * self.size + col] != 0
    }

    /// Side length including a quiet zone of 4 modules on each side.
    pub const fn size_with_quiet_zone(&self) -> usize {
        self.size + 8
    }
}

/// A string built at compile time in a fixed buffer.
///
/// The default buffer sizes used by the convenience constructors cover all
/// versions up to roughly 10–15 for adversarial payloads and far beyond for
/// typical URLs; use [`svg_str`] with an explicit `N` (turbofish) for very
/// large symbols.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ConstStr<const N: usize> {
    /// Number of bytes used in [`Self::buf`].
    pub len: usize,
    /// Backing buffer (the first `len` bytes are the string).
    pub buf: [u8; N],
}

impl<const N: usize> ConstStr<N> {
    /// The string as a byte slice.
    pub const fn as_bytes(&self) -> &[u8] {
        // `Index` is not const-stable, so slice via `split_at` (const since 1.71).
        let whole: &[u8] = &self.buf;
        let (head, _) = whole.split_at(self.len);
        head
    }

    /// The string as `&str`. Const-evaluable, so the result can be promoted
    /// to `'static` inside a `const` item.
    pub const fn as_str(&self) -> &str {
        match core::str::from_utf8(self.as_bytes()) {
            Ok(s) => s,
            Err(_) => panic!("ConstStr contains invalid UTF-8"),
        }
    }

    /// `true` when nothing has been written.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    const fn push(&mut self, byte: u8) {
        if self.len >= N {
            panic!("ConstStr buffer exceeded; use svg_str::<N> with a larger N");
        }
        self.buf[self.len] = byte;
        self.len += 1;
    }

    const fn push_num(&mut self, value: usize) {
        if value == 0 {
            self.push(b'0');
            return;
        }
        let mut digits = [0u8; 7];
        let mut n = 0;
        let mut v = value;
        while v > 0 {
            digits[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
        while n > 0 {
            n -= 1;
            self.push(digits[n]);
        }
    }
}

// ── Public constructors ─────────────────────────────────────────────────────

/// Compile-time QR Code matrix for `payload`, EC level M.
pub const fn qr_matrix(payload: &[u8]) -> ConstQr {
    qr_matrix_ec(payload, EcLevel::M)
}

/// Compile-time QR Code matrix for `payload` at an explicit EC level.
pub const fn qr_matrix_ec(payload: &[u8], ec: EcLevel) -> ConstQr {
    let (size, version, mask, m) = encode(payload, ec);
    ConstQr {
        size,
        version,
        mask,
        modules: m,
    }
}

/// Compile-time `<svg>` document (quiet zone 4, black on white) for `payload`,
/// EC level M. Buffer: 16 KiB.
pub const fn qr_svg(payload: &[u8]) -> ConstStr<16384> {
    svg_str::<16384>(payload, EcLevel::M, true)
}

/// Compile-time `<svg>` document at an explicit EC level. Buffer: 16 KiB.
pub const fn qr_svg_ec(payload: &[u8], ec: EcLevel) -> ConstStr<16384> {
    svg_str::<16384>(payload, ec, true)
}

/// Compile-time SVG path `d` string (dark modules only; coordinates include
/// the quiet-zone offset of 4, so pair it with `viewBox="0 0 {size+8} {size+8}"`).
/// Buffer: 8 KiB.
pub const fn qr_svg_path(payload: &[u8]) -> ConstStr<8192> {
    svg_str::<8192>(payload, EcLevel::M, false)
}

/// Core string builder: full document when `full_doc`, else only the path.
pub const fn svg_str<const N: usize>(payload: &[u8], ec: EcLevel, full_doc: bool) -> ConstStr<N> {
    let (size, _version, _mask, m) = encode(payload, ec);
    let n = size + 8; // quiet zone 4 on every side
    let mut s = ConstStr {
        len: 0,
        buf: [0u8; N],
    };

    if full_doc {
        push_str(
            &mut s,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 ",
        );
        s.push_num(n);
        push_str(&mut s, " ");
        s.push_num(n);
        push_str(&mut s, "\" shape-rendering=\"crispEdges\"><rect width=\"");
        s.push_num(n);
        push_str(&mut s, "\" height=\"");
        s.push_num(n);
        push_str(&mut s, "\" fill=\"#ffffff\"/><path fill=\"#000000\" d=\"");
    }

    // Per-row dark runs: M{x} {y}h{w}v1h-{w}z  (offset by quiet zone 4)
    let mut y = 0usize;
    while y < size {
        let mut x = 0usize;
        while x < size {
            if m[y * size + x] != 0 {
                let mut w = 1usize;
                while x + w < size && m[y * size + x + w] != 0 {
                    w += 1;
                }
                push_str(&mut s, "M");
                s.push_num(x + 4);
                push_str(&mut s, " ");
                s.push_num(y + 4);
                push_str(&mut s, "h");
                s.push_num(w);
                push_str(&mut s, "v1h-");
                s.push_num(w);
                push_str(&mut s, "z");
                x += w;
            } else {
                x += 1;
            }
        }
        y += 1;
    }

    if full_doc {
        push_str(&mut s, "\"/></svg>");
    }
    s
}

const fn push_str<const N: usize>(s: &mut ConstStr<N>, text: &str) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        s.push(bytes[i]);
        i += 1;
    }
}

// ── EC block layout tables (ISO 18004 Table 9) ──────────────────────────────
//
// One entry per version 1..=40: (g1_count, g1_data, ec_per_block, g2_count,
// g2_data). Transcribed from the verified runtime table in
// `super::version::version_info`; a test asserts parity for all 160 rows.

type Layout = (u8, u8, u8, u8, u8);

const BLOCKS_L: [Layout; 40] = [
    (1, 19, 7, 0, 0),
    (1, 34, 10, 0, 0),
    (1, 55, 15, 0, 0),
    (1, 80, 20, 0, 0),
    (1, 108, 26, 0, 0),
    (2, 68, 18, 0, 0),
    (2, 78, 20, 0, 0),
    (2, 97, 24, 0, 0),
    (2, 116, 30, 0, 0),
    (2, 68, 18, 2, 69),
    (4, 81, 20, 0, 0),
    (2, 92, 24, 2, 93),
    (4, 107, 26, 0, 0),
    (3, 115, 30, 1, 116),
    (5, 87, 22, 1, 88),
    (5, 98, 24, 1, 99),
    (1, 107, 28, 5, 108),
    (5, 120, 30, 1, 121),
    (3, 113, 28, 4, 114),
    (3, 107, 28, 5, 108),
    (4, 116, 28, 4, 117),
    (2, 111, 28, 7, 112),
    (4, 121, 30, 5, 122),
    (6, 117, 30, 4, 118),
    (8, 106, 26, 4, 107),
    (10, 114, 28, 2, 115),
    (8, 122, 30, 4, 123),
    (3, 117, 30, 10, 118),
    (7, 116, 30, 7, 117),
    (5, 115, 30, 10, 116),
    (13, 115, 30, 3, 116),
    (17, 115, 30, 0, 0),
    (17, 115, 30, 1, 116),
    (13, 115, 30, 6, 116),
    (12, 121, 30, 7, 122),
    (6, 121, 30, 14, 122),
    (17, 122, 30, 4, 123),
    (4, 122, 30, 18, 123),
    (20, 117, 30, 4, 118),
    (19, 118, 30, 6, 119),
];

const BLOCKS_M: [Layout; 40] = [
    (1, 16, 10, 0, 0),
    (1, 28, 16, 0, 0),
    (1, 44, 26, 0, 0),
    (2, 32, 18, 0, 0),
    (2, 43, 24, 0, 0),
    (4, 27, 16, 0, 0),
    (4, 31, 18, 0, 0),
    (2, 38, 22, 2, 39),
    (3, 36, 22, 2, 37),
    (4, 43, 26, 1, 44),
    (1, 50, 30, 4, 51),
    (6, 36, 22, 2, 37),
    (8, 37, 22, 1, 38),
    (4, 40, 24, 5, 41),
    (5, 41, 24, 5, 42),
    (7, 45, 28, 3, 46),
    (10, 46, 28, 1, 47),
    (9, 43, 26, 4, 44),
    (3, 44, 26, 11, 45),
    (3, 41, 26, 13, 42),
    (17, 42, 26, 0, 0),
    (17, 46, 28, 0, 0),
    (4, 47, 28, 14, 48),
    (6, 45, 28, 14, 46),
    (8, 47, 28, 13, 48),
    (19, 46, 28, 4, 47),
    (22, 45, 28, 3, 46),
    (3, 45, 28, 23, 46),
    (21, 45, 28, 7, 46),
    (19, 47, 28, 10, 48),
    (2, 46, 28, 29, 47),
    (10, 46, 28, 23, 47),
    (14, 46, 28, 21, 47),
    (14, 46, 28, 23, 47),
    (12, 47, 28, 26, 48),
    (6, 47, 28, 34, 48),
    (29, 46, 28, 14, 47),
    (13, 46, 28, 32, 47),
    (40, 47, 28, 7, 48),
    (18, 47, 28, 31, 48),
];

const BLOCKS_Q: [Layout; 40] = [
    (1, 13, 13, 0, 0),
    (1, 22, 22, 0, 0),
    (2, 17, 18, 0, 0),
    (2, 24, 26, 0, 0),
    (2, 15, 18, 2, 16),
    (4, 19, 24, 0, 0),
    (2, 14, 18, 4, 15),
    (4, 18, 22, 2, 19),
    (4, 16, 20, 4, 17),
    (6, 19, 24, 2, 20),
    (4, 22, 28, 4, 23),
    (4, 20, 26, 6, 21),
    (8, 20, 24, 4, 21),
    (11, 16, 20, 5, 17),
    (5, 24, 30, 7, 25),
    (15, 19, 24, 2, 20),
    (1, 22, 28, 15, 23),
    (17, 22, 28, 1, 23),
    (17, 21, 26, 4, 22),
    (15, 24, 30, 5, 25),
    (17, 22, 28, 6, 23),
    (7, 24, 30, 16, 25),
    (11, 24, 30, 14, 25),
    (11, 24, 30, 16, 25),
    (7, 24, 30, 22, 25),
    (28, 22, 28, 6, 23),
    (8, 23, 30, 26, 24),
    (4, 24, 30, 31, 25),
    (1, 23, 30, 37, 24),
    (15, 24, 30, 25, 25),
    (42, 24, 30, 1, 25),
    (10, 24, 30, 35, 25),
    (29, 24, 30, 19, 25),
    (44, 24, 30, 7, 25),
    (39, 24, 30, 14, 25),
    (46, 24, 30, 10, 25),
    (49, 24, 30, 10, 25),
    (48, 24, 30, 14, 25),
    (43, 24, 30, 22, 25),
    (34, 24, 30, 34, 25),
];

const BLOCKS_H: [Layout; 40] = [
    (1, 9, 17, 0, 0),
    (1, 16, 28, 0, 0),
    (2, 13, 22, 0, 0),
    (4, 9, 16, 0, 0),
    (2, 11, 22, 2, 12),
    (4, 15, 28, 0, 0),
    (4, 13, 26, 1, 14),
    (4, 14, 26, 2, 15),
    (4, 12, 24, 4, 13),
    (6, 15, 28, 2, 16),
    (3, 12, 24, 8, 13),
    (7, 14, 28, 4, 15),
    (12, 11, 22, 4, 12),
    (11, 12, 24, 5, 13),
    (11, 12, 24, 7, 13),
    (3, 15, 30, 13, 16),
    (2, 14, 28, 17, 15),
    (2, 14, 28, 19, 15),
    (9, 13, 26, 16, 14),
    (15, 15, 28, 10, 16),
    (19, 16, 30, 6, 17),
    (34, 13, 24, 0, 0),
    (16, 15, 30, 14, 16),
    (30, 16, 30, 2, 17),
    (22, 15, 30, 13, 16),
    (33, 16, 30, 4, 17),
    (12, 15, 30, 28, 16),
    (11, 15, 30, 31, 16),
    (19, 15, 30, 26, 16),
    (23, 15, 30, 25, 16),
    (23, 15, 30, 28, 16),
    (19, 15, 30, 35, 16),
    (11, 15, 30, 46, 16),
    (59, 16, 30, 1, 17),
    (22, 15, 30, 41, 16),
    (2, 15, 30, 64, 16),
    (24, 15, 30, 46, 16),
    (42, 15, 30, 32, 16),
    (10, 15, 30, 67, 16),
    (20, 15, 30, 61, 16),
];

const fn block_layout(version: u8, ec: EcLevel) -> Layout {
    let table = match ec {
        EcLevel::L => &BLOCKS_L,
        EcLevel::M => &BLOCKS_M,
        EcLevel::Q => &BLOCKS_Q,
        EcLevel::H => &BLOCKS_H,
    };
    if version < 1 || version as usize > table.len() {
        panic!("QR version out of range 1..=40");
    }
    table[version as usize - 1]
}

const fn data_codewords(version: u8, ec: EcLevel) -> usize {
    let (g1c, g1d, _ecn, g2c, g2d) = block_layout(version, ec);
    g1c as usize * g1d as usize + g2c as usize * g2d as usize
}

/// Character-count field width for byte mode: 8 bits for versions 1–9,
/// 16 bits for versions 10–40 (ISO 18004 §7.4.1 / §8.4).
const fn char_count_bits(version: u8) -> usize {
    if version < 10 {
        8
    } else {
        16
    }
}

// ── GF(256) arithmetic on the QR field (0x11D) ──────────────────────────────
//
// Uses the const-built log/antilog tables from tpt-barcode-core directly
// (trait methods cannot be `const fn` on stable).

use tpt_barcode_core::gf256::{EXP, LOG};

const fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        EXP[LOG[a as usize] as usize + LOG[b as usize] as usize]
    }
}

/// Reed-Solomon EC codewords for one data block (QR convention: roots
/// α^0…α^(n_ec-1)). Mirrors `reed_solomon::encode_with::<QrField>`.
const fn rs_encode(data: &[u8], n_ec: usize, out: &mut [u8]) {
    // Generator polynomial ∏ (x + α^i), coefficients high→low in `gen[0..=n_ec]`.
    let mut gen = [0u8; MAX_ECN + 1];
    gen[0] = 1;
    let mut degree = 1usize;
    let mut i = 0usize;
    while i < n_ec {
        let a = EXP[i];
        let mut j = degree;
        while j > 0 {
            gen[j] = gen[j - 1] ^ gf_mul(gen[j], a);
            j -= 1;
        }
        gen[0] = gf_mul(gen[0], a);
        degree += 1;
        i += 1;
    }

    let mut k = 0;
    while k < n_ec {
        out[k] = 0;
        k += 1;
    }

    // Polynomial long division.
    let mut d = 0usize;
    while d < data.len() {
        let factor = data[d] ^ out[0];
        let mut j = 0usize;
        while j + 1 < n_ec {
            out[j] = out[j + 1] ^ gf_mul(factor, gen[n_ec - 1 - j]);
            j += 1;
        }
        out[n_ec - 1] = gf_mul(factor, gen[0]);
        d += 1;
    }
}

// ── Format information ──────────────────────────────────────────────────────

const FORMAT_GENERATOR: u16 = 0b10100110111;
const FORMAT_MASK: u16 = 0b101010000010010;

/// 15-bit format information (BCH + XOR mask) for the 2-bit EC indicator and
/// mask id. Identical to `super::matrix::format_bits`.
const fn format_bits(ec_bits: u8, mask_id: u8) -> u16 {
    let data = (((ec_bits & 0b11) as u16) << 3) | (mask_id & 0b111) as u16;
    let mut rem = data;
    let mut i = 0;
    while i < 10 {
        let overflow = rem >> 9 & 1;
        rem = (rem << 1) & 0x3FF;
        if overflow != 0 {
            rem ^= FORMAT_GENERATOR & 0x3FF;
        }
        i += 1;
    }
    (data << 10 | rem) ^ FORMAT_MASK
}

const fn ec_bits_of(ec: EcLevel) -> u8 {
    match ec {
        EcLevel::L => 0b01,
        EcLevel::M => 0b00,
        EcLevel::Q => 0b11,
        EcLevel::H => 0b10,
    }
}

// ── Encoder ─────────────────────────────────────────────────────────────────

/// The complete const pipeline: returns (size, version, mask, modules).
const fn encode(payload: &[u8], ec: EcLevel) -> (usize, u8, u8, [u8; MAX_MODULES]) {
    // 1. Smallest version whose byte-mode capacity fits (same rule as
    //    `version::select_version(.., Mode::Byte, ..)`).
    let mut version = 1u8;
    while version <= 40 {
        let need = 4 + char_count_bits(version) + payload.len() * 8;
        if data_codewords(version, ec) * 8 >= need {
            break;
        }
        version += 1;
    }
    if version > 40 {
        panic!("payload too long for a version-40 QR Code at this EC level (byte mode)");
    }
    let size = modules(version);
    let (g1c, g1d, ecn, g2c, g2d) = block_layout(version, ec);
    let data_cw = data_codewords(version, ec);
    let capacity_bits = data_cw * 8;

    // 2. Bit stream: mode 0100, char count, payload, terminator (≤4 bits),
    //    byte alignment, alternating pad codewords.
    let mut bits = [0u8; MAX_DATA_CW];
    let mut blen = 0usize;
    push_bits(&mut bits, &mut blen, 0b0100, 4);
    push_bits(
        &mut bits,
        &mut blen,
        payload.len() as u32,
        char_count_bits(version),
    );
    let mut pi = 0usize;
    while pi < payload.len() {
        push_bits(&mut bits, &mut blen, payload[pi] as u32, 8);
        pi += 1;
    }
    let remaining = capacity_bits - blen;
    let mut t = 0;
    while t < 4 && t < remaining {
        push_bits(&mut bits, &mut blen, 0, 1);
        t += 1;
    }
    while blen % 8 != 0 {
        push_bits(&mut bits, &mut blen, 0, 1);
    }
    let mut pad = 0u8;
    while blen < capacity_bits {
        let byte = if pad == 0 { 0xEC } else { 0x11 };
        push_bits(&mut bits, &mut blen, byte as u32, 8);
        pad ^= 1;
    }
    // `bits` now holds exactly `data_cw` bytes.

    // 3. Per-block Reed-Solomon EC + codeword interleaving.
    let total_blocks = (g1c + g2c) as usize;
    let mut ec_store = [0u8; MAX_BLOCKS * MAX_ECN];
    let mut b = 0usize;
    let mut offset = 0usize;
    while b < total_blocks {
        let len = if b < g1c as usize {
            g1d as usize
        } else {
            g2d as usize
        };
        // `Index` is not const-stable; slice via const `split_at(_mut)`.
        let (bits_head, _) = bits.split_at(offset + len);
        let block = bits_head.split_at(offset).1;
        let (_, ec_tail) = ec_store.split_at_mut(b * ecn as usize);
        rs_encode(block, ecn as usize, ec_tail);
        offset += len;
        b += 1;
    }
    let mut codewords = [0u8; MAX_TOTAL_CW];
    let mut out_idx = 0usize;
    let mut i = 0usize; // position within a block
    loop {
        let mut any = false;
        b = 0;
        while b < total_blocks {
            let len = if b < g1c as usize {
                g1d as usize
            } else {
                g2d as usize
            };
            if i < len {
                let start = if b < g1c as usize {
                    b * g1d as usize
                } else {
                    g1c as usize * g1d as usize + (b - g1c as usize) * g2d as usize
                };
                codewords[out_idx] = bits[start + i];
                out_idx += 1;
                any = true;
            }
            b += 1;
        }
        if !any {
            break;
        }
        i += 1;
    }
    let mut j = 0usize;
    while j < ecn as usize {
        b = 0;
        while b < total_blocks {
            codewords[out_idx] = ec_store[b * ecn as usize + j];
            out_idx += 1;
            b += 1;
        }
        j += 1;
    }

    // 4. Function patterns (finder/timing/alignment/dark/version/format).
    let mut m = [0u8; MAX_MODULES];
    let mut reserved = [false; MAX_MODULES];
    draw_function_patterns(&mut m, &mut reserved, version);

    // 5. Zigzag data placement.
    let (cw_head, _) = codewords.split_at(out_idx);
    place_data(&mut m, &reserved, size, cw_head);

    // 6. Penalty-optimal mask selection (format info written per candidate,
    //    exactly like the runtime `select_mask`).
    let ec_bits = ec_bits_of(ec);
    let mut scratch = [0u8; MAX_MODULES];
    let mut best_mask = 0u8;
    let mut best_score = u32::MAX;
    let mut mask = 0u8;
    while mask < 8 {
        copy_matrix(&m, &mut scratch, size);
        apply_mask(&mut scratch, &reserved, size, mask);
        write_format(&mut scratch, size, ec_bits, mask);
        let score = penalty(&scratch, size);
        if score < best_score {
            best_score = score;
            best_mask = mask;
        }
        mask += 1;
    }
    apply_mask(&mut m, &reserved, size, best_mask);
    write_format(&mut m, size, ec_bits, best_mask);

    (size, version, best_mask, m)
}

const fn push_bits(buf: &mut [u8], len: &mut usize, value: u32, n: usize) {
    let mut i = 0usize;
    while i < n {
        let bit = ((value >> (n - 1 - i)) & 1) as u8;
        buf[*len / 8] |= bit << (7 - *len % 8);
        *len += 1;
        i += 1;
    }
}

const fn copy_matrix(src: &[u8], dst: &mut [u8], size: usize) {
    let mut i = 0;
    let total = size * size;
    while i < total {
        dst[i] = src[i];
        i += 1;
    }
}

/// Function patterns: finder patterns + separators, timing patterns,
/// alignment patterns, dark module, version information (7+) and the
/// reserved (light) format areas. Mirrors `super::matrix::function_patterns`.
const fn draw_function_patterns(m: &mut [u8], reserved: &mut [bool], version: u8) {
    let size = modules(version);

    // Finder patterns (7×7) in three corners.
    place_finder(m, reserved, size, 0, 0);
    place_finder(m, reserved, size, 0, size - 7);
    place_finder(m, reserved, size, size - 7, 0);

    // Separators (light ring around each finder area).
    let mut i = 0usize;
    while i < 8 {
        mark(7, i, 0, m, reserved, size);
        mark(i, 7, 0, m, reserved, size);
        mark(7, size - 8 + i, 0, m, reserved, size);
        mark(i, size - 8, 0, m, reserved, size);
        mark(size - 8, i, 0, m, reserved, size);
        mark(size - 8 + i, 7, 0, m, reserved, size);
        i += 1;
    }

    // Timing patterns (row 6 and column 6).
    let mut i = 8usize;
    while i < size - 8 {
        let v = if i % 2 == 0 { 1 } else { 0 };
        mark(6, i, v, m, reserved, size);
        mark(i, 6, v, m, reserved, size);
        i += 1;
    }

    // Alignment patterns (5×5), skipping finder-overlapping corners.
    if version >= 2 {
        let positions = alignment_positions(version);
        let mut pi = 0usize;
        while pi < positions.len() {
            let r = positions[pi] as usize;
            let mut ci = 0usize;
            while ci < positions.len() {
                let c = positions[ci] as usize;
                if (r <= 8 && c <= 8) || (r <= 8 && c >= size - 8) || (r >= size - 8 && c <= 8) {
                    ci += 1;
                    continue;
                }
                let mut dr = -2i32;
                while dr <= 2 {
                    let mut dc = -2i32;
                    while dc <= 2 {
                        let on_border = dr.abs() == 2 || dc.abs() == 2;
                        let center = dr == 0 && dc == 0;
                        let v = if on_border || center { 1 } else { 0 };
                        mark(
                            (r as i32 + dr) as usize,
                            (c as i32 + dc) as usize,
                            v,
                            m,
                            reserved,
                            size,
                        );
                        dc += 1;
                    }
                    dr += 1;
                }
                ci += 1;
            }
            pi += 1;
        }
    }

    // Dark module.
    mark(4 * version as usize + 9, 8, 1, m, reserved, size);

    // Version information (versions 7+): 18-bit BCH codeword, two 3×6 copies,
    // bit 0 (LSB) first — bottom-left and top-right.
    if version >= 7 {
        let bits = super::matrix::version_info_bits(version);
        let mut i = 0usize;
        while i < 18 {
            let bit = ((bits >> i) & 1) as u8;
            let a = size - 11 + i % 3;
            let b = i / 3;
            mark(a, b, bit, m, reserved, size);
            mark(b, a, bit, m, reserved, size);
            i += 1;
        }
    }

    // Format areas: reserve as light; final values are written after masking.
    let mut i = 0usize;
    while i < 9 {
        if i != 6 {
            mark(8, i, 0, m, reserved, size);
            mark(i, 8, 0, m, reserved, size);
        }
        i += 1;
    }
    let mut c = size - 8;
    while c < size {
        mark(8, c, 0, m, reserved, size);
        c += 1;
    }
    let mut r = size - 7;
    while r < size {
        mark(r, 8, 0, m, reserved, size);
        r += 1;
    }
}

const fn mark(r: usize, c: usize, v: u8, m: &mut [u8], reserved: &mut [bool], size: usize) {
    m[r * size + c] = v;
    reserved[r * size + c] = true;
}

const fn place_finder(m: &mut [u8], reserved: &mut [bool], size: usize, row: usize, col: usize) {
    let mut dr = 0usize;
    while dr < 7 {
        let mut dc = 0usize;
        while dc < 7 {
            let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
            let in_inner = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
            let v = if on_border || in_inner { 1 } else { 0 };
            mark(row + dr, col + dc, v, m, reserved, size);
            dc += 1;
        }
        dr += 1;
    }
}

/// ISO 18004 zigzag placement of the codeword bit stream (bottom-right,
/// upward column pairs, timing column skipped by mutating right 6 → 5).
const fn place_data(m: &mut [u8], reserved: &[bool], size: usize, codewords: &[u8]) {
    let mut bit_idx = 0usize;
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        let upward = (right + 1) & 2 == 0;
        let mut vert = 0usize;
        while vert < size {
            let mut j = 0usize;
            while j < 2 {
                let c = (right - j as i32) as usize;
                let r = if upward { size - 1 - vert } else { vert };
                let idx = r * size + c;
                if !reserved[idx] {
                    let bit = if bit_idx / 8 < codewords.len() {
                        (codewords[bit_idx / 8] >> (7 - bit_idx % 8)) & 1
                    } else {
                        0
                    };
                    m[idx] = bit;
                    bit_idx += 1;
                }
                j += 1;
            }
            vert += 1;
        }
        right -= 2;
    }
}

const fn apply_mask(m: &mut [u8], reserved: &[bool], size: usize, mask_id: u8) {
    let mut i = 0usize;
    let total = size * size;
    while i < total {
        if !reserved[i] && is_masked(mask_id, i / size, i % size) {
            m[i] ^= 1;
        }
        i += 1;
    }
}

/// Write both 15-bit format copies + dark module (mirror of
/// `super::matrix::write_format`).
const fn fmt_bit(bits: u16, i: u16) -> u8 {
    ((bits >> i) & 1) as u8
}

const fn write_format(m: &mut [u8], size: usize, ec_bits: u8, mask_id: u8) {
    let bits = format_bits(ec_bits, mask_id);

    let mut i = 0usize;
    while i < 6 {
        m[i * size + 8] = fmt_bit(bits, i as u16);
        i += 1;
    }
    m[7 * size + 8] = fmt_bit(bits, 6);
    m[8 * size + 8] = fmt_bit(bits, 7);
    m[8 * size + 7] = fmt_bit(bits, 8);
    let mut i = 9usize;
    while i < 15 {
        m[8 * size + (14 - i)] = fmt_bit(bits, i as u16);
        i += 1;
    }

    let mut i = 0usize;
    while i < 8 {
        m[8 * size + (size - 1 - i)] = fmt_bit(bits, i as u16);
        i += 1;
    }
    let mut i = 8usize;
    while i < 15 {
        m[(size - 15 + i) * size + 8] = fmt_bit(bits, i as u16);
        i += 1;
    }

    m[(size - 8) * size + 8] = 1;
}

// ── Penalty scoring (ISO 18004 §8.8.2) ──────────────────────────────────────

const fn penalty(m: &[u8], size: usize) -> u32 {
    penalty_rule1(m, size)
        + penalty_rule2(m, size)
        + penalty_rule3(m, size)
        + penalty_rule4(m, size)
}

const fn penalty_rule1(m: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    // Rows.
    let mut r = 0usize;
    while r < size {
        let mut run = 1usize;
        let mut c = 1usize;
        while c < size {
            if m[r * size + c] == m[r * size + c - 1] {
                run += 1;
            } else {
                if run >= 5 {
                    score += (run - 5) as u32 + 3;
                }
                run = 1;
            }
            c += 1;
        }
        if run >= 5 {
            score += (run - 5) as u32 + 3;
        }
        r += 1;
    }
    // Columns.
    let mut c = 0usize;
    while c < size {
        let mut run = 1usize;
        let mut r = 1usize;
        while r < size {
            if m[r * size + c] == m[(r - 1) * size + c] {
                run += 1;
            } else {
                if run >= 5 {
                    score += (run - 5) as u32 + 3;
                }
                run = 1;
            }
            r += 1;
        }
        if run >= 5 {
            score += (run - 5) as u32 + 3;
        }
        c += 1;
    }
    score
}

const fn penalty_rule2(m: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    let mut r = 0usize;
    while r + 1 < size {
        let mut c = 0usize;
        while c + 1 < size {
            let v = m[r * size + c];
            if m[r * size + c + 1] == v
                && m[(r + 1) * size + c] == v
                && m[(r + 1) * size + c + 1] == v
            {
                score += 3;
            }
            c += 1;
        }
        r += 1;
    }
    score
}

const PATTERN_A: [u8; 11] = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
const PATTERN_B: [u8; 11] = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];

const fn matches_pattern(data: &[u8; 11]) -> bool {
    let mut eq_a = true;
    let mut eq_b = true;
    let mut i = 0;
    while i < 11 {
        if data[i] != PATTERN_A[i] {
            eq_a = false;
        }
        if data[i] != PATTERN_B[i] {
            eq_b = false;
        }
        i += 1;
    }
    eq_a || eq_b
}

const fn penalty_rule3(m: &[u8], size: usize) -> u32 {
    let mut score = 0u32;
    let mut window = [0u8; 11];
    // Rows.
    let mut r = 0usize;
    while r < size {
        let mut start = 0usize;
        while start + 11 <= size {
            let mut i = 0;
            while i < 11 {
                window[i] = m[r * size + start + i];
                i += 1;
            }
            if matches_pattern(&window) {
                score += 40;
            }
            start += 1;
        }
        r += 1;
    }
    // Columns.
    let mut c = 0usize;
    while c < size {
        let mut start = 0usize;
        while start + 11 <= size {
            let mut i = 0;
            while i < 11 {
                window[i] = m[(start + i) * size + c];
                i += 1;
            }
            if matches_pattern(&window) {
                score += 40;
            }
            start += 1;
        }
        c += 1;
    }
    score
}

const fn penalty_rule4(m: &[u8], size: usize) -> u32 {
    let total = (size * size) as u32;
    let mut dark = 0u32;
    let mut i = 0usize;
    let n = size * size;
    while i < n {
        if m[i] != 0 {
            dark += 1;
        }
        i += 1;
    }
    let pct = dark * 100 / total;
    let prev5 = (pct / 5) * 5;
    let next5 = prev5 + 5;
    let prev_diff = prev5.abs_diff(50);
    let next_diff = next5.abs_diff(50);
    let a = if prev_diff < next_diff {
        prev_diff
    } else {
        next_diff
    };
    (a / 5) * 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qr::QrCode;
    use alloc::vec::Vec;

    /// The const tables must agree with the (thonky-verified) runtime table
    /// for all 160 (version, EC) combinations.
    #[test]
    fn block_tables_match_runtime() {
        for v in 1u8..=40 {
            for ec in [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
                let info = crate::qr::version::version_info(v, ec).unwrap();
                let (g1c, g1d, ecn, g2c, g2d) = block_layout(v, ec);
                assert_eq!((g1c, g1d, ecn, g2c, g2d), {
                    (
                        info.group1.count,
                        info.group1.data_codewords,
                        info.group1.ec_codewords,
                        info.group2.count,
                        info.group2.data_codewords,
                    )
                });
                let total = info.total_codewords as usize;
                assert_eq!(
                    data_codewords(v, ec) + (g1c as usize + g2c as usize) * ecn as usize,
                    total
                );
            }
        }
    }

    /// Payload sizes (deterministic high-bit bytes force byte mode) must yield
    /// byte-identical matrices from the const and runtime pipelines.
    #[test]
    fn const_matches_runtime_all_versions() {
        for ec in [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
            let mut covered = alloc::collections::BTreeSet::new();
            for v in 1u8..=40 {
                let dcw = crate::qr::version::version_info(v, ec)
                    .unwrap()
                    .data_codewords();
                // Max byte-mode payload for this exact version: mode indicator
                // (4 bits) + char-count indicator (8 bits for v1-9, 16 for
                // v10-40) + payload bits must fit within the data codewords.
                let len = (dcw * 8 - 4 - char_count_bits(v)) / 8;
                let payload: Vec<u8> = (0..len)
                    .map(|i| 0x80u8 | ((i as u8).wrapping_mul(31) & 0x7F))
                    .collect();

                let runtime = QrCode::encode(&payload, ec).unwrap();
                let constant = qr_matrix_ec(&payload, ec);
                assert_eq!(runtime.version, constant.version, "v{v} {ec:?}: version");
                assert_eq!(runtime.mask_id, constant.mask, "v{v} {ec:?}: mask");
                assert_eq!(
                    runtime.matrix,
                    constant.modules[..runtime.size * runtime.size],
                    "v{v} {ec:?}: matrices differ"
                );
                // The const symbol must decode (validates internal consistency).
                assert_eq!(
                    crate::qr::decode_grid(
                        &constant.modules[..constant.size * constant.size],
                        constant.size
                    )
                    .unwrap(),
                    payload,
                    "v{v} {ec:?}: const symbol does not decode"
                );
                covered.insert(constant.version);
            }
            // Every version 1..=40 must have been exercised.
            assert_eq!(covered.len(), 40);
        }
    }

    /// The whole pipeline must run inside a `const` item — this test only
    /// compiles if const evaluation succeeds — and the compile-time result
    /// must equal the runtime call.
    #[test]
    #[allow(clippy::assertions_on_constants, clippy::absurd_extreme_comparisons)]
    fn const_eval_and_svg() {
        const URL: &[u8] = b"https://example.com/tickets/1234";

        static MATRIX: ConstQr = qr_matrix(URL);
        const SVG: &str = qr_svg(URL).as_str();
        const PATH: &str = qr_svg_path(URL).as_str();

        // 30 bytes + overhead at EC M → version 3 (29×29).
        assert!(MATRIX.version == 3);
        assert!(MATRIX.size == 29);
        assert!(MATRIX.mask <= 7);

        assert!(SVG.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(SVG.contains("viewBox=\"0 0 37 37\""));
        assert!(SVG.ends_with("</svg>"));
        assert!(PATH.contains('M'));

        let runtime = qr_matrix(URL);
        assert_eq!(MATRIX.modules, runtime.modules);

        // The const matrix decodes back to the payload.
        assert_eq!(
            crate::qr::decode_grid(&MATRIX.modules[..MATRIX.size * MATRIX.size], MATRIX.size)
                .unwrap(),
            URL
        );
    }

    #[test]
    fn svg_document_is_valid_for_tiny_payload() {
        let svg_holder = qr_svg(b"hi");
        let svg = svg_holder.as_str();
        assert!(svg.contains("viewBox=\"0 0 29 29\""), "v1-M is 21+8 = 29");
        assert_eq!(svg.matches("<path").count(), 1);
        assert_eq!(svg.matches("</svg>").count(), 1);
    }
}

//! QR Code encoding modes and bit-stream packing.

/// QR Code encoding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Digits 0–9 only. 10 bits per 3 digits.
    Numeric,
    /// 0–9, A–Z, space, $%*+-./: — 11 bits per 2 characters.
    Alphanumeric,
    /// 8-bit binary / UTF-8. One byte per character.
    Byte,
    /// Kanji (Shift JIS). 13 bits per character.
    Kanji,
}

impl Mode {
    /// 4-bit mode indicator value.
    pub fn indicator(self) -> u8 {
        match self {
            Mode::Numeric => 0b0001,
            Mode::Alphanumeric => 0b0010,
            Mode::Byte => 0b0100,
            Mode::Kanji => 0b1000,
        }
    }

    /// Number of bits used for the character count field at the given version group.
    pub fn char_count_bits(self, version: u8) -> u8 {
        match (self, version) {
            (Mode::Numeric, 1..=9) => 10,
            (Mode::Numeric, 10..=26) => 12,
            (Mode::Numeric, _) => 14,
            (Mode::Alphanumeric, 1..=9) => 9,
            (Mode::Alphanumeric, 10..=26) => 11,
            (Mode::Alphanumeric, _) => 13,
            (Mode::Byte, 1..=9) => 8,
            (Mode::Byte, _) => 16,
            (Mode::Kanji, 1..=9) => 8,
            (Mode::Kanji, 10..=26) => 10,
            (Mode::Kanji, _) => 12,
        }
    }

    /// Detect the most compact mode for the given byte slice.
    pub fn detect(data: &[u8]) -> Mode {
        if data.iter().all(|&b| b.is_ascii_digit()) {
            return Mode::Numeric;
        }
        if data.iter().all(|&b| ALPHANUMERIC_CHARSET.contains(&b)) {
            return Mode::Alphanumeric;
        }
        Mode::Byte
    }
}

/// Alphanumeric charset value for encoding.
pub fn alphanumeric_value(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'A'..=b'Z' => b - b'A' + 10,
        b' ' => 36,
        b'$' => 37,
        b'%' => 38,
        b'*' => 39,
        b'+' => 40,
        b'-' => 41,
        b'.' => 42,
        b'/' => 43,
        b':' => 44,
        _ => 0xFF, // invalid
    }
}

/// The 45-character QR alphanumeric mode alphabet.
pub const ALPHANUMERIC_CHARSET: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// Character-count field width for a raw 4-bit mode `indicator` at `version`.
pub fn char_count_bits_of(indicator: u8, version: u8) -> u8 {
    match indicator {
        0b0001 => Mode::Numeric.char_count_bits(version),
        0b0010 => Mode::Alphanumeric.char_count_bits(version),
        0b0100 => Mode::Byte.char_count_bits(version),
        0b1000 => Mode::Kanji.char_count_bits(version),
        _ => 8,
    }
}

/// A growable bit buffer for building QR Code data streams.
///
/// Bits are packed MSB-first into bytes. Requires the `alloc` feature.
#[cfg(feature = "alloc")]
pub struct BitBuffer {
    data: alloc::vec::Vec<u8>,
    bit_len: usize,
}

#[cfg(feature = "alloc")]
impl BitBuffer {
    /// Create an empty bit buffer.
    pub fn new() -> Self {
        Self {
            data: alloc::vec::Vec::new(),
            bit_len: 0,
        }
    }

    /// Total number of bits written.
    pub fn len(&self) -> usize {
        self.bit_len
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bit_len == 0
    }

    /// Append `n_bits` (1..=16) bits from `value`, MSB first.
    pub fn push_bits(&mut self, value: u32, n_bits: usize) {
        debug_assert!(n_bits <= 16);
        for i in (0..n_bits).rev() {
            let bit = ((value >> i) & 1) as u8;
            let byte_idx = self.bit_len / 8;
            let bit_idx = 7 - (self.bit_len % 8);
            if self.bit_len % 8 == 0 {
                self.data.push(0);
            }
            self.data[byte_idx] |= bit << bit_idx;
            self.bit_len += 1;
        }
    }

    /// Return the packed byte slice. May have trailing zero bits in the last byte.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(feature = "alloc")]
impl Default for BitBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode `data` in Numeric mode, appending bits to `buf`.
#[cfg(feature = "alloc")]
pub fn encode_numeric(data: &[u8], buf: &mut BitBuffer) {
    let chunks = data.chunks(3);
    for chunk in chunks {
        let value: u32 = chunk
            .iter()
            .fold(0u32, |acc, &b| acc * 10 + (b - b'0') as u32);
        let bits = match chunk.len() {
            1 => 4,
            2 => 7,
            3 => 10,
            _ => unreachable!(),
        };
        buf.push_bits(value, bits);
    }
}

/// Encode `data` in Alphanumeric mode, appending bits to `buf`.
#[cfg(feature = "alloc")]
pub fn encode_alphanumeric(data: &[u8], buf: &mut BitBuffer) {
    let pairs = data.chunks(2);
    for pair in pairs {
        if pair.len() == 2 {
            let v = alphanumeric_value(pair[0]) as u32 * 45 + alphanumeric_value(pair[1]) as u32;
            buf.push_bits(v, 11);
        } else {
            buf.push_bits(alphanumeric_value(pair[0]) as u32, 6);
        }
    }
}

/// Encode `data` in Byte mode, appending bits to `buf`.
#[cfg(feature = "alloc")]
pub fn encode_byte(data: &[u8], buf: &mut BitBuffer) {
    for &b in data {
        buf.push_bits(b as u32, 8);
    }
}

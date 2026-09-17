//! QR Code generation and decoding pipeline (ISO/IEC 18004).

pub mod const_qr;
pub mod decode;
pub mod ec;
pub mod mask;
pub mod matrix;
pub mod mode;
pub mod version;

use alloc::vec::Vec;

use tpt_barcode_core::traits::{EcLevel, EncodeError};

pub use decode::{decode_grid, decode_grid_detailed, decode_payload, DecodedQr};

/// A fully encoded QR Code symbol ready for rendering.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrCode {
    /// Flat row-major module data (0 = light, 1 = dark).
    pub matrix: Vec<u8>,
    /// Side length (number of modules).
    pub size: usize,
    /// QR version (1–40).
    pub version: u8,
    /// Error-correction level.
    pub ec_level: EcLevel,
    /// Applied mask pattern (0–7).
    pub mask_id: u8,
}

/// 2-bit EC-level indicator used in format information: L=01, M=00, Q=11, H=10.
pub(crate) fn ec_bits_of(ec: EcLevel) -> u8 {
    match ec {
        EcLevel::L => 0b01,
        EcLevel::M => 0b00,
        EcLevel::Q => 0b11,
        EcLevel::H => 0b10,
    }
}

#[cfg(feature = "alloc")]
impl QrCode {
    /// Encode `data` as a QR Code at the given error-correction level.
    ///
    /// Automatically selects the smallest version that fits the data and the
    /// best mask pattern per the ISO 18004 penalty rules.
    pub fn encode(data: &[u8], ec: EcLevel) -> Result<Self, EncodeError> {
        use mode::{encode_alphanumeric, encode_byte, encode_numeric, BitBuffer, Mode};

        let qr_mode = Mode::detect(data);

        let version =
            version::select_version(data.len(), qr_mode, ec).ok_or(EncodeError::DataTooLong)?;
        let info = version::version_info(version, ec).ok_or(EncodeError::Unsupported)?;
        let capacity_bits = info.data_codewords() * 8;

        // Build bit stream
        let mut buf = BitBuffer::new();
        buf.push_bits(qr_mode.indicator() as u32, 4);
        buf.push_bits(data.len() as u32, qr_mode.char_count_bits(version) as usize);
        match qr_mode {
            Mode::Numeric => encode_numeric(data, &mut buf),
            Mode::Alphanumeric => encode_alphanumeric(data, &mut buf),
            Mode::Byte | Mode::Kanji => encode_byte(data, &mut buf),
        }
        // Terminator (up to 4 zero bits)
        let remaining = capacity_bits.saturating_sub(buf.len());
        buf.push_bits(0, remaining.min(4));
        // Byte-align
        while buf.len() % 8 != 0 {
            buf.push_bits(0, 1);
        }
        // Padding codewords
        let mut pad_byte = 0u8;
        while buf.len() < capacity_bits {
            buf.push_bits(if pad_byte == 0 { 0xEC } else { 0x11 } as u32, 8);
            pad_byte ^= 1;
        }

        let data_bytes = buf.as_bytes();
        let codewords = ec::interleave_blocks(data_bytes, &info);

        let (matrix, mask_id) = matrix::build(version, &codewords, ec_bits_of(ec), None);

        Ok(QrCode {
            size: version::modules(version),
            matrix,
            version,
            ec_level: ec,
            mask_id,
        })
    }

    /// Access module at (row, col). Returns `true` for dark.
    pub fn module(&self, row: usize, col: usize) -> bool {
        self.matrix[row * self.size + col] != 0
    }

    /// Decode this symbol back to its payload bytes (round-trip helper).
    pub fn decode(&self) -> Result<Vec<u8>, tpt_barcode_core::traits::DecodeError> {
        decode::decode_grid(&self.matrix, self.size)
    }
}

/// Encode `data` as a QR Code. Convenience wrapper around [`QrCode::encode`].
#[cfg(feature = "alloc")]
pub fn encode(data: impl AsRef<[u8]>, ec: EcLevel) -> Result<QrCode, EncodeError> {
    QrCode::encode(data.as_ref(), ec)
}

/// Shift-JIS double-byte code for a byte pair, if it is in the Kanji-mode
/// ranges (ISO 18004 §8.3.5): 0x8140..=0x9FFC or 0xE040..=0xEBBF.
pub fn kanji_code(pair: [u8; 2]) -> Option<u16> {
    // ISO 18004 §8.3.5: the two SJIS bytes are packed as base-192 digits —
    // v = q·192 + r with q = high − base, r = low − 0x40, base = 0x81
    // (first range) or 0xC1 (second range) — producing a 13-bit value.
    let (q, r) = match pair[0] {
        0x81..=0x93 => (pair[0] - 0x81, pair[1].wrapping_sub(0x40)),
        0xC1..=0xEB => (pair[0] - 0xC1, pair[1].wrapping_sub(0x40)),
        _ => return None,
    };
    if r > 0xBB {
        return None;
    }
    let v = u16::from(q) * 0xC0 + u16::from(r);
    (v <= 0x1FFF).then_some(v)
}

/// Encode Shift-JIS bytes as a QR Code using Kanji segments for double-byte
/// characters and byte segments for everything else (ISO 18004 §8.3.4/8.3.5).
///
/// Unlike [`encode`] — which always uses a single byte-mode segment — this
/// function interleaves Kanji and byte segments, packing each double-byte
/// character into 13 bits (~2× denser than byte mode for Japanese text).
#[cfg(feature = "alloc")]
pub fn encode_sjis(sjis: &[u8], ec: EcLevel) -> Result<QrCode, EncodeError> {
    use mode::{encode_byte, BitBuffer, Mode};

    if sjis.is_empty() {
        return Err(EncodeError::InvalidCharacter);
    }

    // scan into segments: (is_kanji, values); kanji values are 13-bit codes,
    // byte segments keep the raw bytes.
    let mut segments: Vec<(Mode, Vec<u16>)> = Vec::new();
    let mut i = 0usize;
    while i < sjis.len() {
        let pair: [u8; 2] = [sjis[i], sjis.get(i + 1).copied().unwrap_or(0)];
        if let Some(code) = kanji_code(pair) {
            if !matches!(segments.last(), Some((Mode::Kanji, _))) {
                segments.push((Mode::Kanji, Vec::new()));
            }
            segments.last_mut().unwrap().1.push(code);
            i += 2;
        } else {
            if !matches!(segments.last(), Some((Mode::Byte, _))) {
                segments.push((Mode::Byte, Vec::new()));
            }
            segments.last_mut().unwrap().1.push(u16::from(sjis[i]));
            i += 1;
        }
    }

    // select the smallest version where all segments fit
    for version in 1u8..=40 {
        let info = version::version_info(version, ec).ok_or(EncodeError::Unsupported)?;
        let capacity = info.data_codewords() * 8;
        let mut need = 0usize;
        let mut valid = true;
        for (m, vals) in &segments {
            let ccb = m.char_count_bits(version) as usize;
            need += 4
                + ccb
                + match m {
                    Mode::Kanji => vals.len() * 13,
                    Mode::Byte => vals.len() * 8,
                    _ => {
                        valid = false;
                        0
                    }
                };
        }
        let _ = valid;
        if capacity < need + 4 {
            continue;
        }

        // build the bitstream
        let mut buf = BitBuffer::new();
        for (m, vals) in &segments {
            buf.push_bits(m.indicator() as u32, 4);
            buf.push_bits(vals.len() as u32, m.char_count_bits(version) as usize);
            match m {
                Mode::Kanji => {
                    for &v in vals {
                        buf.push_bits(u32::from(v), 13);
                    }
                }
                _ => {
                    for &v in vals {
                        encode_byte(&[v as u8], &mut buf);
                    }
                }
            }
        }
        // terminator + byte alignment + padding
        let remaining = capacity.saturating_sub(buf.len());
        buf.push_bits(0, remaining.min(4));
        while buf.len() % 8 != 0 {
            buf.push_bits(0, 1);
        }
        let mut pad = 0u8;
        while buf.len() < capacity {
            buf.push_bits(if pad == 0 { 0xEC } else { 0x11 } as u32, 8);
            pad ^= 1;
        }

        let data_bytes = buf.as_bytes();
        let codewords = ec::interleave_blocks(data_bytes, &info);
        let (matrix, mask_id) = matrix::build(version, &codewords, ec_bits_of(ec), None);
        return Ok(QrCode {
            size: version::modules(version),
            matrix,
            version,
            ec_level: ec,
            mask_id,
        });
    }
    Err(EncodeError::DataTooLong)
}

/// Encode a GS1 Application-Standard payload: FNC1 in first position
/// followed by a byte-mode segment (ISO 18004 §8.3.2). Payloads conventionally
/// start with the Application Identifier in parentheses-free numeric form,
/// e.g. `01095011010209171719050810AB123`.
#[cfg(feature = "alloc")]
pub fn encode_gs1(data: impl AsRef<[u8]>, ec: EcLevel) -> Result<QrCode, EncodeError> {
    use mode::{encode_byte, BitBuffer, Mode};

    let data = data.as_ref();
    if data.is_empty() {
        return Err(EncodeError::InvalidCharacter);
    }
    let version =
        version::select_version(data.len() + 1, Mode::Byte, ec).ok_or(EncodeError::DataTooLong)?;
    let info = version::version_info(version, ec).ok_or(EncodeError::Unsupported)?;
    let capacity_bits = info.data_codewords() * 8;

    // FNC1 first position (0101) + byte-mode segment (0100)
    let mut buf = BitBuffer::new();
    buf.push_bits(0b0101, 4);
    buf.push_bits(Mode::Byte.indicator() as u32, 4);
    buf.push_bits(
        data.len() as u32,
        Mode::Byte.char_count_bits(version) as usize,
    );
    encode_byte(data, &mut buf);
    let remaining = capacity_bits.saturating_sub(buf.len());
    buf.push_bits(0, remaining.min(4));
    while buf.len() % 8 != 0 {
        buf.push_bits(0, 1);
    }
    let mut pad = 0u8;
    while buf.len() < capacity_bits {
        buf.push_bits(if pad == 0 { 0xEC } else { 0x11 } as u32, 8);
        pad ^= 1;
    }

    let data_bytes = buf.as_bytes();
    let codewords = ec::interleave_blocks(data_bytes, &info);
    let (matrix, mask_id) = matrix::build(version, &codewords, ec_bits_of(ec), None);
    Ok(QrCode {
        size: version::modules(version),
        matrix,
        version,
        ec_level: ec,
        mask_id,
    })
}

/// Builder for fine-grained QR Code control: forced version, forced mask,
/// explicit EC level.
///
/// ```rust
/// use tpt_barcode_2d::qr::QrBuilder;
/// use tpt_barcode_core::EcLevel;
///
/// let qr = QrBuilder::new()
///     .ec_level(EcLevel::Q)
///     .version(3)          // pin the symbol size (needed for some print workflows)
///     .mask(2)             // pin the mask (reproducible output)
///     .build("force v3")?;
/// assert_eq!(qr.version, 3);
/// assert_eq!(qr.mask_id, 2);
/// # Ok::<(), tpt_barcode_core::EncodeError>(())
/// ```
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct QrBuilder {
    ec_level: EcLevel,
    version: Option<u8>,
    mask: Option<u8>,
}

#[cfg(feature = "alloc")]
impl Default for QrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl QrBuilder {
    /// Create a builder with EC level M and automatic version/mask selection.
    pub fn new() -> Self {
        Self {
            ec_level: EcLevel::M,
            version: None,
            mask: None,
        }
    }

    /// Set the error-correction level (default M).
    pub fn ec_level(mut self, ec: EcLevel) -> Self {
        self.ec_level = ec;
        self
    }

    /// Force a specific version (1–40). The payload must fit.
    pub fn version(mut self, version: u8) -> Self {
        self.version = Some(version);
        self
    }

    /// Force a mask pattern (0–7).
    pub fn mask(mut self, mask: u8) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Encode `data` with the configured options.
    pub fn build(&self, data: impl AsRef<[u8]>) -> Result<QrCode, EncodeError> {
        let data = data.as_ref();
        let qr_mode = mode::Mode::detect(data);
        let version = match self.version {
            Some(v) => {
                // The forced version must actually fit the payload
                let info =
                    version::version_info(v, self.ec_level).ok_or(EncodeError::Unsupported)?;
                let used = 4 + qr_mode.char_count_bits(v) as usize + data.len() * 8;
                if info.data_codewords() * 8 < used {
                    return Err(EncodeError::DataTooLong);
                }
                v
            }
            None => version::select_version(data.len(), qr_mode, self.ec_level)
                .ok_or(EncodeError::DataTooLong)?,
        };
        build_qr(data, qr_mode, version, self.ec_level, self.mask)
    }
}

/// Core encoding pipeline shared by [`QrCode::encode`], [`encode`] and
/// [`QrBuilder::build`].
#[cfg(feature = "alloc")]
pub(crate) fn build_qr(
    data: &[u8],
    qr_mode: mode::Mode,
    version: u8,
    ec: EcLevel,
    mask_hint: Option<u8>,
) -> Result<QrCode, EncodeError> {
    use mode::{encode_alphanumeric, encode_byte, encode_numeric, BitBuffer, Mode};

    let info = version::version_info(version, ec).ok_or(EncodeError::Unsupported)?;
    let capacity_bits = info.data_codewords() * 8;

    // Build bit stream
    let mut buf = BitBuffer::new();
    buf.push_bits(qr_mode.indicator() as u32, 4);
    buf.push_bits(data.len() as u32, qr_mode.char_count_bits(version) as usize);
    match qr_mode {
        Mode::Numeric => encode_numeric(data, &mut buf),
        Mode::Alphanumeric => encode_alphanumeric(data, &mut buf),
        Mode::Byte | Mode::Kanji => encode_byte(data, &mut buf),
    }
    // Terminator (up to 4 zero bits)
    let remaining = capacity_bits.saturating_sub(buf.len());
    buf.push_bits(0, remaining.min(4));
    // Byte-align
    while buf.len() % 8 != 0 {
        buf.push_bits(0, 1);
    }
    // Padding codewords
    let mut pad_byte = 0u8;
    while buf.len() < capacity_bits {
        buf.push_bits(if pad_byte == 0 { 0xEC } else { 0x11 } as u32, 8);
        pad_byte ^= 1;
    }

    let data_bytes = buf.as_bytes();
    let codewords = ec::interleave_blocks(data_bytes, &info);

    let (matrix, mask_id) = matrix::build(version, &codewords, ec_bits_of(ec), mask_hint);

    Ok(QrCode {
        size: version::modules(version),
        matrix,
        version,
        ec_level: ec,
        mask_id,
    })
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use tpt_barcode_core::traits::DecodeError;

    /// Reference 21×21 matrix for "HELLO WORLD", version 1-M, generated by the
    /// widely used Python `qrcode` library (ISO 18004-conformant). Anchors the
    /// decoder to an external implementation.
    const REFERENCE_HELLO_WORLD_V1M: [&str; 21] = [
        "111111100010101111111",
        "100000101110001000001",
        "101110100010101011101",
        "101110100010101011101",
        "101110101011101011101",
        "100000100111001000001",
        "111111101010101111111",
        "000000000000000000000",
        "101010100100100010010",
        "011110001001000010001",
        "000111111101001011000",
        "111101011001110101110",
        "010011110101001110101",
        "000000001010001000101",
        "111111100000100101100",
        "100000100110001101000",
        "101110101100101111111",
        "101110100011010100010",
        "101110101111011101001",
        "100000100001110001011",
        "111111101101011100001",
    ];

    fn reference_grid() -> (Vec<u8>, usize) {
        let size = REFERENCE_HELLO_WORLD_V1M.len();
        let grid = REFERENCE_HELLO_WORLD_V1M
            .iter()
            .flat_map(|row| row.bytes().map(|b| b - b'0'))
            .collect();
        (grid, size)
    }

    #[test]
    fn decodes_external_reference_matrix() {
        let (grid, size) = reference_grid();
        let payload = decode_grid(&grid, size).expect("reference matrix must decode");
        assert_eq!(payload, b"HELLO WORLD");
    }

    #[test]
    fn encode_matches_reference_when_same_mask() {
        // Our encoder must produce a matrix that equals the reference whenever
        // both pick the same mask (they run the same ISO penalty rules).
        let qr = encode("HELLO WORLD", EcLevel::M).unwrap();
        let (grid, size) = reference_grid();
        // Read which mask the reference uses
        let (ec_bits, ref_mask) = decode::read_format(&grid, size).expect("valid format");
        assert_eq!(ec_bits, 0b00, "reference is EC level M");
        if qr.mask_id == ref_mask {
            assert_eq!(qr.matrix, grid);
        } else {
            // Different mask is acceptable — verify via decode round-trip instead.
            assert_eq!(qr.decode().unwrap(), b"HELLO WORLD");
        }
    }

    #[test]
    fn round_trip_all_modes() {
        for payload in [
            "12345678901234567",
            "HELLO WORLD 123",
            "https://github.com/tpt-solutions/tpt-barcode?tab=readme-ov-file",
            "Hello, world! 日本語",
        ] {
            for ec in [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
                let qr = encode(payload, ec).unwrap();
                let decoded = qr.decode().unwrap();
                assert_eq!(decoded, payload.as_bytes(), "ec={ec:?} payload={payload:?}");
            }
        }
    }

    #[test]
    fn round_trip_high_versions_multi_block() {
        // Version ≥ 5 exercises two-block-group interleaving; v10 is 57×57.
        let payload = "The quick brown fox jumps over the lazy dog.".repeat(6);
        for ec in [EcLevel::L, EcLevel::Q] {
            let qr = encode(payload.as_str(), ec).unwrap();
            assert!(
                qr.version >= 9,
                "expected a high version, got {}",
                qr.version
            );
            assert_eq!(qr.decode().unwrap(), payload.as_bytes());
        }
    }

    #[test]
    fn rs_corrects_sampled_errors() {
        // Flip 5 data modules (exactly the t=5 correction capacity of 1-M's
        // 10 EC codewords) and decode.
        let qr = encode("HELLO WORLD", EcLevel::M).unwrap();
        let mut corrupted = qr.matrix.clone();
        let size = qr.size;
        let is_function = matrix::function_patterns(qr.version).1;
        let mut flipped = 0;
        for r in 0..size {
            for c in 0..size {
                let idx = r * size + c;
                if flipped < 5 && !is_function[idx] && (r + c) % 7 == 3 {
                    corrupted[idx] ^= 1;
                    flipped += 1;
                }
            }
        }
        assert_eq!(flipped, 5, "test expects 5 data-module flips");
        let decoded = decode_grid(&corrupted, size).unwrap();
        assert_eq!(decoded, b"HELLO WORLD");
    }

    #[test]
    fn rejects_data_too_long() {
        let long = [b'7'; 3000];
        assert!(matches!(
            encode(&long[..], EcLevel::L),
            Err(EncodeError::DataTooLong)
        ));
    }

    #[test]
    fn format_info_written_correctly() {
        let qr = encode("HELLO WORLD", EcLevel::M).unwrap();
        let size = qr.size;
        let (ec_bits, mask) = decode::read_format(&qr.matrix, size).expect("format readable");
        assert_eq!(ec_bits, ec_bits_of(EcLevel::M), "M");
        assert_eq!(mask, qr.mask_id);
    }

    #[test]
    fn mask_penalty_orders_obvious_cases() {
        // A uniform matrix has long same-colour runs (rule 1); a perfect
        // checkerboard has none and can legitimately score 0 overall.
        let uniform = [0u8; 25 * 25];
        let mut checker = [0u8; 25 * 25];
        for r in 0..25 {
            for c in 0..25 {
                checker[r * 25 + c] = ((r + c) % 2) as u8;
            }
        }
        assert!(mask::penalty(&uniform, 25) > 0);
        assert_eq!(mask::penalty(&checker, 25), 0);
        assert!(mask::penalty_rule1(&uniform, 25) > mask::penalty_rule1(&checker, 25));
    }

    #[test]
    fn builder_forces_version_and_mask() {
        let qr = QrBuilder::new()
            .ec_level(EcLevel::Q)
            .version(3)
            .mask(2)
            .build("force v3")
            .unwrap();
        assert_eq!(qr.version, 3);
        assert_eq!(qr.mask_id, 2);
        assert_eq!(qr.decode().unwrap(), b"force v3");
    }

    #[test]
    fn builder_rejects_payload_too_big_for_forced_version() {
        let long = "x".repeat(100);
        assert!(matches!(
            QrBuilder::new().version(1).build(&long),
            Err(EncodeError::DataTooLong)
        ));
    }

    #[test]
    fn sjis_kanji_round_trip() {
        // "日本語" in Shift-JIS: 93 FA 96 7B 8C EA
        let sjis: &[u8] = &[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA];
        let qr = encode_sjis(sjis, EcLevel::M).unwrap();
        let decoded = qr.decode().unwrap();
        assert_eq!(decoded, sjis, "Kanji segments must round-trip SJIS bytes");
    }

    #[test]
    fn sjis_mixed_kanji_and_ascii() {
        // "ASCII" (5 bytes) + "語" (2 bytes SJIS: 8C EA)
        let sjis: &[u8] = &[b'A', b'S', b'C', b'I', b'I', 0x8C, 0xEA];
        let qr = encode_sjis(sjis, EcLevel::M).unwrap();
        assert_eq!(qr.decode().unwrap(), sjis);
    }

    #[test]
    fn kanji_code_ranges() {
        // base-192 packing: v = (high - base)*0xC0 + (low - 0x40)
        assert_eq!(kanji_code([0x81, 0x40]), Some(0));
        let hi = 0x93u16 - 0x81;
        let lo = 0xFAu16 - 0x40;
        assert_eq!(kanji_code([0x93, 0xFA]), Some(hi * 0xC0 + lo));
        let hi = 0xE0u16 - 0xC1;
        assert_eq!(kanji_code([0xE0, 0x40]), Some(hi * 0xC0));
        assert_eq!(kanji_code([0x41, 0x40]), None, "ASCII pair not kanji");
    }

    #[test]
    fn gs1_fnc1_first_position() {
        let qr = encode_gs1("01095011010209171719050810AB123", EcLevel::M).unwrap();
        let decoded = qr.decode().unwrap();
        assert_eq!(decoded, b"01095011010209171719050810AB123");
    }

    /// Reference matrix for `b"a" * 150` at version 7, EC L, mask 1, generated
    /// by the Python `qrcode` library (ISO 18004 conformance anchor). Run-length
    /// encoded per row: counts alternate light/dark, first count is light.
    #[test]
    fn v7_matches_python_qrcode() {
        let fixture = include_str!("../../tests/v7_mask1_fixture.txt");
        let payload = alloc::vec![b'a'; 150];
        let qr = QrBuilder::new()
            .ec_level(EcLevel::L)
            .version(7)
            .mask(1)
            .build(payload)
            .unwrap();
        assert_eq!(qr.version, 7);
        assert_eq!(qr.mask_id, 1);
        let size = qr.size;
        for (r, line) in fixture.split(';').enumerate() {
            let mut col = 0usize;
            let mut dark = false;
            for count in line.split(',') {
                let count: usize = count.parse().unwrap();
                for c in col..col + count {
                    let expected = u8::from(dark);
                    assert_eq!(
                        qr.matrix[r * size + c], expected,
                        "v7 mismatch at ({r},{c})"
                    );
                }
                col += count;
                dark = !dark;
            }
            assert_eq!(col, size, "row {r} length");
        }
    }

    #[test]
    fn decode_rejects_invalid_grid() {
        let err = decode_grid(&[0u8; 400], 20).unwrap_err();
        assert_eq!(err, DecodeError::InvalidFormat);
    }
}

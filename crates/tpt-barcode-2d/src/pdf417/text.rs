//! PDF417 Text Compaction (ISO/IEC 15438 §5.4.3): four sub-modes
//! (Upper/Lower/Mixed/Punct), two characters per codeword via base-30
//! packing, sub-mode latch codes, and single-character shifts.

use alloc::vec::Vec;

use tpt_barcode_core::traits::DecodeError;

/// Latch: Text Compaction mode.
pub const LATCH_TEXT: u16 = 900;

/// Interim code marking an unassigned position (character not in sub-mode).
const ABSENT: u8 = 255;

/// Upper sub-mode: A–Z (0–25), space (26), latches 27/28, punct shift 29.
const UPPER: [u8; 256] = table_upper();
/// Lower sub-mode: a–z (0–25), space (26), latches 27/28, punct shift 29.
const LOWER: [u8; 256] = table_lower();
/// Mixed sub-mode: digits + symbols (0–24), space (26), latches 25/27/28.
const MIXED: [u8; 256] = table_mixed();
/// Punct sub-mode: punctuation (0–28), alpha latch 29.
const PUNCT: [u8; 256] = table_punct();

const fn table_upper() -> [u8; 256] {
    let mut t = [ABSENT; 256];
    t[b' ' as usize] = 26;
    let mut i = 0usize;
    while i < 26 {
        t[b'A' as usize + i] = i as u8;
        i += 1;
    }
    t
}

const fn table_lower() -> [u8; 256] {
    let mut t = [ABSENT; 256];
    t[b' ' as usize] = 26;
    let mut i = 0usize;
    while i < 26 {
        t[b'a' as usize + i] = i as u8;
        i += 1;
    }
    t
}

const fn table_mixed() -> [u8; 256] {
    let mut t = [ABSENT; 256];
    // digits 0-9 → 0-9
    let mut i = 0usize;
    while i < 10 {
        t[b'0' as usize + i] = i as u8;
        i += 1;
    }
    t[b'&' as usize] = 10;
    t[b'\r' as usize] = 11;
    t[b'\t' as usize] = 12;
    t[b',' as usize] = 13;
    t[b':' as usize] = 14;
    t[b'#' as usize] = 15;
    t[b'-' as usize] = 16;
    t[b'.' as usize] = 17;
    t[b'$' as usize] = 18;
    t[b'/' as usize] = 19;
    t[b'+' as usize] = 20;
    t[b'%' as usize] = 21;
    t[b'*' as usize] = 22;
    t[b'=' as usize] = 23;
    t[b'^' as usize] = 24;
    t[b' ' as usize] = 26;
    t
}

const fn table_punct() -> [u8; 256] {
    let mut t = [ABSENT; 256];
    t[b'\t' as usize] = 12;
    t[b'\n' as usize] = 15;
    t[b'\r' as usize] = 11;
    t[b'!' as usize] = 10;
    t[b'"' as usize] = 20;
    t[b'$' as usize] = 18;
    t[b'\'' as usize] = 28;
    t[b'(' as usize] = 23;
    t[b')' as usize] = 24;
    t[b'*' as usize] = 22;
    t[b',' as usize] = 13;
    t[b'-' as usize] = 16;
    t[b'.' as usize] = 17;
    t[b'/' as usize] = 19;
    t[b':' as usize] = 14;
    t[b';' as usize] = 0;
    t[b'<' as usize] = 1;
    t[b'=' as usize] = 255; // '=' lives in Mixed only
    t[b'>' as usize] = 2;
    t[b'?' as usize] = 25;
    t[b'@' as usize] = 3;
    t[b'[' as usize] = 4;
    t[b'\\' as usize] = 5;
    t[b']' as usize] = 6;
    t[b'_' as usize] = 7;
    t[b'`' as usize] = 8;
    t[b'{' as usize] = 26;
    t[b'|' as usize] = 21;
    t[b'}' as usize] = 27;
    t[b'~' as usize] = 9;
    t
}

/// Reverse tables: interim code -> byte, built from the byte->code tables.
/// `reserved` marks codes that are latches/shifts in that sub-mode.
const fn rev(forward: &[u8; 256], reserved: &[u8]) -> [Option<u8>; 30] {
    let mut t: [Option<u8>; 30] = [None; 30];
    let mut b = 0usize;
    while b < 256 {
        let v = forward[b];
        let mut is_reserved = false;
        let mut r = 0usize;
        while r < reserved.len() {
            if v == reserved[r] {
                is_reserved = true;
            }
            r += 1;
        }
        if v != ABSENT && !is_reserved {
            t[v as usize] = Some(b as u8);
        }
        b += 1;
    }
    t
}

const UPPER_REV: [Option<u8>; 30] = rev(&UPPER, &[27, 28, 29]);
const LOWER_REV: [Option<u8>; 30] = rev(&LOWER, &[27, 28, 29]);
const MIXED_REV: [Option<u8>; 30] = rev(&MIXED, &[25, 27, 28, 29]);
const PUNCT_REV: [Option<u8>; 30] = rev(&PUNCT, &[29]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sm {
    Upper,
    Lower,
    Mixed,
    Punct,
}

/// One interim code's effect: emit a byte, latch a sub-mode, or shift a
/// sub-mode for the next code only (ISO Table 3).
enum Step {
    Byte(u8),
    Latch(Sm),
    Shift(Sm),
}

fn value_for(submode: Sm, b: u8) -> Option<u8> {
    let t = match submode {
        Sm::Upper => &UPPER,
        Sm::Lower => &LOWER,
        Sm::Mixed => &MIXED,
        Sm::Punct => &PUNCT,
    };
    let v = t[b as usize];
    if v == ABSENT {
        None
    } else {
        Some(v)
    }
}

fn byte_for(submode: Sm, h: usize) -> Option<u8> {
    let t = match submode {
        Sm::Upper => &UPPER_REV,
        Sm::Lower => &LOWER_REV,
        Sm::Mixed => &MIXED_REV,
        Sm::Punct => &PUNCT_REV,
    };
    t.get(h).copied().flatten()
}

fn apply(sm: Sm, h: u16) -> Result<Step, DecodeError> {
    let byte = |code: u16| {
        byte_for(sm, code as usize)
            .map(Step::Byte)
            .ok_or(DecodeError::InvalidFormat)
    };
    match (sm, h) {
        (Sm::Upper, 0..=26) => byte(h),
        (Sm::Upper, 27) => Ok(Step::Latch(Sm::Lower)),
        (Sm::Upper, 28) => Ok(Step::Latch(Sm::Mixed)),
        (Sm::Upper, 29) => Ok(Step::Shift(Sm::Punct)),
        (Sm::Lower, 0..=26) => byte(h),
        (Sm::Lower, 27) => Ok(Step::Shift(Sm::Upper)),
        (Sm::Lower, 28) => Ok(Step::Latch(Sm::Mixed)),
        (Sm::Lower, 29) => Ok(Step::Shift(Sm::Punct)),
        (Sm::Mixed, 0..=24 | 26) => byte(h),
        (Sm::Mixed, 25) => Ok(Step::Latch(Sm::Punct)),
        (Sm::Mixed, 27) => Ok(Step::Latch(Sm::Lower)),
        (Sm::Mixed, 28) => Ok(Step::Latch(Sm::Upper)),
        (Sm::Mixed, 29) => Ok(Step::Shift(Sm::Punct)),
        (Sm::Punct, 0..=28) => byte(h),
        (Sm::Punct, 29) => Ok(Step::Latch(Sm::Upper)),
        _ => Err(DecodeError::InvalidFormat),
    }
}

/// Whether every byte of `data` can be encoded in Text compaction.
pub fn is_text_encodable(data: &[u8]) -> bool {
    data.iter()
        .all(|&b| (0x20..=0x7e).contains(&b) || matches!(b, 9 | 10 | 13))
}

/// Encode `data` into text-compaction codewords: latch 900 followed by pairs
/// of interim codes packed as 30a + b (odd count padded with 29, which is a
/// switch code in every sub-mode and adds no data). Encoding starts in Upper
/// and greedily latches sub-modes.
pub fn encode_text(data: &[u8]) -> Result<Vec<u16>, DecodeError> {
    fn latch(from: Sm, to: Sm) -> Option<u16> {
        use Sm::*;
        Some(match (from, to) {
            (Upper, Lower) => 27,
            (Upper, Mixed) => 28,
            (Lower, Mixed) => 28,
            (Mixed, Lower) => 27,
            (Mixed, Upper) => 28,
            (Mixed, Punct) => 25,
            (Punct, Upper) => 29,
            _ => return None,
        })
    }
    fn via(from: Sm, to: Sm) -> Option<Sm> {
        use Sm::*;
        // from Punct, everything goes through an Upper latch first
        match (from, to) {
            (Upper, Punct) | (Lower, Punct) => Some(Sm::Mixed),
            (Punct, Lower) | (Punct, Mixed) => Some(Sm::Upper),
            _ => None,
        }
    }

    let mut out = alloc::vec![LATCH_TEXT];
    let mut interim: Vec<u16> = Vec::with_capacity(data.len());
    let mut sm = Sm::Upper;

    for &b in data {
        let target = match b {
            b'A'..=b'Z' => Sm::Upper,
            b'a'..=b'z' => Sm::Lower,
            _ => {
                if value_for(Sm::Mixed, b).is_some() {
                    Sm::Mixed
                } else {
                    Sm::Punct
                }
            }
        };
        if sm != target {
            if let Some(code) = latch(sm, target) {
                interim.push(code);
                sm = target;
            } else {
                let mid = match via(sm, target) {
                    Some(m) => m,
                    None => panic!("NO VIA from {sm:?} to {target:?} on byte {b}"),
                };
                let c1 = match latch(sm, mid) {
                    Some(v) => v,
                    None => panic!("C1 NONE sm={sm:?} mid={mid:?} b={b}"),
                };
                let c2 = match latch(mid, target) {
                    Some(v) => v,
                    None => panic!("C2 NONE mid={mid:?} target={target:?} b={b}"),
                };
                interim.push(c1);
                interim.push(c2);
                sm = target;
            }
        }
        interim.push(value_for(sm, b).ok_or(DecodeError::InvalidFormat)? as u16);
    }

    // pack pairs
    let mut i = 0usize;
    while i + 1 < interim.len() {
        out.push(interim[i] * 30 + interim[i + 1]);
        i += 2;
    }
    if i < interim.len() {
        out.push(interim[i] * 30 + 29);
    }
    Ok(out)
}

/// Decode text-compaction codewords (the 900-latched segment) into bytes.
pub fn decode_text(codewords: &[u16]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut sm = Sm::Upper;
    let mut shift: Option<Sm> = None;

    for &cw in codewords {
        if cw >= 900 {
            return Err(DecodeError::InvalidFormat);
        }
        for h in [cw / 30, cw % 30] {
            let active = shift.unwrap_or(sm);
            match apply(active, h)? {
                Step::Byte(b) => out.push(b),
                Step::Latch(next) => sm = next,
                Step::Shift(next) => shift = Some(next),
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_printable_ascii() {
        let payloads: [&[u8]; 6] = [
            b"Hello, PDF417 text compaction!",
            b"Mixed CASE 123 with punctuation?!",
            b"tab\tnewline\nreturn\r chars",
            b"ALL CAPS NO LOWERCASE",
            b"no-uppercase at all except X",
            b"symbols: # $ % & * + - . / : = ^",
        ];
        for (idx, &payload) in payloads.iter().enumerate() {
            let cw = encode_text(payload).unwrap_or_else(|e| panic!("idx {idx} ENC {e:?}"));
            let d = decode_text(&cw[1..]).unwrap_or_else(|e| panic!("idx {idx} DEC {e:?}"));
            assert_eq!(d, payload, "idx {idx}");
        }
    }
}

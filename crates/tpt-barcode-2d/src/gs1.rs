//! GS1 Application Identifier (AI) parsing for FNC1-carrying symbols.
//!
//! GS1 element strings are `AI + value` sequences where the AI is 2–4 digits
//! and the value is either fixed-length (per the GS1 general specifications)
//! or terminated by an FNC1 separator (ASCII 29 / 0x1D in the decoded
//! payload; the codeword-level FNC1 was consumed by the symbol itself).
//!
//! [`parse`] splits such a string into `(AI, value)` pairs.

/// Value kind for a known AI: fixed byte length, or variable (runs until
/// the FNC1 separator or the end of the payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Fixed(usize),
    Variable,
}

/// Value kind for a known GS1 Application Identifier: fixed byte length, or
/// variable (runs until the FNC1 separator or the end of the payload).
/// `num_digits` is the AI's digit length (2–4); an AI candidate is only
/// recognized when the table has an entry for its digit length.
fn ai_kind(ai: u16, num_digits: usize) -> Option<ValueKind> {
    match (num_digits, ai) {
        // 2-digit AIs (match values are the AI digits as a number:
        // 0 = "00", 1|2 = "01"|"02", ...)
        (2, 0) => Some(ValueKind::Fixed(18)),      // SSCC
        (2, 1 | 2) => Some(ValueKind::Fixed(14)),  // GTIN / contained GTIN
        (2, 10) => Some(ValueKind::Variable),      // batch/lot
        (2, 11..=17) => Some(ValueKind::Fixed(6)), // dates
        (2, 20) => Some(ValueKind::Fixed(2)),      // variant
        (2, 21 | 22) => Some(ValueKind::Variable), // serial / consumer variant
        (2, 30 | 37) => Some(ValueKind::Variable), // count
        (2, 90..=99) => Some(ValueKind::Variable), // internal
        // 3-digit AIs (n = decimal position / qualifier digit)
        (3, 310..=369) => Some(ValueKind::Fixed(6)), // measures
        (3, 390..=393) => Some(ValueKind::Variable), // amount / price
        // 4-digit AIs
        (4, 8005) => Some(ValueKind::Fixed(6)), // price per unit
        (4, 8017 | 8018) => Some(ValueKind::Fixed(18)), // GSRN
        (4, 4000..=4999) => Some(ValueKind::Variable), // order, invoice, ...
        (4, 7000..=7099) => Some(ValueKind::Variable), // healthcare rebate etc.
        _ => None,
    }
}

/// One parsed GS1 element: Application Identifier + raw value bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiElement {
    /// Application Identifier (2–4 digits).
    pub ai: u16,
    /// Raw value bytes (ASCII for standard AIs; binary payloads are passed
    /// through unchanged).
    pub value: Vec<u8>,
}

impl AiElement {
    /// Value as human-readable text (lossy for non-ASCII bytes).
    pub fn text(&self) -> alloc::string::String {
        alloc::string::String::from_utf8_lossy(&self.value).into_owned()
    }
}

use alloc::vec::Vec;

const FNC1_SEPARATOR: u8 = 0x1D; // ASCII 29 (GS)

/// Parse a decoded GS1 element string into `(AI, value)` pairs.
///
/// Accepts the payload as returned by the QR/DataMatrix decoders for
/// FNC1-carrying symbols: AI digits, then the value, with an optional FNC1
/// separator byte (0x1D) terminating variable-length elements.
pub fn parse(data: &[u8]) -> Result<Vec<AiElement>, tpt_barcode_core::traits::DecodeError> {
    use tpt_barcode_core::traits::DecodeError;

    let mut out = Vec::new();
    let mut i = 0usize;

    while i < data.len() {
        // take the longest known AI prefix (2-4 digits)
        let mut ai: Option<(u16, usize)> = None;
        for len in (2..=4).rev() {
            if i + len <= data.len() {
                let digits = &data[i..i + len];
                if digits.iter().all(|d| d.is_ascii_digit()) {
                    let v: u16 = digits
                        .iter()
                        .fold(0u16, |a, &d| a * 10 + u16::from(d - b'0'));
                    if ai_kind(v, len).is_some() {
                        ai = Some((v, len));
                        break;
                    }
                }
            }
        }
        let (ai, ai_len) = match ai {
            Some(a) => a,
            None => return Err(DecodeError::InvalidFormat),
        };
        i += ai_len;

        // value length: fixed per AI, or variable until FNC1/segment end
        let value = match ai_kind(ai, ai_len) {
            Some(ValueKind::Fixed(len)) => {
                let end = (i + len).min(data.len());
                let v = &data[i..end];
                i = end;
                v
            }
            _ => {
                let start = i;
                while i < data.len() && data[i] != FNC1_SEPARATOR {
                    i += 1;
                }
                &data[start..i]
            }
        };
        if value.is_empty() {
            return Err(DecodeError::InvalidFormat);
        }

        // consume the FNC1 separator between elements
        if i < data.len() && data[i] == FNC1_SEPARATOR {
            i += 1;
        }

        out.push(AiElement {
            ai,
            value: value.to_vec(),
        });
    }
    Ok(out)
}

/// Convenience: parse and return the elements as `(AI, text)` pairs.
pub fn parse_text(
    data: &[u8],
) -> Result<Vec<(u16, alloc::string::String)>, tpt_barcode_core::traits::DecodeError> {
    Ok(parse(data)?
        .into_iter()
        .map(|el| (el.ai, el.text()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    /// GS1 element string: (01) GTIN-14, (10) batch — the batch value runs
    /// to the end of the payload.
    #[test]
    fn parses_gtin_and_batch() {
        let data = b"010614141000009710AB123";
        let els = parse(data).unwrap();
        assert_eq!(els.len(), 2);
        assert_eq!(els[0].ai, 1);
        assert_eq!(els[0].value, b"06141410000097");
        assert_eq!(els[1].ai, 10);
        assert_eq!(els[1].value, b"AB123");
    }

    /// AI 17 (expiration date) is fixed 6 digits: the next AI starts without
    /// any separator.
    #[test]
    fn parses_fixed_length_date_then_batch() {
        let data = b"1719081510ABC";
        let els = parse(data).unwrap();
        assert_eq!(els.len(), 2);
        assert_eq!(els[0].ai, 17);
        assert_eq!(els[0].value, b"190815");
        assert_eq!(els[1].ai, 10);
        assert_eq!(els[1].value, b"ABC");
    }

    /// Batch-only payload.
    #[test]
    fn parses_batch_only() {
        let els = parse(b"10AB123").unwrap();
        assert_eq!(els.len(), 1);
        assert_eq!(els[0].ai, 10);
        assert_eq!(els[0].value, b"AB123");
    }

    #[test]
    fn text_pairs() {
        // (17)190815 + (10)ABC
        let data = b"1719081510ABC";
        let pairs = parse_text(data).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (17, "190815".to_string()));
        assert_eq!(pairs[1], (10, "ABC".to_string()));
    }

    #[test]
    fn rejects_short_or_missing_ai() {
        assert!(parse(b"1").is_err());
        // an empty payload parses to an empty element list
        assert!(parse(b"").unwrap().is_empty());
    }
}

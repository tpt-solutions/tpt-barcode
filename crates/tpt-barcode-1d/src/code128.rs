//! Code 128 barcode — subsets A (control chars), B (ASCII printable), C (digit pairs).

use tpt_barcode_core::traits::EncodeError;

/// Encoded Code 128 bar pattern as a sequence of module widths.
///
/// Each value is a module width (1–4). Bars and spaces alternate starting
/// with a bar. Total width = sum of all elements.
#[cfg(feature = "alloc")]
pub struct Code128 {
    /// Module widths (bar, space, bar, space, …).
    pub modules: alloc::vec::Vec<u8>,
}

// Symbol table for Code 128 Subset B (index = code value).
// Each entry is a 6-element [bar1, sp1, bar2, sp2, bar3, sp3] pattern.
const SYMBOLS: [[u8; 6]; 109] = [
    [2, 1, 2, 2, 2, 2],
    [2, 2, 2, 1, 2, 2],
    [2, 2, 2, 2, 2, 1],
    [1, 2, 1, 2, 2, 3],
    [1, 2, 1, 3, 2, 2],
    [1, 3, 1, 2, 2, 2],
    [1, 2, 2, 2, 1, 3],
    [1, 2, 2, 3, 1, 2],
    [1, 3, 2, 2, 1, 2],
    [2, 2, 1, 2, 1, 3],
    [2, 2, 1, 3, 1, 2],
    [2, 3, 1, 2, 1, 2],
    [1, 1, 2, 2, 3, 2],
    [1, 2, 2, 1, 3, 2],
    [1, 2, 2, 2, 3, 1],
    [1, 1, 3, 2, 2, 2],
    [1, 2, 3, 1, 2, 2],
    [1, 2, 3, 2, 2, 1],
    [2, 2, 3, 2, 1, 1],
    [2, 2, 1, 1, 3, 2],
    [2, 2, 1, 2, 3, 1],
    [2, 1, 3, 2, 1, 2],
    [2, 2, 3, 1, 1, 2],
    [3, 1, 2, 1, 3, 1],
    [3, 1, 1, 2, 2, 2],
    [3, 2, 1, 1, 2, 2],
    [3, 2, 1, 2, 2, 1],
    [3, 1, 2, 2, 1, 2],
    [3, 2, 2, 1, 1, 2],
    [3, 2, 2, 2, 1, 1],
    [2, 1, 2, 1, 2, 3],
    [2, 1, 2, 3, 2, 1],
    [2, 3, 2, 1, 2, 1],
    [1, 1, 1, 3, 2, 3],
    [1, 3, 1, 1, 2, 3],
    [1, 3, 1, 3, 2, 1],
    [1, 1, 2, 3, 1, 3],
    [1, 3, 2, 1, 1, 3],
    [1, 3, 2, 3, 1, 1],
    [2, 1, 1, 3, 1, 3],
    [2, 3, 1, 1, 1, 3],
    [2, 3, 1, 3, 1, 1],
    [1, 1, 2, 1, 3, 3],
    [1, 1, 2, 3, 3, 1],
    [1, 3, 2, 1, 3, 1],
    [1, 1, 3, 1, 2, 3],
    [1, 1, 3, 3, 2, 1],
    [1, 3, 3, 1, 2, 1],
    [3, 1, 3, 1, 2, 1],
    [2, 1, 1, 3, 3, 1],
    [2, 3, 1, 1, 3, 1],
    [2, 1, 3, 1, 1, 3],
    [2, 1, 3, 3, 1, 1],
    [2, 1, 3, 1, 3, 1],
    [3, 1, 1, 1, 2, 3],
    [3, 1, 1, 3, 2, 1],
    [3, 3, 1, 1, 2, 1],
    [3, 1, 2, 1, 1, 3],
    [3, 1, 2, 3, 1, 1],
    [3, 3, 2, 1, 1, 1],
    [3, 1, 4, 1, 1, 1],
    [2, 2, 1, 4, 1, 1],
    [4, 3, 1, 1, 1, 1],
    [1, 1, 1, 2, 2, 4],
    [1, 1, 1, 4, 2, 2],
    [1, 2, 1, 1, 2, 4],
    [1, 2, 1, 4, 2, 1],
    [1, 4, 1, 1, 2, 2],
    [1, 4, 1, 2, 2, 1],
    [1, 1, 2, 2, 1, 4],
    [1, 1, 2, 4, 1, 2],
    [1, 2, 2, 1, 1, 4],
    [1, 2, 2, 4, 1, 1],
    [1, 4, 2, 1, 1, 2],
    [1, 4, 2, 2, 1, 1],
    [2, 4, 1, 2, 1, 1],
    [2, 2, 1, 1, 1, 4],
    [4, 1, 3, 1, 1, 1],
    [2, 4, 1, 1, 1, 2],
    [1, 3, 4, 1, 1, 1],
    [1, 1, 1, 2, 4, 2],
    [1, 2, 1, 1, 4, 2],
    [1, 2, 1, 2, 4, 1],
    [1, 1, 4, 2, 1, 2],
    [1, 2, 4, 1, 1, 2],
    [1, 2, 4, 2, 1, 1],
    [4, 1, 1, 2, 1, 2],
    [4, 2, 1, 1, 1, 2],
    [4, 2, 1, 2, 1, 1],
    [2, 1, 2, 1, 4, 1],
    [2, 1, 4, 1, 2, 1],
    [4, 1, 2, 1, 2, 1],
    [1, 1, 1, 1, 4, 3],
    [1, 1, 1, 3, 4, 1],
    [1, 3, 1, 1, 4, 1],
    [1, 1, 4, 1, 1, 3],
    [1, 1, 4, 3, 1, 1],
    [4, 1, 1, 1, 1, 3],
    [4, 1, 1, 3, 1, 1],
    [1, 1, 3, 1, 4, 1],
    [1, 1, 4, 1, 3, 1],
    [3, 1, 1, 1, 4, 1],
    [4, 1, 1, 1, 3, 1],
    [2, 1, 1, 4, 1, 2],
    [2, 1, 1, 2, 1, 4],
    [2, 1, 1, 2, 3, 2],
    // Special: Start B=104, Start C=105, Stop=106
    [2, 1, 1, 4, 1, 2],
    [2, 1, 1, 2, 1, 4],
    [2, 3, 3, 1, 1, 1],
];

// Positions in the extended SYMBOLS table where the special characters live.
const START_B: u8 = 106;
const STOP: u8 = 108;

/// Encode `data` (printable ASCII 0x20–0x7E) as a Code 128 Subset B barcode.
#[cfg(feature = "alloc")]
pub fn encode_b(data: &[u8]) -> Result<Code128, EncodeError> {
    // Validate input
    if data.iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
        return Err(EncodeError::InvalidCharacter);
    }

    let mut modules = alloc::vec::Vec::new();
    let mut checksum = START_B as u32;

    // Start symbol B
    push_symbol(&mut modules, START_B);

    // Data symbols
    for (i, &b) in data.iter().enumerate() {
        let code_val = b - 0x20; // Subset B: space=0, !+1…~=94
        checksum += code_val as u32 * (i as u32 + 1);
        push_symbol(&mut modules, code_val);
    }

    // Check symbol (value mod 103)
    let check_val = (checksum % 103) as u8;
    push_symbol(&mut modules, check_val);

    // Stop symbol + extra bar for termination
    push_symbol(&mut modules, STOP);
    modules.push(2); // extra 2-module bar to complete stop pattern

    Ok(Code128 { modules })
}

fn push_symbol(modules: &mut alloc::vec::Vec<u8>, code: u8) {
    let sym = SYMBOLS[code as usize];
    modules.extend_from_slice(&sym);
}

/// Scan a Code 128 barcode from module widths and decode the text (Subset B).
///
/// `widths`: alternating bar/space widths starting with a bar.
#[cfg(feature = "alloc")]
pub fn decode_b(
    widths: &[u8],
) -> Result<alloc::string::String, tpt_barcode_core::traits::DecodeError> {
    use tpt_barcode_core::traits::DecodeError;

    if widths.len() < 13 {
        return Err(DecodeError::InvalidFormat);
    }

    // Find start B pattern
    if !symbol_matches(&widths[..6], START_B) {
        return Err(DecodeError::InvalidFormat);
    }

    let data_symbols = &widths[6..widths.len().saturating_sub(7)];
    let n_syms = data_symbols.len() / 6;
    let mut out = alloc::string::String::new();
    let mut checksum = START_B as u32;

    for i in 0..n_syms.saturating_sub(1) {
        let sym = &data_symbols[i * 6..(i + 1) * 6];
        let code_val = find_symbol(sym).ok_or(DecodeError::InvalidFormat)?;
        checksum += code_val as u32 * (i as u32 + 1);
        out.push((code_val + 0x20) as char);
    }

    // Check digit
    if n_syms > 0 {
        let check_sym = &data_symbols[(n_syms - 1) * 6..n_syms * 6];
        let check_val = find_symbol(check_sym).ok_or(DecodeError::InvalidFormat)?;
        if (checksum % 103) as u8 != check_val {
            return Err(DecodeError::TooManyErrors);
        }
    }

    Ok(out)
}

fn symbol_matches(widths: &[u8], code: u8) -> bool {
    let sym = SYMBOLS[code as usize];
    widths.len() == 6 && widths.iter().zip(sym.iter()).all(|(&a, &b)| a == b)
}

fn find_symbol(widths: &[u8]) -> Option<u8> {
    SYMBOLS
        .iter()
        .position(|s| s.iter().eq(widths.iter()))
        .map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_hello() {
        let bc = encode_b(b"HELLO").unwrap();
        // Should produce a non-empty module sequence
        assert!(!bc.modules.is_empty());
        // All widths should be 1–4
        assert!(bc.modules.iter().all(|&w| w >= 1 && w <= 4));
    }

    #[test]
    fn encode_invalid_char_fails() {
        assert!(encode_b(&[0x01]).is_err());
    }
}

//! Run-length input for 1D symbologies.
//!
//! Optical scanners (and the image pipeline in `tpt-barcode-image`) see bar
//! codes as alternating dark/light **run lengths** rather than per-module
//! arrays. The helpers here convert run lengths into the module arrays and
//! width vectors the per-symbology decoders expect, so a scanline can be
//! decoded directly:
//!
//! ```rust
//! use tpt_barcode_1d::runs::{self, modules_to_runs};
//!
//! // Encode digits 4006381333931 (12 digits + auto checksum) at 1 px/module.
//! let encoded =
//!     tpt_barcode_1d::ean13::encode(&[4, 0, 0, 6, 3, 8, 1, 3, 3, 3, 9, 3])
//!         .unwrap();
//! let runs = modules_to_runs(&encoded.modules);
//! let digits = runs::decode_ean13_runs(&runs).unwrap();
//! assert_eq!(digits, encoded.digits);
//! ```
//!
//! (The exact run pattern above is illustrative — see the round-trip tests
//! in the symbology modules for known-good inputs.)

use alloc::vec::Vec;
use tpt_barcode_core::traits::DecodeError;

/// Convert a binary module array (1 = dark) to alternating run lengths,
/// starting with a dark run. Returns `None` for an empty array.
pub fn modules_to_runs(modules: &[u8]) -> Vec<u32> {
    let mut runs = Vec::new();
    let mut current = 0u32;
    let mut dark = true;
    for &m in modules {
        if (m != 0) == dark {
            current += 1;
        } else {
            runs.push(current);
            current = 1;
            dark = !dark;
        }
    }
    if current > 0 {
        runs.push(current);
    }
    runs
}

/// Normalize alternating run lengths so the narrowest run becomes unit 1.
///
/// Each run is scaled by `min(run)` and rounded. Noise rejection happens at
/// the next stage — `runs_to_modules` validates the total module count, and
/// the symbology decoders validate structure and checksums.
/// Returns `None` when `runs` is empty or contains a zero-length run.
pub fn normalize_runs(runs: &[u32]) -> Option<Vec<u8>> {
    let unit = runs.iter().copied().min()?;
    if unit == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(runs.len());
    for &r in runs {
        // Clamp to u8 (Code 128 elements are ≤ 4 units; larger values only
        // occur from extreme resolutions, which decoders reject anyway).
        // Integer math keeps this `no_std`-safe (no `f32::round`).
        let scaled = (r + unit / 2) / unit; // round-half-up
        out.push(scaled.clamp(1, 255) as u8);
    }
    Some(out)
}

/// Expand alternating run lengths into a binary module array of `total`
/// modules (1 = dark, starting with a dark run). Returns `None` if the runs
/// do not sum to exactly `total` modules.
pub fn runs_to_modules(runs: &[u32], total: usize) -> Option<Vec<u8>> {
    let normalized = normalize_runs(runs)?;
    let sum: usize = normalized.iter().map(|&w| w as usize).sum();
    if sum != total {
        return None;
    }
    let mut out = Vec::with_capacity(total);
    for (i, &w) in normalized.iter().enumerate() {
        let dark = i % 2 == 0; // runs start with a dark bar
        out.extend(core::iter::repeat_n(if dark { 1 } else { 0 }, w as usize));
    }
    Some(out)
}

/// Decode an EAN-13 symbol from alternating run lengths. Returns the 13
/// digit ASCII values.
pub fn decode_ean13_runs(runs: &[u32]) -> Result<[u8; 13], DecodeError> {
    let modules = runs_to_modules(runs, 95).ok_or(DecodeError::InvalidFormat)?;
    let mut arr = [0u8; 95];
    arr.copy_from_slice(&modules);
    crate::ean13::decode(&arr)
}

/// Decode a UPC-A symbol from alternating run lengths. Returns the 12
/// digit ASCII values.
pub fn decode_upca_runs(runs: &[u32]) -> Result<[u8; 12], DecodeError> {
    let modules = runs_to_modules(runs, 95).ok_or(DecodeError::InvalidFormat)?;
    let mut arr = [0u8; 95];
    arr.copy_from_slice(&modules);
    crate::upca::decode(&arr)
}

/// Decode a Code 39 symbol from alternating run lengths.
pub fn decode_code39_runs(runs: &[u32]) -> Result<alloc::string::String, DecodeError> {
    // Code 39 decoders work on wide/narrow widths; keep raw proportions but
    // clamp to the u8 the decoder accepts.
    let mut widths = Vec::with_capacity(runs.len());
    for &r in runs {
        widths.push(r.min(255) as u8);
    }
    crate::code39::decode(&widths)
}

/// Decode a Code 128 (Subset B) symbol from alternating run lengths.
pub fn decode_code128_runs(runs: &[u32]) -> Result<alloc::string::String, DecodeError> {
    let mut widths = Vec::with_capacity(runs.len());
    for &r in runs {
        widths.push(r.min(255) as u8);
    }
    crate::code128::decode_b(&widths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_scales_to_unit() {
        let runs = [4u32, 4, 8, 4];
        assert_eq!(normalize_runs(&runs), Some(alloc::vec![1, 1, 2, 1]));
    }

    #[test]
    fn normalize_rejects_zero_and_empty() {
        assert_eq!(normalize_runs(&[0u32, 1]), None);
        assert_eq!(normalize_runs(&[]), None);
        // Non-integer ratios still normalize (rounded); the module-count
        // check in runs_to_modules provides the noise rejection.
        assert_eq!(normalize_runs(&[2u32, 3]), Some(alloc::vec![1, 2]));
    }

    #[test]
    fn runs_to_modules_expands_and_validates() {
        let runs = [1u32, 1, 2, 2];
        assert_eq!(
            runs_to_modules(&runs, 6),
            Some(alloc::vec![1, 0, 1, 1, 0, 0])
        );
        assert_eq!(runs_to_modules(&runs, 5), None); // wrong total
    }
}

//! Reed-Solomon encoder and decoder over GF(256).
//!
//! Encoder: polynomial long division to produce error-correction codewords.
//! Decoder: Berlekamp-Massey (error locator polynomial) → Chien search
//! (error positions) → Forney algorithm (error magnitudes).
//!
//! The engine is generic over the GF(256) [`Field`](crate::gf256::Field) and
//! the syndrome start exponent: QR Code uses the 0x11D field with generator
//! roots α^0…α^(n-1); Data Matrix (ECC 200) uses 0x12D with roots α^1…α^n.
//!
//! All operations are `no_std` / allocation-free. Internal scratch buffers use
//! compile-time maximum sizes. QR Code's largest EC block is 68 codewords
//! (version 40-H), which governs the constants below.

// Polynomial arithmetic over scratch buffers is index-centric; iterators
// obscure the BM / Chien / Forney algorithm structure.
#![allow(clippy::needless_range_loop)]

use crate::gf256::{add, Field, QrField};

/// Maximum EC codewords per block (QR Code v40-H).
pub const MAX_EC: usize = 68;
/// Maximum message length we can decode (RS(255,k) worst case).
const MAX_BLOCK: usize = 255;

/// Errors returned by the RS decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsError {
    /// More errors than the code can correct.
    TooManyErrors,
    /// Error locator polynomial has no valid roots in the received word.
    ChienSearchFailed,
}

// ── Generator polynomial ────────────────────────────────────────────────────

/// Write the RS generator polynomial for `n_ec` EC codewords into `out[..=n_ec]`:
/// `g(x) = ∏_{i=start}^{start+n_ec-1} (x + α^i)` where `+` is XOR in GF(256).
fn generator_poly<F: Field>(n_ec: usize, start: usize, out: &mut [u8]) {
    debug_assert!(n_ec <= MAX_EC);
    debug_assert!(out.len() > n_ec);
    for b in out[..=n_ec].iter_mut() {
        *b = 0;
    }
    out[0] = 1;
    for i in start..start + n_ec {
        let ai = F::exp(i % 255); // α^i
                                  // multiply current poly by (x + α^i): degree increases by 1
        for j in (1..=i - start + 1).rev() {
            out[j] = add(out[j - 1], F::mul(out[j], ai));
        }
        out[0] = F::mul(out[0], ai);
    }
}

// ── Encoder ─────────────────────────────────────────────────────────────────

/// Compute `n_ec` Reed-Solomon error-correction codewords for `data` (QR Code
/// convention: 0x11D field, generator roots α^0…α^(n_ec-1)) and write them into
/// `out[..n_ec]`.
///
/// `out` must have length ≥ `n_ec` and ≤ [`MAX_EC`] = 68.
pub fn encode(data: &[u8], n_ec: usize, out: &mut [u8]) {
    encode_with::<QrField>(data, n_ec, 0, out);
}

/// Field-explicit variant of [`encode`]. `root_start` is the exponent of the
/// first generator root (0 for QR Code, 1 for Data Matrix).
pub fn encode_with<F: Field>(data: &[u8], n_ec: usize, root_start: usize, out: &mut [u8]) {
    debug_assert!(n_ec <= MAX_EC, "n_ec exceeds maximum of {MAX_EC}");
    debug_assert!(out.len() >= n_ec);

    let mut gen = [0u8; MAX_EC + 1];
    generator_poly::<F>(n_ec, root_start, &mut gen);

    for b in out[..n_ec].iter_mut() {
        *b = 0;
    }

    for &byte in data {
        let factor = add(byte, out[0]);
        for j in 0..n_ec - 1 {
            // gen is stored with leading coeff at index 0; gen[n_ec] is the constant term
            out[j] = add(out[j + 1], F::mul(factor, gen[n_ec - 1 - j]));
        }
        out[n_ec - 1] = F::mul(factor, gen[0]);
    }
}

// ── Decoder ─────────────────────────────────────────────────────────────────

/// Attempt to correct errors in `received` in-place (QR Code convention).
/// Returns the number of errors corrected, or [`RsError`] if correction is not
/// possible.
///
/// `n_ec` is the number of EC codewords appended to the message; `t = n_ec/2`
/// is the error-correction capacity.
pub fn decode(received: &mut [u8], n_ec: usize) -> Result<usize, RsError> {
    decode_with::<QrField>(received, n_ec, 0)
}

/// Field-explicit variant of [`decode`]. `syndrome_start` is the exponent of
/// the first syndrome evaluation point (0 for QR Code, 1 for Data Matrix),
/// i.e. syndromes are `S_j = R(α^(syndrome_start + j))` for `j = 0..n_ec`.
pub fn decode_with<F: Field>(
    received: &mut [u8],
    n_ec: usize,
    syndrome_start: usize,
) -> Result<usize, RsError> {
    debug_assert!(n_ec <= MAX_EC);
    debug_assert!(received.len() <= MAX_BLOCK);

    // 1. Compute syndromes S_j = R(α^(syndrome_start + j))
    let mut syndromes = [0u8; MAX_EC];
    for j in 0..n_ec {
        let x = F::exp((syndrome_start + j) % 255);
        let mut s = 0u8;
        for &byte in received.iter() {
            s = add(F::mul(s, x), byte);
        }
        syndromes[j] = s;
    }

    // If all syndromes are zero, no errors.
    if syndromes[..n_ec].iter().all(|&s| s == 0) {
        return Ok(0);
    }

    // 2. Berlekamp-Massey: find error locator polynomial σ(x).
    //    Uses the iterative BM algorithm; σ[0] = 1 always.
    let t = n_ec / 2;
    let mut sigma = [0u8; MAX_EC + 1]; // error locator
    let mut prev = [0u8; MAX_EC + 1]; // previous sigma
    sigma[0] = 1;
    prev[0] = 1;
    let mut l = 0usize; // current length of sigma
    let mut m = 1usize; // shift register
    let mut bm = 1u8; // previous discrepancy denominator

    for n in 0..n_ec {
        // Compute discrepancy delta
        let mut delta = syndromes[n];
        for i in 1..=l {
            delta = add(delta, F::mul(sigma[i], syndromes[n - i]));
        }
        if delta == 0 {
            m += 1;
            continue;
        }
        let tmp = sigma;
        let scale = F::mul(delta, F::inv(bm));
        // σ_new = σ - scale * x^m * b
        for i in m..=l + m {
            sigma[i] = add(sigma[i], F::mul(scale, prev[i - m]));
        }
        if 2 * l <= n {
            l = n + 1 - l;
            prev = tmp;
            bm = delta;
            m = 1;
        } else {
            m += 1;
        }
    }

    let nu = l; // number of errors
    if nu > t {
        return Err(RsError::TooManyErrors);
    }

    // 3. Chien search: find roots of σ(x) → error positions.
    let n = received.len();
    let mut err_pos = [0usize; MAX_EC / 2 + 1];
    let mut found = 0usize;

    for i in 0..n {
        // Evaluate σ at α^(-i) = α^(255-i)
        let xi_inv = F::exp(255 - i % 255);
        let mut val = 0u8;
        let mut xpow = 1u8;
        for k in 0..=nu {
            val = add(val, F::mul(sigma[k], xpow));
            xpow = F::mul(xpow, xi_inv);
        }
        if val == 0 {
            err_pos[found] = i;
            found += 1;
        }
    }

    if found != nu {
        return Err(RsError::ChienSearchFailed);
    }

    // 4. Forney algorithm: compute error magnitudes.
    //    Omega(x) = S(x) * σ(x) mod x^n_ec  (error evaluator polynomial)
    let mut omega = [0u8; MAX_EC];
    for i in 0..n_ec {
        for j in 0..=nu {
            if i + j < n_ec {
                omega[i + j] = add(omega[i + j], F::mul(syndromes[i], sigma[j]));
            }
        }
    }

    // σ'(x) = formal derivative of σ (in GF(2^8), d/dx x^k = 0 for even k, x^(k-1) for odd k)
    // i.e., sigma'[i] = sigma[i+1] for i even, 0 for i odd (0-indexed from degree 0)
    let mut sigma_prime = [0u8; MAX_EC + 1];
    for i in (0..nu).step_by(2) {
        sigma_prime[i] = sigma[i + 1];
    }

    // Apply corrections. The magnitude carries an X^(1-b) factor where b is
    // the syndrome start exponent: X^1 for QR Code (b=0), 1 for Data Matrix
    // (b=1).
    for k in 0..found {
        let pos = err_pos[k];
        let xi_inv = F::exp((255 - pos % 255) % 255);

        // Evaluate omega at xi_inv
        let mut omega_val = 0u8;
        let mut xpow = 1u8;
        for j in 0..n_ec {
            omega_val = add(omega_val, F::mul(omega[j], xpow));
            xpow = F::mul(xpow, xi_inv);
        }

        // Evaluate sigma_prime at xi_inv
        let mut sp_val = 0u8;
        let mut xpow = 1u8;
        for j in 0..nu {
            sp_val = add(sp_val, F::mul(sigma_prime[j], xpow));
            xpow = F::mul(xpow, xi_inv);
        }

        if sp_val == 0 {
            return Err(RsError::TooManyErrors);
        }

        // α^(pos·(1-b) mod 255): with b=0 → α^pos, with b=1 → 1
        let x_factor = F::exp((pos % 255) * (255 + 1 - syndrome_start) % 255);
        let magnitude = F::mul(x_factor, F::div(omega_val, sp_val));
        received[n - 1 - pos] = add(received[n - 1 - pos], magnitude);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gf256::DmField;

    #[test]
    fn encode_known_qr_v1m() {
        // QR Code version 1-M "HELLO WORLD" (alphanumeric), the worked example in
        // thonky.com's QR tutorial: 16 data codewords produce 10 EC codewords.
        // Data: 0100 00001011 … padded with 11101100/00010001 alternating bytes.
        let data: &[u8] = &[
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        let mut ec = [0u8; 10];
        encode(data, 10, &mut ec);
        assert_eq!(ec, [196, 35, 39, 119, 235, 215, 231, 226, 93, 23]);
    }

    #[test]
    fn encode_known_datamatrix() {
        // Data Matrix ECC 200 "123456" at 10×10: three ASCII digit-pair
        // codewords (130 + value), 5 EC codewords over the 0x12D field with
        // generator roots α^1…α^5.
        let data: &[u8] = &[142, 164, 186];
        let mut ec = [0u8; 5];
        encode_with::<DmField>(data, 5, 1, &mut ec);
        // Reference EC codewords for "123456" (ISO/IEC 16022 worked example,
        // cross-checked with the zxing reference implementation).
        assert_eq!(ec, [114, 25, 5, 88, 102]);
    }

    #[test]
    fn decode_no_errors() {
        let data: &[u8] = &[
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        let mut ec = [0u8; 10];
        encode(data, 10, &mut ec);

        let mut received = [0u8; 26];
        received[..16].copy_from_slice(data);
        received[16..].copy_from_slice(&ec);
        let n_errors = decode(&mut received, 10).unwrap();
        assert_eq!(n_errors, 0);
        assert_eq!(&received[..16], data);
    }

    #[test]
    fn decode_single_error() {
        let data: &[u8] = &[
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        let mut ec = [0u8; 10];
        encode(data, 10, &mut ec);

        let mut received = [0u8; 26];
        received[..16].copy_from_slice(data);
        received[16..].copy_from_slice(&ec);

        // Corrupt one byte
        received[3] ^= 0xFF;

        let n_errors = decode(&mut received, 10).unwrap();
        assert_eq!(n_errors, 1);
        assert_eq!(&received[..16], data);
    }

    #[test]
    fn decode_max_errors_at_capacity() {
        let data: &[u8] = &[
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        let mut ec = [0u8; 10];
        encode(data, 10, &mut ec);

        let mut received = [0u8; 26];
        received[..16].copy_from_slice(data);
        received[16..].copy_from_slice(&ec);

        // Corrupt t=5 bytes (at capacity for 10 EC codewords)
        received[0] ^= 0x01;
        received[5] ^= 0x02;
        received[10] ^= 0x04;
        received[15] ^= 0x08;
        received[20] ^= 0x10;

        let result = decode(&mut received, 10);
        assert!(result.is_ok());
        assert_eq!(&received[..16], data);
    }

    #[test]
    fn decode_too_many_errors_fails() {
        let data: &[u8] = &[
            32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        let mut ec = [0u8; 10];
        encode(data, 10, &mut ec);

        let mut received = [0u8; 26];
        received[..16].copy_from_slice(data);
        received[16..].copy_from_slice(&ec);

        // Corrupt t+1=6 bytes (exceeds capacity)
        received[0] ^= 0x01;
        received[1] ^= 0x02;
        received[2] ^= 0x04;
        received[3] ^= 0x08;
        received[4] ^= 0x10;
        received[5] ^= 0x20;

        assert!(decode(&mut received, 10).is_err());
    }

    #[test]
    fn datamatrix_round_trip_with_errors() {
        // ECC 200 convention (0x12D field, syndrome start 1) corrects up to
        // t = n_ec/2 errors like the QR convention.
        let data: &[u8] = &[142, 164, 186];
        let n_ec = 5;
        let mut ec = [0u8; 5];
        encode_with::<DmField>(data, n_ec, 1, &mut ec);

        let mut received = [0u8; 8];
        received[..3].copy_from_slice(data);
        received[3..].copy_from_slice(&ec);
        received[2] ^= 0x55;
        received[6] ^= 0xAA;

        let corrected = decode_with::<DmField>(&mut received, n_ec, 1).unwrap();
        assert_eq!(corrected, 2);
        assert_eq!(&received[..3], data);
    }
}

//! Reed-Solomon over GF(929) — the PDF417 (ISO/IEC 15438) error-correction field.
#![allow(clippy::needless_range_loop)] // polynomial arithmetic is index-centric
//!
//! 929 is prime, so arithmetic is integer arithmetic mod 929 (unlike the
//! GF(2^8) engines used by QR Code and Data Matrix). The generator element is
//! α = 3 with multiplicative order 928; error-correction codewords use
//! generator roots α^1…α^k.

/// The field modulus.
pub const MODULUS: u16 = 929;
/// Multiplicative group order (929 is prime → 928).
const ORDER: usize = 928;

/// `EXP[i]` = 3^i mod 929 for `i` in `0..928`.
const EXP: [u16; ORDER] = build_exp();
/// `LOG[x]` = log₃(x) for `x` in `1..929`; `LOG[0]` is a sentinel.
const LOG: [u16; 930] = build_log();

const fn build_exp() -> [u16; ORDER] {
    let mut exp = [1u16; ORDER];
    let mut i = 1usize;
    while i < ORDER {
        exp[i] = exp[i - 1] * 3 % MODULUS;
        i += 1;
    }
    exp
}

const fn build_log() -> [u16; 930] {
    let mut log = [0u16; 930];
    let mut x: u16 = 1;
    let mut i: u16 = 0;
    while i < ORDER as u16 {
        log[x as usize] = i;
        x = x * 3 % MODULUS;
        i += 1;
    }
    log
}

/// Addition mod 929.
#[inline(always)]
pub fn add(a: u16, b: u16) -> u16 {
    (a + b) % MODULUS
}

/// Subtraction mod 929.
#[inline(always)]
pub fn sub(a: u16, b: u16) -> u16 {
    (a + MODULUS - b % MODULUS) % MODULUS
}

/// Multiplication mod 929.
#[inline(always)]
pub fn mul(a: u16, b: u16) -> u16 {
    (a as u32 * b as u32 % MODULUS as u32) as u16
}

/// Inverse via `a^927` (Fermat: a^928 = 1).
#[inline(always)]
pub fn inv(a: u16) -> u16 {
    debug_assert!(a != 0, "GF(929): inverse of zero");
    EXP[(ORDER - LOG[a as usize] as usize) % ORDER]
}

/// Evaluate the received codeword polynomial (highest degree first) at
/// `α^exponent`.
fn eval_at(codeword: &[u16], exponent: usize) -> u16 {
    let mut s = 0u16;
    for &c in codeword {
        s = add(mul(s, EXP[exponent % ORDER]), c);
    }
    s
}

/// Compute the `k` PDF417 error-correction codewords for `data` (which must
/// already include the symbol length descriptor and padding).
///
/// This mirrors ISO/IEC 15438 §4.10: generator roots α^1…α^k; the emitted
/// codewords are the negated remainder coefficients from degree `k−1` down to
/// the constant term.
pub fn rs_encode(data: &[u16], k: usize, out: &mut [u16]) {
    debug_assert!(out.len() >= k);

    // Generator polynomial coefficients (low degree first) for roots 3^1..3^k:
    // g(x) = Π (x − 3^i); coefficient tables per ISO/IEC 15438 Annex F.
    let mut gen = alloc::vec![0u16; k + 1];
    gen[0] = 1;
    for i in 1..=k {
        let root = EXP[i % ORDER];
        for j in (1..=i).rev() {
            gen[j] = sub(gen[j - 1], mul(gen[j], root));
        }
        gen[0] = sub(0, mul(gen[0], root));
    }

    // ISO §4.10 recurrence (synthetic division with the negation folded in)
    let mut e = alloc::vec![0u16; k];
    for &d in data {
        let t1 = add(d, e[k - 1]);
        for j in (1..k).rev() {
            let t2 = mul(t1, gen[j]);
            let t3 = sub(0, t2);
            e[j] = add(e[j - 1], t3);
        }
        let t2 = mul(t1, gen[0]);
        let t3 = sub(0, t2);
        e[0] = t3;
    }

    // Emit negated, high degree first
    for j in 0..k {
        let v = e[k - 1 - j];
        out[j] = if v != 0 { sub(0, v) } else { 0 };
    }
}

/// Errors returned by the PDF417 Reed-Solomon decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rs929Error {
    /// Too many errors for the correction capacity.
    TooManyErrors,
    /// Error locator roots do not correspond to valid positions.
    ChienSearchFailed,
}

/// Correct `received` (data codewords followed by `k` EC codewords) in place.
/// Returns the number of corrected codewords.
pub fn rs_decode(received: &mut [u16], k: usize) -> Result<usize, Rs929Error> {
    let n = received.len();
    if k == 0 || n <= k {
        return Ok(0);
    }
    let t = k / 2;

    // Syndromes S_j = R(3^j) for j = 1..=k
    let mut syndromes = alloc::vec![0u16; k + 1]; // 1-based
    for j in 1..=k {
        syndromes[j] = eval_at(received, j);
    }
    if syndromes[1..=k].iter().all(|&s| s == 0) {
        return Ok(0);
    }

    // Berlekamp-Massey (1-based σ coefficients, σ[0] = 1)
    let mut sigma = alloc::vec![0u16; k + 1];
    let mut prev = alloc::vec![0u16; k + 1];
    sigma[0] = 1;
    prev[0] = 1;
    let mut l = 0usize;
    let mut shift = 1usize;
    let mut b_inv = 1u16;

    for n_i in 0..k {
        let mut delta = syndromes[n_i + 1];
        for i in 1..=l {
            delta = add(delta, mul(sigma[i], syndromes[n_i + 1 - i]));
        }
        if delta == 0 {
            shift += 1;
            continue;
        }
        let scale = mul(delta, inv(b_inv));
        let mut next = sigma.clone();
        for i in 0..=l {
            next[i + shift] = sub(next[i + shift], mul(scale, prev[i]));
        }
        if 2 * l <= n_i {
            let new_l = n_i + 1 - l;
            l = new_l;
            prev.copy_from_slice(&sigma);
            b_inv = delta;
            shift = 1;
        } else {
            shift += 1;
        }
        sigma = next;
    }

    if l > t {
        return Err(Rs929Error::TooManyErrors);
    }

    // Chien search: position p (0-based from the start) has locator
    // X = 3^(N−1−p); σ(3^(−p)) = 0 marks an error.
    let mut err_pos = alloc::vec![0usize; l];
    let mut found = 0usize;
    for p in 0..n {
        let x_inv = EXP[(ORDER - p % ORDER) % ORDER];
        let mut val = 0u16;
        let mut xpow = 1u16;
        for i in 0..=l {
            val = add(val, mul(sigma[i], xpow));
            xpow = mul(xpow, x_inv);
        }
        if val == 0 {
            if found == l {
                return Err(Rs929Error::TooManyErrors);
            }
            err_pos[found] = p;
            found += 1;
        }
    }
    if found != l {
        return Err(Rs929Error::ChienSearchFailed);
    }

    // Error evaluator Ω(x) = S(x)·σ(x) mod x^k, with S in syndrome order
    let mut omega = alloc::vec![0u16; k + 1];
    for i in 1..=k {
        for j in 0..=l {
            if i + j <= k {
                omega[i + j - 1] = add(omega[i + j - 1], mul(syndromes[i], sigma[j]));
            }
        }
    }

    // Formal derivative σ'(x) = Σ (i+1)·σ_{i+1}·x^i — unlike GF(2^8), the
    // (i+1) scalar factors matter in odd characteristic.
    let mut sigma_prime = alloc::vec![0u16; k + 1];
    for i in 0..l {
        sigma_prime[i] = mul((i as u16 + 1) % MODULUS, sigma[i + 1]);
    }

    for &p in err_pos.iter() {
        let x_inv = EXP[(ORDER - p % ORDER) % ORDER];
        let mut omega_val = 0u16;
        let mut xpow = 1u16;
        for j in 0..k {
            omega_val = add(omega_val, mul(omega[j], xpow));
            xpow = mul(xpow, x_inv);
        }
        let mut sp_val = 0u16;
        let mut xpow = 1u16;
        for j in 0..l {
            sp_val = add(sp_val, mul(sigma_prime[j], xpow));
            xpow = mul(xpow, x_inv);
        }
        if sp_val == 0 {
            return Err(Rs929Error::TooManyErrors);
        }
        // Syndromes start at α^1 (b = 1). In odd characteristic the Forney
        // result carries a sign: e = −Ω(X⁻¹)/σ'(X⁻¹), so the correction is
        // r + Ω/σ'.
        let magnitude = div929(omega_val, sp_val);
        received[n - 1 - p] = add(received[n - 1 - p], magnitude);
    }

    Ok(found)
}

fn div929(a: u16, b: u16) -> u16 {
    mul(a, inv(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_log_consistent() {
        for i in 0..ORDER {
            assert_eq!(LOG[EXP[i] as usize] as usize, i);
        }
        // 3 is primitive: exp has period 928, so EXP[927] = 3^-1 = 310
        assert_eq!(EXP[0], 1);
        assert_eq!(EXP[ORDER - 1], inv(3));
    }

    #[test]
    fn inverse_round_trip() {
        for a in 1u16..MODULUS {
            assert_eq!(mul(a, inv(a)), 1);
        }
    }

    #[test]
    fn encode_produces_zero_syndromes() {
        let data: &[u16] = &[5, 900, 17, 901, 42, 128, 902, 7];
        let k = 8;
        let mut ec = alloc::vec![0u16; k];
        rs_encode(data, k, &mut ec);
        let mut block = alloc::vec::Vec::from(data);
        block.extend_from_slice(&ec);
        for j in 1..=k {
            assert_eq!(eval_at(&block, j), 0, "syndrome {j} nonzero");
        }
    }

    #[test]
    fn decode_corrects_errors() {
        let data: &[u16] = &[1, 2, 3, 901, 500, 928];
        let k = 8;
        let mut ec = alloc::vec![0u16; k];
        rs_encode(data, k, &mut ec);
        let mut block = alloc::vec::Vec::from(data);
        block.extend_from_slice(&ec);

        // Corrupt t = 4 codewords (capacity for k = 8)
        block[0] = add(block[0], 311);
        block[3] = add(block[3], 87);
        block[7] = add(block[7], 913);
        block[12] = add(block[12], 456);

        let corrected = rs_decode(&mut block, k).unwrap();
        assert_eq!(corrected, 4);

        let mut expected = alloc::vec::Vec::from(data);
        let mut ec2 = alloc::vec![0u16; k];
        rs_encode(data, k, &mut ec2);
        expected.extend_from_slice(&ec2);
        assert_eq!(block, expected);
    }

    #[test]
    fn decode_fails_beyond_capacity() {
        let data: &[u16] = &[10, 20, 30];
        let k = 4;
        let mut ec = alloc::vec![0u16; k];
        rs_encode(data, k, &mut ec);
        let mut block = alloc::vec::Vec::from(data);
        block.extend_from_slice(&ec);
        // t = 2; corrupt 3
        for slot in block.iter_mut().take(3) {
            *slot = add(*slot, 100);
        }
        assert!(rs_decode(&mut block, k).is_err());
    }
}

//! Galois Field GF(2^8) arithmetic.
//!
//! Two primitive polynomials are in use across barcode symbologies:
//! `0x11D` (QR Code) and `0x12D` (Data Matrix ECC 200). Both log/antilog
//! table sets are built `const` — zero runtime init cost — and exposed through
//! the zero-sized [`QrField`] / [`DmField`] descriptors so the Reed-Solomon
//! engine can be shared between them.
//!
//! The free functions ([`add`], [`mul`], [`inv`], [`div`], [`pow`]) operate on
//! the QR field (0x11D) and are kept for convenience.

/// Log/antilog tables for the primitive polynomial `poly`.
const fn build_exp_log<const POLY: u32>() -> ([u8; 512], [u8; 256]) {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x: u32 = 1;
    let mut i = 0usize;
    while i < 255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= POLY;
        }
        i += 1;
    }
    exp[255] = 1; // α^255 = α^0 = 1 (cyclic group of order 255)
    let mut i = 256usize;
    while i < 512 {
        exp[i] = exp[i - 255];
        i += 1;
    }
    (exp, log)
}

/// QR Code field: primitive polynomial x^8 + x^4 + x^3 + x^2 + 1 (0x11D).
const QR_TABLES: ([u8; 512], [u8; 256]) = build_exp_log::<0x11D>();

/// Data Matrix (ECC 200) field: primitive polynomial x^8 + x^5 + x^3 + x^2 + 1 (0x12D).
const DM_TABLES: ([u8; 512], [u8; 256]) = build_exp_log::<0x12D>();

/// `EXP[i]` = α^i in the QR field. Extended to 512 entries so `LOG[a] + LOG[b]`
/// never needs a `% 255` operation in [`mul`].
pub const EXP: [u8; 512] = QR_TABLES.0;

/// `LOG[x]` = log_α(x) for x ≠ 0. `LOG[0]` is `0` — a sentinel, undefined value.
pub const LOG: [u8; 256] = QR_TABLES.1;

/// A GF(256) field defined by a primitive polynomial's tables.
pub trait Field {
    /// `EXP[i]` = α^i (512 entries, cyclic).
    fn exp(i: usize) -> u8;
    /// `LOG[x]` = log_α(x); `LOG[0]` is a sentinel.
    fn log(x: u8) -> u8;

    /// Multiply two field elements.
    #[inline(always)]
    fn mul(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        Self::exp(Self::log(a) as usize + Self::log(b) as usize)
    }

    /// Multiplicative inverse of `a`. Panics (debug) if `a == 0`.
    #[inline(always)]
    fn inv(a: u8) -> u8 {
        debug_assert!(a != 0, "GF(256): inverse of zero is undefined");
        Self::exp(255 - Self::log(a) as usize)
    }

    /// Divide `a / b`. Panics (debug) if `b == 0`.
    #[inline(always)]
    fn div(a: u8, b: u8) -> u8 {
        debug_assert!(b != 0, "GF(256): division by zero");
        if a == 0 {
            return 0;
        }
        Self::exp(Self::log(a) as usize + 255 - Self::log(b) as usize)
    }

    /// Raise `a` to the power `n`.
    #[inline(always)]
    fn pow(a: u8, n: usize) -> u8 {
        if a == 0 {
            return 0;
        }
        Self::exp((Self::log(a) as usize * n) % 255)
    }
}

/// Zero-sized descriptor of the QR Code field (0x11D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QrField;

impl Field for QrField {
    #[inline(always)]
    fn exp(i: usize) -> u8 {
        QR_TABLES.0[i]
    }
    #[inline(always)]
    fn log(x: u8) -> u8 {
        QR_TABLES.1[x as usize]
    }
}

/// Zero-sized descriptor of the Data Matrix field (0x12D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmField;

impl Field for DmField {
    #[inline(always)]
    fn exp(i: usize) -> u8 {
        DM_TABLES.0[i]
    }
    #[inline(always)]
    fn log(x: u8) -> u8 {
        DM_TABLES.1[x as usize]
    }
}

/// Add two GF(256) elements (XOR; subtraction is identical).
#[inline(always)]
pub const fn add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Multiply two GF(256) elements in the QR field (0x11D).
#[inline(always)]
pub fn mul(a: u8, b: u8) -> u8 {
    QrField::mul(a, b)
}

/// Multiplicative inverse of `a` in the QR field. Panics (debug) if `a == 0`.
#[inline(always)]
pub fn inv(a: u8) -> u8 {
    QrField::inv(a)
}

/// Divide `a / b` in the QR field. Panics (debug) if `b == 0`.
#[inline(always)]
pub fn div(a: u8, b: u8) -> u8 {
    QrField::div(a, b)
}

/// Raise `a` to the power `n` in the QR field.
#[inline(always)]
pub fn pow(a: u8, n: usize) -> u8 {
    QrField::pow(a, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_is_xor() {
        assert_eq!(add(0x53, 0xCA), 0x53 ^ 0xCA);
    }

    #[test]
    fn mul_by_zero_is_zero() {
        for a in 0u8..=255 {
            assert_eq!(mul(a, 0), 0);
            assert_eq!(mul(0, a), 0);
        }
    }

    #[test]
    fn mul_by_one_is_identity() {
        for a in 1u8..=255 {
            assert_eq!(mul(a, 1), a);
            assert_eq!(mul(1, a), a);
        }
    }

    #[test]
    fn mul_commutative() {
        assert_eq!(mul(0x53, 0xCA), mul(0xCA, 0x53));
    }

    #[test]
    fn mul_known_value() {
        // 0x53 * 0xCA = 0x8F in GF(256) with poly 0x11D.
        // (0x53 · 0xCA = 0x01 is the famous inverse pair for the AES field 0x11B.)
        assert_eq!(mul(0x53, 0xCA), 0x8F);
    }

    #[test]
    fn mul_matches_baseline_shift_reduce() {
        // Cross-check the whole log/exp table against a table-free
        // shift-and-reduce implementation of carryless multiplication mod 0x11D.
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                let expected = {
                    let (mut acc, mut a, mut b) = (0u16, a as u16, b as u16);
                    while b != 0 {
                        if b & 1 != 0 {
                            acc ^= a;
                        }
                        a <<= 1;
                        if a & 0x100 != 0 {
                            a ^= 0x11D;
                        }
                        b >>= 1;
                    }
                    acc as u8
                };
                assert_eq!(mul(a, b), expected, "mul({a:#04x}, {b:#04x})");
            }
        }
    }

    #[test]
    fn inv_round_trip() {
        for a in 1u8..=255 {
            assert_eq!(mul(a, inv(a)), 1);
        }
    }

    #[test]
    fn div_round_trip() {
        for a in 1u8..=255 {
            for b in 1u8..=255 {
                assert_eq!(mul(div(a, b), b), a);
            }
        }
    }

    #[test]
    fn pow_known() {
        // α^8 = α^4 + α^3 + α^2 + 1 = 0x1D in GF(0x11D)
        // But EXP[8] was built by the table; just verify pow matches EXP
        for (n, &exp) in EXP.iter().enumerate() {
            assert_eq!(pow(2, n), exp);
        }
    }

    #[test]
    fn exp_period_is_255() {
        // The multiplicative group has order 255
        for i in 0usize..255 {
            assert_eq!(EXP[i], EXP[i + 255]);
        }
    }

    #[test]
    fn dm_field_is_a_valid_field() {
        // The 0x12D tables must also describe a consistent field: 2 must be a
        // primitive element (its powers cycle with period 255 and hit all
        // non-zero values).
        let mut seen = [false; 256];
        for i in 0..255 {
            let v = DmField::exp(i);
            assert_ne!(v, 0);
            assert!(!seen[v as usize], "α^{i} repeats for 0x12D");
            seen[v as usize] = true;
        }
        assert_eq!(DmField::exp(255), 1);
        for a in 1u8..=255 {
            assert_eq!(DmField::mul(a, DmField::inv(a)), 1);
        }
        // The two fields genuinely differ
        assert_ne!(DmField::exp(8), QrField::exp(8));
    }
}

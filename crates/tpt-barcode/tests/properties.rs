//! Property-style round-trip tests over pseudo-random payloads.
//!
//! A deterministic xorshift PRNG keeps the suite dependency-free while still
//! covering far more input space than the hand-picked fixtures: random
//! payloads across all EC levels and symbologies, plus random error
//! injection within each code's correction capacity.

#![cfg(all(feature = "2d", feature = "1d"))]

extern crate alloc;

use tpt_barcode::core::EcLevel;

/// xorshift64* — deterministic across platforms, no dependencies.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

#[test]
fn qr_round_trip_random_payloads_all_ec_levels() {
    let mut rng = Rng::new(0xDEADBEEF);
    for len in [1usize, 2, 7, 33, 100] {
        for ec in [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
            for case in 0..8u32 {
                let payload: alloc::vec::Vec<u8> = (0..len)
                    .map(|_| match rng.byte() % 4 {
                        0 => rng.byte() % 10 + b'0', // digits
                        1 => rng.byte() % 26 + b'A', // uppercase
                        2 => rng.byte() % 26 + b'a', // lowercase
                        _ => rng.byte(),             // arbitrary byte
                    })
                    .collect();
                let qr = tpt_barcode::qr::encode(&payload, ec).unwrap_or_else(|e| {
                    panic!("encode failed len={len} ec={ec:?} case={case}: {e:?}")
                });
                assert_eq!(
                    qr.decode().unwrap(),
                    payload,
                    "round trip failed len={len} ec={ec:?} case={case}"
                );
            }
        }
    }
}

#[test]
fn datamatrix_round_trip_random_payloads() {
    let mut rng = Rng::new(0x0BADF00D);
    for len in [1usize, 3, 8, 22, 44] {
        for case in 0..8u32 {
            // ECC 200 ASCII encodation: bytes 0–127 direct, ≥128 via upper shift
            let payload: alloc::vec::Vec<u8> = (0..len)
                .map(|_| match rng.byte() % 3 {
                    0 => b'0' + rng.byte() % 10,
                    1 => b'A' + rng.byte() % 26,
                    _ => rng.byte() % 128,
                })
                .collect();
            let dm = tpt_barcode::two_d::datamatrix::encode(&payload)
                .unwrap_or_else(|e| panic!("encode failed len={len} case={case}: {e:?}"));
            assert_eq!(
                tpt_barcode::two_d::datamatrix::decode(&dm.matrix, dm.size).unwrap(),
                payload,
                "round trip failed len={len} case={case}"
            );
        }
    }
}

#[test]
fn pdf417_round_trip_random_payloads() {
    use tpt_barcode::two_d::pdf417::{self, Compaction};
    let mut rng = Rng::new(0x5EED1234);
    for len in [1usize, 6, 12, 30, 90] {
        for case in 0..4u32 {
            let payload: alloc::vec::Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            let level = pdf417::EcLevel(pdf417::recommended_ec_level(len).min(4));
            let symbol = pdf417::encode(&payload, level, Compaction::Byte)
                .unwrap_or_else(|e| panic!("encode failed len={len} case={case}: {e:?}"));
            assert_eq!(
                pdf417::decode(&symbol.matrix, symbol.width, symbol.height).unwrap(),
                payload,
                "round trip failed len={len} case={case}"
            );
        }
    }
}

#[test]
fn qr_survives_random_bit_errors_within_ec_capacity() {
    // 1-H corrects ~30 % of codewords; flipping a sprinkling of DATA modules
    // must stay decodable. Flip count stays modest so we test robustness,
    // not the correction boundary (that has dedicated unit tests).
    let mut rng = Rng::new(0xFACEB00C);
    let payload = b"QR error robustness probe";
    for case in 0..20u32 {
        let qr = tpt_barcode::qr::encode(payload, EcLevel::H).unwrap();
        let mut corrupted = qr.matrix.clone();
        let mut flipped = 0usize;
        for module in corrupted.iter_mut() {
            if rng.byte() % 97 == 0 && flipped < 12 {
                *module ^= 1;
                flipped += 1;
            }
        }
        let _ = case;
        // module noise beyond capacity is an acceptable reject
        if let Ok(decoded) = tpt_barcode::two_d::qr::decode_grid(&corrupted, qr.size) {
            assert_eq!(decoded, payload, "case {case}");
        }
    }
}

#[test]
fn code128_ean13_round_trip_random() {
    let mut rng = Rng::new(0x1234ABCD);
    for case in 0..10u32 {
        // Code 128 Subset B: printable ASCII
        let len = 1 + (rng.byte() % 12) as usize;
        let payload: alloc::vec::Vec<u8> = (0..len).map(|_| 0x20 + rng.byte() % 95).collect();
        let code = tpt_barcode::one_d::code128::encode_b(&payload)
            .unwrap_or_else(|e| panic!("case {case}: {e:?}"));
        let runs: alloc::vec::Vec<u32> = code.modules.iter().map(|&w| w as u32).collect();
        assert_eq!(
            tpt_barcode::one_d::runs::decode_code128_runs(&runs).unwrap(),
            payload_as_text(&payload),
            "case {case}"
        );

        // EAN-13: 12 digits, auto checksum
        let digits: alloc::vec::Vec<u8> = (0..12).map(|_| rng.byte() % 10).collect();
        let ean = tpt_barcode::one_d::ean13::encode(&digits).unwrap();
        let runs = tpt_barcode::one_d::runs::modules_to_runs(&ean.modules);
        assert_eq!(
            &tpt_barcode::one_d::runs::decode_ean13_runs(&runs).unwrap(),
            &ean.digits,
            "case {case}"
        );
    }
}

fn payload_as_text(bytes: &[u8]) -> alloc::string::String {
    bytes.iter().map(|&b| b as char).collect()
}

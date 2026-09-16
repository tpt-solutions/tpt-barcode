//! Criterion benchmarks for encode and scan paths.
#![cfg(all(feature = "scan", feature = "2d"))]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_encode(c: &mut Criterion) {
    c.bench_function("encode_qr_v5m_100b", |b| {
        let payload: Vec<u8> = (0..100u32).map(|i| b'A' + (i % 26) as u8).collect();
        b.iter(|| tpt_barcode::qr::encode(black_box(&payload), tpt_barcode::core::EcLevel::M))
    });
    c.bench_function("encode_datamatrix_50b", |b| {
        let payload: Vec<u8> = (0..50u32).map(|i| b'a' + (i % 26) as u8).collect();
        b.iter(|| tpt_barcode::two_d::datamatrix::encode(black_box(&payload)))
    });
    c.bench_function("encode_pdf417_40b_l3", |b| {
        let payload: Vec<u8> = (0..40u32).map(|i| b'0' + (i % 10) as u8).collect();
        b.iter(|| {
            tpt_barcode::two_d::pdf417::encode(
                black_box(&payload),
                tpt_barcode::two_d::pdf417::EcLevel(3),
                tpt_barcode::two_d::pdf417::Compaction::Byte,
            )
        })
    });
}

fn bench_decode(c: &mut Criterion) {
    let qr =
        tpt_barcode::qr::encode("benchmark decode payload", tpt_barcode::core::EcLevel::M).unwrap();
    c.bench_function("decode_qr_grid_v1m", |b| {
        b.iter(|| tpt_barcode::two_d::qr::decode_grid(black_box(&qr.matrix), qr.size))
    });
}

fn bench_binarize(c: &mut Criterion) {
    let w = 400usize;
    let h = 300usize;
    let mut seed = 0x12345678u32;
    let pixels: Vec<u8> = (0..w * h)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    let mut out = vec![0u8; pixels.len()];
    c.bench_function("binarize_adaptive_400x300", |b| {
        b.iter(|| {
            tpt_barcode::image_scan::binarize::binarize_adaptive(
                black_box(&pixels),
                w,
                h,
                &mut out,
                w / 8,
                0.15,
            )
        })
    });
}

criterion_group!(benches, bench_encode, bench_decode, bench_binarize);
criterion_main!(benches);

# tpt-barcode — Task Checklist

## Phase 0 — Scaffold
- [x] Create workspace `Cargo.toml`
- [x] Create `crates/tpt-barcode-core/Cargo.toml` + `src/lib.rs`
- [x] Create `crates/tpt-barcode-1d/Cargo.toml` + `src/lib.rs`
- [x] Create `crates/tpt-barcode-2d/Cargo.toml` + `src/lib.rs`
- [x] Create `crates/tpt-barcode-image/Cargo.toml` + `src/lib.rs`
- [x] Create `crates/tpt-barcode-render/Cargo.toml` + `src/lib.rs`
- [x] Create `crates/tpt-barcode/Cargo.toml` + `src/lib.rs`
- [x] Create `LICENSE-MIT` and `LICENSE-APACHE`
- [x] Create `README.md` skeleton
- [x] Create `.github/workflows/ci.yml`
- [x] Create `todo.md` (this file)

## Phase 1 — Mathematical Heart: `tpt-barcode-core` (Weeks 1–2)
- [x] Create `src/gf256.rs` — GF(256) struct with const Exp/Log lookup tables
- [x] Implement GF(256) addition (XOR)
- [x] Implement GF(256) multiplication (via log/exp tables)
- [x] Implement GF(256) inversion
- [x] Create `src/reed_solomon.rs` — RS encoder (polynomial division)
- [x] Implement RS decoder — Berlekamp-Massey syndrome computation
- [x] Implement RS decoder — Chien search (error location)
- [x] Implement RS decoder — Forney algorithm (error magnitude)
- [x] Define `Symbology` trait in `src/traits.rs`
- [x] Define `Symbol` / `EncodedSymbol` traits
- [x] Write unit tests: GF(256) round-trip (add/mul/inv)
- [x] Write unit tests: RS encode → decode with known QR payloads
      (thonky "HELLO WORLD" 1-M vector: EC = 196 35 39 119 235 215 231 226 93 23)
- [x] Write unit tests: RS decode with injected errors (up to t errors)
- [x] Verify `no_std` build: `cargo build --no-default-features --features alloc -p tpt-barcode-core`

## Phase 2 — QR Generation: `tpt-barcode-2d` + `tpt-barcode-render` (Weeks 3–4)
### tpt-barcode-2d (QR)
- [x] Create `src/qr/mode.rs` — Numeric, Alphanumeric, Byte, Kanji mode indicators
- [x] Create `src/qr/version.rs` — version table (1–40), capacity lookup
      (full 160-row block table from the thonky EC table, consistency-checked)
- [x] Create `src/qr/encode.rs` — character count + bit-stream packing
      (bit packing lives in `mode.rs` `BitBuffer`; encode pipeline in `qr/mod.rs`)
- [x] Create `src/qr/matrix.rs` — function patterns (finder, timing, alignment, format)
- [x] Create `src/qr/mask.rs` — all 8 mask patterns + penalty rule evaluator
- [x] Create `src/qr/ec.rs` — integrate tpt-barcode-core RS for EC codeword generation
- [x] `qr::encode(data, EcLevel)` public API
- [x] Write unit tests: encode "HELLO WORLD" at version 1-M
      (matrix-equality anchor against the Python `qrcode` reference; decoder
      also decodes a foreign reference matrix)
- [x] Write unit tests: mask penalty scoring
- [x] Write unit tests: format information bits
      (ISO published format strings for L/M/Q/H)

### tpt-barcode-render
- [x] Create `src/svg.rs` — SVG string builder (module squares, quiet zone)
- [x] Create `src/png.rs` — PNG output via `image` crate (behind `png` feature)
- [x] Create `src/ansi.rs` — ANSI terminal block-character renderer
- [x] Builder API: `.render().svg().module_size(4).build()`
- [x] Write unit tests: SVG output contains expected `<rect>` elements
- [x] Write unit tests: ANSI renders correct column count

## Phase 3 — 1D Symbologies: `tpt-barcode-1d` (Weeks 3–4)
- [x] Create `src/code128.rs` — Code 128 encoder (subsets A/B/C)
- [x] Create `src/code128.rs` — Code 128 decoder
- [x] Create `src/ean13.rs` — EAN-13 encoder
- [x] Create `src/ean13.rs` — EAN-13 decoder + checksum verification
- [x] Create `src/upca.rs` — UPC-A encoder (derive from EAN-13)
- [x] Create `src/upca.rs` — UPC-A decoder
- [x] Create `src/code39.rs` — Code 39 encoder
- [x] Create `src/code39.rs` — Code 39 decoder
- [x] Write unit tests: Code 128 encode/decode round-trip
- [x] Write unit tests: EAN-13 checksum digit validation
- [x] Write unit tests: Code 39 known symbol bars
- [x] Verify `no_std` build: `cargo build --no-default-features --features alloc -p tpt-barcode-1d`

## Phase 4 — Computer Vision Scanner: `tpt-barcode-image` (Weeks 5–8)
- [x] Create `src/binarize.rs` — adaptive thresholding (Bradley local mean)
- [x] Add `simd` feature path in `src/binarize.rs` for SIMD-accelerated threshold
      (SSE2 threshold stage, runtime-detected; bit-identical with scalar, tested)
- [x] Create `src/edge.rs` — Sobel edge detection
- [x] Create `src/finder.rs` — QR finder pattern detection (1:1:3:1:1 ratio scan)
      (+ horizontal/vertical cross-check, centroid merge, isosceles-right triple
      selection, rotation-tolerant)
- [x] Integrate `tpt-math-geometry` for distance/ratio verification in `src/finder.rs`
- [x] Create `src/homography.rs` — construct 3×3 homography from 4 corner points
- [x] Compute homography inverse via `tpt-math-linalg-fixed::Matrix3`
- [x] Perspective-corrected pixel sampling via `tpt-math-geometry` projections
- [x] Feed sampled bit grid into `tpt-barcode-2d` QR format reader
      (`qr::decode_grid`: format info → unmask → zigzag extraction → de-interleave → RS → payload)
- [x] Feed decoded bits into Phase 1 Reed-Solomon decoder
- [x] Write unit tests: binarize a synthetic gradient image
- [x] Write unit tests: homography inverse of identity is identity
- [x] Write integration test: scan a pre-generated QR PNG
      (facade `tests/scan_roundtrip.rs`: encode → render → scan round trip,
      PNG codec round trip, 10°-rotated symbol via homography, adaptive mode)

## Phase 5 — 2D Scanning: DataMatrix + PDF417 (Weeks 7–8)
### DataMatrix
- [x] Create `src/datamatrix/finder.rs` — L-shape finder pattern detection
      (`decode::validate_finders` validates the solid L + timing borders)
- [x] Create `src/datamatrix/grid.rs` — module grid sampler
      (`placement::for_each_placed_bit` replays the Annex M walk to map cells
      ↔ codeword bits)
- [x] Create `src/datamatrix/decode.rs` — data region decode + RS error correction
- [x] Write unit tests: DataMatrix decode known payload
      (encoder matches libdmtx matrix-for-matrix; decoder reads libdmtx/zxing-
      verified reference symbols; round trips across all nine sizes; module-
      error correction)
- [x] DataMatrix ECC 200 encode (ASCII encodation, digit pairs, upper shift,
      129/130 padding, GF(256)/0x12D RS with roots α^1…α^n, Annex M placement)

### PDF417
- [x] Row indicator decoding / geometry (rows, columns, EC level from the
      per-row indicator codewords; `decode::decode_geometry` cross-checks
      every row) — superseded the planned row.rs/column.rs split
- [x] Codeword extraction + RS decode (`pdf417/decode.rs` +
      `pdf417/gf929.rs`: Berlekamp-Massey / Chien / Forney over GF(929),
      with the odd-characteristic Forney sign and full σ′ scalar factors)
- [x] 3×929 codeword cluster pattern table (`pdf417/tables.rs`, transcribed
      from the ISO/IEC 15438 table via the Apache-2.0 ZXing reference)
- [x] Byte compaction encode + decode (901/924 latch semantics, sixpack
      base-900 packing, partial-group tail disambiguation)
- [x] Write unit tests: PDF417 decode known payload
      (round trips incl. binary payloads and multi-row symbols; valid-codeword
      substitution error-correction test; fixture from `pdf417gen` decoded
      cross-checked with `zxing-cpp`; our encoder's output decoded by
      `zxing-cpp` during development)
- [ ] Text and numeric compaction (encode + decode) — the only remaining
      PDF417 capability gap

## Phase 6 — Facade + Polish (Week 9)
- [x] Wire all feature flags in `crates/tpt-barcode/Cargo.toml`
- [x] Implement `src/prelude.rs` re-exports
- [x] Implement `scan()` builder API (`.formats()`, `.try_harder()`, `.execute()`)
- [x] `ScanResult::text()`, `ScanResult::format()`, `ScanResult::bounding_box()`
- [x] Add `#![deny(missing_docs)]` and fill all public doc comments
- [x] Write integration test: full pipeline — encode QR → render PNG → scan PNG
- [x] Create `CHANGELOG.md`
- [x] Review all `Cargo.toml` metadata (description, keywords, categories)
- [x] `cargo publish --dry-run` for each crate in dependency order
      (tpt-barcode-core verifies clean; downstream crates require their
      dependencies to be published first — run `cargo publish` in order:
      core → 1d → 2d → image → render → facade)
- [ ] Publish: tpt-barcode-core → tpt-barcode-1d → tpt-barcode-2d → tpt-barcode-image → tpt-barcode-render → tpt-barcode
      (requires crates.io credentials; mechanical once approved)

## Ongoing / Cross-Cutting
- [x] `cargo fmt --check` — keep clean throughout
- [x] `cargo clippy --workspace --all-features -- -D warnings` — zero warnings
- [ ] CI passes on every push (workflow exists; needs a pushed git remote)
- [x] Keep `rust-version = "1.84"` — test on MSRV in CI
      (note: current toolchain is newer; run `rustup run 1.84 cargo check` in CI)

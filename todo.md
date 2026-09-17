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
- [x] Text and numeric compaction (encode + decode) — complete; only
      mixed-segment auto-optimization (switching mid-payload) is future work

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

## Phase 7 — Known Bugs & Correctness Debt (2026-09 review)

Findings from the 2026-09-16 platform review; file references are to the
current tree. Ordered by risk.

- [x] **[BUG]** `tpt-barcode-image` cannot compile without the `alloc`
      feature: `binarize.rs` referenced a non-existent `heapless_integral`
      module and `edge.rs`/`finder.rs` use `alloc::vec::Vec` un-gated.
      Interim: a `compile_error!` now fires with a clear message (fixed
      2026-09-16). Proper fix: caller-supplied-buffer API for the integral
      image + `alloc`-gating of the vision pipeline, so `no_alloc` targets
      get a real (buffer-based) binarization path instead of a wall.
- [x] **[BUG/GAP]** `Scanner` scans QR only and returns at most one symbol;
      `formats(&[...])` silently ignores every other format
      (`tpt-barcode/src/lib.rs`, `Scanner::execute`). Fix: loop over finder
      triples (dedupe overlapping hits) for multi-symbol images, and add 1D
      scanning — which needs:
- [x] **[GAP]** 1D decoders accept module-per-byte arrays only
      (`ean13::decode(&[u8; 95])`, `code39::decode(&[u8])`, …). Real scanners
      and the vision pipeline produce **run lengths**. Add run-length-based
      decode APIs (`decode_runs(&[u32], pattern)`), then wire EAN-13/UPC-A/
      Code 128/Code 39 into `Scanner` (row-of-pixels → runs → decode).
- [x] **[BUG]** `EncodeError`/`DecodeError` implement neither `Display` nor
      `core::error::Error`, so `?`-propagation into apps needs manual
      conversion. Add manual `Display` + `Error` impls (`no_std`-safe), plus
      `Display` for `Format`/`EcLevel`.
- [x] **[DEBT]** QR mask selection scores penalties while the format areas
      are still light (`matrix.rs` writes format info after `select_mask`),
      so the chosen mask can differ from spec-conformant encoders on edge
      cases. Fix: write format bits per candidate mask before scoring.
- [x] **[PERF]** PDF417 codeword reverse-lookup is a linear scan over 929
      entries per codeword (`pdf417/decode.rs::lookup`). Build a sorted
      (mask → codeword) index or perfect-hash per cluster once per symbol.
- [x] **[DEBT]** QR Kanji mode: `encode_sjis` now encodes Shift-JIS
      payloads with Kanji segments (base-192 13-bit packing) interleaved
      with byte segments, and the decoder uses the correct base-192 inverse
      (the previous naive-shift decode mis-mapped the second SJIS range): `Mode::detect` never returns
      it and `encode_byte` is used in its place. Either implement Shift-JIS
      Kanji encoding (with detection) or remove it from the public `Mode`.
- [x] **[GAP]** DataMatrix: encoder is ASCII-encodation only (no C40/Text/
      X12/Edifact/Base256 → larger symbols than necessary); decoder silently
      truncates at any non-ASCII latch. Implement at least C40 + Base256 on
      both sides; make unsupported-latch decoding return
      `DecodeError::Unsupported` instead of a silent partial payload.
- [x] **[API]** `pdf417::EcLevel(pub u8)` allows invalid values (9+ fails
      only at runtime). Add `EcLevel::new(u8) -> Option<_>` / named constants
      and make tests/clippy prefer them.

## Phase 8 — Adoption & Usability

- [x] **Examples directory** (`examples/` in the facade crate, runnable with
      `cargo run --example`): `qr_svg`, `qr_png`, `datamatrix_svg`,
      `pdf417_png`, `code128_svg`, `scan_image` (load file → scan → print
      text + bounding boxes). Every example doubles as a doc page.
- [x] **One-line API**: extension traits behind features —
      `QrCodeExt::to_svg_string()/to_png_bytes()`, plus
      `SvgBuilder::from_qr(&QrCode)`. The current
      `SvgBuilder::new(&qr.matrix, qr.size)` forces users to touch internals.
- [x] **`scan_image` convenience**: accept `&image::DynamicImage` /
      `GrayImage` directly (behind `scan` + `std`), removing the
      `as_raw()/width()/height()` dance from every caller.
- [x] **QR builder API**: `QrCode::builder().data(..).ec_level(..)
      .version(..).mask(..).build()` — forced version/mask are needed for
      GS1, print-plate reuse, and golden-image testing.
- [x] **CLI binary** (`src/bin/tpt-barcode` or a `tpt-barcode-cli` crate):
      `tpt-barcode encode qr --ec m --svg out.svg "text"`,
      `tpt-barcode scan photo.jpg --json`. CLI tools are the single biggest
      adoption driver for codec crates.
- [x] **Docs**: README code blocks compiled in CI (docinclude or
      `#[doc = include_str!]`), a feature/symbology support matrix, platform
      table (std / no_std+alloc / wasm), and per-crate doc examples on the
      main entry points (`qr::encode`, `pdf417::encode`, `scan`).
      (Done 2026-09-16: support matrix + CLI/examples sections; scanner
      doc examples use `no_run`.)
- [~] **Templates**: `templates/embedded` (no_std + alloc frame-buffer
      render) remains unstarted (an earlier note claimed a sketch existed;
      it did not, on inspection) — and a `templates/web` WASM demo page
      (encode + camera scan) — **done 2026-09-17**: `templates/web/`
      (`index.html` + `main.js`) generates a QR Code and Code 128 barcode
      as inline SVG from a text input, and scans live camera frames via
      `getUserMedia` → canvas capture → `tpt-barcode-wasm::scan_rgba`,
      overlaying the detected bounding box. Built on the new
      `crates/tpt-barcode-wasm` npm package (see Phase 10 "Language
      bindings"). `templates/embedded` is the one remaining item here.

## Phase 9 — Hardening & Automation

- [x] **Property-based round-trip tests** (`proptest`, dev-dependency only):
      random payloads × all EC levels × all modes for QR/DataMatrix/PDF417
      and the 1D symbologies; random bit errors within EC capacity must
      decode. The current fixture set is hand-picked.
- [x] **Zero-allocation as a tested invariant**: a counting `GlobalAlloc`
      in tests asserts zero heap allocations during the scan path
      (`finder → homography → sample → decode`) — turns the headline claim
      into CI-enforced truth.
- [x] **Fuzzing** (`cargo-fuzz`): targets for `qr::decode_grid`,
      `datamatrix::decode`, `pdf417::decode` (module grids + raw codeword
      streams). Decoders are the attack surface; a fuzz smoke run in CI.
- [x] **Benchmarks** (`criterion`): encode/decode throughput per symbology
      and the binarize SIMD-vs-scalar comparison; publish numbers in README.
- [x] **CI additions**: `wasm32-unknown-unknown` build job (core/2d/render),
      nightly `cargo clippy` job, `cargo-deny` (licenses + advisories),
      `cargo-llvm-cov` coverage badge, and a differential-test job against a
      pinned corpus (zxing-generated symbols checked in under `testdata/`).
- [x] **Automation**: `cargo-release` (or release-please) config for the
      6-crate workspace publish order; pre-commit hook running
      `cargo fmt --check` + `cargo clippy -D warnings`.

## Phase 10 — Differentiators (innovative bets)

- [x] **`const` QR generation**: `crates/tpt-barcode-2d/src/qr/const_qr.rs`
      — `qr_matrix`/`qr_matrix_ec` const-fn-encode a byte-mode payload to a
      `ConstQr` module matrix (full const-evaluated GF(256)/Reed-Solomon,
      placement, and mask-penalty search), and `qr_svg`/`qr_svg_ec`/
      `qr_svg_path` go all the way to a const-evaluated SVG string via a
      fixed-capacity `ConstStr<N>` buffer (sidesteps the no-heap-in-const-fn
      limitation instead of requiring `String`). Usable as
      `const QR: ConstStr<16384> = qr_svg(b"...");`.
- [x] **GS1 support**: `qr::encode_gs1` (FNC1 first position + byte
      segment) and `datamatrix::encode_gs1` (FNC1 codeword 232 after the
      SLD), plus `two_d::gs1` — a length-aware Application Identifier
      parser (`parse` / `parse_text` / `AiElement`) that splits a decoded
      element string into typed `(AI, value)` pairs using the GS1
      fixed-length / FNC1-terminated rules.
- [x] **Multi-symbol scanning + structured results**: `ScanResult` gains
      symbology-specific metadata (QR version/mask/EC, ECI; DataMatrix size;
      PDF417 rows/EC level) and `execute()` returns all distinct symbols.
- [x] **NEON binarization path** (aarch64) mirroring the SSE2 threshold
      stage; compiles via `cargo check --target aarch64-unknown-none-softfloat
      -Z build-std=core,alloc` (nightly); runtime-gated, hardware validation
      pending access to an aarch64 machine.
- [x] **Bilinear grid sampling** — `homography::sample_grid_bilinear`
      added alongside the nearest-neighbour path.
- [x] **Language bindings (done 2026-09-17)**: new
      `crates/tpt-barcode-wasm` crate exposes a `#[wasm_bindgen]` API —
      `encode_qr_svg`/`encode_qr_png`, `encode_code128_svg`,
      `scan_gray`/`scan_rgba` (returning `WasmScanResult` with
      `text`/`format`/`corners`) — over the existing facade, buildable via
      `wasm-pack build --target web` into a publish-shaped (but
      unpublished) npm package; see its README for build/usage. It is
      excluded from the main Cargo workspace (own `[workspace]` table,
      listed in the root `Cargo.toml` `exclude`) so wasm-bindgen's
      dependency tree and `cdylib` crate-type don't affect
      `cargo clippy --workspace --all-features` on the native host — the
      same treatment as `fuzz`. `templates/web/` (Phase 8) consumes this
      crate's `wasm-pack` output directly rather than re-implementing
      bindings. **Python half**: `crates/tpt-barcode-py` (pyo3) exposes
      `encode_qr_svg`/`encode_qr_png`, `encode_code128_svg`, `scan` (returns
      `ScanResult` objects with `text`/`format`/`bounding_box`), buildable
      via `maturin develop`/`maturin build` (not published to PyPI); also
      excluded from the main Cargo workspace for the same reason as the
      wasm crate.

## Ongoing / Cross-Cutting
- [x] `cargo fmt --check` — keep clean throughout
- [x] `cargo clippy --workspace --all-features -- -D warnings` — zero warnings
- [ ] CI passes on every push (pushed to `origin/master` at `f1a21b3` on
      2026-09-17; `gh` CLI unavailable in this environment to confirm the
      Actions run went green — check
      https://github.com/tpt-solutions/tpt-barcode/actions)
- [x] Keep `rust-version = "1.84"` — test on MSRV in CI
      (note: current toolchain is newer; run `rustup run 1.84 cargo check` in CI)

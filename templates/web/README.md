# templates/web

A minimal, dependency-free browser demo for `tpt-barcode`:

- **Generate**: type text, pick a QR error-correction level, and get a live
  QR Code and Code 128 barcode rendered as inline SVG.
- **Scan with camera**: grabs frames from `getUserMedia`, draws them to a
  hidden `<canvas>`, and runs the WASM scan pipeline on every frame,
  overlaying the detected barcode's bounding box and printing the decoded
  text live.

It's plain HTML + JS (ES modules) — no framework, no bundler, no build step
of its own — so it's easy to copy into any project. All barcode logic comes
from the [`tpt-barcode-wasm`](../../crates/tpt-barcode-wasm) crate; this
directory only contains DOM/camera plumbing.

## 1. Build the WASM package

From the repo root:

```sh
rustup target add wasm32-unknown-unknown   # once
cargo install wasm-pack                     # once, if not already installed

cd crates/tpt-barcode-wasm
wasm-pack build --target web --out-dir ../../templates/web/vendor
```

This compiles `tpt-barcode-wasm` and writes the JS glue + `.wasm` binary
directly into `templates/web/vendor/` (git-ignored — it's build output,
regenerate it whenever the Rust source changes).

## 2. Serve the demo

Camera access (`getUserMedia`) requires a secure context, so opening
`index.html` directly via `file://` won't work in most browsers — serve it
over HTTP instead. From this directory:

```sh
python -m http.server 8000
# or: npx serve .
# or: cargo install miniserve && miniserve . --index index.html
```

Then open `http://localhost:8000/` and allow camera access when prompted.

## Notes

- The camera button requests the rear-facing camera (`facingMode:
  "environment"`) where available, falling back to whatever the browser
  picks otherwise.
- Frames are converted from RGBA (canvas `ImageData`) to grayscale in Rust
  (`scan_rgba`), so no extra JS image processing is needed.
- Scanning runs once per animation frame via `requestAnimationFrame`; a
  failed/empty scan on a given frame is normal (no barcode in view, motion
  blur, etc.) and is not treated as an error.
- To try a different symbology set or stricter/looser QR error correction,
  edit the calls to `encode_qr_svg` / `encode_code128_svg` / `scan_rgba` in
  `main.js` — the full public API is documented in
  `../../crates/tpt-barcode-wasm/README.md`.

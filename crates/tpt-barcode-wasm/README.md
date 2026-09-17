# tpt-barcode-wasm

WASM/npm bindings for [`tpt-barcode`](../tpt-barcode) — encode QR Codes and
Code 128 barcodes, and scan camera/canvas frames for barcodes, from
JavaScript in the browser (or any WASM host).

This crate is **not** part of the main Cargo workspace (see the `exclude`
list in the repo root `Cargo.toml`): `wasm-bindgen` pulls in its own
toolchain-sensitive dependency tree that only needs to resolve against
`wasm32-unknown-unknown`, and its `cdylib` crate-type doesn't need to
participate in `cargo clippy --workspace --all-features` runs on the native
host. It has its own `[workspace]` table so Cargo treats it as an
independent root, and it depends on the sibling `tpt-barcode` crate via a
relative path.

## Building

Install prerequisites once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack   # if not already installed
```

Build the npm package (from this directory):

```sh
wasm-pack build --target web --out-dir pkg
```

This produces `pkg/` containing:

- `tpt_barcode_wasm_bg.wasm` — the compiled module
- `tpt_barcode_wasm.js` — the JS glue / ES module entry point
- `tpt_barcode_wasm.d.ts` — TypeScript type declarations
- `package.json` — ready to `npm publish` (this repo does not publish it;
  see `todo.md`)

`pkg/` is generated output and is git-ignored. Regenerate it any time the
Rust source changes.

A plain `cargo build --target wasm32-unknown-unknown` (without `wasm-pack`)
also works and is useful as a fast compile-check, but it won't produce the
JS glue or `package.json` — use `wasm-pack build` for anything that needs to
run in a browser.

Other supported `wasm-pack` targets: `--target bundler` (webpack/Vite/etc.)
and `--target nodejs`. `templates/web/` uses `--target web` because it's
loaded directly by the browser with no bundler step.

## JS usage

```js
import init, { encode_qr_svg, scan_rgba } from "./pkg/tpt_barcode_wasm.js";

await init(); // loads and instantiates the .wasm module

// Encode
const svg = encode_qr_svg("https://example.com", "M", 4);
document.getElementById("out").innerHTML = svg;

// Scan (e.g. pixels from a <canvas> 2D context's ImageData)
const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
const results = scan_rgba(new Uint8Array(data.buffer), width, height);
for (const r of results) {
    console.log(r.format, r.text, r.corners); // corners: [x0,y0,x1,y1,x2,y2,x3,y3]
}
```

See `templates/web/` for a complete demo page (text-to-barcode generation
plus live camera scanning) built on this package.

## Public API

| Function                                              | Description                                                        |
|--------------------------------------------------------|----------------------------------------------------------------------|
| `encode_qr_svg(text, ec_level, module_size) -> string`  | QR Code → self-contained SVG string. `ec_level` is `"L"/"M"/"Q"/"H"`. |
| `encode_qr_png(text, ec_level, module_px, quiet_zone) -> Uint8Array` | QR Code → PNG bytes.                                 |
| `encode_code128_svg(text, module_size, height_px) -> string` | Code 128 (Subset B, printable ASCII) → SVG string.              |
| `scan_gray(pixels, width, height) -> WasmScanResult[]`  | Scan a luma8 buffer (`pixels.len() == width * height`).            |
| `scan_rgba(pixels, width, height) -> WasmScanResult[]`  | Scan an RGBA buffer straight from canvas `ImageData` (converts to grayscale internally). |

`WasmScanResult` exposes `.text`, `.format`, and `.corners` (flattened
`[x0,y0,x1,y1,x2,y2,x3,y3]` pixel coordinates) as getters.

`scan_gray`/`scan_rgba` try QR Code, Data Matrix, PDF417, Code 128, EAN-13,
UPC-A, and Code 39, and return every distinct symbol found (possibly empty —
that's not an error, it just means nothing decoded in this frame).

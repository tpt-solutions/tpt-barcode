# tpt-barcode (Python bindings)

Python bindings for [`tpt-barcode`](https://github.com/tpt-solutions/tpt-barcode),
a zero-allocation Rust barcode generation/scanning library, built with
[`pyo3`](https://pyo3.rs) and packaged with [`maturin`](https://www.maturin.rs).

This crate/package is **not published to PyPI**. It is meant to be built and
installed locally (or vendored into your own build pipeline).

## Building

Requires a Rust toolchain (edition 2021, `rustc >= 1.84`) and Python 3.8+.

Install `maturin` (in a virtualenv is recommended):

```sh
pip install maturin
```

From this directory (`crates/tpt-barcode-py`):

```sh
# Build + install into the active virtualenv/interpreter for local development
maturin develop --release

# Or build a wheel without installing it
maturin build --release
```

## Usage

```python
import tpt_barcode

# Encode a QR code as SVG
svg = tpt_barcode.encode_qr_svg("https://example.com", "M")
print(svg[:40])

# Encode a QR code as PNG bytes
png_bytes = tpt_barcode.encode_qr_png("https://example.com", "M", module_px=8, quiet_zone=4)
with open("qr.png", "wb") as f:
    f.write(png_bytes)

# Encode a Code 128 barcode as SVG
code128_svg = tpt_barcode.encode_code128_svg("HELLO-123")

# Scan a grayscale image buffer (one byte per pixel, row-major, width*height bytes)
results = tpt_barcode.scan(image_bytes, width, height, try_harder=True)
for r in results:
    print(r.text, r.format, r.bounding_box)
```

## API

- `encode_qr_svg(data: str, ec_level: str, module_size: int = 4) -> str`
  Encode `data` as a QR code and return a self-contained SVG string.
  `ec_level` is one of `"L"`, `"M"`, `"Q"`, `"H"` (case-insensitive).

- `encode_qr_png(data: str, ec_level: str, module_px: int = 8, quiet_zone: int = 4) -> bytes`
  Encode `data` as a QR code and return PNG-encoded bytes.

- `encode_code128_svg(data: str, module_size: int = 2, height_px: int = 80) -> str`
  Encode `data` (ASCII) as a Code 128 (subset B) barcode and return SVG.

- `scan(image_bytes: bytes, width: int, height: int, try_harder: bool = False) -> list[ScanResult]`
  Scan a grayscale (8-bit luma, one byte per pixel) image buffer for
  barcodes. `try_harder` enables adaptive binarization and perspective
  correction for QR codes at the cost of scan time.

- `ScanResult` — a result object with attributes:
  - `text: str` — decoded payload
  - `format: str` — e.g. `"QR Code"`, `"Code 128"`, `"EAN-13"`
  - `bounding_box: list[tuple[float, float]]` — four `(x, y)` corner points
    in source-image pixel coordinates

## Notes

- This package wraps the `std`, `1d`, `2d`, `scan`, `render`, and `png`
  features of the `tpt-barcode` Rust crate.
- The crate is intentionally excluded from the main Cargo workspace (see the
  root `Cargo.toml`) — `pyo3`'s `extension-module` feature disables linking
  against `libpython`, which would otherwise break `cargo test`/`cargo build`
  for the rest of the workspace. Build and test it standalone from this
  directory (`cargo build`, `maturin develop`/`maturin build`).

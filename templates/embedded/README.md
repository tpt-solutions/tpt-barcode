# Embedded template (`no_std` + alloc)

Minimal sketch showing how to generate a QR Code on a `no_std` target with
the `alloc` feature (e.g. an RTIC/embassy firmware crate).

## Cargo.toml snippet

```toml
[dependencies]
tpt-barcode = { version = "0.1", default-features = false, features = ["alloc", "2d"] }
```

## Sketch

See [`src/main.rs`](src/main.rs). The generated module grid can be drawn to
any framebuffer by iterating `qr.matrix` (`qr.size × qr.size`, 1 = dark) —
pair it with your display driver; no heap beyond the symbol itself.

This template is intentionally **not** a workspace member; copy it into your
own project.

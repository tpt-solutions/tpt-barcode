"""Smoke test for the tpt_barcode Python extension module.

Run with: .venv/Scripts/python smoke_test.py
"""

import tpt_barcode


def main():
    # 1. Encode QR as SVG
    svg = tpt_barcode.encode_qr_svg("https://example.com", "M")
    assert svg.startswith("<svg"), "QR SVG should start with <svg"
    print("encode_qr_svg OK:", svg[:60], "...")

    # 2. Encode QR as PNG bytes
    png_bytes = tpt_barcode.encode_qr_png("https://example.com", "M", module_px=8, quiet_zone=4)
    assert isinstance(png_bytes, bytes)
    assert png_bytes[:8] == b"\x89PNG\r\n\x1a\n", "should be a valid PNG signature"
    print(f"encode_qr_png OK: {len(png_bytes)} bytes, PNG signature verified")

    # 3. Encode Code128 as SVG
    code128_svg = tpt_barcode.encode_code128_svg("HELLO-123")
    assert code128_svg.startswith("<svg")
    print("encode_code128_svg OK:", code128_svg[:60], "...")

    # 4. Error handling: invalid EC level
    try:
        tpt_barcode.encode_qr_svg("x", "Z")
        raise AssertionError("expected ValueError for invalid EC level")
    except ValueError as e:
        print("invalid EC level correctly raised ValueError:", e)

    # 5. Scan: build a synthetic grayscale buffer (blank -> should find nothing)
    width, height = 64, 64
    blank = bytes([255]) * (width * height)
    results = tpt_barcode.scan(blank, width, height)
    assert results == [], "blank image should yield no results"
    print("scan (blank image) OK: 0 results as expected")

    # 6. Scan: wrong buffer size should raise ValueError
    try:
        tpt_barcode.scan(bytes([0]) * 10, width, height)
        raise AssertionError("expected ValueError for mismatched buffer size")
    except ValueError as e:
        print("mismatched buffer size correctly raised ValueError:", e)

    # 7. Round-trip: render the QR to PNG, load it with PIL if available, and
    #    feed the raw luma bytes back into scan() to confirm decode works.
    try:
        from io import BytesIO

        from PIL import Image

        img = Image.open(BytesIO(png_bytes)).convert("L")
        w, h = img.size
        raw = img.tobytes()
        scan_results = tpt_barcode.scan(raw, w, h, try_harder=True)
        assert len(scan_results) >= 1, "expected to decode the QR we just encoded"
        r = scan_results[0]
        assert r.text == "https://example.com", f"unexpected decoded text: {r.text}"
        assert r.format == "QR Code"
        assert len(r.bounding_box) == 4
        print("round-trip encode->scan OK:", repr(r))
    except ImportError:
        print("PIL not installed; skipping round-trip encode->scan test")

    print("\nAll smoke tests passed.")


if __name__ == "__main__":
    main()

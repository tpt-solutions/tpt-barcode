// tpt-barcode WASM demo — plain ES modules, no bundler.
//
// Expects the wasm-pack output of `tpt-barcode-wasm` to be available at
// ./vendor/tpt_barcode_wasm.js (see README.md in this directory for the
// build command). All barcode logic — encoding and scanning — lives in
// that crate; this file only wires it up to the DOM/camera.

import init, {
  encode_qr_svg,
  encode_code128_svg,
  scan_rgba,
} from "./vendor/tpt_barcode_wasm.js";

const $ = (id) => document.getElementById(id);

async function main() {
  await init();

  wireGenerate();
  wireScanner();
}

// ── Generate ─────────────────────────────────────────────────────────────

function wireGenerate() {
  const textInput = $("text-input");
  const ecLevel = $("ec-level");
  const qrOutput = $("qr-output");
  const code128Output = $("code128-output");
  const errorEl = $("generate-error");

  function generate() {
    errorEl.textContent = "";
    const text = textInput.value;

    try {
      qrOutput.innerHTML = encode_qr_svg(text, ecLevel.value, 4);
    } catch (err) {
      qrOutput.innerHTML = "";
      errorEl.textContent = `QR: ${err}`;
    }

    try {
      // Code 128 Subset B only supports printable ASCII.
      code128Output.innerHTML = encode_code128_svg(text, 2, 80);
    } catch (err) {
      code128Output.innerHTML = "";
      errorEl.textContent += (errorEl.textContent ? " | " : "") + `Code 128: ${err}`;
    }
  }

  $("generate-btn").addEventListener("click", generate);
  generate(); // render the default value on load
}

// ── Scan with camera ─────────────────────────────────────────────────────

function wireScanner() {
  const video = $("video");
  const overlay = $("overlay");
  const capture = $("capture");
  const status = $("camera-status");
  const results = $("scan-results");
  const scanBtn = $("scan-btn");
  const stopBtn = $("stop-btn");

  let stream = null;
  let rafHandle = null;

  async function start() {
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
        audio: false,
      });
    } catch (err) {
      status.textContent = `Camera access failed: ${err}`;
      return;
    }

    video.srcObject = stream;
    await video.play();

    capture.width = video.videoWidth;
    capture.height = video.videoHeight;
    overlay.width = video.videoWidth;
    overlay.height = video.videoHeight;

    status.textContent = "Scanning…";
    scanBtn.disabled = true;
    stopBtn.disabled = false;

    scheduleFrame();
  }

  function stop() {
    if (rafHandle !== null) {
      cancelAnimationFrame(rafHandle);
      rafHandle = null;
    }
    if (stream) {
      stream.getTracks().forEach((t) => t.stop());
      stream = null;
    }
    status.textContent = "Camera not started.";
    scanBtn.disabled = false;
    stopBtn.disabled = true;
    overlay.getContext("2d").clearRect(0, 0, overlay.width, overlay.height);
  }

  function scheduleFrame() {
    rafHandle = requestAnimationFrame(processFrame);
  }

  function processFrame() {
    if (!stream) return;

    const ctx = capture.getContext("2d", { willReadFrequently: true });
    ctx.drawImage(video, 0, 0, capture.width, capture.height);
    const { data, width, height } = ctx.getImageData(0, 0, capture.width, capture.height);

    let hits = [];
    try {
      hits = scan_rgba(new Uint8Array(data.buffer), width, height);
    } catch (err) {
      // Scanning errors on a live feed are expected (blur, no barcode in
      // frame, partial frame) — log and keep scanning rather than stopping.
      console.debug("scan_rgba error:", err);
    }

    drawOverlay(hits, width, height);
    renderResults(hits);

    scheduleFrame();
  }

  function drawOverlay(hits, width, height) {
    const ctx = overlay.getContext("2d");
    ctx.clearRect(0, 0, width, height);
    ctx.strokeStyle = "#00e676";
    ctx.lineWidth = 3;
    for (const hit of hits) {
      const c = hit.corners; // [x0,y0,x1,y1,x2,y2,x3,y3]
      ctx.beginPath();
      ctx.moveTo(c[0], c[1]);
      ctx.lineTo(c[2], c[3]);
      ctx.lineTo(c[4], c[5]);
      ctx.lineTo(c[6], c[7]);
      ctx.closePath();
      ctx.stroke();
    }
  }

  function renderResults(hits) {
    if (hits.length === 0) {
      results.textContent = "";
      return;
    }
    results.textContent = hits
      .map((h) => `[${h.format}] ${h.text}`)
      .join("\n");
  }

  scanBtn.addEventListener("click", start);
  stopBtn.addEventListener("click", stop);
}

main();

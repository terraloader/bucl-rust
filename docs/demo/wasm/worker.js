// worker.js — BUCL WASM execution in a dedicated thread.
//
// Runs the evaluator off the main thread so the UI stays responsive during
// long-running scripts (especially sleep calls).
//
// js_sleep uses Atomics.wait() when SharedArrayBuffer is available (a real
// OS-level sleep, zero CPU spin).  When the page is not cross-origin-isolated
// (e.g. plain GitHub Pages without extra headers), SharedArrayBuffer is
// unavailable and we fall back to a Date.now() spin-loop.  Either way the
// main thread is never blocked.

'use strict';

const enc = new TextEncoder();
const dec = new TextDecoder();

// ── sleep host function ─────────────────────────────────────────────────────

function js_sleep(ms) {
  if (typeof SharedArrayBuffer !== 'undefined') {
    // Genuine OS-level block — no CPU spin.
    // Atomics.wait is allowed in workers and blocks the worker thread only.
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, Math.ceil(ms));
  } else {
    // SharedArrayBuffer unavailable (page not cross-origin-isolated).
    // Spin-loop: still only blocks this worker thread — UI stays responsive.
    const end = Date.now() + ms;
    while (Date.now() < end) { /* spin */ }
  }
}

// ── WASM bootstrap ──────────────────────────────────────────────────────────

let wasmExports = null;

const imports = {
  env: {
    js_math_random: () => Math.random(),
    js_sleep,
  },
};

async function init() {
  const response = await fetch('pkg/bucl_wasm.wasm');
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const { instance } = await WebAssembly.instantiateStreaming(response, imports);
  wasmExports = instance.exports;
  postMessage({ type: 'ready' });
}

// ── BUCL run ────────────────────────────────────────────────────────────────

function runBucl(source) {
  const { memory, bucl_alloc, bucl_free, bucl_run } = wasmExports;

  const srcBytes = enc.encode(source);
  const srcPtr   = bucl_alloc(srcBytes.length);
  new Uint8Array(memory.buffer, srcPtr, srcBytes.length).set(srcBytes);

  const outPtr = bucl_run(srcPtr, srcBytes.length);
  bucl_free(srcPtr, srcBytes.length);

  // Output layout: [u32-le length][utf-8 bytes]
  const view     = new DataView(memory.buffer, outPtr);
  const outLen   = view.getUint32(0, /*littleEndian=*/true);
  const outBytes = new Uint8Array(memory.buffer, outPtr + 4, outLen);
  const output   = dec.decode(outBytes);
  bucl_free(outPtr, 4 + outLen);

  return output;
}

// ── Message handler ─────────────────────────────────────────────────────────

self.onmessage = ({ data }) => {
  if (data.type === 'run') {
    let output;
    try {
      output = runBucl(data.source);
    } catch (err) {
      output = '[runtime error] ' + err.message;
    }
    postMessage({ type: 'result', output });
  }
};

// ── Start ────────────────────────────────────────────────────────────────────

init().catch(err => {
  postMessage({ type: 'load-error', message: err.message });
});

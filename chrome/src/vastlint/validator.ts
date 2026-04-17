/**
 * Thin async wrapper around the vastlint WASM module.
 *
 * The bundler-target `vastlint_wasm.js` uses a static `import * as wasm from "...bg.wasm"`
 * which doesn't work in a Chrome extension (no static WASM imports in content scripts).
 *
 * Instead we manually instantiate the WASM from the bg bindings:
 *  1. Fetch the binary via chrome.runtime.getURL (the .wasm is copied to dist/ by esbuild)
 *  2. Call WebAssembly.instantiate with the import object from _bg.js
 *  3. Wire up __wbg_set_wasm + __wbindgen_start
 *  4. Expose validate() for use in the content script
 */

import type { ValidationResult } from '../types/vastlint';
// @ts-ignore — no bundler types for the raw wasm-pack bg shim
import * as bg from 'vastlint/vastlint_wasm_bg.js';

let initPromise: Promise<void> | null = null;

async function ensureInit(): Promise<void> {
  if (initPromise) return initPromise;

  initPromise = (async () => {
    const binaryUrl = chrome.runtime.getURL('vastlint_wasm_bg.wasm');

    // Build the import object from the bg bindings (all __wbg_* and __wbindgen_* exports)
    const importObject: Record<string, Record<string, unknown>> = {};
    for (const [key, val] of Object.entries(bg as Record<string, unknown>)) {
      if (typeof val === 'function' && (key.startsWith('__wbg_') || key.startsWith('__wbindgen_'))) {
        importObject['./vastlint_wasm_bg.js'] ??= {};
        importObject['./vastlint_wasm_bg.js'][key] = val;
      }
    }

    // Use instantiateStreaming — Chrome MV3 CSP blocks instantiate(ArrayBuffer)
    // but allows streaming instantiation from extension-own URLs.
    const { instance } = await WebAssembly.instantiateStreaming(
      fetch(binaryUrl),
      importObject as WebAssembly.Imports,
    );
    (bg as unknown as { __wbg_set_wasm: (w: WebAssembly.Instance['exports']) => void })
      .__wbg_set_wasm(instance.exports);

    // wasm-bindgen calls __wbindgen_start to init the extern ref table etc.
    const start = instance.exports.__wbindgen_start as (() => void) | undefined;
    start?.();
  })();

  return initPromise;
}

export async function validateVast(xml: string): Promise<ValidationResult> {
  await ensureInit();
  return (bg as unknown as { validate: (xml: string) => ValidationResult }).validate(xml);
}

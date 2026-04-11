#!/usr/bin/env node
/**
 * scripts/assemble.js
 *
 * Run after both `wasm-pack build` targets to assemble the publishable package.
 *
 * Copies the wasm-pack output files from pkg/ (bundler) and pkg-node/ (nodejs)
 * into the npm/ root so the package is fully self-contained and flat.
 * npm's `files` field then includes them without any .gitignore interference.
 *
 * Usage (called by package.json scripts — always run both builds first):
 *   wasm-pack build crates/vastlint-wasm --target bundler --out-dir ../../npm/pkg
 *   wasm-pack build crates/vastlint-wasm --target nodejs  --out-dir ../../npm/pkg-node
 *   node npm/scripts/assemble.js   (from the vastlint/ workspace root)
 */

import { copyFileSync, existsSync, readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');

function copy(src, dst) {
  if (!existsSync(src)) {
    console.error(`[assemble] ERROR: missing ${src}`);
    process.exit(1);
  }
  copyFileSync(src, dst);
  console.log(`[assemble] ${src.replace(root, '.')} → ${dst.replace(root, '.')}`);
}

// From bundler target: the ESM glue + wasm binary
copy(join(root, 'pkg/vastlint_wasm.js'),         join(root, 'vastlint_wasm.js'));
copy(join(root, 'pkg/vastlint_wasm.d.ts'),        join(root, 'vastlint_wasm.d.ts'));
copy(join(root, 'pkg/vastlint_wasm_bg.js'),       join(root, 'vastlint_wasm_bg.js'));
copy(join(root, 'pkg/vastlint_wasm_bg.wasm'),     join(root, 'vastlint_wasm_bg.wasm'));
copy(join(root, 'pkg/vastlint_wasm_bg.wasm.d.ts'),join(root, 'vastlint_wasm_bg.wasm.d.ts'));

// From nodejs target: the CJS glue (different JS, same wasm binary)
copy(join(root, 'pkg-node/vastlint_wasm.js'),     join(root, 'vastlint_wasm_cjs.js'));

// Patch vastlint_wasm_cjs.js: guard wasm.__wbindgen_start() which crashes when
// the WASM binary doesn't export __wbindgen_start (wasm-bindgen omits it when
// there is no #[wasm_bindgen(start)] function).
const cjsPath = join(root, 'vastlint_wasm_cjs.js');
const cjs = readFileSync(cjsPath, 'utf8');
const patched = cjs.replace(
  /^wasm\.__wbindgen_start\(\);$/m,
  'if (typeof wasm.__wbindgen_start === "function") wasm.__wbindgen_start();'
);
if (patched !== cjs) {
  writeFileSync(cjsPath, patched);
  console.log('[assemble] patched vastlint_wasm_cjs.js — guarded __wbindgen_start');
}

console.log('[assemble] done');

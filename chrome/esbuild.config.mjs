// esbuild.config.mjs
// Bundles the content script, service worker, and popup JS.
// The WASM binary is copied to dist/ directly and loaded at runtime via
// chrome.runtime.getURL — no custom plugin needed.

import * as esbuild from 'esbuild';
import { copy } from 'esbuild-plugin-copy';
import { dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const watch = process.argv.includes('--watch');

const sharedConfig = {
  bundle: true,
  format: 'iife',
  target: 'chrome120',
  logLevel: 'info',
};

const builds = [
  // Content script — injected into every page
  {
    ...sharedConfig,
    entryPoints: ['src/content/index.ts'],
    outfile: 'dist/content.js',
    plugins: [
      copy({
        resolveFrom: __dirname,
        assets: [
          { from: ['src/popup/popup.html'],                              to: ['dist/popup.html'] },
          { from: ['src/analysis/analysis.html'],                        to: ['dist/analysis.html'] },
          { from: ['manifest.json'],                                     to: ['dist/manifest.json'] },
          { from: ['node_modules/vastlint/vastlint_wasm_bg.wasm'],       to: ['dist/vastlint_wasm_bg.wasm'] },
          { from: ['src/icons/*'],                                       to: ['dist/icons'] },
        ],
      }),
    ],
  },
  // Service worker (MV3 background)
  {
    ...sharedConfig,
    format: 'esm',
    entryPoints: ['src/background/service-worker.ts'],
    outfile: 'dist/service-worker.js',
  },
  // Popup page script
  {
    ...sharedConfig,
    entryPoints: ['src/popup/popup.ts'],
    outfile: 'dist/popup.js',
  },
  // Main-world nav hook — patches history.pushState/replaceState in the page context
  {
    ...sharedConfig,
    entryPoints: ['src/nav-hook/nav-hook.ts'],
    outfile: 'dist/nav-hook.js',
  },
  // Full-page analysis tab
  {
    ...sharedConfig,
    entryPoints: ['src/analysis/analysis.ts'],
    outfile: 'dist/analysis.js',
  },
];

if (watch) {
  const ctxs = await Promise.all(builds.map(c => esbuild.context(c)));
  await Promise.all(ctxs.map(c => c.watch()));
  console.log('Watching for changes…');
} else {
  await Promise.all(builds.map(c => esbuild.build(c)));
}

// esbuild.config.mjs
// Bundles the content script, service worker, and popup JS.
// The WASM binary is copied to dist/ directly and loaded at runtime via
// chrome.runtime.getURL — no custom plugin needed.

import * as esbuild from 'esbuild';
import { cp, mkdir } from 'node:fs/promises';
import { watch as watchDirectory } from 'node:fs';
import { basename, dirname } from 'path';
const watch = process.argv.includes('--watch');

const assets = [
  ['src/popup/popup.html', 'dist/popup.html'],
  ['src/analysis/analysis.html', 'dist/analysis.html'],
  ['manifest.json', 'dist/manifest.json'],
  ['node_modules/vastlint/vastlint_wasm_bg.wasm', 'dist/vastlint_wasm_bg.wasm'],
  ['src/icons', 'dist/icons'],
];

async function copyAssets() {
  await mkdir('dist', { recursive: true });
  await Promise.all(assets.map(([from, to]) => cp(from, to, {
    recursive: true,
    filter: (file) => !basename(file).startsWith('.'),
  })));
}

const copyPlugin = {
  name: 'extension-assets',
  setup(build) {
    build.onEnd(async (result) => {
      if (!result.errors.length) await copyAssets();
    });
  },
};

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
    plugins: [copyPlugin],
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
  const directories = new Map();
  for (const [from] of assets) {
    const directory = from === 'src/icons' ? from : dirname(from);
    if (!directories.has(directory)) directories.set(directory, new Set());
    directories.get(directory).add(from === 'src/icons' ? '*' : basename(from));
  }
  let pending;
  for (const [directory, names] of directories) {
    watchDirectory(directory, (_event, file) => {
      if (file && !names.has('*') && !names.has(String(file))) return;
      clearTimeout(pending);
      pending = setTimeout(() => copyAssets().catch(console.error), 50);
    });
  }
  console.log('Watching for changes…');
} else {
  await Promise.all(builds.map(c => esbuild.build(c)));
}

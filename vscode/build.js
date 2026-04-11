const esbuild = require('esbuild');
const path = require('path');
const fs = require('fs');

fs.mkdirSync(path.resolve(__dirname, 'out'), { recursive: true });

// Copy the whole vastlint package into out/deps/vastlint so that it ships
// inside the VSIX.  We avoid "node_modules" in the path because vsce refuses
// to recurse into any directory called node_modules.
// Resolve the symlink so cpSync does a true deep copy (npm `file:` deps are
// symlinks and cpSync preserves them by default on macOS).
const pkgSrcResolved = fs.realpathSync(path.resolve(__dirname, 'node_modules/vastlint'));
const pkgDst = path.resolve(__dirname, 'out/deps/vastlint');
fs.rmSync(pkgDst, { recursive: true, force: true });
fs.cpSync(pkgSrcResolved, pkgDst, { recursive: true, dereference: true });

// Strip the copy down to the 4 files needed at runtime (CJS path only):
//   index.cjs, vastlint_wasm_cjs.js, vastlint_wasm_bg.wasm, package.json
const keep = new Set(['index.cjs', 'vastlint_wasm_cjs.js', 'vastlint_wasm_bg.wasm', 'package.json']);
for (const entry of fs.readdirSync(pkgDst)) {
  if (!keep.has(entry)) {
    fs.rmSync(path.join(pkgDst, entry), { recursive: true, force: true });
  }
}

// Patch the copied package.json: remove "type":"module" so Node treats all
// .js files as CJS (vastlint_wasm_cjs.js uses `exports` and must run as CJS).
// Also expose ./index.cjs as an explicit export subpath so require() can find it.
const pkgJsonPath = path.join(pkgDst, 'package.json');
const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf8'));
delete pkgJson.type;
pkgJson.exports['./index.cjs'] = './index.cjs';
pkgJson.exports['./vastlint_wasm_cjs.js'] = './vastlint_wasm_cjs.js';
fs.writeFileSync(pkgJsonPath, JSON.stringify(pkgJson, null, 2));

console.log('Copied node_modules/vastlint → out/deps/vastlint (patched type:module)');

esbuild.build({
  entryPoints: ['src/extension.ts'],
  bundle: true,
  outfile: 'out/extension.js',
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  sourcemap: true,
  loader: { '.wasm': 'file' },
  define: {},
  plugins: [{
    name: 'vastlint-redirect',
    setup(build) {
      // Rewrite bare 'vastlint' imports to the local deps copy so the
      // extension works without a real node_modules tree in the VSIX.
      build.onResolve({ filter: /^vastlint(\/|$)/ }, args => ({
        path: args.path.replace(/^vastlint/, './deps/vastlint'),
        external: true,
      }));
    },
  }],
}).then(() => {
  console.log('esbuild: extension.js built');
}).catch((e) => {
  console.error(e);
  process.exit(1);
});

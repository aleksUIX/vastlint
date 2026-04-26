#!/usr/bin/env bash
# test-local.sh — rebuild everything from Rust → VSIX and install locally.
#
# Run from the vastlint/ workspace root:
#   cd /path/to/vastlint
#   bash vscode/test-local.sh
#
# Prerequisites: wasm-pack, node, npm, vsce

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== 1/5  Build WASM (nodejs + bundler targets) ==="
wasm-pack build crates/vastlint-wasm \
  --target nodejs  --out-dir ../../npm/pkg-node --release
wasm-pack build crates/vastlint-wasm \
  --target bundler --out-dir ../../npm/pkg      --release

echo "=== 2/5  Assemble npm package ==="
node npm/scripts/assemble.js

echo "=== 3/5  Verify new rule is present in WASM ==="
node - <<'JS'
const w = require('./npm/vastlint_wasm_cjs.js');
const rs = w.rules();
const r = rs.find(r => r.id === 'VAST-2.0-linear-tracking-quartiles');
if (!r) { console.error('ERROR: VAST-2.0-linear-tracking-quartiles NOT in WASM'); process.exit(1); }
console.log('OK  rule present:', r.id, '|', r.source, '|', r.default_severity);
console.log('    total rules:', rs.length);

// Smoke-test the new rule fires on the test fixture
const fs = require('fs');
const xml = fs.readFileSync(
  'crates/vastlint-core/tests/fixtures/warn_linear_no_quartile_tracking.xml', 'utf8');
const res = w.validate(xml);
const hit = res.issues.find(i => i.id === 'VAST-2.0-linear-tracking-quartiles');
if (!hit) { console.error('ERROR: rule did not fire on test fixture'); process.exit(1); }
console.log('OK  rule fires on fixture:', hit.message.slice(0, 60) + '…');
JS

echo "=== 4/5  Bundle extension ==="
cd vscode
npm install
node build.js

echo "=== 5/5  Package VSIX ==="
# Remove the old 0.4.0 VSIX if present so vsce creates a fresh one
rm -f vastlint-0.4.0.vsix
npx vsce package --no-dependencies

VSIX=$(ls vastlint-*.vsix | tail -1)
echo ""
echo "Built: vscode/$VSIX"
echo ""
echo "=== Install locally ==="
echo "In VS Code: Cmd+Shift+P → 'Extensions: Install from VSIX...' → select vscode/$VSIX"
echo ""
echo "=== Smoke test ==="
echo "1. Open crates/vastlint-core/tests/fixtures/warn_linear_no_quartile_tracking.xml"
echo "2. Expect 1 warning squiggle on <Linear> for VAST-2.0-linear-tracking-quartiles"
echo "3. Hover → tooltip should show 'Add <TrackingEvents> inside <Linear>…'"
echo "4. Open a fully-valid VAST file — no squiggles expected"

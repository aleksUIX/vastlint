# vastlint

VAST XML validator for JavaScript and TypeScript. Checks ad tags against IAB Tech Lab VAST 2.0 through 4.3. Powered by a Rust/WASM core.

[![npm](https://img.shields.io/npm/v/vastlint)](https://www.npmjs.com/package/vastlint)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](../vastlint/LICENSE)

---

## Install

```sh
npm install vastlint
```

## Environment support

| Environment | Support | Notes |
|---|---|---|
| Node.js (ESM `import`) | ✅ | Tested |
| Node.js (CJS `require`) | ✅ | Tested |
| Vite | ✅ | Static `.wasm` import handled natively |
| Webpack 5 | ✅ | Requires `experiments: { asyncWebAssembly: true }` |
| Rollup | ✅ | Requires `@rollup/plugin-wasm` |
| Deno | ✅ | Supports static WASM imports natively |
| Cloudflare Workers | ⚠️ | WASM supported but requires Workers-specific binding syntax |
| `<script type="module">` (no bundler) | ❌ | Browsers cannot statically import `.wasm` - use a bundler |

This package uses the `wasm-pack --target bundler` output. It is designed for use inside a build pipeline (Vite, Webpack, Rollup, etc.) or in Node.js directly. For a raw browser drop-in without a bundler, a separate `web` target build is needed.

## Usage

```ts
import { validate } from 'vastlint';

const result = validate(xmlString);

if (!result.summary.valid) {
  for (const issue of result.issues) {
    console.error(`[${issue.severity}] ${issue.id}: ${issue.message}`);
    if (issue.path) console.error(`  at ${issue.path}`);
  }
}
```

### With options

```ts
import { validateWithOptions } from 'vastlint';

const result = validateWithOptions(xml, {
  wrapper_depth: 2,
  max_wrapper_depth: 5,
  rule_overrides: {
    'VAST-2.0-mediafile-https': 'off',   // silence HTTP advisory
    'VAST-2.0-root-version': 'error',    // treat missing version as hard error
  },
});
```

### Filter by severity

```ts
import { validateFiltered } from 'vastlint';

// Only return errors, ignore warnings and infos
const result = validateFiltered(xml, 'error');
```

### List all rules

```ts
import { rules } from 'vastlint';

for (const rule of rules()) {
  console.log(rule.id, rule.default_severity, rule.description);
}
```

---

## Result shape

```ts
{
  version: string | null,        // detected VAST version, e.g. "4.2"
  summary: {
    errors: number,
    warnings: number,
    infos: number,
    valid: boolean,              // true when errors === 0
  },
  issues: Array<{
    id: string,                  // stable rule ID, e.g. "VAST-4.1-universal-ad-id"
    severity: "error" | "warning" | "info",
    message: string,
    path: string | null,         // XPath-like location
    spec_ref: string,            // e.g. "IAB VAST 4.1 §3.8.1"
  }>,
}
```

---

## CommonJS

```js
const { validate } = require('vastlint');
const result = validate(xml);
```

---

## Source

The Rust source and CLI tool live at [github.com/aleksUIX/vastlint](https://github.com/aleksUIX/vastlint).

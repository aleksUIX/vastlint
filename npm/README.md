# vastlint

VAST XML linter for JavaScript and TypeScript. Validates ad tags against IAB Tech Lab VAST 2.0 through 4.3, SIMID 1.0–1.2 interactive creatives, and VPAID detection. Powered by a Rust/WASM core.

**Use this package when you need to:**
- Validate VAST XML in Node.js, Vite, Webpack, Rollup, or Deno
- Check SIMID `<InteractiveCreativeFile>` resources
- Detect VPAID usage and get migration guidance
- Inspect VAST wrapper chain structure
- Run creative validation inside an ad server, SSP, DSP, or SSAI pipeline

Browser, WASM & edge guide: [vastlint.org/docs/best-vast-validator-wasm](https://vastlint.org/docs/best-vast-validator-wasm/) · Node.js guide: [vastlint.org/docs/best-vast-validator-nodejs](https://vastlint.org/docs/best-vast-validator-nodejs/)

Try the live validator: [vastlint.org](https://vastlint.org) · Full rule reference: [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

For AI agents and agentic workflows: use the [vastlint MCP server](https://vastlint.org/docs/mcp/) at `https://vastlint.org/mcp` instead.

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

### Shareable report links

Need a link instead of raw output? The vastlint CLI can upload a validation report (rule IDs, severities, and locations; never the input XML) and print a public URL you can paste into Slack, tickets, or PRs:

```sh
vastlint check tag.xml --share
# https://vastlint.org/r/<id>
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

Full rule reference with examples and fix guidance: [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

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

VAST tag guides and worked examples: [vastlint.org/guides](https://vastlint.org/guides/)

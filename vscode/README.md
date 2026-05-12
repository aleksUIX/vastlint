# vastlint - VAST XML Validator for VS Code

Inline linting for [IAB VAST](https://iabtechlab.com/standards/vast/) ad tags directly in VS Code.
Supports VAST 2.0 through 4.3 with clean Problems entries, concise hovers, fix guidance, and direct docs links. Web validator and full documentation: [vastlint.org](https://vastlint.org)

## Features

- **Inline squiggles** — red (error), yellow (warning), blue (info) underlines on the offending XML tag
- **Hover tooltips** — concise issue summary, fix guidance, and a direct rule docs link
- **Problems panel** — clean issue titles with file, line, and column; click to jump directly to the tag
- **Any file type** — works in `.xml`, `.html`, `.js`, `.ts`, `.json`, templates — anywhere `<VAST>` appears
- **Multi-block** — validates every `<VAST>...</VAST>` block in a file independently
- **Live as you type** — re-validates 500 ms after you stop typing, and on every save
- **CLI backend** — uses the `vastlint` CLI binary when available, falls back to WASM in-process
- **121 rules** across VAST 2.0–4.3 plus SIMID validation: required fields, schema structure, URLs, deprecations, and CTV/SSAI advisories

## How it looks

Hover over a squiggled tag:

```
🔴 <InLine> is missing required <AdSystem>

✅ Fix: Add `<AdSystem>` inside `<InLine>`, e.g. `<AdSystem>My Ad Server</AdSystem>`.

Docs: vastlint.org/docs/rules/VAST-2.0-inline-adsystem
```

## Settings

| Setting | Default | Description |
|---|---|---|
| `vastlint.enable` | `true` | Enable/disable diagnostics |
| `vastlint.minSeverity` | `"info"` | Minimum severity to show: `"error"`, `"warning"`, or `"info"` |
| `vastlint.ruleOverrides` | `{}` | Per-rule severity overrides (WASM fallback only — use `vastlint.toml` in CLI mode) |
| `vastlint.templateIgnoreRegex` | `""` | Regex for template expressions to blank out before validation (e.g. `\$\{[^}]+\}`) |
| `vastlint.vastVersion` | `""` | Force a VAST version (`"2.0"`, `"3.0"`, `"4.0"`, `"4.1"`, `"4.2"`, `"4.3"`) — blank = auto-detect |
| `vastlint.cliPath` | `"vastlint"` | Path to the `vastlint` CLI binary. Falls back to common install locations, then WASM. |

### Example: silence HTTP warnings

```json
// .vscode/settings.json
{
  "vastlint.ruleOverrides": {
    "VAST-2.0-mediafile-https": "off",
    "VAST-2.0-tracking-https": "off"
  }
}
```

### Example: only show errors

```json
{
  "vastlint.minSeverity": "error"
}
```

## Rules

vastlint checks 121 rules across:
- Required elements and attributes (VAST 2.0–4.3)
- Value formats (durations, URLs, enums)
- Schema conformance (unknown elements/attributes)
- Deprecation warnings ([VPAID](https://vastlint.org/guides/vast-vpaid-migration), Flash, Survey, conditionalAd)
- [SIMID](https://vastlint.org/docs/simid/) interactive creative validation (`<InteractiveCreativeFile apiFramework="SIMID">`)
- Security (HTTP vs HTTPS)
- CTV/SSAI best practices (Mezzanine, AdServingId)
- Structural issues ([wrapper depth](https://vastlint.org/guides/vast-wrapper-chains), ad sequence, duplicate impressions)

Full rule reference with examples and fix guidance: [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

Canonical rule catalog:

- [RULES.md](../RULES.md) in this repo
- [vastlint.org/docs/rules](https://vastlint.org/docs/rules/) for the hosted per-rule pages

## Requirements

No external tools needed - the validator runs entirely in-process via WebAssembly.

## License

Apache-2.0 - see [LICENSE](../LICENSE)

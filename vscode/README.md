# vastlint - VAST XML Validator for VS Code

Inline linting for [IAB VAST](https://iabtechlab.com/standards/vast/) ad tags directly in VS Code.
Supports VAST 2.0 through 4.3.

## Features

- **Inline squiggles** - red (error), yellow (warning), blue (info) underlines on the offending XML tag
- **Hover tooltips** - hover over any squiggle to see the rule ID, what's wrong, and exactly how to fix it
- **Problems panel** - all issues listed with file, line, and column; click to jump directly to the tag
- **Live as you type** - re-validates 500ms after you stop typing, and on every save
- **Configurable** - silence rules, change severities, or set a minimum severity threshold

## How it looks

Hover over a squiggled tag:

```
🔴 vastlint `VAST-2.0-inline-adsystem`

<InLine> is missing required <AdSystem>

✅ Fix: Add `<AdSystem>` inside `<InLine>`, e.g. `<AdSystem>My Ad Server</AdSystem>`.

📖 IAB VAST 2.0 §2.3.1  -  `/VAST/Ad[0]/InLine`
```

## Settings

| Setting | Default | Description |
|---|---|---|
| `vastlint.enable` | `true` | Enable/disable diagnostics |
| `vastlint.minSeverity` | `"info"` | Minimum severity to show: `"error"`, `"warning"`, or `"info"` |
| `vastlint.ruleOverrides` | `{}` | Per-rule overrides, e.g. `{ "VAST-2.0-mediafile-https": "off" }` |

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

vastlint checks ~80 rules across:
- Required elements and attributes (VAST 2.0–4.3)
- Value formats (durations, URLs, enums)
- Schema conformance (unknown elements/attributes)
- Deprecation warnings (VPAID, Flash, Survey, conditionalAd)
- Security (HTTP vs HTTPS)
- CTV/SSAI best practices (Mezzanine, AdServingId)
- Structural issues (wrapper depth, ad sequence, duplicate impressions)

See the [full rule catalog](https://github.com/aleksUIX/vastlint) for details.

## Requirements

No external tools needed - the validator runs entirely in-process via WebAssembly.

## License

Apache-2.0 - see [LICENSE](../LICENSE)

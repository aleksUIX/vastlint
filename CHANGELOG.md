# Changelog

All notable changes to vastlint are documented here.
Versions follow [Semantic Versioning](https://semver.org/).
GitHub Releases: <https://github.com/aleksUIX/vastlint/releases>

---

## [0.4.3] - 2026-04-30

### CLI (`vastlint-cli`)

- **`--vast-version <version>`** (`check` and `fix`) — override the VAST version used for validation, ignoring the `version=` attribute in the XML. Accepts `2.0`, `3.0`, `4.0`, `4.1`, `4.2`, `4.3`. Useful for enforcing a floor version across all incoming tags or testing how a tag scores against a target version.
- **`--ignore-pattern <regex>`** (`check` and `fix`) — replace all matches of the supplied regular expression with a valid HTTPS placeholder before validation. Designed for ad-server templating macros (`${IMPRESSION_URL}`, `%%CACHEBUSTER%%`) that would otherwise trigger URL-format errors on unresolved placeholders. The substitution is in-memory only — the original file is never modified.

### vastlint-core

- `ValidationContext` gains `forced_version: Option<VastVersion>` — when `Some`, skips XML version detection entirely and uses the supplied value. Used by the CLI flags above; available to library consumers.
- `VastVersion` now derives `Copy`.

### OTP port daemon

- **`vastlint daemon`** subcommand — speaks the Erlang `{:packet, 4}` binary framing protocol over stdin/stdout. Reads 4-byte big-endian length + raw UTF-8 VAST XML; writes 4-byte big-endian length + JSON validation result. Safe for production Elixir pipelines via `NimblePool` (each worker holds one persistent `Port`). Does not require the NIF.

---

## [0.4.2] - 2026-04-28

### VS Code extension

- **Kiro compatibility**: lowered `engines.vscode` minimum from `1.116.0` to `1.85.0` so the extension installs on Amazon Kiro and other VS Code forks with older API versions

---

## [0.4.1] - 2026-04-26

### VS Code extension

- **Hover tooltip redesign**: severity icons replaced with flat color squares (🟥 error, 🟨 warning, 🟦 info); 🔧 for fix hints
- **Compact hover layout**: collapsed from 5 spaced lines to 3 tight lines per issue
- **Rule ID links to docs**: each rule ID in the hover footer is now a clickable link to `vastlint.org/docs/rules/<id>/`
- **Fix hints coverage**: added missing hints for `VAST-3.0-bitrate-conflict`, `VAST-3.0-minmaxbitrate-pair`, `VAST-2.0-nonlinear-resource`, `VAST-4.0-interactive-creative-no-api`, `VAST-4.1-interactive-creative-type`, `VAST-3.0-pricing-model-case`; removed stale key `VAST-2.0-mediafile-bitrate-conflict`

---

## [0.4.0] - 2026-04-29

### Revenue impact classification (vastlint-core)

- **New `RuleSource::IndustryBestPractice`** - distinct from `VastSpec` and `Inferred`; renders as `"revenue impact"` in all output formats
- **New `RuleMeta::revenue_impact()`** - returns `true` for 12 rules where a structural defect causes direct measurement or delivery loss; no catalog field added, no breaking schema change
- **5 rules reclassified** from `Inferred` → `IndustryBestPractice`: `VAST-2.0-mediafile-https`, `VAST-2.0-tracking-https`, `VAST-2.0-duplicate-impression`, `VAST-4.1-mezzanine-recommended`, `VAST-4.1-vpaid-in-interactive-context`
- **HTTP tracker rules promoted** `Info` → `Warning`: `VAST-2.0-mediafile-https` and `VAST-2.0-tracking-https` - on HTTPS inventory these are guaranteed delivery failures, not advisory notices
- **New rule `VAST-2.0-linear-tracking-quartiles`** (`Warning`, `IndustryBestPractice`) - fires when a `<Linear>` creative has no `<TrackingEvents>` containing any of `start`, `firstQuartile`, `midpoint`, `thirdQuartile`, or `complete`; absence of all five is a complete measurement blackout. Spec reference: IAB VAST 4.1 §3.14.2

### Monitoring-friendly CLI (vastlint-cli)

- **`--fail-on-warning`** - exits non-zero when any warning is found; all 12 revenue-impact rules fire at `Warning` or `Error` severity, making this flag sufficient for a CI revenue gate
- **URL input with wrapper chain following** - `vastlint check https://…` fetches the tag and recursively follows `<VASTAdTagURI>` wrapper chains
- **`--max-depth N`** (default `5`) - controls how deep wrapper chains are followed, matching the IAB VAST 4.x recommendation
- **`--summary`** - prints aggregate pass/fail counts after validation; includes a `$revenue` line when any revenue-impact rules fired; works in both plain and JSON output modes

### Rule list (`vastlint rules`)

- New `$` column - marks the 12 revenue-impact rules
- Legend line added at the bottom of the table

### Rule count

118 rules total (was 108 before v0.3.x additions; 117 before this release).

---

## [0.3.7] - 2026-04-25

- CI: harden release pipeline (SLSA provenance, deploy key scoping)
- VS Code: align `engines.vscode` to `^1.116.0`

## [0.3.6] - 2026-04-25

- **Chrome extension**: v0.2.0 - HTML-rendered VAST detection, inline overlay annotations, privacy policy; CWS submission workflow
- CI: SLSA provenance signing; Smithery and MCP Registry idempotent publish

## [0.3.4] - 2026-04-18

- Security: patched two advisories (`idna` RUSTSEC-2024-0421, `rustls-webpki` RUSTSEC-2026-0098/0099); added `cargo audit` to CI
- Fuzz: cargo-fuzz targets for `validate`, `fix`, and `validate_wrapper`

## [0.3.3] - 2026-04-18

- **SIMID rules**: 9 rules covering SIMID 1.0 (linear) and SIMID 1.1 (nonlinear) - type, URL, HTTPS, `variableDuration`, `<MediaFile>` fallback, `<IFrameResource>` presence
- Docs: SIMID coverage expanded to all spec versions (1.0, 1.1, 1.2) on vastlint.org

## [0.3.2] - 2026-04-17

- **MCP server**: `vastlint-mcp` crate published to MCP Registry; tools: `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, `fix_vast`

## [0.3.1] - 2026-04-17

- **Auto-fix in VS Code**: inline quick-fix actions wired up; fix API exported from npm package
- **Open VSX**: extension now published to Open VSX Registry in addition to VS Code Marketplace

## [0.3.0] - 2026-04-17

- **Erlang/Elixir NIF** (`vastlint_nif`): native binding for BEAM-based ad servers and RTB platforms
- Performance docs updated to production-realistic benchmarks (17–44 KB tags)

---

## [0.2.6] - 2026-04-12

- Build: idempotent `cargo publish` and `vsce publish` (skip if version already exists)
- WASM: smoke test fixes; both targets built before assemble step

## [0.2.5] - 2026-04-11

- npm + WASM packages added; `vastlint` available on npm for browser and Node.js use

## [0.2.4] - 2026-04-11

- **Line/column positions**: all issues now include `line` and `col` in JSON output and VS Code diagnostics

## [0.2.3] - 2026-04-08

- **FFI C layer** (`vastlint-ffi`): `libvastlint` shared library with C header; Go binding (`vastlint-go`) backed by the same core
- `mimalloc` global allocator in CLI and FFI for lower memory overhead

## [0.2.2] - 2026-04-08

- Release pipeline fixes: provenance cascade on skipped jobs, version bump order

## [0.2.1] - 2026-04-08

- Telemetry endpoint fix

## [0.2.0] - 2026-04-08

- **Go binding** (`vastlint-go`): full Go FFI wrapper; same 108 rules, zero CGO complexity for callers
- Version equalization: all crates and bindings move to a unified version scheme

---

## [0.1.0] - 2026-04-03

- Initial public release
- **vastlint-core**: 108 rules derived from IAB VAST 2.0–4.3; zero-dependency, zero-I/O Rust library; validates in under 1 ms on typical production tags
- **CLI**: `vastlint check` with single-file, glob, stdin, JSON output; `vastlint fix` auto-repair with `--dry-run` and `--out`
- **Web validator**: vastlint.org/validate - client-side WASM, no data leaves the browser
- **VS Code extension**: inline diagnostics with rule IDs and spec references
- **REST API**: `/api/validate` on RapidAPI, WASM-powered, sub-millisecond response
- **Homebrew tap**: `brew install aleksUIX/tap/vastlint`

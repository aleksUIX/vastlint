# Changelog

All notable changes to vastlint are documented here.
Versions follow [Semantic Versioning](https://semver.org/).
GitHub Releases: <https://github.com/aleksUIX/vastlint/releases>

---

## [0.4.10] - 2026-05-05

### Crates

- **Version alignment**: bumped all crate `Cargo.toml` files to `0.4.10` in-repo so `crates.io` always reflects the current release even when the CI auto-bump skips a version.

## [0.4.9] - 2026-05-05

### CLI

- **Line/column in output**: `vastlint` CLI now prints `file:line:col` (or `file:line`) alongside the XPath location for every issue, making it easier to jump to the exact source position.

### Chrome extension

- **Release stamping fixed**: both Chrome publish workflows now inject the extension version correctly before build instead of dropping the `version` field from `manifest.json`.
- **Release guardrail**: both workflows now assert that `manifest.json` contains the expected version before packaging or publishing.
- **Chrome package version**: bumped source `chrome/package.json` and `chrome/manifest.json` to `0.4.9` for the next Web Store release.

### Docs

- Main README no longer says the Chrome Web Store listing is pending review.
- Release checklist now documents the actual Chrome publish paths and reminds you to commit both Chrome version files.

## [0.4.7] - 2026-05-01

### CI / release workflow

- **Smoke test**: updated MCP tool-count assertion from 5 → 6 to include `inspect_vast`; added `inspect_vast` to the per-tool name check.
- **Chrome extension publish**: replaced non-existent `trmcnvn/upload-google-chrome-extension` action (fake SHA) with the correct `mnao305/chrome-extension-upload@fdfe79400af990f5145a319e834aee64907ccff4` (v6.0.0); corrected input name `extension` → `file-path`; pinned `actions/setup-node` in chrome job to SHA `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e` (v6.4.0).

### Other

- Committed docs, GitHub workflow helpers, METHODOLOGY.md, and test fixtures that were staged but not included in the v0.4.6 release commit.

---

## [0.4.6] - 2026-05-01

### MCP server (`vastlint-mcp`)

- **`inspect_vast`** — new tool that follows a VAST wrapper chain hop-by-hop, returning creative metadata (AdSystem, AdTitle, Duration, impressions, tracking events, media files, companions) and full validation results for every level of the chain. Accepts a starting URL and an optional `max_depth` (default 5). Returns `hop_count`, `resolved`, `chain_valid`, `total_errors`, `total_warnings`, and a `stopped_reason` (`resolved` | `max_depth` | `fetch_error` | `parse_error`) alongside per-hop detail.
- `quick-xml` added as a direct dependency for hop metadata extraction.
- Server info string updated to describe all six tools.

---

## [0.4.5] - 2026-05-01

### VS Code extension

- **Updated extension description** — flyout text in the VS Code extensions panel now reflects CLI backend, any-file-type support, and 108 rules.
- **Settings table** — README now documents all six settings including `vastlint.templateIgnoreRegex`, `vastlint.vastVersion`, and `vastlint.cliPath`.
- **Full rule reference** — collapsed rule table (108 rules, each linking to `vastlint.org/docs/rules/{id}/`) added to the extension README.

---

## [0.4.4] - 2026-04-30

### VS Code extension

- **CLI backend** — the extension now spawns the local `vastlint` binary (searched on PATH and common install locations: `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`) and falls back to the bundled WASM when no binary is found. Using the CLI enables `vastlint.toml` config, rule overrides, and future CLI features automatically.
- **Multi-block support** — files with multiple `<VAST>…</VAST>` documents (e.g. batch response files) are now fully validated; each block gets its own set of squiggles with accurate positions.
- **Template ignore regex** (`vastlint.templateIgnoreRegex`) — a JS regex whose matches are replaced with same-length zeros before validation, preserving all line/col offsets. Strips Mustache `{{…}}`, ERB `<%…%>`, Go templates, or any ad-server macro syntax without shifting squiggle positions.
- **VAST version override** (`vastlint.vastVersion`) — force a specific spec version (2.0–4.3) regardless of the `version=` attribute. Passed as `--vast-version` to the CLI.
- **Any file type** — activation changed to `onStartupFinished`; `<VAST` anywhere in a file triggers linting regardless of file extension (`.erb`, `.go`, `.html`, etc.).
- **`vastlint.cliPath`** setting — override the binary path when not on PATH.

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

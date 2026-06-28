# Roadmap

Where vastlint is today and where it's going.

> This roadmap is best-effort. Priorities may shift based on community feedback -
> open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) if you'd like to influence what comes next.

---

## ✅ Shipped

| Milestone | Detail |
|---|---|
| **Core library** | `vastlint-core` v0.5 on crates.io - zero-dependency Rust library, 187 rules, VAST 2.0–4.3 plus SIMID, OMID, VMAP 1.0, and DAAST 1.0 validation. |
| **CLI** | `vastlint` on crates.io and Homebrew (`brew install aleksUIX/tap/vastlint`). JS/WASM on npm, Go bindings on GitHub. |
| **Web validator** | Paste or drop a VAST tag at [vastlint.org/validate](https://vastlint.org/validate) - structured report, nothing stored. |
| **VS Code extension** | Inline VAST XML validation as you type - errors and warnings with rule IDs and spec refs, directly in your editor. [Install →](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint) |
| **REST API** | Authenticated `/validate` endpoint on [RapidAPI](https://rapidapi.com/aleksUIX/api/vastlint). Same 187 rules, WASM-powered, sub-millisecond response. |
| **VMAP 1.0 validation** | 24 rules covering `<AdBreak>` structure, `timeOffset`/`breakType`/`repeatAfter` formats (including `repeatAfter` + `start`/`end` conflict detection), `<AdSource>` content constraints, VMAP tracking events, and full VAST validation of inline `<vmap:VASTAdData>` ad data. |
| **DAAST 1.0 validation** | 29 rules covering audio-specific VAST 3.0 deltas: required `<Category>`, `<DAASTAdTagURI>` wrappers, `<AdInteractions>` (with VAST-leftover detection), audio MediaFile attributes, DAAST tracking event set, DAAST pricing models, and root-level `<Error>` URI and `[ERRORCODE]` macro checks. |
| **MCP server** | Model Context Protocol server wrapping vastlint-core. Hosted SSE endpoint at `https://vastlint.org/mcp` - no install needed. Six tools: `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, `fix_vast`, `inspect_vast`. Published to [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io/servers/io.github.aleksUIX/vastlint) and Smithery. Works with Claude Desktop, Cursor, Windsurf, VS Code, and any MCP-compatible client. |
| **AAMP / ARTF compatibility** | vastlint-core and the MCP server are callable by AI buyer and seller agents operating under IAB Tech Lab's [AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) and [ARTF](https://iabtechlab.com/standards/artf/) frameworks via the live MCP endpoint. |
| **SIMID validation (all versions)** | `<InteractiveCreativeFile apiFramework="SIMID">` and `<IFrameResource apiFramework="SIMID">` validated against all published SIMID versions (1.0, 1.0.1, 1.1, 1.2) - required `type="text/html"`, HTTPS enforcement, video fallback presence, nonlinear placement, and `apiFramework` casing. |
| **OMID validation** | `<AdVerifications><Verification>` validation is now built into the core validator - vendor presence and format, duplicate vendor detection, OMID `apiFramework="omid"` semantics, HTTPS enforcement, `VerificationParameters`, `verificationNotExecuted` tracking validation, and pre-4.1 `Extension type="AdVerifications"` compatibility blocks. |
| **Revenue impact classification** | `IndustryBestPractice` rule source + `revenue_impact()` API - 12 rules marked `$` in `vastlint rules` output. HTTP tracker rules promoted to `Warning`. New `VAST-2.0-linear-tracking-quartiles` rule detects Linear ads with no measurement signal. |
| **Monitoring-friendly CLI** | `--fail-on-warning` exits non-zero on any warning (including all `$` revenue-impact rules). `--max-depth` controls wrapper chain follow depth. `--summary` prints aggregate pass/fail counts with a `$revenue` line. URL input with automatic wrapper chain following. |

## 🔧 In progress

| Milestone | Detail |
|---|---|
| **Language bindings** | Go bindings shipped. Python, Ruby, and Java bindings in progress - all backed by the same `vastlint-core`. |
| **Erlang / Elixir bindings** | Rustler DirtyCpu NIF - native BEAM terms, zero JSON overhead. `vastlint_nif` crate complete, Elixir/Erlang package scaffolded, precompiled NIF CI pipeline wired. Available on hex.pm shortly. |
| **VMAP validation** | Core rules implemented (23 rules): [VMAP 1.0](https://iabtechlab.com/standards/vmap/) `<AdBreak>` structure, `timeOffset`/`breakType`/`repeatAfter` formats, `<AdSource>` content constraints, VMAP tracking events - plus full VAST validation of inline `<vmap:VASTAdData>` ad data with VMAP-prefixed issue paths. Ships in the next core release; CLI/MCP/web surfaces follow. |
| **DAAST validation** | Core rules implemented (27 rules): [DAAST 1.0](https://iabtechlab.com/standards-old/digital-audio-ad-serving-template-daast/) audio-first deltas from VAST 3.0 - required `<Category>`, `<DAASTAdTagURI>` wrappers, `<AdInteractions>` (with VAST-leftover detection), audio MediaFile attributes, DAAST tracking event set, DAAST pricing models. Ships in the next core release. |

## 🗺️ Upcoming

### Side-spec validation

vastlint validates VAST XML. The VAST spec is not a standalone document - it references and is extended by several adjacent IAB Tech Lab standards. Each one below introduces elements or attribute values in VAST XML that are currently unvalidated beyond basic structural checks.

| Milestone | Detail |
|---|---|
| **IAB Content Taxonomy authority validation** | Validate the `authority` attribute on `<Category>` and `<BlockedAdCategories>` elements against the IAB Content Taxonomy registry - known authority URIs (`iabtechlab.com/IABTC`, `ads.iabtechlab.com`, etc.) and version-qualified formats. |

### Infrastructure

| Milestone | Detail |
|---|---|
| **AWS Marketplace** | Deploy vastlint as a private Lambda or container in your own AWS account - no data leaves your infrastructure. |
| **Realtime pipeline integration** | Continuous validation of live VAST traffic - per-partner error rates, alerting, and revenue impact dashboards. The structural validation layer is now available: `vastlint check --fail-on-warning <url>` follows wrapper chains and exits non-zero on any revenue-impact warning, suitable for periodic monitoring and CI gates. |

## 💡 What else?

Have an idea? Open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) or a feature-request issue.

# Roadmap

Where vastlint is today and where it's going.

> This roadmap is best-effort. Priorities may shift based on community feedback -
> open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) if you'd like to influence what comes next.

---

## ✅ Shipped

| Milestone | Detail |
|---|---|
| **Core library** | `vastlint-core` v0.8 on crates.io - pure-Rust library, 212 rules, VAST 2.0–4.4 plus SIMID, OMID, VMAP 1.0, and DAAST 1.0 validation. |
| **CLI** | `vastlint` on crates.io and Homebrew (`brew install aleksUIX/tap/vastlint`). JS/WASM on npm, Go bindings on GitHub. |
| **Web validator** | Paste or drop a VAST tag at [vastlint.org/validate](https://vastlint.org/validate) - structured report, nothing stored. |
| **VS Code extension** | Inline VAST XML validation as you type - errors and warnings with rule IDs and spec refs, directly in your editor. [Install →](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint) |
| **REST API** | Authenticated `/validate` endpoint on [RapidAPI](https://rapidapi.com/aleksUIX/api/vastlint). Same 212 rules, WASM-powered, sub-millisecond response. |
| **VMAP 1.0 validation** | 24 rules covering `<AdBreak>` structure, `timeOffset`/`breakType`/`repeatAfter` formats (including `repeatAfter` + `start`/`end` conflict detection), `<AdSource>` content constraints, VMAP tracking events, and full VAST validation of inline `<vmap:VASTAdData>` ad data. |
| **DAAST 1.0 validation** | 29 rules covering audio-specific VAST 3.0 deltas: required `<Category>`, `<DAASTAdTagURI>` wrappers, `<AdInteractions>` (with VAST-leftover detection), audio MediaFile attributes, DAAST tracking event set, DAAST pricing models, and root-level `<Error>` URI and `[ERRORCODE]` macro checks. |
| **MCP server** | Model Context Protocol server wrapping vastlint-core. Hosted SSE endpoint at `https://vastlint.org/mcp` - no install needed. Thirteen tools: `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, `fix_vast`, plus the AdCP 3.0 governance set (`get_adcp_capabilities`, content-standards CRUD, `calibrate_content`, `validate_content_delivery`, `list_creatives`). Published to [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io/servers/io.github.aleksUIX/vastlint) and Smithery. Works with Claude Desktop, Cursor, Windsurf, VS Code, and any MCP-compatible client. |
| **AAMP / ARTF compatibility** | vastlint-core and the MCP server are callable by AI buyer and seller agents operating under IAB Tech Lab's [AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) and [ARTF](https://iabtechlab.com/standards/artf/) frameworks via the live MCP endpoint. |
| **SIMID validation (all versions)** | `<InteractiveCreativeFile apiFramework="SIMID">` and `<IFrameResource apiFramework="SIMID">` validated against all published SIMID versions (1.0, 1.0.1, 1.1, 1.2) - required `type="text/html"`, HTTPS enforcement, video fallback presence, nonlinear placement, and `apiFramework` casing. |
| **OMID validation** | `<AdVerifications><Verification>` validation is now built into the core validator - vendor presence and format, duplicate vendor detection, OMID `apiFramework="omid"` semantics, HTTPS enforcement, `VerificationParameters`, `verificationNotExecuted` tracking validation, and pre-4.1 `Extension type="AdVerifications"` compatibility blocks. |
| **Revenue impact classification** | `IndustryBestPractice` rule source + `revenue_impact()` API - 12 rules marked `$` in `vastlint rules` output. HTTP tracker rules promoted to `Warning`. New `VAST-2.0-linear-tracking-quartiles` rule detects Linear ads with no measurement signal. |
| **Monitoring-friendly CLI** | `--fail-on-warning` exits non-zero on any warning (including all `$` revenue-impact rules). `--max-depth` controls wrapper chain follow depth. `--summary` prints aggregate pass/fail counts with a `$revenue` line. URL input with automatic wrapper chain following. |
| **VAST macro validation** | Macro substitution tokens validated across tracking, click, error, impression, and media URLs: unknown `[MACRO]` detection, lowercase casing mistakes (players match case-sensitively), deprecated tokens (`[CONTENTPLAYHEAD]`/`[MEDIAPLAYHEAD]` folded into `[ADPLAYHEAD]` at VAST 4.1), context violations (`[ERRORCODE]` outside `<Error>`, `[REASON]` outside `verificationNotExecuted`), and missing RFC 3986 percent-encoding. Full per-macro reference with resolved values and valid contexts at [vastlint.org/docs/vast-macros](https://vastlint.org/docs/vast-macros/). |
| **IAB Content Taxonomy authority validation** | `authority` attribute values on `<Category>` (4.0+) and `<BlockedAdCategories>` (4.1+) validated: URL well-formedness plus recognition against the IAB Content Taxonomy registry hosts (`iabtechlab.com` and subdomains, `iab.com`), including version-qualified forms such as `iabtechlab.com/IABTC/2.2`. Custom taxonomies stay legal: unrecognised authorities are Info, malformed ones are Warnings. |
| **Language bindings** | Python on PyPI (`vastlint`), Ruby on RubyGems (`vastlint`), Elixir/Erlang on hex.pm (`vastlint`, Rustler DirtyCpu NIF with precompiled binaries), Go bindings on GitHub - all backed by the same `vastlint-core`. |
| **Shareable report links** | `vastlint check --share` uploads the validation report (rule IDs and messages, never the raw XML) and prints a public `vastlint.org/r/<id>` link for Slack, tickets, and PRs. |
| **Content quality rules** | Beyond presence and structure: placeholder `<AdTitle>` values (`test`, `Ad 1`, `untitled`) that make creatives unidentifiable in reporting, placeholder `<AdSystem>` values that break provenance tracing, and missing `AdSystem` `version` attributes. Conservative lists, near-zero false positives, each rule disableable in `vastlint.toml`. Plus a VMAP advisory when a `display` ad break carries inline VAST with no `<CompanionAds>`. |
| **`vastlint init`** | Generates a starter `vastlint.toml` with all 212 rules listed at their default severities, commented out - a zero-behaviour-change starting point for tuning, and durable evidence of adoption in a repo. |

## 🔧 In progress

| Milestone | Detail |
|---|---|
| **Java bindings** | In progress - backed by the same `vastlint-core`. |

## 🗺️ Upcoming

### Infrastructure

| Milestone | Detail |
|---|---|
| **AWS Marketplace** | Deploy vastlint as a private Lambda or container in your own AWS account - no data leaves your infrastructure. |
| **Realtime pipeline integration** | Continuous validation of live VAST traffic - per-partner error rates, alerting, and revenue impact dashboards. The structural validation layer is now available: `vastlint check --fail-on-warning <url>` follows wrapper chains and exits non-zero on any revenue-impact warning, suitable for periodic monitoring and CI gates. |

## 💡 What else?

Have an idea? Open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) or a feature-request issue.

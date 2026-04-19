# Roadmap

Where vastlint is today and where it's going.

> This roadmap is best-effort. Priorities may shift based on community feedback —
> open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) if you'd like to influence what comes next.

---

## ✅ Shipped

| Milestone | Detail |
|---|---|
| **Core library** | `vastlint-core` v0.2 on crates.io — zero-dependency Rust library, 108 rules, VAST 2.0–4.3. |
| **CLI** | `vastlint` on crates.io and Homebrew (`brew install aleksUIX/tap/vastlint`). JS/WASM on npm, Go bindings on GitHub. |
| **Web validator** | Paste or drop a VAST tag at [vastlint.org/validate](https://vastlint.org/validate) — structured report, nothing stored. |
| **VS Code extension** | Inline VAST XML validation as you type — errors and warnings with rule IDs and spec refs, directly in your editor. [Install →](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint) |
| **REST API** | Authenticated `/validate` endpoint on [RapidAPI](https://rapidapi.com/aleksUIX/api/vastlint). Same 108 rules, WASM-powered, sub-millisecond response. |
| **MCP server** | Model Context Protocol server wrapping vastlint-core. Hosted SSE endpoint at `https://vastlint.org/mcp` — no install needed. Five tools: `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, `fix_vast`. Published to [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io/servers/io.github.aleksUIX/vastlint) and Smithery. Works with Claude Desktop, Cursor, Windsurf, VS Code, and any MCP-compatible client. |
| **AAMP / ARTF compatibility** | vastlint-core and the MCP server are callable by AI buyer and seller agents operating under IAB Tech Lab's [AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) and [ARTF](https://iabtechlab.com/standards/artf/) frameworks via the live MCP endpoint. |

## 🔧 In progress

| Milestone | Detail |
|---|---|
| **Language bindings** | Go bindings shipped. Python, Ruby, and Java bindings in progress — all backed by the same `vastlint-core`. |
| **Erlang / Elixir bindings** | Rustler DirtyCpu NIF — native BEAM terms, zero JSON overhead. `vastlint_nif` crate complete, Elixir/Erlang package scaffolded, precompiled NIF CI pipeline wired. Available on hex.pm shortly. |

## 🗺️ Upcoming

| Milestone | Detail |
|---|---|
| **SIMID validation (all versions)** | Validate the VAST 4.x `<InteractiveCreativeFile>` element and `apiFramework="simid"` usage across all published SIMID versions (1.0, 1.1, 1.2) — required fields, secure session ID format, `<Duration>` pairing, nonlinear ad support, and interaction type rules. SIMID is the IAB-sanctioned VPAID replacement for interactive CTV and video ads. |
| **AWS Marketplace** | Deploy vastlint as a private Lambda or container in your own AWS account — no data leaves your infrastructure. |
| **Realtime pipeline integration** | Continuous validation of live VAST traffic — per-partner error rates, alerting, and revenue impact dashboards. |

## 💡 What else?

Have an idea? Open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) or a feature-request issue.

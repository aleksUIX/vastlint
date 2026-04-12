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

## 🔧 In progress

| Milestone | Detail |
|---|---|
| **Language bindings** | Go bindings shipped. Python, Ruby, and Java bindings in progress — all backed by the same `vastlint-core`. |

## 🗺️ Upcoming

| Milestone | Detail |
|---|---|
| **Erlang / Elixir bindings** | NIFs for BEAM-based ad servers and RTB platforms. High-throughput VAST validation inside your OTP supervision tree. |
| **Agentic workflows** | MCP server + REST API integrations so AI agents and LLM-powered pipelines can validate and fix VAST tags inline during code generation, creative QA, and campaign launch. |
| **AWS Marketplace** | Deploy vastlint as a private Lambda or container in your own AWS account — no data leaves your infrastructure. |
| **Realtime pipeline integration** | Continuous validation of live VAST traffic — per-partner error rates, alerting, and revenue impact dashboards. |

## 💡 What else?

Have an idea? Open a [Discussion](https://github.com/aleksUIX/vastlint/discussions) or a feature-request issue.

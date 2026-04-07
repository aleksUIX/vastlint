# vastlint — Design Decisions

## Overview

vastlint is an open-source VAST validation and certification tool.
Goal: validate VAST XML against IAB spec, flag errors/warnings/ambiguities,
and support cross-enterprise contract negotiation and compliance workflows.

---

## Core Language: Rust

**Decision:** Core validation logic is written in Rust.

**Rationale:**
- Compiles to WASM → runs in browser, Node, Deno, edge workers
- C ABI / FFI → bindable from any language: Erlang NIF (via Rustler), Python, Ruby, Java JNI, etc.
- Single static binary for CLI and server — zero runtime dependencies
- `quick-xml` is extremely fast for large/complex VAST documents
- Same core can be exposed as native lib, WASM, CLI binary, and HTTP service

---

## Architecture

```
vastlint-core (Rust)
├── vastlint (binary)                → CLI  [shipped]
├── vastlint-server (binary)         → HTTP service  [planned]
├── vastlint-wasm → npm: vastlint    → browser / edge / Node  [shipped]
└── libvastlint.so / .dylib / .dll   → FFI for any language  [planned]
```

The core is a no-I/O Rust library. Three dependencies: `quick-xml` (XML parsing), `url` (RFC 3986), and `phf` (compile-time hash maps).
All consumers (CLI, server, UI) are thin wrappers around it.

---

## Distribution Targets

| Target | Format | Status | Use case |
|---|---|---|---|
| CLI | Native binary | ✅ Shipped | Dev-time validation, CI/CD |
| WASM / npm | `vastlint` on npm | ✅ Shipped | Browser UI, edge workers, Node/Deno |
| HTTP service | Native binary | Planned | Enterprise ad serving integration |
| FFI | .so / .dylib / .dll | Planned | Erlang, Python, Java, etc. |

The npm package is built from `crates/vastlint-wasm` via `wasm-pack` and published from the `npm/` directory. It ships ESM (bundler target), CJS (Node.js target), and TypeScript types in a single package.

---

## VAST Version Handling

1. Read `version` attribute from `<VAST version="4.2">`
2. If missing → `WARN: no version declared`
3. Infer version from structure/contents (e.g. `<UniversalAdId>` = 4.x, `<Verification>` = 4.x, `<MediaFile>` type patterns = 2.x/3.x)
4. If inferred version differs from declared → `WARN: declared version inconsistent with content`

Supported spec versions: VAST 2.0, 3.0, 4.0, 4.1, 4.2, 4.3

---

## Validation Result Format

Structured, JSON-serializable output. Severity levels:

- `error` — spec violation, tag will likely fail
- `warning` — deprecated, ambiguous, or missing recommended field
- `info` — advisory, unsafe pattern (e.g. HTTP asset in HTTPS context)

---

## Rule Categories

- Required elements missing
- Schema (unknown elements and attributes)
- Deprecated elements (version-specific)
- Ambiguous behavior (spec says "should" not "must")
- Security concerns (HTTP vs HTTPS assets, redirect depth limits)
- Wrapper chain depth violations
- Version declaration consistency
- Value format validation (Duration, URL, enum attributes)
- CTV-specific advisories (Mezzanine, VPAID in CTV context)

---

## Intended Use Cases

1. **CLI** — integrate into dev workflows and local toolsets; validate VAST at dev time
2. **Realtime service** — enterprise teams stand up the service and integrate into ad serving pipelines
3. **UI tool** — visual report showing where VAST deviates from IAB spec, with severity and remediation hints

---

## Naming

`vastlint` — follows the `*lint` naming convention familiar to developers.
`vastlint check tag.xml` feels natural.
Signals: "this tool checks your stuff against a spec."

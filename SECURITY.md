# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest minor (0.x) | ✅ |
| Older minor releases | ⚠️ Best effort |

Security fixes are released as patch versions and back-ported to the current
minor series. Older minor releases receive fixes on a best-effort basis.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report security issues privately via
[GitHub Security Advisories](https://github.com/aleksUIX/vastlint/security/advisories/new).

You will receive a response within **48 hours** acknowledging your report.
We aim to release a fix within **7 days** for critical issues and **30 days**
for others. We will credit reporters in the release notes unless you prefer to
remain anonymous.

## Supply-Chain Security

vastlint targets **SLSA Build Level 2** for all release artifacts.

### Verifying release artifacts

Every binary, library, and `.vsix` attached to a
[GitHub Release](https://github.com/aleksUIX/vastlint/releases) has a signed
SLSA provenance attestation stored in GitHub's attestation store. To verify
any artifact:

```sh
gh attestation verify <file> --repo aleksUIX/vastlint
```

### Verifying the npm package

The `vastlint` npm package is published with provenance. Verify the published
package is linked to its CI run:

```sh
npm audit signatures vastlint
```

### Build integrity

- Release builds are performed exclusively by GitHub Actions from tagged source.
- The release workflow requires all smoke tests to pass before any artifact is
  published or uploaded.
- All release artifacts are built from a clean checkout of the tagged commit -
  no local developer machines are involved in producing release binaries.

## Security Standards

We are working toward alignment with the following:

| Standard | Status |
|----------|--------|
| [SLSA Build L2](https://slsa.dev) | ✅ Implemented |
| [SLSA Build L3](https://slsa.dev) | 🔄 In progress |
| [OpenSSF Best Practices (Passing)](https://www.bestpractices.dev) | 🔄 In progress |
| [OpenSSF Scorecard](https://scorecard.dev) | 🔄 In progress |

## Assurance Case

This section explains why we believe vastlint is secure for its intended use:
validating untrusted VAST XML.

### Threat model

The attacker-controlled input is the VAST XML document itself. Ad tags arrive
from third-party ad servers and cannot be trusted. The threats we defend
against:

- **Denial of service**: crafted XML that causes excessive CPU or memory use,
  hangs, or crashes the host process.
- **Panic / crash**: malformed, truncated, or hostile input that triggers a
  Rust panic and takes down the embedding application.
- **Incorrect output**: input that causes vastlint to mis-report, e.g. declare
  a malicious or broken tag valid.

Out of scope: vastlint does not execute, fetch, or render any resource
referenced by the tag (media files, VPAID/SIMID JavaScript, tracking URLs), so
threats inside those resources are not part of its attack surface. It flags
suspicious constructs; it never runs them.

### Trust boundaries

- **Input** is a single untrusted XML string handed to `vastlint-core`.
- **Output** is a structured validation report (rule IDs, severities,
  locations). No part of the input is executed or interpreted beyond XML
  parsing.
- `vastlint-core` performs **no network calls, no filesystem access, and no
  environment access**. Components that do I/O (the CLI reading files or URLs,
  the hosted API, the MCP server) sit outside the core and pass fetched
  content across the same string-in / report-out boundary. Wrapper-chain
  fetching happens only in those outer layers and is capped at a default depth
  of 5 (`max_wrapper_depth`, per the IAB VAST 4.x recommendation).

### Secure design principles applied

- **Fail-safe defaults**: malformed XML is rejected at the parser with a
  structured parse error before any rule runs. A document that cannot be
  parsed is never reported as valid.
- **Complete mediation**: every input byte passes through the XML parser;
  rules only ever see the parsed tree, never raw input.
- **Least privilege**: the core needs, and has, no I/O capability at all. An
  embedding process grants it nothing but CPU and memory.
- **Economy of mechanism**: the core has three dependencies (`quick-xml`,
  `url`, `phf`), all pure Rust and compiled in. Rules are plain compiled Rust
  functions; there is no schema interpreter, regex engine, dynamic loading, or
  plugin system to attack.

### Common weaknesses countered

- **CWE-20 (improper input validation)**: all input is parsed by `quick-xml`
  before any rule runs; malformed structure and invalid UTF-8 are rejected or
  safely replaced rather than propagated. Field values are then validated
  against allowlists: enum values, RFC 3986 URI syntax, IANA MIME types,
  ISO 4217 currency codes.
- **CWE-400 (uncontrolled resource consumption)**: parsing is a single
  streaming pass with memory proportional to input size, and the caller
  controls input size at the boundary. `<!DOCTYPE>` declarations are skipped
  and custom entities are never expanded (only the five XML predefined
  entities and numeric character references are resolved), so
  entity-expansion amplification ("billion laughs") does not apply.
  Wrapper-chain fetching outside the core is depth-limited as described above.
- **CWE-835 (infinite loop)**: the parser is a single forward pass that
  terminates on end-of-input or on the first structural error. There is no
  backtracking and no loop whose bound is controlled by input content, so
  termination holds by construction.
- **CWE-119 / CWE-120 / CWE-125 (memory corruption)**: `vastlint-core`
  contains no `unsafe` blocks; Rust's memory safety guarantees rule out
  buffer overflows and out-of-bounds reads in the validator itself. The FFI
  (`vastlint-ffi`) and Erlang NIF (`vastlint-nif`) binding crates necessarily
  use `unsafe` at the C ABI boundary; that code is confined to argument
  marshalling and is not reachable from the parsing or rule logic.

These claims are exercised continuously: three libFuzzer targets fuzz the
parser, validator, and auto-fix engine on every CI push, and
`cargo clippy -D warnings` plus `cargo audit` run on every push and PR.

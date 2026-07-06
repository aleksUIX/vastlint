# Governance

This document describes how VASTlint is run: who makes decisions, how they are
made, who holds which keys, and what happens if the maintainer becomes
unavailable.

## Project model

VASTlint is a single-maintainer project.

**Maintainer: Aleksander Sekowski ([@aleksUIX](https://github.com/aleksUIX))**

## Decision making

The maintainer makes final decisions on scope, design, releases, and which
contributions are merged. Community input is welcome and actively used:

- **GitHub Issues** for bug reports, rule proposals, and spec-reference
  corrections. Issues are the primary channel for influencing the rule set.
- **GitHub Discussions** for design questions, direction, and anything that is
  not a concrete bug or feature.
- **Pull requests** for concrete changes. Every PR gets a review and a stated
  reason if it is declined.

Disagreements are resolved by discussion in the relevant issue or PR. If no
consensus emerges, the maintainer decides and records the reasoning in the
thread.

## Roles and responsibilities

### Maintainer

- Reviews and merges all pull requests.
- Triages issues and security reports (see [SECURITY.md](SECURITY.md) for the
  response SLAs).
- Cuts releases and publishes artifacts.
- Holds all release and infrastructure credentials:
  - crates.io publish token (`vastlint-core`, `vastlint-cli`)
  - npm publish token (`vastlint`)
  - Cloudflare API token (vastlint.org site, hosted API, MCP server)
  - DNS control for vastlint.org
  - GitHub repository admin, including Actions secrets

### Contributor

Anyone who opens issues or pull requests. Contributors are expected to follow
[CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md). No signup or CLA is required; commits
must carry a DCO sign-off.

### Co-maintainer (open)

Any contributor with 3 or more merged, non-trivial pull requests may be offered
co-maintainer access: merge rights, release participation, and shared custody
of publish credentials. If you are at that point and interested, say so in an
issue or by email (aleksander@uixlimited.com).

## Access continuity

- All project credentials (crates.io, npm, Cloudflare, DNS registrar, GitHub
  recovery codes) are stored in a 1Password vault, not on any single machine.
- An emergency contact document, held privately by the maintainer's designated
  contact, explains how to reach the vault and what to do with each credential
  if the maintainer is incapacitated or unreachable for an extended period.
- The source of truth is the public GitHub repository. Everything needed to
  build, test, and release is in the repo and its CI workflows; nothing depends
  on the maintainer's local machine.

## Bus factor

The current bus factor is 1. That is a known weakness, mitigated by the
continuity plan above and by the fact that the project is Apache 2.0 licensed
with a fully public build: anyone can fork and continue it.

Goal: bus factor 2 within 12 months, by promoting an active contributor to
co-maintainer under the path described above.

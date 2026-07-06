# Contributing

Bug reports, rule proposals, and pull requests are all welcome. This document
covers how to set up a development environment, what a PR needs to be merged,
and how versioning works. Governance and roles are described in
[GOVERNANCE.md](GOVERNANCE.md); conduct expectations in
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Developer quick start

You need a stable Rust toolchain (Rust 1.91 or later, per `rust-version` in the
workspace `Cargo.toml`). Nothing else is required for core work.

```sh
git clone https://github.com/aleksUIX/vastlint
cd vastlint
cargo build --all
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

If all three commands pass, your environment matches CI. The WASM package and
the Go/Erlang bindings need extra toolchains, but you only need those if you
are changing them; the Rust workspace builds and tests without them.

## Filing bugs

File bugs via GitHub Issues. Include:

- the VAST tag that triggered the problem (or a minimal reproduction),
- the vastlint version,
- the output you got vs. what you expected.

If you have a rule idea or a spec reference that vastlint gets wrong, open an
issue. That kind of feedback is valuable even if you never touch the code.

**Security issues are the one exception:** do not open a public issue. Report
them privately as described in [SECURITY.md](SECURITY.md).

## Pull requests

### Developer Certificate of Origin (DCO)

Every commit must include a `Signed-off-by` line certifying the
[Developer Certificate of Origin](https://developercertificate.org/):

```
Signed-off-by: Your Name <you@example.com>
```

Add it automatically with `git commit -s`. PRs with unsigned commits will be
asked to rebase before merge. There is no CLA; the sign-off is the only
paperwork.

### Coding standards

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
  for public API design.
- Formatting is the official Rust style, enforced by `cargo fmt --all -- --check`.
- Lints are enforced with `cargo clippy --all-targets --all-features -- -D warnings`.

Both checks run in CI on every push and PR and fail the build on any
violation, so run them locally first.

### Test policy

Every new validation rule MUST include at least one positive test fixture and
one negative test fixture in `crates/vastlint-core/tests/fixtures/`:

- a fixture that triggers the rule (named `err_*.xml`, `warn_*.xml`, or
  `info_*.xml` to match the rule's severity), and
- a fixture that passes it (a `valid_*.xml` fixture, new or existing, that
  exercises the same element without triggering the rule).

More generally, any new feature or behavior change MUST come with automated
tests. PRs without tests for new functionality will not be merged.

### Regression tests

Every bug fix MUST include a reproducer fixture added to
`crates/vastlint-core/tests/fixtures/` (or a unit test where a fixture does not
apply) that fails before the fix and passes after it. This keeps fixed bugs
fixed.

### Keeping docs in sync

If your change alters user-visible behavior, update the relevant docs in the
same PR. In particular:

- Rule additions or changes must update [RULES.md](RULES.md); the test suite
  (`rules_markdown` in `crates/vastlint-core/tests/`) fails if RULES.md and the
  rule catalog disagree.
- CLI flag or output changes must update the README and `docs/`.

## Versioning and upgrade path

vastlint follows [semantic versioning](https://semver.org/). While the project
is pre-1.0:

- **Patch releases** (0.x.Y) are backward compatible: bug fixes, new fixtures,
  doc updates. Security fixes ship as patch releases and are back-ported to the
  current minor series (see [SECURITY.md](SECURITY.md)).
- **Minor releases** (0.X.0) may add rules, add API surface, or make breaking
  changes to the library API or JSON output schema. Every breaking change is
  listed in [CHANGELOG.md](CHANGELOG.md) with migration notes.
- Rule IDs are stable: a rule keeps its ID across releases. Rules are added or
  deprecated, never renumbered, so rule-ID-based suppressions and CI gates
  survive upgrades.

To upgrade between minor versions: read the CHANGELOG entry for each version
you cross, apply any listed migration steps (typically renamed API items or
JSON field changes), then re-run your test suite against the new version. New
rules may surface findings in tags that previously passed; that is expected
behavior, not a compatibility break. After 1.0, breaking changes will only
occur in major releases.

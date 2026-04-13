# vastlint-core Architecture

Private. Gitignored.

---

## Purpose

vastlint-core is a zero-dependency, zero-I/O Rust library. It takes a VAST XML string as input and returns a structured validation result. Nothing else. No file reading, no network calls, no logging. All of that is the caller's responsibility.

This constraint keeps the core embeddable anywhere: CLI, HTTP server, WASM, FFI bindings. The same compiled artifact runs identically in all contexts.

---

## VAST Versions in Scope

2.0, 3.0, 4.0, 4.1, 4.2, 4.3

All six published IAB Tech Lab versions are implemented from v1. The CTV Addendum 2024 extends 4.x but is a separate document, not a version bump. Rules derived from the addendum are tagged separately.

XSD schemas are available from the IAB GitHub repo (InteractiveAdvertisingBureau/vast) for 2.0.1, 3.0, 4.0, 4.1, 4.2. These live in specs/ locally and are gitignored. VAST 4.3 has no published XSD; rules are derived from the spec prose. They are the ground truth for structural validation.

---

## Public API

The entire public surface of the library is three functions and a handful of types.

```rust
pub fn validate(input: &str) -> ValidationResult
pub fn validate_with_context(input: &str, context: ValidationContext) -> ValidationResult
pub fn all_rules() -> &'static [RuleMeta]
```

```rust
pub struct ValidationResult {
    pub version: DetectedVersion,
    pub issues: Vec<Issue>,
    pub summary: Summary,
}

pub struct Issue {
    pub id: &'static str,       // e.g. "VAST-4.2-3.4.1"
    pub severity: Severity,
    pub message: &'static str,
    pub path: Option<String>,   // XPath-like location in the document
    pub spec_ref: &'static str, // e.g. "IAB VAST 4.2 §3.4.1"
}

pub enum Severity {
    Error,    // spec violation, tag will likely fail
    Warning,  // deprecated, ambiguous, or missing recommended field
    Info,     // advisory, unsafe pattern
}

pub enum DetectedVersion {
    Declared(VastVersion),          // found in version attribute
    Inferred(VastVersion),          // deduced from structure
    DeclaredAndInferred {
        declared: VastVersion,
        inferred: VastVersion,
        consistent: bool,
    },
    Unknown,                        // could not determine
}

pub enum VastVersion {
    V2_0,
    V3_0,
    V4_0,
    V4_1,
    V4_2,
    V4_3,
}
```

The result is fully serializable to JSON. No types in the public API depend on any external crate.

---

## Internal Pipeline

Input string goes through four stages in sequence. Each stage is independent and testable on its own.

```
&str
  |
  v
[1] Parse        raw XML into a document tree (quick-xml, zero-copy where possible)
  |
  v
[2] Detect       VAST version from attribute, then infer from structure if needed
  |
  v
[3] Validate     run all applicable rules against the tree, collect issues
  |
  v
[4] Summarize    count by severity, produce ValidationResult
```

Stage 1 failure (malformed XML) short-circuits immediately with a single Error-level issue. Stages 2-4 always complete even if earlier stages produce issues.

---

## Stage 1: Parsing

Use `quick-xml` in borrowing mode where possible. The goal is to avoid allocations on the hot path for large VAST documents with deep wrapper chains.

The parse stage produces an internal document model, not a generic DOM. Only the elements and attributes vastlint cares about are materialized. Everything else is skipped during traversal.

The internal model is not part of the public API.

---

## Stage 2: Version Detection

```
1. Read version attribute from root <VAST> element
2. If present and recognized → Declared(version)
3. If missing → emit WARN "no version attribute on root VAST element"
4. Regardless, scan document for version-discriminating elements:
     <UniversalAdId>        present → 4.x
     <Verification>         present → 4.x
     <AdServingId>          present → 4.1+
     <InteractiveCreativeFile type="SIMID"> → 4.2+
     <ViewableImpression>   present → 3.0+
     (absence of above + MediaFile present) → 2.x
5. If inferred version differs from declared → emit WARN
6. Produce DetectedVersion
```

Version inference is best-effort. Ambiguous documents get Unknown with an advisory.

---

## Stage 3: Rule Engine

Rules are statically defined. There is no dynamic rule loading in the core. Extensibility is an enterprise feature.

Each rule is a function with this signature:

```rust
fn check(tree: &VastDocument, version: &DetectedVersion, issues: &mut Vec<Issue>)
```

Rules are organized by version applicability and category. A rule that applies to all versions is registered once. A rule that applies only to 4.x is gated on the detected version.

Rule categories:

- `required` — elements that must be present
- `deprecated` — elements that existed in older versions but are removed or superseded
- `ambiguous` — spec uses "should" not "must"; flag as warning
- `security` — HTTP assets in HTTPS context, redirect depth, unsafe URLs
- `structure` — wrapper chain depth, ad pod constraints, sequence numbering
- `consistency` — version declaration vs content, duplicate IDs, conflicting attributes

Each rule has a stable ID tied to the spec section it enforces. The ID format is:

```
VAST-{version}-{section}
e.g. VAST-4.2-3.4.1
```

For rules that apply across versions, the version segment is the earliest version where the rule applies:

```
VAST-2.0-2.1.3   (applies from 2.0 onwards)
```

---

## Rule Configuration

Rules ship with a recommended severity. Callers can override any rule's severity or turn it off entirely.

The four valid levels mirror the Severity enum plus an off state:

```
"error"    maps to Severity::Error
"warning"  maps to Severity::Warning
"info"     maps to Severity::Info
"off"      rule does not run
```

The override map is passed into the core through ValidationContext:

```rust
pub struct ValidationContext {
    pub wrapper_depth: u8,
    pub max_wrapper_depth: u8,
    pub rule_overrides: Option<HashMap<&'static str, RuleLevel>>,
}

pub enum RuleLevel {
    Error,
    Warning,
    Info,
    Off,
}
```

When `rule_overrides` is None, all rules run at their recommended severity. When a rule ID is present in the map, that severity is used instead. The resolved severity is what appears in the Issue output — the recommended default is not exposed to callers.

A rule set to Off does not run at all. It produces no Issue entry. This matters for performance at scale when callers want to skip entire categories.

Config loading is not a core concern. The CLI reads a config file (vastlint.toml or vastlint.json) and constructs the override map before calling validate_with_context. The server does the same per-request or per-tenant. The core stays pure.

A minimal config file looks like:

```toml
[rules]
"VAST-2.0-mediafile-https" = "error"
"VAST-4.1-mezzanine-recommended" = "off"
```

The recommended config is the shipped default. Zero-config usage gives you the recommended set with no file required. This is the same model as ESLint's "extends: recommended".

The recommended severity for each rule is documented in the per-version reference files in specs/. Those values should not change without a version bump. Overrides are the user's responsibility.

---

## Rule Coverage

108 rules implemented across all versions. Categories:

- `required` — elements and attributes that must be present per spec
- `schema` — unknown/misplaced elements and attributes
- `structure` — wrapper chain depth, ad pod sequence constraints
- `security` — HTTP vs HTTPS for media and tracking URLs
- `consistency` — version declaration vs content, duplicate impressions
- `deprecated` — removed or superseded elements (VPAID, Survey, conditionalAd, Flash)
- `ambiguous` — spec uses "should" not "must"; flagged as warning
- `values` — Duration format, delivery enum, tracking event names, bitrate pairs
- `ctv` — CTV/SSAI advisories (Mezzanine presence, VPAID in interactive context)

---

## Wrapper Chain Handling

Wrapper chain validation is special. The core validates a single VAST document. It does not follow wrapper URLs. The caller (CLI or server) is responsible for fetching and chaining.

The core does validate:
- That wrapper depth is declared correctly if the caller passes it in
- That the wrapper element contains required fields
- That VASTAdTagURI is present and is a valid URL format

The CLI and server pass a depth counter into validate_with_context() for wrapper traversal:

```rust
let ctx = ValidationContext {
    wrapper_depth: 3,
    max_wrapper_depth: 5,
    ..Default::default()
};
let result = validate_with_context(input, ctx);
```

This keeps the core stateless while still being able to flag depth violations.

---

## Error vs Warning Philosophy

The rule is: if the spec says "must" or "required", it is an Error. If the spec says "should" or "recommended", it is a Warning. If it is a security or interoperability concern not explicitly stated in the spec, it is an Info.

This maps directly to spec language. When a rule is written, the spec quote that justifies the severity is recorded in a comment next to the rule.

---

## Performance Targets

A single VAST document (typical size 5-50KB) should validate in under 1ms on modern hardware. Wrapper chains of depth 5 with 5 documents of 50KB each should complete in under 10ms when called sequentially by the CLI.

The WASM build should stay under 500KB gzipped for browser viability.

---

## Crate Structure

```
crates/vastlint-core/
  src/
    lib.rs          pub API, validate(), validate_with_context(), all_rules(), types
    parse.rs        XML parsing, internal document model
    detect.rs       version detection logic
    rules/
      mod.rs        rule registry, CATALOG, dispatch by category
      required.rs
      schema.rs
      structure.rs
      security.rs
      consistency.rs
      deprecated.rs
      ambiguous.rs
      values.rs
      ctv.rs
    summarize.rs    aggregate issues into Summary
  tests/
    fixtures/       sample VAST XML files for each version (valid + invalid cases)
    required.rs
    deprecated.rs
    ...
  Cargo.toml

crates/vastlint-ffi/
  src/
    lib.rs          #[no_mangle] extern "C" functions; opaque VastlintResult handle
  vastlint.h        C header — committed to repo, also regenerated by cbindgen
  cbindgen.toml     cbindgen config (run: cbindgen --config cbindgen.toml --crate vastlint-ffi)
  Cargo.toml        crate-type = ["cdylib", "staticlib"]

  The FFI crate is the load-bearing layer for all non-WASM language bindings.
  It exposes a minimal, ABI-stable C surface:

    VastlintResult *vastlint_validate(const char *xml, size_t xml_len);
    VastlintResult *vastlint_validate_with_options(const char *xml, size_t xml_len,
                       unsigned int wrapper_depth, unsigned int max_wrapper_depth,
                       const char *rule_overrides_json);
    const char     *vastlint_result_json(const VastlintResult *r);
    size_t          vastlint_result_errors(const VastlintResult *r);
    size_t          vastlint_result_warnings(const VastlintResult *r);
    size_t          vastlint_result_infos(const VastlintResult *r);
    int             vastlint_result_valid(const VastlintResult *r);
    void            vastlint_result_free(VastlintResult *r);
    const char     *vastlint_version(void);

  JSON is the only data format that crosses the FFI boundary. All struct fields
  are kept inside the opaque VastlintResult; callers deserialise the JSON string
  in their own language. This means:
  - No per-language type mapping needed in the C layer.
  - The data contract is the JSON schema, not C struct layout.
  - Adding fields to the result never breaks the ABI.

  Planned language bindings on top of this layer (separate repos):
    vastlint-go      — CGo + bundled .a per platform  → go get github.com/aleksUIX/vastlint-go
    vastlint-ruby    — Magnus gem                     → gem install vastlint
    vastlint-python  — PyO3 (direct, not via C layer) → pip install vastlint
    vastlint-erlang  — Rustler NIF (direct)           → hex.pm vastlint_nif
    vastlint-java    — JNI via C layer                → Maven/Gradle

  PyO3 (Python) and Rustler (Erlang) bind directly to vastlint-core because
  both have first-class Rust integration that is cleaner than going through C.
  Go, Ruby, and Java use the C layer.
```

---

## Erlang / Elixir NIF Binding

### Approach: Rustler DirtyCpu NIF, direct dependency on vastlint-core

The Erlang binding does **not** go through the C FFI layer (`vastlint-ffi`). It uses
[Rustler](https://github.com/rusterlium/rustler) to bind Rust directly to the BEAM
via the `erl_nif.h` ABI. Rustler is a zero-overhead abstraction over the same NIF
mechanism you would use writing C by hand — the difference is compile-time type
mapping and no manual `enif_make_*` bookkeeping.

**Why not the C FFI layer?**

The Go binding uses the C layer because CGo is the only option. Rustler gives Rust
first-class access to BEAM term types, so the NIF can build the result as a native
Erlang map directly — no JSON serialization/deserialization crossing the boundary.
The Go binding spends ~19µs per call on JSON roundtrip. The NIF eliminates that
entirely. On a 17 KB production tag (363µs total), that is a ~5% saving, and it
removes a class of encoding bugs.

**Why DirtyCpu, not a regular NIF?**

A regular NIF must return in under ~1ms or it blocks a BEAM scheduler thread,
degrading the entire VM. At production tag sizes (17–44 KB), `vastlint-core`
takes 363–2,104µs — well above the safe threshold for a regular NIF on large tags.
`DirtyCpu` moves execution to the BEAM's dedicated dirty scheduler pool, which runs
on separate OS threads that never block the normal schedulers. This is a one-line
annotation in Rustler (`schedule = DirtyCpu`) and is the correct model for any
CPU-bound NIF with non-trivial runtime.

**Why not write a native Erlang implementation?**

The BEAM is not designed for CPU-bound XML parsing. A pure Erlang re-implementation
of the same 108 rules and `quick-xml`-backed parser would be 10–50× slower than
Rust for this class of computation. The validation logic lives in `vastlint-core`
exactly once. The NIF is a thin call boundary, not a reimplementation.

**Why not a port (external OS process)?**

Ports give crash isolation (a crashing port never takes down the VM) but add
1–5ms of IPC overhead per call. On a 363µs operation that is 3–14× the validation
cost. For a high-throughput BEAM ad server this is unacceptable. The DirtyCpu NIF
gives the same isolation at the scheduler level (dirty schedulers are independent
of normal ones) with microsecond-range call overhead.

### Crate: `crates/vastlint-nif`

New crate added to the monorepo workspace. Never published to crates.io separately
— it exists only as the native library compiled into the hex.pm package.

```toml
# crates/vastlint-nif/Cargo.toml
[package]
name    = "vastlint_nif"
version = "0.1.0"   # version-locked to monorepo tag at release time

[lib]
name       = "vastlint_nif"
crate-type = ["cdylib"]   # .so / .dylib loaded by the BEAM at runtime

[dependencies]
vastlint-core = { path = "../vastlint-core" }
rustler       = "0.36"
# NOTE: do NOT add mimalloc here. The BEAM owns the process allocator.
# Setting a global allocator in a cdylib loaded into the BEAM will corrupt
# the heap. The BEAM's own allocator (erts_alloc) is tuned for concurrent
# workloads and performs adequately without override.
```

### Exported NIFs

```rust
// All three are registered in rustler::init!

/// Validate a VAST XML binary with default settings.
/// Marked DirtyCpu — runs on dirty scheduler, never blocks normal schedulers.
#[rustler::nif(schedule = "DirtyCpu")]
fn validate(xml: Binary) -> NifResult<Term>

/// Validate with caller-supplied options.
/// wrapper_depth, max_wrapper_depth, rule_overrides (map of binary→binary).
#[rustler::nif(schedule = "DirtyCpu")]
fn validate_with_opts(
    xml: Binary,
    wrapper_depth: u32,
    max_wrapper_depth: u32,
    rule_overrides: HashMap<String, String>,
) -> NifResult<Term>

/// Return the vastlint-core version string as a binary.
/// Fast path — regular NIF, not dirty (nanosecond runtime).
#[rustler::nif]
fn version() -> &'static str
```

### Result term shape

The NIF builds the result as a native Erlang map — no JSON crosses the boundary.
Rustler's `Term` API constructs the map directly on the BEAM heap.

```erlang
%% {:ok, result} on success, {:error, reason} on bad input
{:ok, %{
  version:  "4.2" | :undefined,
  valid:    true | false,
  errors:   non_neg_integer(),
  warnings: non_neg_integer(),
  infos:    non_neg_integer(),
  issues: [
    %{
      id:       binary(),     # e.g. "VAST-4.2-3.4.1"
      severity: :error | :warning | :info,
      message:  binary(),
      path:     binary() | :undefined,
      spec_ref: binary()      # e.g. "IAB VAST 4.2 §3.4.1"
    }
  ]
}}
```

Severity values are Erlang atoms (`:error`, `:warning`, `:info`), not binaries.
Atoms are interned in the BEAM atom table and compare in O(1). Pattern matching
on severity in Erlang/Elixir code is therefore a single instruction.

Nullable fields (`version`, `path`) use the atom `:undefined` rather than
`nil`/`null`. This is idiomatic Erlang. The Elixir wrapper module translates
`:undefined` to `nil` for Elixir callers.

### Repository: `vastlint-erlang`

Separate repo, mirrors the `vastlint-go` pattern. Does not require a Rust
toolchain to install — precompiled `.so`/`.dylib` NIFs are bundled per platform
via `RustlerPrecompiled`.

```
vastlint-erlang/
  mix.exs                     # Elixir package definition, hex.pm metadata
  lib/
    vastlint.ex               # public Elixir API (validate/1, validate!/1, version/0)
    vastlint/result.ex        # %Vastlint.Result{} and %Vastlint.Issue{} structs
  src/
    vastlint.erl              # pure Erlang module for rebar3 / OTP callers
  native/
    vastlint_nif/             # the Rust NIF crate (symlinked or vendored)
  priv/
    native/                   # compiled .so/.dylib lands here at runtime
  test/
    vastlint_test.exs
  README.md
  LICENSE                     # Apache-2.0, matches core
```

**Elixir public API** (`lib/vastlint.ex`):

```elixir
# Returns {:ok, %Vastlint.Result{}} or {:error, reason}
Vastlint.validate(xml :: binary()) :: {:ok, Result.t()} | {:error, term()}

# Returns %Vastlint.Result{} or raises Vastlint.ValidationError
Vastlint.validate!(xml :: binary()) :: Result.t()

# Returns {:ok, %Vastlint.Result{}} with options
Vastlint.validate(xml, opts :: keyword()) :: {:ok, Result.t()} | {:error, term()}
# opts: [wrapper_depth: 0, max_wrapper_depth: 5, rule_overrides: %{"VAST-..." => "off"}]

Vastlint.version() :: binary()
```

**Pure Erlang API** (`src/vastlint.erl`) — same calls without Elixir conventions,
for `rebar3` projects that do not use Mix:

```erlang
vastlint:validate(Xml :: binary()) -> {ok, map()} | {error, term()}.
vastlint:validate_with_opts(Xml, WrapperDepth, MaxDepth, Overrides) -> {ok, map()} | {error, term()}.
vastlint:version() -> binary().
```

### Allocator note (important)

`mimalloc` **must not** be set as the global allocator in `vastlint_nif`. The
BEAM process owns the allocator. Loading a `cdylib` that overrides `GlobalAlloc`
will corrupt the BEAM heap and produce non-deterministic crashes, potentially
hours after the override takes effect.

The measured 8× throughput gain from mimalloc applies to standalone Rust processes
(CLI, server) where Rust owns the process. It does not apply here. Under concurrent
load from multiple BEAM dirty schedulers, `erts_alloc` performs well for this
workload — each dirty scheduler thread has its own carrier, so contention is low.

### Precompiled NIF strategy

Users must not need a Rust toolchain. `RustlerPrecompiled` fetches the correct
`.so`/`.dylib` from GitHub Releases at `mix deps.get` time, matching the same
4-platform matrix as `build-ffi`:

| Platform | NIF artifact |
|---|---|
| `x86_64-unknown-linux-gnu` | `vastlint_nif-x86_64-linux.so.tar.gz` |
| `aarch64-unknown-linux-gnu` | `vastlint_nif-aarch64-linux.so.tar.gz` |
| `x86_64-apple-darwin` | `vastlint_nif-x86_64-macos.dylib.tar.gz` |
| `aarch64-apple-darwin` | `vastlint_nif-aarch64-macos.dylib.tar.gz` |

Checksums are embedded in `mix.exs` and verified at install time. If a precompiled
NIF is not available for the target platform, `RustlerPrecompiled` falls back to
compiling from source (requires Rust toolchain — documented in README).

### Release workflow additions

New job `build-nif` in `release.yml` — same 4-platform matrix as `build-ffi`,
runs after `smoke-test`:

```yaml
build-nif:
  name: Build NIF ${{ matrix.target }}
  needs: smoke-test
  strategy:
    matrix:
      include:
        - { target: x86_64-unknown-linux-gnu,   os: ubuntu-latest }
        - { target: aarch64-unknown-linux-gnu,  os: ubuntu-latest }
        - { target: x86_64-apple-darwin,        os: macos-latest  }
        - { target: aarch64-apple-darwin,       os: macos-latest  }
  steps:
    - build cdylib for vastlint_nif
    - package as vastlint_nif-{platform}.tar.gz with checksum
    - upload-artifact
    - attach to GitHub Release
```

New job `sync-erlang` (gated `ENABLE_ERLANG_SYNC=true`, deploy key secret
`VASTLINT_ERLANG_DEPLOY_KEY`) — mirrors `sync-go`:
- Checks out `vastlint-erlang`
- Updates NIF artifact checksums in `mix.exs`
- Bumps package version to match tag
- Commits, pushes, tags
- Triggers `vastlint-erlang` CI (`mix test` on ubuntu + macos, OTP 26+)

### Maintenance contract

Adding a new VAST spec version to `vastlint-core` requires zero changes to
`vastlint-nif`. The NIF crate has no knowledge of rule IDs, version numbers, or
issue structure — it calls `vastlint_core::validate()` and maps the
`ValidationResult` to BEAM terms. New rules appear automatically in the result
map's `issues` list. The Erlang/Elixir API surface is unchanged.

The only thing that changes on a `vastlint-core` version bump is the `Cargo.toml`
dependency version in `vastlint-nif`, which is updated automatically by the
monorepo release workflow.

---

## Dependencies

Intentionally minimal.

- `quick-xml` — XML parsing, zero-copy, actively maintained
- `url` — URL validation for tracking and media URLs (small, well-tested)
- `phf` — compile-time perfect hash maps for enum/value lookups (no runtime cost)

No async runtime. No serde in the core (serialization is the caller's job). No regex crate (use direct string/pattern matching for performance).

---

## Testing Strategy

Every rule has at minimum:
- one test with a valid document that should not trigger the rule
- one test with an invalid document that must trigger the rule
- one test per version boundary (rule applies in 4.x but not 3.x, etc.)

Fixture files in tests/fixtures/ are named clearly:

```
valid_4.2_linear.xml
invalid_4.2_missing_adsystem.xml
valid_3.0_wrapper.xml
invalid_wrapper_depth_exceeded.xml
```

The test suite runs against all fixture files as integration tests in addition to unit tests per rule.

---

## WASM Build Notes

The crate compiles to WASM via `wasm-pack` without modification. No conditional compilation needed as long as dependencies are WASM-compatible. `quick-xml` and `url` both support WASM targets.

The WASM bindings live in `crates/vastlint-wasm`. The npm package is assembled in `npm/` and published as `vastlint`. Build steps:

```sh
# ESM / bundler target (Webpack, Vite, Rollup, etc.)
wasm-pack build crates/vastlint-wasm --target bundler --out-dir ../../npm/pkg

# CommonJS / Node.js target
wasm-pack build crates/vastlint-wasm --target nodejs --out-dir ../../npm/pkg-node

# Copy built files to package root
node npm/scripts/assemble.js

# Publish
cd npm && npm publish
```

---

## Release and Publish Graph

A single git tag (`vX.Y.Z`) triggers everything via `.github/workflows/release.yml`:

```
git tag vX.Y.Z → push tag
  │
  ├── build (4 targets: linux x86_64/aarch64, macos x86_64/aarch64)
  │     │
  │     ├── release       → GitHub Release with CLI binary tarballs
  │     │
  │     ├── publish       → crates.io: vastlint-core, then vastlint-cli
  │     │   [ENABLE_CRATES_PUBLISH=true]
  │     │
  │     └── publish-npm   → npm: vastlint (WASM, built fresh from source)
  │         [ENABLE_NPM_PUBLISH=true]
  │
  ├── build-ffi (4 targets: linux x86_64/aarch64, macos x86_64/aarch64)
  │     │
  │     └── release       → GitHub Release with FFI tarballs
  │                          (vastlint-ffi-{os}-{arch}.tar.gz)
  │                          each contains: vastlint.h + libvastlint.a + libvastlint.{so,dylib}
  │
  └── sync-go             → vastlint-go repo (github.com/aleksUIX/vastlint-go)
      [ENABLE_GO_SYNC=true, requires VASTLINT_GO_DEPLOY_KEY secret]
        │
        ├── unpacks FFI tarballs into vastlint-go/libs/{platform}/libvastlint.a
        ├── commits "chore: update libs to vX.Y.Z"
        ├── pushes to vastlint-go main
        └── tags vastlint-go at vX.Y.Z
              │
              └── triggers vastlint-go CI: go test (ubuntu + macos)
```

**Version sync:** The npm package version is set automatically from the git tag during the workflow (`TAG=v1.2.3` → `npm version 1.2.3`). You do not need to manually bump `npm/package.json`.

**Required secrets (Settings → Secrets → Actions):**
- `CARGO_REGISTRY_TOKEN` — crates.io API token
- `NPM_TOKEN` — npm automation token (create at npmjs.com → Access Tokens → Automation)

**Required repo variables (Settings → Variables → Actions):**
- `ENABLE_CRATES_PUBLISH` — set to `true` to enable crates.io publish
- `ENABLE_NPM_PUBLISH` — set to `true` to enable npm publish

Both flags default to off so a misconfigured tag push doesn't accidentally publish.

**To do a full release:**
```sh
# Bump versions in Cargo.toml files first, commit, then:
git tag v0.2.0
git push origin v0.2.0
```

---

## Open Questions

- CTV Addendum 2024 rules: treat as a separate optional rule set, opt-in via ValidationContext flag.
- Rule extensibility for enterprise: not in core. Enterprise layer wraps core and adds rules post-validation before returning to caller.

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
```

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

# Publish
cd npm && npm publish
```

---

## Open Questions

- CTV Addendum 2024 rules: treat as a separate optional rule set, opt-in via ValidationContext flag.
- Rule extensibility for enterprise: not in core. Enterprise layer wraps core and adds rules post-validation before returning to caller.

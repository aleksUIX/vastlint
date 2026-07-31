# Changelog

All notable changes to vastlint are documented here.
Versions follow [Semantic Versioning](https://semver.org/).
GitHub Releases: <https://github.com/aleksUIX/vastlint/releases>

---

## [Unreleased]

## [0.11.0] - 2026-07-30

### Added

- CTV Ad Portfolio support for the **VAST 2.0 extension path**. The pull
  request that landed `vast_4.4.xsd` on 2026-07-17 also added
  `extensions/ctv_ad_portfolio.md` and `extensions/ctv_qrcode.md`, which
  deliver the same six formats through VAST 2.0 rather than 4.x. That path
  had no coverage: none of the AdCOM value or QR checks were reachable, and
  the generic extension rules reported IAB's own examples as defective.
  Six new rules for the container's own failure modes:
  `VAST-2.0-ctv-portfolio-creative-id-required` (the binding that fails
  silently when an ad has multiple creatives),
  `VAST-2.0-ctv-portfolio-creative-id-unmatched`,
  `VAST-2.0-ctv-portfolio-mediafiles-required`,
  `VAST-2.0-ctv-portfolio-mediafiles-empty`,
  `VAST-2.0-ctv-portfolio-no-renderable-asset` and
  `VAST-2.0-ctv-portfolio-no-duration`. Catalog: 212 to 218 rules.

- **Format-to-Signal consistency**, two rules that read the AdCOM signals as a
  set rather than one at a time: `VAST-4.4-adcom-pos-format-mismatch` and
  `VAST-4.4-adcom-playbackmethod-format-mismatch`. `plcmt`, `pos` and
  `playbackmethod` can each sit inside its own enumeration and still describe
  two formats at once, which is invisible to the per-value rules: `plcmt=5`
  (Pause) with `pos=12` (a Squeezeback layout) passed all three. The scope is
  the container that carries the signals, so both encodings are covered: the
  whole `<Extensions>` block on 4.x, the single
  `<Extension type="ctv_ad_portfolio">` on 2.0. Catalog: 218 to 220 rules.

  Both stay quiet where the guidance hedges. `pos` is checked for the four
  formats with a published value set, never for In-Scene, whose table cell
  reads "NA", and never for `pos=0`, AdCOM "unknown". `playbackmethod` is
  checked only against the two rows the reference table states outright, 8/9
  Pause and 10/11 Screensaver, in both directions; the other three rows read
  "typically 1 or 2", which is advice rather than a constraint, so an Overlay
  declaring an autoplay method is not reported. An out-of-range `plcmt`
  suppresses both, since `VAST-4.4-adcom-plcmt-value` has already reported it
  and the pairing would be guesswork.

- `explain_rule` in the MCP server returns fix guidance for the CTV Ad
  Portfolio rules instead of the generic fallback.

### Fixed

- `VAST-2.0-extension-misplaced-element` no longer fires inside a
  standardised IAB extension container. `<Extension
  type="ctv_ad_portfolio">` carries `MediaFiles`, `Duration`,
  `TrackingEvents`, `Icons` and `AdParameters` by design, because it exists
  to give VAST 2.0 a NonLinear delivery model it never had.
- Version inference ignores elements inside those containers. A conforming
  2.0 tag carrying a SIMID `InteractiveCreativeFile` in its extension was
  reported as `VAST-2.0-version-mismatch`, which is the one thing the
  extension is designed to do.
- `VAST-4.4-adcom-attr-not-motion` accepts 19 and 20. AdCOM 1.0-202607 added
  19 Contains advertiser QR Code and 20 Support alpha channel transparency
  alongside the three motion attributes, and IAB's own examples declare 19.
- The media rules only apply when the extension is the delivery vehicle. A
  creative that renders from a native `StaticResource` and uses the container
  only to declare AdCOM signals is conforming.

All five VAST 2.0 examples published in the two extension documents now
validate clean.

### Changed

- VS Code extension 0.10.0 and Chrome extension 0.9.0 pick up the new rules.
  The `vastlint.vastVersion` setting offers `4.4`, which the CLI has accepted
  since 0.10.0.
- `specs/vast_4.4_reference.md` covers the VAST 2.0 extension path, the
  Format-to-Signal cross-check and its deliberate gaps, and corrects the
  `attr=20` entry in its list of defects in IAB's published examples: AdCOM
  1.0-202607 defines 20, so it was never a defect.

## [0.10.2] - 2026-07-27

### Maintenance

- Dependency updates via Dependabot: `tokio` 1.53.0 → 1.53.1, `clap` 4.6.2 → 4.6.4 (`clap_derive` 4.6.1 → 4.6.4, which moves its macro backend to `syn` 3). Both are semver-compatible inside the existing `tokio = "1"` and `clap = "4"` ranges, so the change is lockfile-only.
- CI action pins refreshed: `actions/checkout` 7.0.0 → 7.0.1, `github/codeql-action/upload-sarif` 4.37.1 → 4.37.3, `docker/login-action` 4.4.0 → 4.5.1, `ossf/scorecard-action` 2.4.3 → 2.4.4.
- `cargo audit`: no known vulnerabilities. `npm audit`: 0 vulnerabilities, unchanged since 0.10.1.

## [0.10.1] - 2026-07-26

### Security

- Cleared all six open Dependabot alerts. Every one is a transitive dev-dependency of the build tooling, so none of them ship in the CLI, the WASM package, the published extensions, or any other runtime artifact.
- VS Code extension build chain (transitive under the `@vscode/vsce` dev-dependency): `brace-expansion` 5.0.7 → 5.0.8 (GHSA-mh99-v99m-4gvg, unbounded expansion length causing an out-of-memory crash), `fast-uri` 3.1.2 → 3.1.4 (GHSA-4c8g-83qw-93j6 host confusion via failed IDN canonicalization, GHSA-v2hh-gcrm-f6hx host confusion via a literal backslash authority delimiter), `linkify-it` 5.0.1 → 5.0.2 (GHSA-v245-v573-v5vm, quadratic-complexity DoS in the `mailto:` validator scan-loop).
- Example apps `vastlint-client-browser`, `vastlint-client-react-scratch` and `vastlint-react-demo` (transitive under `vite`): `postcss` 8.5.15 → 8.5.23 (GHSA-r28c-9q8g-f849, path traversal in previous-source-map auto-loading via `sourceMappingURL` leading to arbitrary `.map` file disclosure).
- The `brace-expansion` 5.0.7 pin from 0.9.1 was superseded by a new advisory a week after it landed. 5.0.8 raises its floor to Node 20.
- All four fixes resolved in-range, so no `overrides` entries were added and no `package.json` changed. Lockfile-only.
- `npm audit`: 0 vulnerabilities in the root workspace, `npm/`, `vscode/`, `chrome/` and all three example apps. `cargo audit`: no known vulnerabilities across 248 crates.

## [0.10.0] - 2026-07-25

### Added

- **VAST 4.4 and the IAB CTV Ad Portfolio** (`ctv_portfolio.rs`, new rule category). IAB Tech Lab finalised the CTV Ad Portfolio signaling guidance on 2026-07-22 and landed `vast_4.4.xsd` in the VAST repo on 2026-07-17. vastlint now recognises `version="4.4"`, accepts the new content model, and adds 17 rules for it. Catalog: 195 → 212 rules.

  The two sources are at different stages of the same process and vastlint weights them accordingly. The **signaling guidance is final**, so rules derived from it (`RuleSource::CtvAdPortfolio`, new) carry normal weight. The **4.4 XSD is a working-group draft** by its own annotation, so schema-only findings stay at warning or info rather than getting ahead of the working group. The only errors are constructs malformed under any version: a non-integer AdCOM payload, a QR position in pixels.

  New content model accepted on **any VAST 4.x document**, not just 4.4: `<MediaFiles>`, `<Duration>` and `<NonLinearCustomClick>` under `<NonLinear>`, and `<Icons>` under `<NonLinearAds>`. This is deliberate: every VAST example in IAB's final guidance declares `version="4.2"` while using these constructs, so gating on 4.4 would have rejected conforming CTV traffic. `VAST-2.0-nonlinear-resource` now accepts `<MediaFiles>` as a fourth resource form. VAST 3.0 and below are unchanged; a regression fixture guards this.

  Rules: `VAST-4.4-version-attribute` (info, flags the draft status), `VAST-4.4-nonlinear-no-renderable-asset`, `VAST-4.4-nonlinear-mediafiles-empty`, `VAST-4.4-nonlinear-simid-iframe`, `VAST-4.4-nonlinear-video-no-duration`, `VAST-4.4-adcom-extension-unknown-signal`, `VAST-4.4-adcom-extension-type-mismatch`, `VAST-4.4-adcom-signal-not-integer`, `VAST-4.4-adcom-plcmt-value`, `VAST-4.4-adcom-playbackmethod-value`, `VAST-4.4-adcom-pos-value`, `VAST-4.4-adcom-attr-not-motion`, `VAST-4.4-qrcode-position-attrs`, `VAST-4.4-qrcode-position-percent`, `VAST-4.4-qrcode-size-attr`, `VAST-4.4-qrcode-size-percent`, `VAST-4.4-qrcode-missing-scan-url`.

- **SIMID in NonLinear `<MediaFiles>`**: the CTV Ad Portfolio guidance names `<InteractiveCreativeFile apiFramework="SIMID">` inside the NonLinear `<MediaFiles>` container "the preferred VAST 4.4 pattern" and deprecates `<IFrameResource apiFramework="SIMID">`. The existing SIMID type/URL/HTTPS/variableDuration rules now apply there. `SIMID-1.0-simid-mediafile-required` stays Linear-only (`VAST-4.4-nonlinear-no-renderable-asset` covers the NonLinear case, where a static resource is an equally valid fallback), and `SIMID-1.1-nonlinear-simid-no-iframe` no longer fires when the creative already uses the preferred form.

- **`specs/vast_4.4_reference.md`**: the delta against 4.3, the full AdCOM signal tables (plcmt 5–9, playbackmethod 8–11, pos to 17, attr 21–23), and two things worth knowing before trusting the draft schema. The draft is **scoped to the CTV work**: `AltText`, `BlockedAdCategories`, `Expires` and `IconClickFallbackImage(s)` are not carried over, and since nothing in the guidance touches them, vastlint reads that as scope rather than deprecation and models 4.4 as 4.3 plus the additions. The draft is also stricter on `Extension` than 4.2 (`@type` required, custom children restricted to `##other`), which would flag a lot of deployed tags, so vastlint waits for the schema to settle before enforcing either.

### Changed

- `--vast-version` accepts `4.4`. `VAST-2.0-root-version-value` recognises `4.4` so a 4.4 tag is told about the draft status rather than that its version string is unrecognised.
- `RuleSource` gains `CtvAdPortfolio` for rules sourced from the finalised signaling guidance, distinct from `VastXsd` for rules sourced from the draft schema.
- VS Code extension 0.9.0 and Chrome extension 0.8.0 pick up the 4.4 rule set. `chrome/package.json` and `chrome/manifest.json` are unified on one version number.

## [0.9.1] - 2026-07-20

### Maintenance

- Dependency updates via Dependabot: `tokio` 1.52.3 → 1.53.0, `criterion` 0.7.0 → 0.8.2 (dev-dependency, benches), `serde` 1.0.228 → 1.0.229, `serde_json` 1.0.150 → 1.0.151, `toml` 1.1.2+spec-1.1.0 → 1.1.3+spec-1.1.0, `regex` 1.13.0 → 1.13.1, `clap` 4.6.1 → 4.6.2.
- CI action pins refreshed: `softprops/action-gh-release` 3.0.1 → 3.0.2, `github/codeql-action/upload-sarif` 4.37.0 → 4.37.1, `dtolnay/rust-toolchain` pinned commit refreshed, `taiki-e/setup-cross-toolchain-action` 1.41.0 → 1.42.0, `actions/setup-node` 6.4.0 → 7.0.0.
- `cargo audit`: no known vulnerabilities.

## [0.9.0] - 2026-07-16

### Added

- **Content quality rules** (`quality.rs`, new rule category): `VAST-2.0-adtitle-quality` (warning) flags known placeholder `<AdTitle>` values (`test`, `Ad 1`, `untitled`, empty, ...); `VAST-2.0-adsystem-quality` (info) flags placeholder `<AdSystem>` values; `VAST-2.0-adsystem-no-version` (info) flags `<AdSystem>` without a `version` attribute. Conservative placeholder lists (phf sets) keep false positives near zero; all three can be disabled per-rule in `vastlint.toml`. Catalog: 191 → 195 rules.
- **`VMAP-1.0-display-break-no-companions`** (info): an `<AdBreak>` whose `breakType` includes `display` but whose inline `<VASTAdData>` VAST contains no `<CompanionAds>` has nothing to display. `<AdTagURI>` sources are not checked (zero-I/O core).
- **`vastlint init`**: generates a starter `vastlint.toml` with every rule listed at its default severity, commented out. `--out <path>` and `--force` flags; refuses to overwrite without `--force` (exit 2).
- **Criterion benchmark** (`cargo bench -p vastlint-core`): validation throughput per fixture plus a 10-ad pod. Confirms the sub-1ms ARCHITECTURE.md target with wide margin (7-16 µs typical, ~140 µs for the pod).
- **WASM: `document_type` in validation results**: `validate()`/`validateWithOptions()` now return `document_type: "VAST" | "VMAP" | "DAAST"`, matching the CLI and MCP surfaces. npm TypeScript definitions updated.

### Fixed

- **Namespace-prefixed attributes no longer false-flag as `VAST-2.0-unknown-attribute`**: attributes in a foreign namespace (`xsi:type`, `xsi:schemaLocation`, `xmlns`, `xmlns:*`, vendor prefixes) are not governed by VAST's per-element attribute allowlists and are now skipped. Schema-annotated, spec-compliant tags stay clean. Separately, `AdID` (the VAST 2.0 casing of the `<Creative>` creative-id attribute, renamed `adId` in 3.0+) is now accepted on `<Creative>`. Both were false positives flagged on real-world compliant tags.

---

## [0.8.4] - 2026-07-16

### Fixed

- **VS Code extension build broken by TypeScript 7**: `vscode/tsconfig.json` set `moduleResolution: "node"` (alias for `node10`), an option TypeScript 7.0 removed outright (`error TS5108`). Introduced by the `typescript` 6.0.3 → 7.0.2 dependabot bump in v0.8.2; PR CI didn't catch it because the VS Code build only runs inside the release workflow's smoke-test job, not the PR CI matrix, so it surfaced as two failed release runs (v0.8.2, v0.8.3) instead. Removed the explicit option; `tsc` infers the equivalent resolution from `module: "commonjs"`.

---

## [0.8.3] - 2026-07-16

### Security

- **`quick-xml` 0.40.1 → 0.41.0 in `fuzz/Cargo.lock`** (RUSTSEC-2026-0194, quadratic-time duplicate-attribute check; RUSTSEC-2026-0195, unbounded namespace-declaration allocation). `fuzz/` is excluded from the main workspace and resolves independently; its lockfile had drifted behind the root `Cargo.lock`, which has carried the patched version since v0.8.1.
- **Removed `crates/vastlint-nif/Cargo.lock`.** `vastlint-nif` is a workspace member, so real builds (including the release workflow's `cargo build -p vastlint_nif`) always resolve against the root `Cargo.lock`. This nested lockfile was orphaned since Release 0.4.24, still pinned `crossbeam-epoch 0.9.18` / `quick-xml 0.40.1`, and was never consumed by any actual build — but Scorecard's OSV scan reads every `Cargo.lock` in the repo regardless of whether cargo would use it, which is why alert #11 (RUSTSEC-2026-0194/0195/0204) kept reappearing across releases going back to April despite the real, shipped artifacts never being affected.

No source or runtime behavior changes; both fixes are lockfile-only.

---

## [0.8.2] - 2026-07-15

### Maintenance

- **`regex`** 1.12.4 → 1.13.0.
- **`rmcp`** 2.1.0 → 2.2.0 (MCP server).
- **`@types/node`** 26.1.0 → 26.1.1 (vscode dev dependency).
- **`typescript`** 6.0.3 → 7.0.2 (vscode dev dependency).
- CI action pin refreshed: `github/codeql-action/upload-sarif` 4.36.3 → 4.37.0.
- Fixed a stale branch-protection configuration on `main`: required status check contexts referenced CI jobs (`CI / Clippy`, `CI / Test (*)`) from before the CI workflow consolidated clippy into the `test` job, which was blocking every PR merge regardless of actual CI result. Updated to the current job names (`Test (ubuntu-latest)`, `Test (macos-latest)`, `Test (windows-latest)`, `Security audit`).

No source or behavior changes; dependency and infra maintenance only. `cargo audit` clean (0 vulnerabilities), 0 open Dependabot security alerts.

---

## [0.8.1] - 2026-07-06

### Security

- **`crossbeam-epoch` 0.9.18 → 0.9.20** (RUSTSEC-2026-0204): the `fmt::Pointer`/`fmt::Display` impl for `Atomic`/`Shared` dereferenced the underlying pointer, causing an invalid dereference when that pointer was null. Transitive dependency via `rayon` (used by the Erlang NIF); not reachable at runtime, patched in the lockfile.

### Maintenance

- **`rmcp` 1.8.0 → 2.1.0** (MCP server): upgrade to the 2.x line of the Rust MCP SDK. MCP smoke tests (initialize, tools/list, and tools/call across validate_vast, fix_vast, list_rules, explain_rule) pass unchanged.
- Dependency update via Dependabot: `@types/node` 26.0.1 → 26.1.0 (vscode dev dependency).
- CI action pins refreshed: `docker/build-push-action` 7.2.0 → 7.3.0, `docker/setup-qemu-action` 4.1.0 → 4.2.0, `docker/setup-buildx-action` 4.1.0 → 4.2.0, `docker/login-action` 4.2.0 → 4.4.0, `github/codeql-action/upload-sarif` 4.36.2 → 4.36.3, and `dtolnay/rust-toolchain` pinned commit refreshed.

---

## [0.8.0] - 2026-07-05

### Core

- **IAB Content Taxonomy authority validation** (4 new rules, catalog now 191): `authority` attribute values on `<Category>` (4.0+) and `<BlockedAdCategories>` (4.1+) are validated for URL well-formedness (`VAST-4.0-category-authority-not-uri`, `VAST-4.1-blockedadcategories-authority-not-uri`, Warning) and recognition against the IAB Content Taxonomy registry hosts, `iabtechlab.com` and subdomains plus `iab.com`, including version-qualified forms such as `iabtechlab.com/IABTC/2.2` (`VAST-4.0-category-authority-unknown`, `VAST-4.1-blockedadcategories-authority-unknown`, Info). Custom taxonomies stay legal; only malformed values warn.

### Project

- New `GOVERNANCE.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), rewritten `CONTRIBUTING.md` (DCO, coding standards, test policy), and a SECURITY.md assurance case, as part of OpenSSF Best Practices Silver work.
- Release workflow: crates.io publish failures are no longer silently swallowed (v0.6.3 through v0.7.2 never reached crates.io; re-published via the new `publish-crates.yml` recovery workflow).

---

## [0.7.2] - 2026-07-04

### CLI

- **`--share`**: uploads a validation report (rule IDs, severities, XPath locations, summary counts) to vastlint.org and prints a public `vastlint.org/r/<id>` link for pasting into Slack/GitHub/PRs. Never sends the input XML.
- **`--contribute-sample`**: opt-in only, off by default. Sends the tag's raw XML to vastlint.org to help refine rules. Known tracking identifiers (device IDs, IPs, consent strings) are redacted server-side before storage; samples are never made public. Independent of `--share` and `--telemetry`.

---

## [0.7.1] - 2026-06-30

### Security

- **`ws` 8.20.1 → 8.21.0** (GHSA-96hv-2xvq-fx4p): memory-exhaustion DoS from tiny fragments and data chunks. Transitive dev-tooling dependency (jsdom), not shipped in any runtime artifact.
- **`quinn-proto` 0.11.14 → 0.11.15** (RUSTSEC-2026-0185): transitive dependency advisory.

### Maintenance

- Dependency updates via Dependabot: `rmcp` 1.7.0 → 1.8.0 (MCP server), `@types/node` 26.0.0 → 26.0.1 and `ovsx` 1.0.0 → 1.0.2 (vscode dev dependencies).
- CI action pins refreshed: `github/codeql-action/upload-sarif` 3.28.18 → 4.36.2, `actions/attest-build-provenance` 4.1.0 → 4.1.1.

---

## [0.7.0] - 2026-06-27

### Core validator

- **VAST macro validation**: a new `macros` rule chain inspects `[MACRO]` substitution tokens in tracking, click, error, impression, and media URLs against the IAB VAST macro list. Five new rules:
  - `VAST-2.0-macro-unknown` (warning): bracketed token that is not a recognised IAB macro (typo, or a vendor-specific macro to allowlist).
  - `VAST-2.0-macro-lowercase` (warning): a recognised macro not written in uppercase, which players never substitute.
  - `VAST-4.1-macro-deprecated` (info): `[CONTENTPLAYHEAD]`/`[MEDIAPLAYHEAD]`, deprecated in 4.1 in favour of `[ADPLAYHEAD]` (fires on 4.1+ only).
  - `VAST-2.0-macro-wrong-context` (info): a context-restricted macro used where it has no defined value (`[ERRORCODE]` outside `<Error>`, `[REASON]` outside `verificationNotExecuted`).
  - `VAST-2.0-macro-uri-unencoded` (warning, RFC 3986): a macro-bearing URL with characters that must be percent-encoded.
- The scanner is a single bounded, linear byte pass (`MAX_MACRO_LEN`): only URL-bearing elements are inspected (free text such as `<AdTitle>` is never treated as a macro), numeric array indices like `key[0]` are skipped, and adversarial input (large runs of `[`) cannot cause super-linear scanning. Root-level `<Error>` beacons are scanned with the correct context.
- **Rule catalog grown to 187** (`all_rules()`, RULES.md, VS Code rule list).

### Documentation / metadata

- **RULES.md and VS Code README updated**: the five macro rules listed with severities and spec references; rule count updated to 187.
- **Rule count updated to 187** across README.md, ROADMAP.md, docs/tutorial.md, docs/mcp-agentic.md, crate/npm/vscode/chrome/MCP descriptions, and the vastlint.org site (new "Macros" rule category with per-rule doc pages).

---

## [0.6.3] - 2026-06-24

### Maintenance

- Dependency updates via Dependabot: `phf` 0.13.1 → 0.14.0 (core crate), `@types/node` 25.9.3 → 26.0.0 (vscode dev dependency).
- CI action pins refreshed: `actions/checkout` 6.0.3 → 7.0.0, `softprops/action-gh-release` 3.0.0 → 3.0.1, `dtolnay/rust-toolchain` advanced to the latest pinned SHA.

---

## [0.6.2] - 2026-06-19

### Security

- Patched 16 Dependabot alerts across npm dependency trees: `undici` 7.28.0, `form-data` 4.0.6, `js-yaml` 4.2.0, `tmp` 0.2.7 (vscode); `vite` 6.4.3 (3 example manifests); `esbuild` 0.28.1 (chrome extension).

### Versioning

- Aligned all package versions (npm, vscode extension, chrome extension, all Rust crates) to the core release version. Previously these drifted across 0.4.24 and 0.5.0.

---

## [0.5.0] - 2026-06-13

### Core validator

- **VMAP 1.0 validation**: `validate()` now recognises `<vmap:VMAP>` documents and runs a dedicated 24-rule VMAP chain — `<AdBreak>` structure, `timeOffset`/`breakType`/`repeatAfter` formats, `<AdSource>` content constraints (exactly one of `VASTAdData`/`AdTagURI`/`CustomAdData`, CDATA requirements), VMAP tracking events, and `repeatAfter`/`timeOffset` conflict detection. Inline VAST inside `<vmap:VASTAdData>` is validated with the full VAST rule chain; issues surface with `/VMAP/AdBreak[i]/AdSource/VASTAdData`-prefixed paths and document-absolute line/col.
- **DAAST 1.0 validation**: `<DAAST>` documents run a dedicated 29-rule chain covering the audio-specific deltas from VAST 3.0 — required `<Category>`, optional `<AdSystem>`, `<DAASTAdTagURI>` wrappers, `<AdInteractions>` (with detection of VAST leftovers like `<VideoClicks>` and `<VASTAdTagURI>`), audio MediaFile attributes, the DAAST tracking event set, DAAST pricing models (incl. `cpo`), root-level `<Error>` URI presence, and `[ERRORCODE]` macro inclusion.
- **`ValidationResult.document_type`**: new `DocumentType` enum (`Vast` / `Vmap` / `Daast`) reporting which rule chain ran. `version` stays VAST-specific and is `Unknown` for VMAP/DAAST documents. New `RuleSource` variants: `VmapSpec`, `DaastSpec`, `DaastXsd`. Breaking change: callers that destructure `ValidationResult` must add the new field.
- **Rule catalog grown to 182** (`all_rules()`, RULES.md, VS Code rule list).

### Documentation / metadata

- **RULES.md and VS Code README updated**: all 182 rules listed with correct severities and spec references.
- **README.md rule count updated**: three occurrences of "129 rules" updated to "182 rules"; VMAP 1.0 and DAAST 1.0 added to category list.
- **vastlint.org rule pages**: 53 new per-rule doc pages for all VMAP and DAAST rule IDs, with category impact descriptions, spec links, and XML examples.

### Release metadata

- **Semver bump to 0.5.0**: breaking change due to new public `document_type` field on `ValidationResult`.

## [0.4.24] - 2026-06-09

### Core validator

- **OMID coverage expanded inside core validation**: `validate()` and `validate_with_context()` now validate OMID compatibility blocks carried in pre-4.1 `Extension type="AdVerifications"` payloads, enforce Verification tracking semantics, and warn when `verificationNotExecuted` tracking URLs omit the `[REASON]` macro.
- **Verification tracking tightened**: `<Tracking>` under `<Verification>` now only accepts `event="verificationNotExecuted"`, matching the VAST verification schema instead of the generic Linear tracking event set.

### Documentation / metadata

- **Public docs updated for shipped OMID support**: README, ROADMAP, MCP docs, tutorial copy, and package metadata now describe OMID validation as shipped instead of upcoming.
- **Rule count aligned to 129**: release-facing docs and package descriptions now reflect the current catalog size.

### Release metadata

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, and tracked lockfiles to `0.4.24`.

## [0.4.23] - 2026-06-09

### Branding

- **VASTlint display branding aligned**: updated the user-facing product name across the root docs, Chrome extension UI, VS Code extension metadata, MCP docs, and package descriptions to use `VASTlint` while keeping commands, repo names, package names, URLs, and config keys unchanged.

### Release metadata

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, and tracked lockfiles to `0.4.23`.

## [0.4.22] - 2026-06-08

### CI / release

- **Ecosystem smoke stabilized**: fixed the Erlang path/build issues in the published package smoke workflow so release verification now covers the shipped ecosystem again.
- **Release workflow inputs refreshed**: updated pinned GitHub Action dependencies including `actions/checkout` and `docker/setup-qemu-action` for the next patch release.

### VS Code extension

- **Packaging toolchain patched**: bumped `@vscode/vsce` to `3.9.2`, `@types/node` to `25.9.2`, and kept `ovsx` on `1.0.0` in the extension release surface.

### Client

- **Client package refreshed**: pulled in the current `vastlint-client` update from `main` for the patch release train.

### Release metadata

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, workflow version pins, and tracked lockfiles to `0.4.22`.

## [0.4.21] - 2026-05-31

### Security / supply chain

- **`vastlint-client` CDATA parsing hardened**: replaced the regex-based CDATA stripper with linear scanning so untrusted XML no longer hits the polynomial-regex CodeQL finding.
- **VS Code packaging dependencies patched**: forced `tmp` to `0.2.6` in the extension build lockfile to clear `GHSA-ph9p-34f9-6g65`.
- **Repo artifact hygiene tightened**: stopped tracking generated example `dist/` output so demo WASM bundles are no longer committed as source artifacts.
- **Action smoke workflow pinned**: the published `vastlint-action` smoke workflow now uses the `v1` commit SHA instead of a mutable tag.

## [0.4.20] - 2026-05-31

### Chrome extension

- **Issue panel rendering fixed**: the inline/floating panel now mounts lint issue rows as DOM nodes instead of coercing them into `[object HTMLDivElement]` strings inside the shadow-root template.

### Packaging / release

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, and tracked lockfiles to `0.4.20` for the patch release.

## [0.4.19] - 2026-05-31

### CI / release

- **MCP canary aligned with the live endpoint**: the GitHub Actions canary now parses `list_rules` as an array payload and validates `get_adcp_capabilities` against the current MCP contract without the removed top-level `status` field.
- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, lockfiles, example lockfile references, and tracked generated package metadata to `0.4.19`.

## [0.4.17] - 2026-05-16

### Core validator

- **Version floor consistency fixed**: declared newer VAST versions no longer raise false `VAST-2.0-version-mismatch` warnings when the XML only uses older structural features.
- **Malformed XML short-circuits cleanly**: parse failures now emit `VAST-2.0-parse-error` without cascading required-field noise.

### Packaging / release

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, Chrome extension, lockfiles, and tracked generated package metadata to `0.4.17`.
- **Packaged runtime sanity-covered**: the npm runtime smoke path now validates the built package against the shared fixture corpus before release tagging.

### Tests

- **Corpus expanded**: added malformed XML, wrapper-heavy, and mixed vendor pod fixtures plus directory-wide fixture sweeps to reduce regression gaps before release.

## [0.4.16] - 2026-05-14

### Security

- **Browser UI hardening**: removed HTML string rendering paths in the Chrome extension and popup so untrusted VAST content is rendered via DOM nodes instead of `innerHTML` sinks.
- **Release supply-chain tightening**: pinned mutable GitHub Actions, Docker image inputs, and CLI install versions used by the release workflow and local packaging script.

### Release metadata

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, and Chrome extension manifests and lockfiles to `0.4.16` so shipped artifacts match the tagged security release.

## [0.4.15] - 2026-05-13

### Docs

- **React drop-in example**: added `npm/examples/VastLintTakeHomePage.jsx`, a copy-paste frontend starting point with live validation, issue filtering, line-aware source navigation, and auto-fix preview.
- **Main README onboarding link**: linked the root README directly to the new `npm/examples` guide so frontend consumers can find the example without hunting through the repo.

### CI / release workflow

- **Homebrew tap sync**: the release workflow now updates `aleksUIX/homebrew-tap` automatically after tagged releases so the formula version and SHA-256 checks stay aligned with published CLI assets.

## [0.4.14] - 2026-05-12

### Release metadata

- **Channel versions aligned**: bumped the Rust crates, npm package, VS Code extension, and Chrome extension manifests and lockfiles to `0.4.14` so all published artifacts share the same release version.
- **Chrome popup version sourced from the manifest**: the extension footer now reads `chrome.runtime.getManifest().version` instead of a hardcoded string, preventing future UI version drift.

## [0.4.13] - 2026-05-11

### VS Code extension

- **Packaging compatibility restored**: aligned the extension typing floor with `engines.vscode` so `vsce package` works without raising the minimum supported VS Code version above 1.85.
- **Release metadata aligned**: the VS Code package manifest and lockfile now carry the correct extension version for packaged builds.

### Dependencies

- **Rust dependency refresh**: bumped `quick-xml` to `0.40.0` and `tokio` to `1.52.3`.
- **Tooling updates**: bumped VS Code dev typing support to `@types/node 25.7.0` and upgraded `actions/dependency-review-action` to `v5.0.0`.

### CI

- **Formatting gate fixed**: normalized Rust test formatting so the cross-platform CI matrix and dependency PR reruns pass cleanly again.

## [0.4.12] - 2026-05-11

### Core validator

- **CDATA-aware leaf payload checks**: the parser now preserves adjacent text and CDATA segments, retains entity references in plain text, and enables accurate warnings for URL, `Extension`, and `CreativeExtension` payloads that should be wrapped in CDATA.
- **New advisory rules**: added `VAST-2.0-url-cdata`, `VAST-2.0-extension-cdata`, and `VAST-2.0-creative-extension-cdata` as warning-level guidance for fragile leaf-text payloads.

### Docs

- **Rules catalog fixed**: `RULES.md` now matches the shipped 121-rule catalog, including SIMID entries, the quartile-tracking warning, and correct warning severity for the HTTP transport rules.
- **Count drift removed**: refreshed stale `118 rules` references across the main README, package metadata, MCP docs, roadmap, and architecture docs.
- **VS Code README resynced**: restored the embedded rule list with `vastlint.org` links and added parity coverage so the extension README stays aligned with the canonical rule catalog.

### Tests

- **Rules markdown parity**: added `crates/vastlint-core/tests/rules_markdown.rs` so `RULES.md` count, IDs, and severities must stay aligned with `all_rules()`.

## [0.4.11] - 2026-05-10

### VS Code extension

- **Cleaner diagnostics UI**: Problems entries now stay focused on the human-readable issue text while hovers link straight to the per-rule docs page. Quick fixes still resolve the correct rule ID even when VS Code does not surface `diagnostic.code` directly.
- **Utility coverage**: extracted template-ignore, multi-block extraction, and block-relative position mapping into `vscode/src/utils.ts`, with dedicated unit tests covering template masking and embedded multi-block coordinates.

### Core tests

- **SIMID fixtures expanded**: added valid and invalid SIMID integration fixtures covering missing media files, missing MIME types, HTTPS enforcement, and variable-duration warnings.
- **Large-tag regression coverage**: added large Publica-style fixtures so oversized production tags stay covered by integration tests.

### CI / release workflow

- **MCP canary sweep**: `validate_vast` canary coverage now runs a deterministic 15-case batch across VAST 2.0-4.2, wrappers, SIMID, and representative error fixtures instead of sampling a smaller XML pool.
- **Release smoke tests**: the release workflow now asserts that default CLI text output includes severity and a rule reference, and it runs the VS Code extension unit test suite before publishing.
- **Fuzz install compatibility**: removed `--locked` from `cargo install cargo-fuzz` in CI to avoid the nightly `rustix` resolver failure.

### Docs

- Updated the main README's VS Code section to match the cleaned Problems/hover experience.
- Corrected the public rules catalog header to reflect the current 118-rule set.

## [0.4.10] - 2026-05-05

### Crates

- **Version alignment**: bumped all crate `Cargo.toml` files to `0.4.10` in-repo so `crates.io` always reflects the current release even when the CI auto-bump skips a version.

## [0.4.9] - 2026-05-05

### CLI

- **Line/column in output**: `vastlint` CLI now prints `file:line:col` (or `file:line`) alongside the XPath location for every issue, making it easier to jump to the exact source position.

### Chrome extension

- **Release stamping fixed**: both Chrome publish workflows now inject the extension version correctly before build instead of dropping the `version` field from `manifest.json`.
- **Release guardrail**: both workflows now assert that `manifest.json` contains the expected version before packaging or publishing.
- **Chrome package version**: bumped source `chrome/package.json` and `chrome/manifest.json` to `0.4.9` for the next Web Store release.

### Docs

- Main README no longer says the Chrome Web Store listing is pending review.
- Release checklist now documents the actual Chrome publish paths and reminds you to commit both Chrome version files.

## [0.4.7] - 2026-05-01

### CI / release workflow

- **Smoke test**: updated MCP tool-count assertion from 5 → 6 to include `inspect_vast`; added `inspect_vast` to the per-tool name check.
- **Chrome extension publish**: replaced non-existent `trmcnvn/upload-google-chrome-extension` action (fake SHA) with the correct `mnao305/chrome-extension-upload@fdfe79400af990f5145a319e834aee64907ccff4` (v6.0.0); corrected input name `extension` → `file-path`; pinned `actions/setup-node` in chrome job to SHA `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e` (v6.4.0).

### Other

- Committed docs, GitHub workflow helpers, METHODOLOGY.md, and test fixtures that were staged but not included in the v0.4.6 release commit.

---

## [0.4.6] - 2026-05-01

### MCP server (`vastlint-mcp`)

- **`inspect_vast`** — new tool that follows a VAST wrapper chain hop-by-hop, returning creative metadata (AdSystem, AdTitle, Duration, impressions, tracking events, media files, companions) and full validation results for every level of the chain. Accepts a starting URL and an optional `max_depth` (default 5). Returns `hop_count`, `resolved`, `chain_valid`, `total_errors`, `total_warnings`, and a `stopped_reason` (`resolved` | `max_depth` | `fetch_error` | `parse_error`) alongside per-hop detail.
- `quick-xml` added as a direct dependency for hop metadata extraction.
- Server info string updated to describe all six tools.

---

## [0.4.5] - 2026-05-01

### VS Code extension

- **Updated extension description** — flyout text in the VS Code extensions panel now reflects CLI backend, any-file-type support, and 108 rules.
- **Settings table** — README now documents all six settings including `vastlint.templateIgnoreRegex`, `vastlint.vastVersion`, and `vastlint.cliPath`.
- **Full rule reference** — collapsed rule table (108 rules, each linking to `vastlint.org/docs/rules/{id}/`) added to the extension README.

---

## [0.4.4] - 2026-04-30

### VS Code extension

- **CLI backend** — the extension now spawns the local `vastlint` binary (searched on PATH and common install locations: `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`) and falls back to the bundled WASM when no binary is found. Using the CLI enables `vastlint.toml` config, rule overrides, and future CLI features automatically.
- **Multi-block support** — files with multiple `<VAST>…</VAST>` documents (e.g. batch response files) are now fully validated; each block gets its own set of squiggles with accurate positions.
- **Template ignore regex** (`vastlint.templateIgnoreRegex`) — a JS regex whose matches are replaced with same-length zeros before validation, preserving all line/col offsets. Strips Mustache `{{…}}`, ERB `<%…%>`, Go templates, or any ad-server macro syntax without shifting squiggle positions.
- **VAST version override** (`vastlint.vastVersion`) — force a specific spec version (2.0–4.3) regardless of the `version=` attribute. Passed as `--vast-version` to the CLI.
- **Any file type** — activation changed to `onStartupFinished`; `<VAST` anywhere in a file triggers linting regardless of file extension (`.erb`, `.go`, `.html`, etc.).
- **`vastlint.cliPath`** setting — override the binary path when not on PATH.

---

## [0.4.3] - 2026-04-30

### CLI (`vastlint-cli`)

- **`--vast-version <version>`** (`check` and `fix`) — override the VAST version used for validation, ignoring the `version=` attribute in the XML. Accepts `2.0`, `3.0`, `4.0`, `4.1`, `4.2`, `4.3`. Useful for enforcing a floor version across all incoming tags or testing how a tag scores against a target version.
- **`--ignore-pattern <regex>`** (`check` and `fix`) — replace all matches of the supplied regular expression with a valid HTTPS placeholder before validation. Designed for ad-server templating macros (`${IMPRESSION_URL}`, `%%CACHEBUSTER%%`) that would otherwise trigger URL-format errors on unresolved placeholders. The substitution is in-memory only — the original file is never modified.

### vastlint-core

- `ValidationContext` gains `forced_version: Option<VastVersion>` — when `Some`, skips XML version detection entirely and uses the supplied value. Used by the CLI flags above; available to library consumers.
- `VastVersion` now derives `Copy`.

### OTP port daemon

- **`vastlint daemon`** subcommand — speaks the Erlang `{:packet, 4}` binary framing protocol over stdin/stdout. Reads 4-byte big-endian length + raw UTF-8 VAST XML; writes 4-byte big-endian length + JSON validation result. Safe for production Elixir pipelines via `NimblePool` (each worker holds one persistent `Port`). Does not require the NIF.

---

## [0.4.2] - 2026-04-28

### VS Code extension

- **Kiro compatibility**: lowered `engines.vscode` minimum from `1.116.0` to `1.85.0` so the extension installs on Amazon Kiro and other VS Code forks with older API versions

---

## [0.4.1] - 2026-04-26

### VS Code extension

- **Hover tooltip redesign**: severity icons replaced with flat color squares (🟥 error, 🟨 warning, 🟦 info); 🔧 for fix hints
- **Compact hover layout**: collapsed from 5 spaced lines to 3 tight lines per issue
- **Rule ID links to docs**: each rule ID in the hover footer is now a clickable link to `vastlint.org/docs/rules/<id>/`
- **Fix hints coverage**: added missing hints for `VAST-3.0-bitrate-conflict`, `VAST-3.0-minmaxbitrate-pair`, `VAST-2.0-nonlinear-resource`, `VAST-4.0-interactive-creative-no-api`, `VAST-4.1-interactive-creative-type`, `VAST-3.0-pricing-model-case`; removed stale key `VAST-2.0-mediafile-bitrate-conflict`

---

## [0.4.0] - 2026-04-29

### Revenue impact classification (vastlint-core)

- **New `RuleSource::IndustryBestPractice`** - distinct from `VastSpec` and `Inferred`; renders as `"revenue impact"` in all output formats
- **New `RuleMeta::revenue_impact()`** - returns `true` for 12 rules where a structural defect causes direct measurement or delivery loss; no catalog field added, no breaking schema change
- **5 rules reclassified** from `Inferred` → `IndustryBestPractice`: `VAST-2.0-mediafile-https`, `VAST-2.0-tracking-https`, `VAST-2.0-duplicate-impression`, `VAST-4.1-mezzanine-recommended`, `VAST-4.1-vpaid-in-interactive-context`
- **HTTP tracker rules promoted** `Info` → `Warning`: `VAST-2.0-mediafile-https` and `VAST-2.0-tracking-https` - on HTTPS inventory these are guaranteed delivery failures, not advisory notices
- **New rule `VAST-2.0-linear-tracking-quartiles`** (`Warning`, `IndustryBestPractice`) - fires when a `<Linear>` creative has no `<TrackingEvents>` containing any of `start`, `firstQuartile`, `midpoint`, `thirdQuartile`, or `complete`; absence of all five is a complete measurement blackout. Spec reference: IAB VAST 4.1 §3.14.2

### Monitoring-friendly CLI (vastlint-cli)

- **`--fail-on-warning`** - exits non-zero when any warning is found; all 12 revenue-impact rules fire at `Warning` or `Error` severity, making this flag sufficient for a CI revenue gate
- **URL input with wrapper chain following** - `vastlint check https://…` fetches the tag and recursively follows `<VASTAdTagURI>` wrapper chains
- **`--max-depth N`** (default `5`) - controls how deep wrapper chains are followed, matching the IAB VAST 4.x recommendation
- **`--summary`** - prints aggregate pass/fail counts after validation; includes a `$revenue` line when any revenue-impact rules fired; works in both plain and JSON output modes

### Rule list (`vastlint rules`)

- New `$` column - marks the 12 revenue-impact rules
- Legend line added at the bottom of the table

### Rule count

118 rules total (was 108 before v0.3.x additions; 117 before this release).

---

## [0.3.7] - 2026-04-25

- CI: harden release pipeline (SLSA provenance, deploy key scoping)
- VS Code: align `engines.vscode` to `^1.116.0`

## [0.3.6] - 2026-04-25

- **Chrome extension**: v0.2.0 - HTML-rendered VAST detection, inline overlay annotations, privacy policy; CWS submission workflow
- CI: SLSA provenance signing; Smithery and MCP Registry idempotent publish

## [0.3.4] - 2026-04-18

- Security: patched two advisories (`idna` RUSTSEC-2024-0421, `rustls-webpki` RUSTSEC-2026-0098/0099); added `cargo audit` to CI
- Fuzz: cargo-fuzz targets for `validate`, `fix`, and `validate_wrapper`

## [0.3.3] - 2026-04-18

- **SIMID rules**: 9 rules covering SIMID 1.0 (linear) and SIMID 1.1 (nonlinear) - type, URL, HTTPS, `variableDuration`, `<MediaFile>` fallback, `<IFrameResource>` presence
- Docs: SIMID coverage expanded to all spec versions (1.0, 1.1, 1.2) on vastlint.org

## [0.3.2] - 2026-04-17

- **MCP server**: `vastlint-mcp` crate published to MCP Registry; tools: `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, `fix_vast`

## [0.3.1] - 2026-04-17

- **Auto-fix in VS Code**: inline quick-fix actions wired up; fix API exported from npm package
- **Open VSX**: extension now published to Open VSX Registry in addition to VS Code Marketplace

## [0.3.0] - 2026-04-17

- **Erlang/Elixir NIF** (`vastlint_nif`): native binding for BEAM-based ad servers and RTB platforms
- Performance docs updated to production-realistic benchmarks (17–44 KB tags)

---

## [0.2.6] - 2026-04-12

- Build: idempotent `cargo publish` and `vsce publish` (skip if version already exists)
- WASM: smoke test fixes; both targets built before assemble step

## [0.2.5] - 2026-04-11

- npm + WASM packages added; `vastlint` available on npm for browser and Node.js use

## [0.2.4] - 2026-04-11

- **Line/column positions**: all issues now include `line` and `col` in JSON output and VS Code diagnostics

## [0.2.3] - 2026-04-08

- **FFI C layer** (`vastlint-ffi`): `libvastlint` shared library with C header; Go binding (`vastlint-go`) backed by the same core
- `mimalloc` global allocator in CLI and FFI for lower memory overhead

## [0.2.2] - 2026-04-08

- Release pipeline fixes: provenance cascade on skipped jobs, version bump order

## [0.2.1] - 2026-04-08

- Telemetry endpoint fix

## [0.2.0] - 2026-04-08

- **Go binding** (`vastlint-go`): full Go FFI wrapper; same 108 rules, zero CGO complexity for callers
- Version equalization: all crates and bindings move to a unified version scheme

---

## [0.1.0] - 2026-04-03

- Initial public release
- **vastlint-core**: 108 rules derived from IAB VAST 2.0–4.3; zero-dependency, zero-I/O Rust library; validates in under 1 ms on typical production tags
- **CLI**: `vastlint check` with single-file, glob, stdin, JSON output; `vastlint fix` auto-repair with `--dry-run` and `--out`
- **Web validator**: vastlint.org/validate - client-side WASM, no data leaves the browser
- **VS Code extension**: inline diagnostics with rule IDs and spec references
- **REST API**: `/api/validate` on RapidAPI, WASM-powered, sub-millisecond response
- **Homebrew tap**: `brew install aleksUIX/tap/vastlint`

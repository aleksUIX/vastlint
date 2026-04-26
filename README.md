# vastlint

**Website & web validator:** [VAST tag validator](https://vastlint.org) Paste a VAST tag and get results in your browser, no install required.

A high-performance VAST XML validator built on a zero-dependency Rust core. Checks ad tags against the IAB Tech Lab VAST specification so you don't have to read it. Over $30 billion in annual CTV and video ad spend flows through VAST XML, and malformed tags are one of the most common causes of lost impressions, broken tracking, and revenue discrepancies between platforms. There is no widely adopted open-source tool that validates VAST XML against the full IAB specification across all published versions.

vastlint ships a native **MCP server** - making VAST validation available as a callable tool from Claude, Cursor, the [AAMP Buyer Agent SDK](https://github.com/IABTechLab/buyer-agent), or any MCP-compatible client. Connect to `vastlint.org/mcp` and call `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, or `fix_vast`. Each tool returns structured JSON with rule IDs, XPath locations, and spec references.

Native bindings for realtime ad pipelines: [`vastlint-go`](https://github.com/aleksUIX/vastlint-go) (CGo, prebuilt static libs — no Rust toolchain needed), [`vastlint-erlang`](https://github.com/aleksUIX/vastlint-erlang) (Elixir/Erlang NIF for high-throughput BEAM pipelines), and a WASM npm package for Node.js and browsers. All bindings share the same compiled Rust core — consistent results everywhere, sub-millisecond latency at scale.

[![crates.io](https://img.shields.io/crates/v/vastlint-cli.svg?label=crates.io)](https://crates.io/crates/vastlint-cli)
[![vastlint-core](https://img.shields.io/crates/v/vastlint-core.svg?label=vastlint-core)](https://crates.io/crates/vastlint-core)
[![npm](https://img.shields.io/npm/v/vastlint.svg?label=npm)](https://www.npmjs.com/package/vastlint)
[![go](https://img.shields.io/github/v/tag/aleksUIX/vastlint-go?label=go&color=00ADD8)](https://github.com/aleksUIX/vastlint-go)
[![license](https://img.shields.io/crates/l/vastlint-cli.svg)](LICENSE)

[![VS Code](https://img.shields.io/visual-studio-marketplace/v/aleksuix.vastlint?label=vs%20code&color=007ACC)](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint)
[![docs.rs](https://docs.rs/vastlint-core/badge.svg)](https://docs.rs/vastlint-core)
[![vastlint.org](https://img.shields.io/badge/vastlint.org-docs%20%26%20validator-blue)](https://vastlint.org)

[![SLSA 3](https://slsa.dev/images/gh-badge-level3.svg)](https://slsa.dev)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/aleksUIX/vastlint/badge)](https://scorecard.dev/viewer/?uri=github.com/aleksUIX/vastlint)
[![CII Best Practices](https://bestpractices.coreinfrastructure.org/projects/10788/badge)](https://bestpractices.coreinfrastructure.org/projects/10788)

Validates VAST documents against:

- [IAB Tech Lab VAST](https://iabtechlab.com/standards/vast/) 2.0, 3.0, 4.0, 4.1, 4.2, and 4.3. Structural rules derived from the published XSD schemas and spec prose
- [W3C XML 1.0](https://www.w3.org/TR/xml/) well-formedness (malformed documents are rejected before any spec rule runs)
- [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) URI syntax (all URL fields)
- [IANA Media Types](https://www.iana.org/assignments/media-types/) (MediaFile and resource MIME types)
- [ISO 4217](https://www.iso.org/iso-4217-currency-codes.html) currency codes (Pricing elements)
- [Ad-ID](https://www.ad-id.org/) registry format (UniversalAdId)
- [IAB Tech Lab SIMID](https://iabtechlab.com/simid/) 1.0, 1.0.1, 1.1, 1.2. Interactive creative validation for `<InteractiveCreativeFile apiFramework="SIMID">` and nonlinear `<IFrameResource>` - the IAB-sanctioned VPAID replacement

118 rules across required fields, schema validation, structural correctness, security, consistency, deprecated features, ambiguous usage, and value formats. Rules marked with `$` have direct revenue impact - use `vastlint check --fail-on-warning` in CI to catch them before they reach production. See [common errors](docs/common-errors.md) for the ones that cost real money. New to vastlint? Start with the [tutorial](docs/tutorial.md).

Full rule reference with examples and fix instructions: [VAST error rule reference](https://vastlint.org/docs/rules)

## Performance

Benchmarked on Apple M4 (10-core), production-realistic VAST tags (17–44 KB):

| Metric | 17 KB tag | 44 KB tag |
|---|---|---|
| Single-thread throughput | 2,747 tags/sec | 475 tags/sec |
| Single-thread latency | 363 µs | 2,104 µs |
| 10-core throughput | 15,760 tags/sec | 2,635 tags/sec |

A typical OpenRTB bid cycle takes 100–300 ms; validation adds less than 2.1% of that budget even on the heaviest tags. An SSAI pipeline doing 1,000 stitches/sec spends more time on DNS than on validating the VAST response.

No async runtime, no regex engine, no schema interpreter. Rules are compiled Rust functions. Three dependencies: `quick-xml`, `url`, and `phf` (compile-time hash maps).

## Install

```
cargo install vastlint
```

CLI crate on crates.io: [crates.io/crates/vastlint](https://crates.io/crates/vastlint)

Or download a pre-built binary from the [releases page](https://github.com/aleksUIX/vastlint/releases).

## Docker

Pull the image from Docker Hub:

```sh
docker pull aleksuix/vastlint
```

**Validate a file:**

```sh
docker run --rm -v "$(pwd)":/data aleksuix/vastlint check /data/tag.xml
```

**Pipe from stdin:**

```sh
cat tag.xml | docker run --rm -i aleksuix/vastlint check -
```
DOCKERHUB_TOKEN
**JSON output:**

```sh
docker run --rm -v "$(pwd)":/data aleksuix/vastlint check /data/tag.xml --format json
```

**Validate a whole directory:**

```sh
docker run --rm -v "$(pwd)/tags":/data aleksuix/vastlint check /data/*.xml
```

The image is built `FROM scratch` - a fully-static musl binary with no OS layer.
Compressed size is under 5 MB. Cold-start to first result is under 10 ms.

## Usage

```
# validate a file
vastlint check tag.xml

# validate multiple files
vastlint check *.xml

# read from stdin
cat tag.xml | vastlint check -

# JSON output (one object per file, newline-delimited)
vastlint check tag.xml --format json

# suppress colours
vastlint check tag.xml --no-color

# exit 0 even on errors (useful in some CI setups)
vastlint check tag.xml --no-fail

# opt in to anonymous usage telemetry (see Telemetry section below)
vastlint check tag.xml --telemetry

# list all rules with default severity
vastlint rules

# automatically fix common issues and overwrite the file
vastlint fix tag.xml

# fix and write to a new path instead of overwriting
vastlint fix tag.xml --out tag-fixed.xml

# preview what would change without writing anything
vastlint fix tag.xml --dry-run

# fix from stdin, repaired XML goes to stdout
cat tag.xml | vastlint fix -
```

Example output:

```
tag.xml  VAST 4.2
  error    <Duration> value does not match required format HH:MM:SS or HH:MM:SS.mmm  VAST-2.0-duration-format
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/Duration
  error    <MediaFile> delivery attribute must be "progressive" or "streaming"  VAST-2.0-mediafile-delivery-enum
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles/MediaFile[0][@delivery]
  info     <MediaFiles> has no <Mezzanine> - ad-stitching servers may reject this tag  VAST-4.1-mezzanine-recommended
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles

✖ 2 errors, 0 warnings, 1 info
```

## Auto-fix ⚠️ experimental

> **`vastlint fix` is opinionated and experimental.** It applies a small set of deterministic, low-risk repairs (HTTPS upgrades, `conditionalAd` removal). Always review the diff before committing. Use `--dry-run` first, and re-run `check` afterward to confirm the result. Future releases may make individual fixes configurable.

`vastlint fix` repairs fixable issues and writes the corrected XML back to the file (or to a separate path with `--out`):

```sh
# preview changes without writing (recommended first step)
vastlint fix tag.xml --dry-run

# overwrite the file in place
vastlint fix tag.xml

# write to a new file instead of overwriting
vastlint fix tag.xml --out tag-fixed.xml

# JSON report of what was fixed
vastlint fix tag.xml --format json

# pipe from stdin → repaired XML to stdout
cat tag.xml | vastlint fix -
```

Not every rule is auto-fixable - some require human judgment (e.g. choosing the right `<AdSystem>` value). After running `fix`, re-run `check` to confirm the remaining issues.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All files valid - no errors found |
| 1 | One or more files have validation errors |
| 2 | Usage error - unreadable file, bad config, or bad arguments |

## Config file

Create `vastlint.toml` anywhere in your project tree. vastlint searches up from the current directory and uses the first one it finds.

```toml
[rules]
"VAST-2.0-mediafile-https" = "off"
"VAST-4.1-vpaid-apiframework" = "warning"
```

Valid levels: `error`, `warning`, `info`, `off`.

Use `--config <path>` to specify a config file explicitly, or `--no-config` to ignore all config files.

## CI

```yaml
# .github/workflows/vast-lint.yml
- name: Install vastlint
  run: cargo install vastlint

- name: Validate VAST tags
  run: vastlint check tags/**/*.xml
```

Or download a release binary instead of building from source:

```yaml
- name: Install vastlint
  run: |
    curl -sL https://github.com/aleksUIX/vastlint/releases/latest/download/vastlint-x86_64-linux-musl.tar.gz \
      | tar xz -C /usr/local/bin

- name: Validate VAST tags
  run: vastlint check tags/**/*.xml
```

## JSON output

`--format json` emits one JSON object per file, one per line (NDJSON). This makes it easy to process output with `jq` or pipe it into other tools.

```json
{"file":"tag.xml","version":"4.2","valid":false,"summary":{"errors":1,"warnings":2,"infos":0},"issues":[{"id":"VAST-2.0-inline-adsystem","severity":"error","message":"<InLine> must contain <AdSystem>","path":"/VAST/Ad[0]/InLine","spec_ref":"IAB VAST 2.0 §2.2.1"}]}
```

Fields:

| Field | Type | Description |
|-------|------|-------------|
| `file` | string | Path as given on the command line, or `"-"` for stdin |
| `version` | string | Detected VAST version, or `"unknown"` |
| `valid` | bool | True when there are zero errors |
| `summary.errors` | number | Count of error-level issues |
| `summary.warnings` | number | Count of warning-level issues |
| `summary.infos` | number | Count of info-level issues |
| `issues[].id` | string | Rule ID (stable, use in config to override) |
| `issues[].severity` | string | `"error"`, `"warning"`, or `"info"` |
| `issues[].message` | string | Human-readable description |
| `issues[].path` | string | XPath-style location in the document |
| `issues[].spec_ref` | string | Section of the IAB VAST spec |

## Use as a library

[`vastlint-core`](https://crates.io/crates/vastlint-core) is published separately as a library crate. Full API documentation is on [docs.rs](https://docs.rs/vastlint-core).

```toml
[dependencies]
vastlint-core = "0.1"
```

```rust
use vastlint_core::validate;

let result = validate(xml_string);
if result.summary.is_valid() {
    println!("valid");
} else {
    for issue in &result.issues {
        println!("{}: {}", issue.id, issue.message);
    }
}
```

To override rule levels programmatically:

```rust
use std::collections::HashMap;
use vastlint_core::{validate_with_context, RuleLevel, ValidationContext};

let mut overrides = HashMap::new();
overrides.insert("VAST-2.0-mediafile-https", RuleLevel::Off);

let ctx = ValidationContext {
    rule_overrides: Some(overrides),
    ..Default::default()
};

let result = validate_with_context(xml_string, ctx);
```

## Use from JavaScript / TypeScript

[`vastlint`](https://www.npmjs.com/package/vastlint) is published on npm. Same 118 rules, same core - compiled to WASM.

```sh
npm install vastlint
```

```ts
import { validate } from 'vastlint';

const result = validate(xmlString);
if (!result.summary.valid) {
  for (const issue of result.issues) {
    console.error(`[${issue.severity}] ${issue.id}: ${issue.message}`);
  }
}
```

Works in Node.js (ESM and CJS), Vite, Webpack 5, and Rollup. Requires a bundler for browser use - see the [npm package README](npm/README.md) for the full environment compatibility table and API reference.

## Use from Go

[`vastlint-go`](https://github.com/aleksUIX/vastlint-go) provides Go bindings via CGo. Prebuilt static libraries are included - no Rust toolchain required.

```sh
go get github.com/aleksUIX/vastlint-go
```

```go
import vastlint "github.com/aleksUIX/vastlint-go"

result, err := vastlint.Validate(xmlString)
if err != nil {
    log.Fatal(err)
}
if !result.Valid {
    for _, issue := range result.Issues {
        fmt.Printf("[%s] %s (%s)\n", issue.Severity, issue.Message, issue.ID)
    }
}
```

Supported platforms: Linux (amd64, arm64), macOS (amd64, arm64).

With options:

```go
result, err := vastlint.ValidateWithOptions(xmlString, vastlint.Options{
    WrapperDepth:    2,
    MaxWrapperDepth: 5,
    RuleOverrides: map[string]string{
        "VAST-2.0-mediafile-https":       "error",
        "VAST-4.1-mezzanine-recommended": "off",
    },
})
```

See the [vastlint-go README](https://github.com/aleksUIX/vastlint-go) for the full API reference.

## Use from VS Code

Install the [vastlint extension](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint) from the VS Code Marketplace. VAST XML files are validated as you type - errors and warnings appear inline with rule IDs and spec references, no terminal required.

```
ext install aleksuix.vastlint
```

Or search for **vastlint** in the VS Code Extensions panel.

## Use from Chrome

The **VAST Lint** Chrome extension detects VAST XML on any page and shows inline validation errors, warnings, and info messages - squiggly underlines, hover tooltips, and a collapsible panel, all powered by the same vastlint core.

**Install from the Chrome Web Store** _(pending review)_:
[VAST Lint – Chrome Web Store](https://chrome.google.com/webstore/detail/chbbcgdpdpcmkocbmeljkfefeeknghnb)

**Or install manually** (no review wait):

1. Download `vastlint-extension.zip` from the [latest GitHub Release](https://github.com/aleksUIX/vastlint/releases/latest)
2. Unzip it anywhere
3. Open `chrome://extensions` and enable **Developer mode** (top-right toggle)
4. Click **Load unpacked** → select the unzipped folder
5. Navigate to any page serving VAST XML - the panel appears automatically

The toolbar icon badge shows the error count for the current tab. Click it for a per-severity summary.

## Use from an AI agent (MCP)

[`vastlint-mcp`](crates/vastlint-mcp) is a [Model Context Protocol](https://modelcontextprotocol.io) server. It exposes `validate_vast`, `validate_vast_url`, `list_rules`, `explain_rule`, and `fix_vast` as tools callable from Claude, Cursor, and any MCP-compatible client.

**In automated advertising pipelines** - as creative trafficking moves into agent-based systems (see [IAB Tech Lab AAMP](https://iabtechlab.com/standards/agentic-advertising-initiative/)), validation needs to happen at the same step. The vastlint MCP server is compatible with the [AAMP Buyer Agent SDK](https://github.com/IABTechLab/buyer-agent): an agent calls `validate_vast` or `validate_vast_url`, gets back rule IDs and XPath locations for any issues, and can reject or escalate the creative before trafficking. The same server works in Claude Desktop, Cursor, Copilot, any MCP client, and CI pipelines.

**No-install hosted endpoint** - connect directly without installing anything:

```json
{
  "mcpServers": {
    "vastlint": {
      "type": "sse",
      "url": "https://vastlint.org/mcp"
    }
  }
}
```

**Local install** (stdio transport):

```sh
cargo install vastlint-mcp
```

```json
{
  "mcpServers": {
    "vastlint": {
      "command": "vastlint-mcp"
    }
  }
}
```

Listed on the [MCP Registry](https://registry.modelcontextprotocol.io) as `io.github.aleksUIX/vastlint`. See [`crates/vastlint-mcp`](crates/vastlint-mcp/README.md) for the full tool reference and [docs/mcp-agentic.md](docs/mcp-agentic.md) for integration patterns, agentic loop examples, and how vastlint fits into the IAB Tech Lab AAMP / ARTF ecosystem.

## Use as a REST API

Available on [RapidAPI](https://rapidapi.com/aleksUIX/api/vastlint). Send a `POST /validate` request with your VAST XML and get a full validation result back - no SDK, no install.

```sh
curl -X POST https://vastlint.p.rapidapi.com/validate \
  -H "Content-Type: application/json" \
  -H "X-RapidAPI-Key: <your-key>" \
  -H "X-RapidAPI-Host: vastlint.p.rapidapi.com" \
  -d '{"xml":"<VAST version=\"4.2\">...</VAST>"}'
```

Returns the same structured result as the CLI and library: version, issues with rule IDs and line/col positions, and a summary. See the [RapidAPI listing](https://rapidapi.com/aleksUIX/api/vastlint) for full endpoint docs and pricing.

## Use from a browser

Paste any VAST tag into the web validator at **[VAST tag validator](https://vastlint.org/validate)** - no install, no account, nothing stored. Runs the same 118 rules as the CLI, entirely in your browser via WebAssembly.

## Telemetry

Off by default. CLI only -- the core library has no network code. Enable with `--telemetry` or `telemetry = true` in `vastlint.toml`.

Sends one HTTP GET per CLI invocation with: version, OS, anonymous install ID, file count. No file names, no file contents, no personal data. The install ID is a random 128-bit hex value stored in `~/.config/vastlint/id`. The ping fires in a background thread with a 2-second timeout and is silently dropped on any error.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for what's shipped, what's in progress, and what's next.

## Supply Chain Security

All release artifacts are built with **[SLSA Build Level 3](https://slsa.dev/spec/v1.0/levels#build-l3)** provenance. Provenance is generated by an isolated [slsa-github-generator](https://github.com/slsa-framework/slsa-github-generator) signing job that is independent of the build - the build process cannot tamper with what is signed.

Every binary, library, and WASM artifact attached to a GitHub Release includes a signed `.intoto.jsonl` provenance file. Verify any artifact:

```sh
# GitHub CLI
gh attestation verify vastlint-linux-x86_64.tar.gz --repo aleksUIX/vastlint

# slsa-verifier
slsa-verifier verify-artifact vastlint-linux-x86_64.tar.gz \
  --provenance-path multiple.intoto.jsonl \
  --source-uri github.com/aleksUIX/vastlint

# npm package
npm audit signatures vastlint
```

The [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/aleksUIX/vastlint) score is updated weekly.

## Fuzzing

vastlint uses [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer) to continuously test the validator and auto-fix engine against arbitrary inputs.

Three fuzz targets run on every CI push and pull request for 30 seconds each, seeded from the test fixture corpus:

| Target | What it covers |
|--------|---------------|
| `validate` | Core validator against arbitrary byte sequences |
| `fix` | Auto-fix pass against arbitrary byte sequences |
| `validate_wrapper` | Wrapper-chain depth logic (all depths 0–255) |

To run locally:

```sh
# Requires nightly Rust and cargo-fuzz
cargo +nightly fuzz run validate -- -max_total_time=60
cargo +nightly fuzz run fix -- -max_total_time=60
cargo +nightly fuzz run validate_wrapper -- -max_total_time=60
```


## Roadmap

VAST XML is not a standalone spec - it references several adjacent IAB Tech Lab standards that introduce validatable elements and attributes. Upcoming work:

| Upcoming | What it adds |
|---|---|
| **OMID** | `<AdVerifications><Verification>` - vendor format, HTTPS on JS/executable resources, duplicate detection |
| **VMAP 1.0** | Ad break schedule documents that embed VAST - `<AdBreak>`, `timeOffset`, `breakType`, pod rules |
| **DAAST 1.0** | Digital audio ad serving - structural sibling of VAST for audio-first creative types |
| **IAB Content Taxonomy** | `<Category authority="...">` - known authority URI validation |

See [ROADMAP.md](ROADMAP.md) for the full plan including infrastructure milestones.

## License

See [FREE_FOREVER.md](FREE_FOREVER.md) for the free-use commitment.

The CLI and library are licensed under [Apache 2.0](LICENSE). Use freely in any project, open-source or proprietary. The only requirement is to retain the [NOTICE](NOTICE) file (and the copyright header in the LICENSE) in any distribution - this provides attribution back to the project.

If you distribute vastlint or a derivative work, include the NOTICE file verbatim. That is the entire obligation.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Research

Sekowski, A. (2026). *VAST XML Validation at Bid-Time Scale: Latency Analysis and Integration Patterns for Programmatic Video Pipelines*. Preprint.
DOI: [10.13140/RG.2.2.11404.27520](https://doi.org/10.13140/RG.2.2.11404.27520)

## Community

Using vastlint in production or in your workflow? [Let us know!](https://github.com/aleksUIX/vastlint/discussions/1)

## Contact

For commercial inquiries, consulting, or enterprise support, open a [GitHub Discussion](https://github.com/aleksUIX/vastlint/discussions).

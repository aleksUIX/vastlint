# vastlint

[![crates.io](https://img.shields.io/crates/v/vastlint-cli.svg?label=crates.io)](https://crates.io/crates/vastlint-cli)
[![crates.io](https://img.shields.io/crates/v/vastlint-core.svg?label=vastlint-core)](https://crates.io/crates/vastlint-core)
[![docs.rs](https://docs.rs/vastlint-core/badge.svg)](https://docs.rs/vastlint-core)
[![npm](https://img.shields.io/npm/v/vastlint.svg?label=npm)](https://www.npmjs.com/package/vastlint)
[![Go](https://img.shields.io/github/v/tag/aleksUIX/vastlint-go?label=go&color=00ADD8)](https://github.com/aleksUIX/vastlint-go)
[![VS Code](https://img.shields.io/visual-studio-marketplace/v/aleksuix.vastlint?label=vs%20code&color=007ACC)](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint)
[![license](https://img.shields.io/crates/l/vastlint-cli.svg)](LICENSE)

A VAST XML validator. Checks ad tags against the IAB Tech Lab VAST specification so you don't have to read it. Over $30 billion in annual CTV and video ad spend flows through VAST XML, and malformed tags are one of the most common causes of lost impressions, broken tracking, and revenue discrepancies between platforms. There is no widely adopted open-source tool that validates VAST XML against the full IAB specification across all published versions.

Validates VAST documents against:

- [IAB Tech Lab VAST](https://iabtechlab.com/standards/vast/) 2.0, 3.0, 4.0, 4.1, 4.2, and 4.3 -- structural rules derived from the published XSD schemas and spec prose
- [W3C XML 1.0](https://www.w3.org/TR/xml/) well-formedness (malformed documents are rejected before any spec rule runs)
- [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986) URI syntax (all URL fields)
- [IANA Media Types](https://www.iana.org/assignments/media-types/) (MediaFile and resource MIME types)
- [ISO 4217](https://www.iso.org/iso-4217-currency-codes.html) currency codes (Pricing elements)
- [Ad-ID](https://www.ad-id.org/) registry format (UniversalAdId)

108 rules across required fields, schema validation, structural correctness, security, consistency, deprecated features, ambiguous usage, and value formats. See [common errors](docs/common-errors.md) for the ones that cost real money. New to vastlint? Start with the [tutorial](docs/tutorial.md).

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

The image is built `FROM scratch` — a fully-static musl binary with no OS layer.
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
```

Example output:

```
tag.xml  VAST 4.2
  error    <Duration> value does not match required format HH:MM:SS or HH:MM:SS.mmm  VAST-2.0-duration-format
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/Duration
  error    <MediaFile> delivery attribute must be "progressive" or "streaming"  VAST-2.0-mediafile-delivery-enum
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles/MediaFile[0][@delivery]
  info     <MediaFiles> has no <Mezzanine> — ad-stitching servers may reject this tag  VAST-4.1-mezzanine-recommended
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles

✖ 2 errors, 0 warnings, 1 info
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All files valid — no errors found |
| 1 | One or more files have validation errors |
| 2 | Usage error — unreadable file, bad config, or bad arguments |

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

[`vastlint`](https://www.npmjs.com/package/vastlint) is published on npm. Same 108 rules, same core — compiled to WASM.

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

Works in Node.js (ESM and CJS), Vite, Webpack 5, and Rollup. Requires a bundler for browser use — see the [npm package README](npm/README.md) for the full environment compatibility table and API reference.

## Use from Go

[`vastlint-go`](https://github.com/aleksUIX/vastlint-go) provides Go bindings via CGo. Prebuilt static libraries are included — no Rust toolchain required.

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

Install the [vastlint extension](https://marketplace.visualstudio.com/items?itemName=aleksuix.vastlint) from the VS Code Marketplace. VAST XML files are validated as you type — errors and warnings appear inline with rule IDs and spec references, no terminal required.

```
ext install aleksuix.vastlint
```

Or search for **vastlint** in the VS Code Extensions panel.

## Use as a REST API

Available on [RapidAPI](https://rapidapi.com/aleksUIX/api/vastlint). Send a `POST /validate` request with your VAST XML and get a full validation result back — no SDK, no install.

```sh
curl -X POST https://vastlint.p.rapidapi.com/validate \
  -H "Content-Type: application/json" \
  -H "X-RapidAPI-Key: <your-key>" \
  -H "X-RapidAPI-Host: vastlint.p.rapidapi.com" \
  -d '{"xml":"<VAST version=\"4.2\">...</VAST>"}'
```

Returns the same structured result as the CLI and library: version, issues with rule IDs and line/col positions, and a summary. See the [RapidAPI listing](https://rapidapi.com/aleksUIX/api/vastlint) for full endpoint docs and pricing.

## Performance

Benchmarked on Apple M4 (10-core), production-realistic VAST tags (17–44 KB):

| Metric | 17 KB tag | 44 KB tag |
|---|---|---|
| Single-thread throughput | 2,747 tags/sec | 475 tags/sec |
| Single-thread latency | 363 µs | 2,104 µs |
| 10-core throughput | 15,760 tags/sec | 2,635 tags/sec |

A typical OpenRTB bid cycle takes 100–300 ms; validation adds less than 2.1% of that budget even on the heaviest tags. An SSAI pipeline doing 1,000 stitches/sec spends more time on DNS than on validating the VAST response.

No async runtime, no regex engine, no schema interpreter. Rules are compiled Rust functions. Three dependencies: `quick-xml`, `url`, and `phf` (compile-time hash maps).

## Telemetry

Off by default. CLI only -- the core library has no network code. Enable with `--telemetry` or `telemetry = true` in `vastlint.toml`.

Sends one HTTP GET per CLI invocation with: version, OS, anonymous install ID, file count. No file names, no file contents, no personal data. The install ID is a random 128-bit hex value stored in `~/.config/vastlint/id`. The ping fires in a background thread with a 2-second timeout and is silently dropped on any error.

## License

See [FREE_FOREVER.md](FREE_FOREVER.md) for the free-use commitment.

The CLI and library are licensed under [Apache 2.0](LICENSE). Use freely in any project, open-source or proprietary. The only requirement is to retain the [NOTICE](NOTICE) file (and the copyright header in the LICENSE) in any distribution — this provides attribution back to the project.

If you distribute vastlint or a derivative work, include the NOTICE file verbatim. That is the entire obligation.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Community

Using vastlint in production or in your workflow? [Let us know!](https://github.com/aleksUIX/vastlint/discussions/1)

## Contact

For commercial inquiries, consulting, or enterprise support, open a [GitHub Discussion](https://github.com/aleksUIX/vastlint/discussions).

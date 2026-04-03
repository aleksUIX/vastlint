# vastlint

[![crates.io](https://img.shields.io/crates/v/vastlint.svg)](https://crates.io/crates/vastlint)
[![crates.io](https://img.shields.io/crates/v/vastlint-core.svg?label=vastlint-core)](https://crates.io/crates/vastlint-core)
[![docs.rs](https://docs.rs/vastlint-core/badge.svg)](https://docs.rs/vastlint-core)
[![License](https://img.shields.io/crates/l/vastlint.svg)](LICENSE)

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

## Performance

Under 100 microseconds per document. 14,000+ files/sec on a single core. A typical OpenRTB bid cycle takes 100-300ms; validation adds less than 0.1% of that budget. An SSAI pipeline doing 1,000 stitches/sec spends more time on DNS than on validating the VAST response.

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

## Contact

For commercial inquiries, consulting, or enterprise support, open a [GitHub Discussion](https://github.com/aleksUIX/vastlint/discussions).

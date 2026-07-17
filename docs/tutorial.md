# Getting started with VASTlint

This tutorial walks through validating a VAST XML tag, reading the output, fixing the issues, and integrating VASTlint into a CI pipeline.

## Install

With Rust installed:

```
cargo install vastlint
```

Or download a pre-built binary from [GitHub Releases](https://github.com/aleksUIX/vastlint/releases) and put it on your PATH.

## Your first lint

Save this as `tag.xml`. It's a VAST 4.2 inline ad with several common mistakes:

```xml
<VAST version="4.2">
  <Ad id="pre-roll-1">
    <InLine>
      <AdTitle>Summer Campaign 2026</AdTitle>
      <Impression>http://track.example.com/imp</Impression>
      <Creatives>
        <Creative>
          <Linear>
            <Duration>30</Duration>
            <MediaFiles>
              <MediaFile delivery="progressive" type="video/mp4"
                         width="1920" height="1080">
                http://cdn.example.com/summer.mp4
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>
```

Run vastlint:

```
vastlint check tag.xml
```

Output:

```
tag.xml  VAST 4.2
  error    <InLine> is missing required <AdSystem>  VAST-2.0-inline-adsystem
           /VAST/Ad[0]/InLine
  error    <Creative> is missing required <UniversalAdId> (required since VAST 4.0)  VAST-4.0-universaladid-present
           /VAST/Ad[0]/InLine/Creatives/Creative[0]
  error    <Duration> value does not match required format HH:MM:SS or HH:MM:SS.mmm  VAST-2.0-duration-format
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/Duration
  warning  <Impression> URL uses http: - most SDKs and players require https  VAST-2.0-tracking-https
           /VAST/Ad[0]/InLine/Impression
  warning  <MediaFile> URL uses http: - most SDKs and players require https  VAST-2.0-mediafile-https
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles/MediaFile[0]
  info     <MediaFiles> has no <Mezzanine> - ad-stitching servers may reject this tag in CTV/SSAI contexts  VAST-4.1-mezzanine-recommended
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles

✖ 3 errors, 2 warnings, 1 info
```

Three errors, two warnings, one info. The exit code is 1 (validation errors found).

## Reading the output

Each issue has:

- A severity: `error`, `warning`, or `info`
- A human-readable message describing the problem
- A rule ID like `VAST-2.0-duration-format` (stable, use this in config to suppress or promote)
- An XPath showing where in the document the issue is

Errors mean the tag violates a "must" or "required" rule in the spec. Most ad SDKs and players will reject or mishandle the tag. Warnings are "should" or "recommended" rules. Info is advisory.

## Auto-fix ⚠️ experimental

> **`vastlint fix` is opinionated and experimental.** It applies a small set of deterministic, low-risk repairs (HTTPS upgrades, `conditionalAd` removal). Always review the diff before committing. Future releases may make individual fixes configurable.

For issues that have a safe, unambiguous fix, VASTlint can repair the file for you - but always preview first:

```
vastlint fix tag.xml --dry-run
```

If the preview looks right, apply it:

```
vastlint fix tag.xml
```

This overwrites the file with the corrected XML and prints a report of what changed. Use `--out` to write to a separate path instead:

```
vastlint fix tag.xml --out tag-fixed.xml
```

Not every issue is auto-fixable - some require a human decision (for example, choosing the right `<AdSystem>` value). After running `fix`, re-run `check` to confirm the remaining issues and address any that need manual attention.

## Fixing the issues manually

Work through them top to bottom.

**Error: missing AdSystem.** Every InLine ad must declare which ad system generated it. Add `<AdSystem>` as a child of `<InLine>`:

```xml
<InLine>
  <AdSystem>MyAdServer</AdSystem>
  <AdTitle>Summer Campaign 2026</AdTitle>
  ...
```

**Error: missing UniversalAdId.** Required since VAST 4.0. Every `<Creative>` needs a `<UniversalAdId>` with an `idRegistry` attribute:

```xml
<Creative>
  <UniversalAdId idRegistry="ad-id.org">ABCD1234567</UniversalAdId>
  <Linear>
    ...
```

**Error: bad Duration format.** The spec requires `HH:MM:SS` or `HH:MM:SS.mmm`. The value `30` is not valid. Fix:

```xml
<Duration>00:00:30</Duration>
```

**Warning: HTTP URLs.** Most modern SDKs require HTTPS. Change `http://` to `https://` on the Impression pixel and the MediaFile URL.

**Info: no Mezzanine.** CTV ad-stitching servers (SSAI) prefer a `<Mezzanine>` element alongside regular MediaFiles. This is advisory -- your tag will still work, but some CTV pipelines may reject it.

After fixing, run again:

```
vastlint check tag.xml
```

```
tag.xml  VAST 4.2
  info     <MediaFiles> has no <Mezzanine> - ad-stitching servers may reject this tag in CTV/SSAI contexts  VAST-4.1-mezzanine-recommended
           /VAST/Ad[0]/InLine/Creatives/Creative[0]/Linear/MediaFiles

✓ 0 errors, 0 warnings, 1 info
```

Exit code 0. The remaining info is advisory and doesn't affect validity.

## Suppressing rules

If a rule doesn't apply to your use case, create `vastlint.toml` in your project root (`vastlint init` generates a starter file with every rule commented out at its default severity):

```toml
[rules]
"VAST-4.1-mezzanine-recommended" = "off"
"VAST-2.0-mediafile-https" = "info"
```

Valid levels: `error`, `warning`, `info`, `off`.

## JSON output

For programmatic consumption:

```
vastlint check tag.xml --format json
```

Outputs one JSON object per file (NDJSON). Pipe to `jq` for filtering:

```
vastlint check *.xml --format json | jq 'select(.valid == false)'
```

## CI integration

Add VASTlint to your CI pipeline so broken tags never ship.

GitHub Actions:

```yaml
# .github/workflows/vast-lint.yml
name: VAST validation
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install vastlint
        run: |
          curl -sL https://github.com/aleksUIX/vastlint/releases/latest/download/vastlint-x86_64-linux-musl.tar.gz \
            | tar xz -C /usr/local/bin
      - name: Validate VAST tags
        run: vastlint check tags/**/*.xml
```

The exit code is 1 when any file has errors, so the CI step fails automatically.

## Using as a Rust library

Add to your `Cargo.toml`:

```toml
[dependencies]
vastlint-core = "1"
```

```rust
use vastlint_core::validate;

fn main() {
    let xml = std::fs::read_to_string("tag.xml").unwrap();
    let result = validate(&xml);

    if result.summary.is_valid() {
        println!("tag is valid");
    } else {
        for issue in &result.issues {
            eprintln!("[{}] {}: {}", issue.severity.as_str(), issue.id, issue.message);
        }
        std::process::exit(1);
    }
}
```

## Listing all rules

```
vastlint rules
```

Prints all 195 rules with their default severity and a short description. Rules marked `$` have direct revenue impact. Use the rule IDs in your `vastlint.toml` to customize behavior.

## Next steps

- See [common errors](common-errors.md) for the VAST mistakes that cost real money.
- Check the [README](../README.md) for the full CLI reference.
- File issues at [github.com/aleksUIX/vastlint](https://github.com/aleksUIX/vastlint/issues).

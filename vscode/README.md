# vastlint - VAST XML Validator for VS Code

Inline linting for [IAB VAST](https://iabtechlab.com/standards/vast/) ad tags directly in VS Code.
Supports VAST 2.0 through 4.3. Web validator and full documentation: [vastlint.org](https://vastlint.org)

## Features

- **Inline squiggles** — red (error), yellow (warning), blue (info) underlines on the offending XML tag
- **Hover tooltips** — rule ID, what's wrong, fix guidance, spec reference, and XPath location
- **Problems panel** — all issues listed with file, line, and column; click to jump directly to the tag
- **Any file type** — works in `.xml`, `.html`, `.js`, `.ts`, `.json`, templates — anywhere `<VAST>` appears
- **Multi-block** — validates every `<VAST>...</VAST>` block in a file independently
- **Live as you type** — re-validates 500 ms after you stop typing, and on every save
- **CLI backend** — uses the `vastlint` CLI binary when available, falls back to WASM in-process
- **108 rules** across VAST 2.0–4.3: required fields, schema structure, URLs, deprecations, CTV/SSAI advisories

## How it looks

Hover over a squiggled tag:

```
🔴 vastlint `VAST-2.0-inline-adsystem`

<InLine> is missing required <AdSystem>

✅ Fix: Add `<AdSystem>` inside `<InLine>`, e.g. `<AdSystem>My Ad Server</AdSystem>`.

📖 IAB VAST 2.0 §2.3.1  -  `/VAST/Ad[0]/InLine`
```

## Settings

| Setting | Default | Description |
|---|---|---|
| `vastlint.enable` | `true` | Enable/disable diagnostics |
| `vastlint.minSeverity` | `"info"` | Minimum severity to show: `"error"`, `"warning"`, or `"info"` |
| `vastlint.ruleOverrides` | `{}` | Per-rule severity overrides (WASM fallback only — use `vastlint.toml` in CLI mode) |
| `vastlint.templateIgnoreRegex` | `""` | Regex for template expressions to blank out before validation (e.g. `\$\{[^}]+\}`) |
| `vastlint.vastVersion` | `""` | Force a VAST version (`"2.0"`, `"3.0"`, `"4.0"`, `"4.1"`, `"4.2"`, `"4.3"`) — blank = auto-detect |
| `vastlint.cliPath` | `"vastlint"` | Path to the `vastlint` CLI binary. Falls back to common install locations, then WASM. |

### Example: silence HTTP warnings

```json
// .vscode/settings.json
{
  "vastlint.ruleOverrides": {
    "VAST-2.0-mediafile-https": "off",
    "VAST-2.0-tracking-https": "off"
  }
}
```

### Example: only show errors

```json
{
  "vastlint.minSeverity": "error"
}
```

## Rules

vastlint checks 118 rules across:
- Required elements and attributes (VAST 2.0–4.3)
- Value formats (durations, URLs, enums)
- Schema conformance (unknown elements/attributes)
- Deprecation warnings ([VPAID](https://vastlint.org/guides/vast-vpaid-migration), Flash, Survey, conditionalAd)
- [SIMID](https://vastlint.org/docs/simid/) interactive creative validation (`<InteractiveCreativeFile apiFramework="SIMID">`)
- Security (HTTP vs HTTPS)
- CTV/SSAI best practices (Mezzanine, AdServingId)
- Structural issues ([wrapper depth](https://vastlint.org/guides/vast-wrapper-chains), ad sequence, duplicate impressions)

Full rule reference with examples and fix guidance: [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

<details>
<summary>All 118 rules</summary>

### VAST 2.0

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-2.0-root-element](https://vastlint.org/docs/rules/VAST-2.0-root-element/) | error | Root element must be `<VAST>` |
| [VAST-2.0-root-version](https://vastlint.org/docs/rules/VAST-2.0-root-version/) | error | `<VAST>` must have a `version` attribute |
| [VAST-2.0-root-version-value](https://vastlint.org/docs/rules/VAST-2.0-root-version-value/) | warning | `version` attribute must be a recognised version string |
| [VAST-2.0-root-has-ad-or-error](https://vastlint.org/docs/rules/VAST-2.0-root-has-ad-or-error/) | error | `<VAST>` must contain at least one `<Ad>` or `<Error>` |
| [VAST-2.0-ad-has-inline-or-wrapper](https://vastlint.org/docs/rules/VAST-2.0-ad-has-inline-or-wrapper/) | error | Each `<Ad>` must contain exactly one `<InLine>` or `<Wrapper>` |
| [VAST-2.0-inline-adsystem](https://vastlint.org/docs/rules/VAST-2.0-inline-adsystem/) | error | `<InLine>` must contain `<AdSystem>` |
| [VAST-2.0-inline-adtitle](https://vastlint.org/docs/rules/VAST-2.0-inline-adtitle/) | error | `<InLine>` must contain `<AdTitle>` |
| [VAST-2.0-inline-impression](https://vastlint.org/docs/rules/VAST-2.0-inline-impression/) | error | `<InLine>` must contain at least one `<Impression>` |
| [VAST-2.0-inline-creatives](https://vastlint.org/docs/rules/VAST-2.0-inline-creatives/) | error | `<InLine>` must contain `<Creatives>` with at least one `<Creative>` |
| [VAST-2.0-linear-duration](https://vastlint.org/docs/rules/VAST-2.0-linear-duration/) | error | `<Linear>` must contain `<Duration>` |
| [VAST-2.0-linear-mediafiles](https://vastlint.org/docs/rules/VAST-2.0-linear-mediafiles/) | error | `<Linear>` must contain `<MediaFiles>` with at least one `<MediaFile>` |
| [VAST-2.0-mediafile-delivery](https://vastlint.org/docs/rules/VAST-2.0-mediafile-delivery/) | error | `<MediaFile>` must have a `delivery` attribute |
| [VAST-2.0-mediafile-delivery-enum](https://vastlint.org/docs/rules/VAST-2.0-mediafile-delivery-enum/) | error | `delivery` must be `"progressive"` or `"streaming"` |
| [VAST-2.0-mediafile-type](https://vastlint.org/docs/rules/VAST-2.0-mediafile-type/) | error | `<MediaFile>` must have a `type` attribute |
| [VAST-2.0-mediafile-dimensions](https://vastlint.org/docs/rules/VAST-2.0-mediafile-dimensions/) | error | `<MediaFile>` must have `width` and `height` attributes |
| [VAST-2.0-mediafile-https](https://vastlint.org/docs/rules/VAST-2.0-mediafile-https/) | info | MediaFile URL uses HTTP instead of HTTPS |
| [VAST-2.0-wrapper-adsystem](https://vastlint.org/docs/rules/VAST-2.0-wrapper-adsystem/) | error | `<Wrapper>` must contain `<AdSystem>` |
| [VAST-2.0-wrapper-impression](https://vastlint.org/docs/rules/VAST-2.0-wrapper-impression/) | error | `<Wrapper>` must contain at least one `<Impression>` |
| [VAST-2.0-wrapper-vastadtaguri](https://vastlint.org/docs/rules/VAST-2.0-wrapper-vastadtaguri/) | error | `<Wrapper>` must contain `<VASTAdTagURI>` |
| [VAST-2.0-wrapper-depth](https://vastlint.org/docs/rules/VAST-2.0-wrapper-depth/) | error | Wrapper chain depth exceeds the configured maximum |
| [VAST-2.0-companion-resource](https://vastlint.org/docs/rules/VAST-2.0-companion-resource/) | error | `<Companion>` must contain at least one resource element |
| [VAST-2.0-companion-dimensions](https://vastlint.org/docs/rules/VAST-2.0-companion-dimensions/) | warning | `<Companion>` missing `width` or `height` |
| [VAST-2.0-nonlinear-resource](https://vastlint.org/docs/rules/VAST-2.0-nonlinear-resource/) | error | `<NonLinear>` must contain at least one resource element |
| [VAST-2.0-nonlinear-dimensions](https://vastlint.org/docs/rules/VAST-2.0-nonlinear-dimensions/) | warning | `<NonLinear>` missing `width` or `height` |
| [VAST-2.0-ad-sequence](https://vastlint.org/docs/rules/VAST-2.0-ad-sequence/) | warning | Inconsistent use of `sequence` attribute across `<Ad>` elements |
| [VAST-2.0-text-only-element](https://vastlint.org/docs/rules/VAST-2.0-text-only-element/) | error | Text-only element contains a child element |
| [VAST-2.0-unknown-attribute](https://vastlint.org/docs/rules/VAST-2.0-unknown-attribute/) | warning | Attribute not defined in the VAST spec |
| [VAST-2.0-inline-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-inline-unknown-child/) | error | `<InLine>` contains an unrecognised child element |
| [VAST-2.0-wrapper-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-wrapper-unknown-child/) | error | `<Wrapper>` contains an unrecognised child element |
| [VAST-2.0-creatives-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-creatives-unknown-child/) | error | `<Creatives>` may only contain `<Creative>` elements |
| [VAST-2.0-creative-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-creative-unknown-child/) | error | `<Creative>` contains an unrecognised child element |
| [VAST-2.0-linear-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-linear-unknown-child/) | error | `<Linear>` contains an unrecognised child element |
| [VAST-2.0-trackingevents-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-trackingevents-unknown-child/) | error | `<TrackingEvents>` may only contain `<Tracking>` elements |
| [VAST-2.0-mediafiles-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-mediafiles-unknown-child/) | error | `<MediaFiles>` contains an unrecognised child element |
| [VAST-2.0-extensions-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-extensions-unknown-child/) | error | `<Extensions>` may only contain `<Extension>` elements |
| [VAST-2.0-videoclicks-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-videoclicks-unknown-child/) | error | `<VideoClicks>` contains an unrecognised child element |
| [VAST-2.0-nonlinearads-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-nonlinearads-unknown-child/) | error | `<NonLinearAds>` contains an unrecognised child element |
| [VAST-2.0-nonlinear-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-nonlinear-unknown-child/) | error | `<NonLinear>` contains an unrecognised child element |
| [VAST-2.0-companionads-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-companionads-unknown-child/) | error | `<CompanionAds>` may only contain `<Companion>` elements |
| [VAST-2.0-companion-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-companion-unknown-child/) | error | `<Companion>` contains an unrecognised child element |
| [VAST-2.0-creativeextensions-unknown-child](https://vastlint.org/docs/rules/VAST-2.0-creativeextensions-unknown-child/) | error | `<CreativeExtensions>` may only contain `<CreativeExtension>` elements |
| [VAST-2.0-extension-misplaced-element](https://vastlint.org/docs/rules/VAST-2.0-extension-misplaced-element/) | warning | `<Extension>` contains an element that has a dedicated location in the VAST spec |
| [VAST-2.0-creative-extension-misplaced-element](https://vastlint.org/docs/rules/VAST-2.0-creative-extension-misplaced-element/) | warning | `<CreativeExtension>` contains an element with a dedicated location in the VAST spec |
| [VAST-2.0-tracking-https](https://vastlint.org/docs/rules/VAST-2.0-tracking-https/) | info | Tracking or click URL uses HTTP instead of HTTPS |
| [VAST-2.0-url-empty](https://vastlint.org/docs/rules/VAST-2.0-url-empty/) | error | URL field is empty |
| [VAST-2.0-url-invalid](https://vastlint.org/docs/rules/VAST-2.0-url-invalid/) | warning | URL field does not appear to be a valid URI |
| [VAST-2.0-parse-error](https://vastlint.org/docs/rules/VAST-2.0-parse-error/) | error | XML parse error — document may be malformed |
| [VAST-2.0-version-mismatch](https://vastlint.org/docs/rules/VAST-2.0-version-mismatch/) | warning | Declared version does not match structural signals |
| [VAST-2.0-duplicate-impression](https://vastlint.org/docs/rules/VAST-2.0-duplicate-impression/) | warning | Duplicate `<Impression>` URL within the same `<Ad>` |
| [VAST-2.0-flash-mediafile](https://vastlint.org/docs/rules/VAST-2.0-flash-mediafile/) | warning | Flash MediaFile type is no longer supported |
| [VAST-2.0-duration-format](https://vastlint.org/docs/rules/VAST-2.0-duration-format/) | error | Duration value does not match `HH:MM:SS[.mmm]` format |

### VAST 3.0

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-3.0-progress-offset](https://vastlint.org/docs/rules/VAST-3.0-progress-offset/) | error | `<Tracking event="progress">` requires an `offset` attribute |
| [VAST-3.0-progress-offset-format](https://vastlint.org/docs/rules/VAST-3.0-progress-offset-format/) | warning | Progress `offset` does not match the required format |
| [VAST-3.0-skipoffset-format](https://vastlint.org/docs/rules/VAST-3.0-skipoffset-format/) | warning | `skipoffset` does not match `HH:MM:SS[.mmm]` or `n%` format |
| [VAST-3.0-skip-event-no-skipoffset](https://vastlint.org/docs/rules/VAST-3.0-skip-event-no-skipoffset/) | warning | `skip` tracking event present but `<Linear>` has no `skipoffset` attribute |
| [VAST-3.0-minmaxbitrate-pair](https://vastlint.org/docs/rules/VAST-3.0-minmaxbitrate-pair/) | error | `<MediaFile>` must have both `minBitrate` and `maxBitrate` or neither |
| [VAST-3.0-bitrate-conflict](https://vastlint.org/docs/rules/VAST-3.0-bitrate-conflict/) | warning | `<MediaFile>` has both `bitrate` and `minBitrate`/`maxBitrate` |
| [VAST-3.0-icon-attrs](https://vastlint.org/docs/rules/VAST-3.0-icon-attrs/) | warning | `<Icon>` missing recommended attributes |
| [VAST-3.0-icon-program](https://vastlint.org/docs/rules/VAST-3.0-icon-program/) | error | `<Icon>` missing required `program` attribute |
| [VAST-3.0-icon-width](https://vastlint.org/docs/rules/VAST-3.0-icon-width/) | error | `<Icon>` missing required `width` attribute |
| [VAST-3.0-icon-height](https://vastlint.org/docs/rules/VAST-3.0-icon-height/) | error | `<Icon>` missing required `height` attribute |
| [VAST-3.0-icon-xposition](https://vastlint.org/docs/rules/VAST-3.0-icon-xposition/) | error | `<Icon>` missing required `xPosition` attribute |
| [VAST-3.0-icon-yposition](https://vastlint.org/docs/rules/VAST-3.0-icon-yposition/) | error | `<Icon>` missing required `yPosition` attribute |
| [VAST-3.0-icon-resource](https://vastlint.org/docs/rules/VAST-3.0-icon-resource/) | error | `<Icon>` must have at least one resource element |
| [VAST-3.0-icons-unknown-child](https://vastlint.org/docs/rules/VAST-3.0-icons-unknown-child/) | error | `<Icons>` may only contain `<Icon>` elements |
| [VAST-3.0-icon-unknown-child](https://vastlint.org/docs/rules/VAST-3.0-icon-unknown-child/) | error | `<Icon>` contains an unrecognised child element |
| [VAST-3.0-iconclicks-unknown-child](https://vastlint.org/docs/rules/VAST-3.0-iconclicks-unknown-child/) | error | `<IconClicks>` contains an unrecognised child element |
| [VAST-3.0-pricing-model](https://vastlint.org/docs/rules/VAST-3.0-pricing-model/) | error | `<Pricing>` missing required `model` attribute |
| [VAST-3.0-pricing-currency](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency/) | error | `<Pricing>` missing required `currency` attribute |
| [VAST-3.0-pricing-model-case](https://vastlint.org/docs/rules/VAST-3.0-pricing-model-case/) | warning | `model` value should be lowercase (`cpm`/`cpc`/`cpe`/`cpv`) |
| [VAST-3.0-pricing-currency-format](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency-format/) | warning | `currency` attribute must be a 3-letter ISO 4217 code |
| [VAST-3.0-companion-required-attr](https://vastlint.org/docs/rules/VAST-3.0-companion-required-attr/) | warning | `<CompanionAds>` `required` attribute must be `all`, `any`, or `none` |

### VAST 4.0

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.0-wrapper-root-error](https://vastlint.org/docs/rules/VAST-4.0-wrapper-root-error/) | warning | `<VAST>` root contains both `<Ad>` and `<Error>` elements |
| [VAST-4.0-universaladid-present](https://vastlint.org/docs/rules/VAST-4.0-universaladid-present/) | error | `<Creative>` must contain `<UniversalAdId>` (VAST 4.0+) |
| [VAST-4.0-universaladid-idregistry](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idregistry/) | error | `<UniversalAdId>` must have an `idRegistry` attribute |
| [VAST-4.0-universaladid-idvalue](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idvalue/) | error | `<UniversalAdId>` missing required `idValue` attribute (VAST 4.0) |
| [VAST-4.0-category-authority](https://vastlint.org/docs/rules/VAST-4.0-category-authority/) | error | `<Category>` missing required `authority` attribute |
| [VAST-4.0-companion-clicktracking-id](https://vastlint.org/docs/rules/VAST-4.0-companion-clicktracking-id/) | error | `<CompanionClickTracking>` missing required `id` attribute |
| [VAST-4.0-wrapper-clickthrough](https://vastlint.org/docs/rules/VAST-4.0-wrapper-clickthrough/) | warning | `<ClickThrough>` inside Wrapper `<VideoClicks>` was removed in VAST 4.0 |
| [VAST-4.0-conditionalad](https://vastlint.org/docs/rules/VAST-4.0-conditionalad/) | warning | `conditionalAd` attribute is deprecated as of VAST 4.1 |
| [VAST-4.0-tracking-event-removed](https://vastlint.org/docs/rules/VAST-4.0-tracking-event-removed/) | warning | Tracking events removed in VAST 4.0 |
| [VAST-4.0-mediafile-apiframework](https://vastlint.org/docs/rules/VAST-4.0-mediafile-apiframework/) | info | `apiFramework` on `<MediaFile>` is deprecated — use `<InteractiveCreativeFile>` |
| [VAST-4.0-interactive-creative-no-api](https://vastlint.org/docs/rules/VAST-4.0-interactive-creative-no-api/) | warning | `<InteractiveCreativeFile>` should have an `apiFramework` attribute |

### VAST 4.1

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.1-adservingid-present](https://vastlint.org/docs/rules/VAST-4.1-adservingid-present/) | error | `<InLine>` must contain `<AdServingId>` (VAST 4.1+) |
| [VAST-4.1-ad-serving-id-empty](https://vastlint.org/docs/rules/VAST-4.1-ad-serving-id-empty/) | warning | `<AdServingId>` is present but empty |
| [VAST-4.1-universaladid-idvalue-removed](https://vastlint.org/docs/rules/VAST-4.1-universaladid-idvalue-removed/) | warning | `idValue` attribute was removed in VAST 4.1 |
| [VAST-4.1-universaladid-content](https://vastlint.org/docs/rules/VAST-4.1-universaladid-content/) | error | `<UniversalAdId>` must have text content in VAST 4.1+ |
| [VAST-4.1-adtype-value](https://vastlint.org/docs/rules/VAST-4.1-adtype-value/) | warning | `adType` must be `video`, `audio`, or `hybrid` |
| [VAST-4.1-survey-deprecated](https://vastlint.org/docs/rules/VAST-4.1-survey-deprecated/) | warning | `<Survey>` is deprecated as of VAST 4.1 |
| [VAST-4.1-vpaid-apiframework](https://vastlint.org/docs/rules/VAST-4.1-vpaid-apiframework/) | warning | VPAID is deprecated as of VAST 4.1 |
| [VAST-4.1-vpaid-in-interactive-context](https://vastlint.org/docs/rules/VAST-4.1-vpaid-in-interactive-context/) | warning | VPAID `<MediaFile>` alongside `<InteractiveCreativeFile>` — unsupported in CTV |
| [VAST-4.1-interactive-creative-type](https://vastlint.org/docs/rules/VAST-4.1-interactive-creative-type/) | warning | `<InteractiveCreativeFile>` should have a `type` attribute |
| [VAST-4.1-mezzanine-delivery](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-delivery/) | error | `<Mezzanine>` missing required `delivery` attribute |
| [VAST-4.1-mezzanine-type](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-type/) | error | `<Mezzanine>` missing required `type` attribute |
| [VAST-4.1-mezzanine-width](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-width/) | error | `<Mezzanine>` missing required `width` attribute |
| [VAST-4.1-mezzanine-height](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-height/) | error | `<Mezzanine>` missing required `height` attribute |
| [VAST-4.1-mezzanine-recommended](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-recommended/) | info | No `<Mezzanine>` present — tag may be rejected in CTV/SSAI contexts |
| [VAST-4.1-verification-vendor](https://vastlint.org/docs/rules/VAST-4.1-verification-vendor/) | error | `<Verification>` missing required `vendor` attribute |
| [VAST-4.1-verification-no-resource](https://vastlint.org/docs/rules/VAST-4.1-verification-no-resource/) | warning | `<Verification>` should have `<JavaScriptResource>` or `<ExecutableResource>` |
| [VAST-4.1-js-resource-apiframework](https://vastlint.org/docs/rules/VAST-4.1-js-resource-apiframework/) | error | `<JavaScriptResource>` missing required `apiFramework` attribute |
| [VAST-4.1-exec-resource-apiframework](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-apiframework/) | error | `<ExecutableResource>` missing required `apiFramework` attribute |
| [VAST-4.1-exec-resource-type](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-type/) | error | `<ExecutableResource>` missing required `type` attribute |
| [VAST-4.1-blockedadcategories-no-authority](https://vastlint.org/docs/rules/VAST-4.1-blockedadcategories-no-authority/) | warning | `<BlockedAdCategories>` should have an `authority` attribute |
| [VAST-4.1-tracking-event-value](https://vastlint.org/docs/rules/VAST-4.1-tracking-event-value/) | error | `event` attribute not in the valid set for this VAST version |
| [VAST-4.1-companion-renderingmode-value](https://vastlint.org/docs/rules/VAST-4.1-companion-renderingmode-value/) | warning | `renderingMode` must be `default`, `end-card`, or `concurrent` |

### VAST 4.2

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.2-closedcaptionfiles-unknown-child](https://vastlint.org/docs/rules/VAST-4.2-closedcaptionfiles-unknown-child/) | error | `<ClosedCaptionFiles>` may only contain `<ClosedCaptionFile>` elements |
| [VAST-4.2-icon-fallback-image-width-height](https://vastlint.org/docs/rules/VAST-4.2-icon-fallback-image-width-height/) | warning | `<IconClickFallbackImage>` should have `width` and `height` attributes |

### VAST 4.3

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.3-js-resource-browser-optional](https://vastlint.org/docs/rules/VAST-4.3-js-resource-browser-optional/) | warning | `<JavaScriptResource>` should have a `browserOptional` attribute |

</details>

## Requirements

No external tools needed - the validator runs entirely in-process via WebAssembly.

## License

Apache-2.0 - see [LICENSE](../LICENSE)

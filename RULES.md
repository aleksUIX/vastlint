# VAST Validation Rules

Full reference documentation for every rule is at **[vastlint.org/docs/rules](https://vastlint.org/docs/rules/)**.

222 rules across IAB VAST 2.0 – 4.4, VMAP 1.0, DAAST 1.0, and SIMID 1.0 – 1.1. Each rule has a stable ID, a default severity (`error` / `warning` / `info`), and a dedicated docs page with the spec reference, examples, and fix guidance.

The severity below is the default. A few rules resolve theirs from the document's declared version, because the schemas disagree about what is required: the `<Icon>` placement attributes are errors under VAST 3.0, which required them, and warnings under 4.x, whose XSDs declare them optional. `VAST-4.1-verification-vendor` and `VAST-4.1-js-resource-apiframework` behave the same way across 4.0 and 4.1. Config overrides always win over both.

---

## VAST 2.0 rules

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
| [VAST-2.0-mediafile-https](https://vastlint.org/docs/rules/VAST-2.0-mediafile-https/) | warning | MediaFile URL uses HTTP instead of HTTPS |
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
| [VAST-2.0-extension-cdata](https://vastlint.org/docs/rules/VAST-2.0-extension-cdata/) | warning | `<Extension>` leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping |
| [VAST-2.0-creative-extension-misplaced-element](https://vastlint.org/docs/rules/VAST-2.0-creative-extension-misplaced-element/) | warning | `<CreativeExtension>` contains an element that has a dedicated location in the VAST spec |
| [VAST-2.0-creative-extension-cdata](https://vastlint.org/docs/rules/VAST-2.0-creative-extension-cdata/) | warning | `<CreativeExtension>` leaf text payload with XML-sensitive characters should be wrapped in CDATA so JSON blobs and URL-rich vendor data do not rely on fragile XML escaping |
| [VAST-2.0-tracking-https](https://vastlint.org/docs/rules/VAST-2.0-tracking-https/) | warning | Tracking or click URL uses HTTP instead of HTTPS |
| [VAST-2.0-url-cdata](https://vastlint.org/docs/rules/VAST-2.0-url-cdata/) | warning | URI value is not wrapped in CDATA |
| [VAST-2.0-url-empty](https://vastlint.org/docs/rules/VAST-2.0-url-empty/) | error | URL field is empty |
| [VAST-2.0-url-invalid](https://vastlint.org/docs/rules/VAST-2.0-url-invalid/) | warning | URL field does not appear to be a valid URI |
| [VAST-2.0-parse-error](https://vastlint.org/docs/rules/VAST-2.0-parse-error/) | error | XML parse error — document may be malformed |
| [VAST-2.0-version-mismatch](https://vastlint.org/docs/rules/VAST-2.0-version-mismatch/) | warning | Declared version does not match structural signals |
| [VAST-2.0-duplicate-impression](https://vastlint.org/docs/rules/VAST-2.0-duplicate-impression/) | warning | Duplicate `<Impression>` URL within the same `<Ad>` |
| [VAST-2.0-flash-mediafile](https://vastlint.org/docs/rules/VAST-2.0-flash-mediafile/) | warning | Flash MediaFile type is no longer supported |
| [VAST-2.0-linear-tracking-quartiles](https://vastlint.org/docs/rules/VAST-2.0-linear-tracking-quartiles/) | warning | `<Linear>` has no standard quartile tracking events — measurement system receives no signal |
| [VAST-2.0-duration-format](https://vastlint.org/docs/rules/VAST-2.0-duration-format/) | error | Duration value does not match `HH:MM:SS[.mmm]` format |
| [VAST-2.0-macro-unknown](https://vastlint.org/docs/rules/VAST-2.0-macro-unknown/) | warning | URL contains a `[MACRO]` that is not a recognised IAB VAST macro |
| [VAST-2.0-macro-lowercase](https://vastlint.org/docs/rules/VAST-2.0-macro-lowercase/) | warning | Recognised macro is not uppercase — players match macro names case-sensitively |
| [VAST-2.0-macro-wrong-context](https://vastlint.org/docs/rules/VAST-2.0-macro-wrong-context/) | info | Context-restricted macro (`[ERRORCODE]`/`[REASON]`) used where it has no defined value |
| [VAST-2.0-macro-uri-unencoded](https://vastlint.org/docs/rules/VAST-2.0-macro-uri-unencoded/) | warning | Macro-bearing URL contains characters that must be percent-encoded per RFC 3986 |
| [VAST-2.0-adtitle-quality](https://vastlint.org/docs/rules/VAST-2.0-adtitle-quality/) | warning | `<AdTitle>` value is a known placeholder string; the creative cannot be identified in reporting |
| [VAST-2.0-adsystem-quality](https://vastlint.org/docs/rules/VAST-2.0-adsystem-quality/) | info | `<AdSystem>` value is a known placeholder string; the tag cannot be traced to its serving system |
| [VAST-2.0-adsystem-no-version](https://vastlint.org/docs/rules/VAST-2.0-adsystem-no-version/) | info | `<AdSystem>` has no `version` attribute; provenance is harder to trace in partner debugging |

---

## VAST 3.0 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-3.0-progress-offset](https://vastlint.org/docs/rules/VAST-3.0-progress-offset/) | error | `<Tracking event="progress">` requires an `offset` attribute |
| [VAST-3.0-progress-offset-format](https://vastlint.org/docs/rules/VAST-3.0-progress-offset-format/) | warning | Progress `offset` does not match the required format |
| [VAST-3.0-skipoffset-format](https://vastlint.org/docs/rules/VAST-3.0-skipoffset-format/) | warning | `skipoffset` does not match `HH:MM:SS[.mmm]` or `n%` format |
| [VAST-3.0-skip-event-no-skipoffset](https://vastlint.org/docs/rules/VAST-3.0-skip-event-no-skipoffset/) | warning | `skip` tracking event present but `<Linear>` has no `skipoffset` attribute |
| [VAST-3.0-minmaxbitrate-pair](https://vastlint.org/docs/rules/VAST-3.0-minmaxbitrate-pair/) | error | `<MediaFile>` must have both `minBitrate` and `maxBitrate` or neither |
| [VAST-3.0-bitrate-conflict](https://vastlint.org/docs/rules/VAST-3.0-bitrate-conflict/) | warning | `<MediaFile>` has both `bitrate` and `minBitrate`/`maxBitrate` |
| [VAST-3.0-icon-attrs](https://vastlint.org/docs/rules/VAST-3.0-icon-attrs/) | warning | `<Icon>` missing recommended attributes (`program`/`width`/`height`/position) |
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

---

## VAST 4.0 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.0-wrapper-root-error](https://vastlint.org/docs/rules/VAST-4.0-wrapper-root-error/) | warning | `<VAST>` root contains both `<Ad>` and `<Error>` elements |
| [VAST-4.0-universaladid-present](https://vastlint.org/docs/rules/VAST-4.0-universaladid-present/) | error | `<Creative>` must contain `<UniversalAdId>` (VAST 4.0+) |
| [VAST-4.0-universaladid-idregistry](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idregistry/) | error | `<UniversalAdId>` must have an `idRegistry` attribute |
| [VAST-4.0-universaladid-idvalue](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idvalue/) | error | `<UniversalAdId>` missing required `idValue` attribute (VAST 4.0) |
| [VAST-4.0-category-authority](https://vastlint.org/docs/rules/VAST-4.0-category-authority/) | error | `<Category>` missing required `authority` attribute |
| [VAST-4.0-category-authority-not-uri](https://vastlint.org/docs/rules/VAST-4.0-category-authority-not-uri/) | warning | `<Category>` `authority` attribute is not a valid authority URL |
| [VAST-4.0-category-authority-unknown](https://vastlint.org/docs/rules/VAST-4.0-category-authority-unknown/) | info | `<Category>` `authority` is not a recognised IAB Content Taxonomy authority |
| [VAST-4.0-companion-clicktracking-id](https://vastlint.org/docs/rules/VAST-4.0-companion-clicktracking-id/) | error | `<CompanionClickTracking>` missing required `id` attribute |
| [VAST-4.0-wrapper-clickthrough](https://vastlint.org/docs/rules/VAST-4.0-wrapper-clickthrough/) | warning | `<ClickThrough>` inside Wrapper `<VideoClicks>` was removed in VAST 4.0 |
| [VAST-4.0-conditionalad](https://vastlint.org/docs/rules/VAST-4.0-conditionalad/) | warning | `conditionalAd` attribute is deprecated as of VAST 4.1 |
| [VAST-4.0-tracking-event-removed](https://vastlint.org/docs/rules/VAST-4.0-tracking-event-removed/) | warning | Tracking events removed in VAST 4.0 |
| [VAST-4.0-mediafile-apiframework](https://vastlint.org/docs/rules/VAST-4.0-mediafile-apiframework/) | info | `apiFramework` on `<MediaFile>` is deprecated — use `<InteractiveCreativeFile>` |
| [VAST-4.0-interactive-creative-no-api](https://vastlint.org/docs/rules/VAST-4.0-interactive-creative-no-api/) | warning | `<InteractiveCreativeFile>` should have an `apiFramework` attribute |

---

## VAST 4.1 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.1-adservingid-present](https://vastlint.org/docs/rules/VAST-4.1-adservingid-present/) | error | `<InLine>` must contain `<AdServingId>` (VAST 4.1+) |
| [VAST-4.1-ad-serving-id-empty](https://vastlint.org/docs/rules/VAST-4.1-ad-serving-id-empty/) | warning | `<AdServingId>` is present but empty |
| [VAST-4.1-universaladid-idvalue-removed](https://vastlint.org/docs/rules/VAST-4.1-universaladid-idvalue-removed/) | warning | `idValue` attribute was removed in VAST 4.1 |
| [VAST-4.1-universaladid-content](https://vastlint.org/docs/rules/VAST-4.1-universaladid-content/) | error | `<UniversalAdId>` must have text content in VAST 4.1+ |
| [VAST-4.1-adtype-value](https://vastlint.org/docs/rules/VAST-4.1-adtype-value/) | warning | `adType` must be `video`, `audio`, or `hybrid` |
| [VAST-4.1-survey-deprecated](https://vastlint.org/docs/rules/VAST-4.1-survey-deprecated/) | warning | `<Survey>` is deprecated as of VAST 4.1 |
| [VAST-4.1-macro-deprecated](https://vastlint.org/docs/rules/VAST-4.1-macro-deprecated/) | info | `[CONTENTPLAYHEAD]`/`[MEDIAPLAYHEAD]` are deprecated as of VAST 4.1 — use `[ADPLAYHEAD]` |
| [VAST-4.1-vpaid-apiframework](https://vastlint.org/docs/rules/VAST-4.1-vpaid-apiframework/) | warning | VPAID is deprecated as of VAST 4.1 |
| [VAST-4.1-vpaid-in-interactive-context](https://vastlint.org/docs/rules/VAST-4.1-vpaid-in-interactive-context/) | warning | VPAID `<MediaFile>` alongside `<InteractiveCreativeFile>` — unsupported in CTV |
| [VAST-4.1-interactive-creative-type](https://vastlint.org/docs/rules/VAST-4.1-interactive-creative-type/) | warning | `<InteractiveCreativeFile>` should have a `type` attribute |
| [VAST-4.1-mezzanine-delivery](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-delivery/) | error | `<Mezzanine>` missing required `delivery` attribute |
| [VAST-4.1-mezzanine-type](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-type/) | error | `<Mezzanine>` missing required `type` attribute |
| [VAST-4.1-mezzanine-width](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-width/) | error | `<Mezzanine>` missing required `width` attribute |
| [VAST-4.1-mezzanine-height](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-height/) | error | `<Mezzanine>` missing required `height` attribute |
| [VAST-4.1-mezzanine-recommended](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-recommended/) | info | No `<Mezzanine>` present — tag may be rejected in CTV/SSAI contexts |
| [VAST-4.1-verification-vendor](https://vastlint.org/docs/rules/VAST-4.1-verification-vendor/) | error | `<Verification>` missing required `vendor` attribute |
| [VAST-4.1-verification-vendor-format](https://vastlint.org/docs/rules/VAST-4.1-verification-vendor-format/) | warning | `<Verification>` vendor should use a domain-qualified identifier such as `company.com-omid` |
| [VAST-4.1-verification-duplicate-vendor](https://vastlint.org/docs/rules/VAST-4.1-verification-duplicate-vendor/) | warning | `<AdVerifications>` contains duplicate vendor identifiers |
| [VAST-4.1-verification-no-resource](https://vastlint.org/docs/rules/VAST-4.1-verification-no-resource/) | warning | `<Verification>` should have `<JavaScriptResource>` or `<ExecutableResource>` |
| [VAST-4.1-verification-parameters](https://vastlint.org/docs/rules/VAST-4.1-verification-parameters/) | warning | OMID `<Verification>` should include non-empty `<VerificationParameters>` |
| [VAST-4.1-verification-tracking-reason](https://vastlint.org/docs/rules/VAST-4.1-verification-tracking-reason/) | warning | `verificationNotExecuted` tracking URI should include the `[REASON]` macro |
| [VAST-4.1-js-resource-apiframework](https://vastlint.org/docs/rules/VAST-4.1-js-resource-apiframework/) | error | `<JavaScriptResource>` missing required `apiFramework` attribute |
| [VAST-4.1-js-resource-apiframework-value](https://vastlint.org/docs/rules/VAST-4.1-js-resource-apiframework-value/) | warning | OMID `<JavaScriptResource>` should declare `apiFramework="omid"` |
| [VAST-4.1-js-resource-https](https://vastlint.org/docs/rules/VAST-4.1-js-resource-https/) | warning | OMID `<JavaScriptResource>` URL should use HTTPS |
| [VAST-4.1-exec-resource-apiframework](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-apiframework/) | error | `<ExecutableResource>` missing required `apiFramework` attribute |
| [VAST-4.1-exec-resource-apiframework-value](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-apiframework-value/) | warning | OMID `<ExecutableResource>` should declare `apiFramework="omid"` |
| [VAST-4.1-exec-resource-type](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-type/) | error | `<ExecutableResource>` missing required `type` attribute |
| [VAST-4.1-exec-resource-https](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-https/) | warning | OMID `<ExecutableResource>` reference should use HTTPS when it is a URL |
| [VAST-4.1-blockedadcategories-no-authority](https://vastlint.org/docs/rules/VAST-4.1-blockedadcategories-no-authority/) | warning | `<BlockedAdCategories>` should have an `authority` attribute |
| [VAST-4.1-blockedadcategories-authority-not-uri](https://vastlint.org/docs/rules/VAST-4.1-blockedadcategories-authority-not-uri/) | warning | `<BlockedAdCategories>` `authority` attribute is not a valid authority URL |
| [VAST-4.1-blockedadcategories-authority-unknown](https://vastlint.org/docs/rules/VAST-4.1-blockedadcategories-authority-unknown/) | info | `<BlockedAdCategories>` `authority` is not a recognised IAB Content Taxonomy authority |
| [VAST-4.1-tracking-event-value](https://vastlint.org/docs/rules/VAST-4.1-tracking-event-value/) | error | `event` attribute not in the valid set for this VAST version |
| [VAST-4.1-companion-renderingmode-value](https://vastlint.org/docs/rules/VAST-4.1-companion-renderingmode-value/) | warning | `renderingMode` must be `default`, `end-card`, or `concurrent` |

---

## VAST 4.2 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.2-closedcaptionfiles-unknown-child](https://vastlint.org/docs/rules/VAST-4.2-closedcaptionfiles-unknown-child/) | error | `<ClosedCaptionFiles>` may only contain `<ClosedCaptionFile>` elements |
| [VAST-4.2-icon-fallback-image-width-height](https://vastlint.org/docs/rules/VAST-4.2-icon-fallback-image-width-height/) | warning | `<IconClickFallbackImage>` should have `width` and `height` attributes |

---

## VAST 4.3 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.3-js-resource-browser-optional](https://vastlint.org/docs/rules/VAST-4.3-js-resource-browser-optional/) | warning | `<JavaScriptResource>` should have a `browserOptional` attribute |

---

## VAST 4.4 rules (CTV Ad Portfolio)

VAST 4.4 is a working-group draft, not a published spec. The CTV Ad Portfolio signaling guidance it accompanies **is** final (2026-07-22), so rules derived from the guidance carry more weight than rules derived from the draft schema. Nothing here errors unless the markup is malformed under any version.

The NonLinear content model these rules cover is accepted on any VAST 4.x document, not only on `version="4.4"`, because every example in IAB's final guidance ships it on a 4.2 tag. The AdCOM signal rules also run on VAST 2.0, where the same five formats are delivered through `<Extension type="ctv_ad_portfolio">`.

The two `-format-mismatch` rules read the signals as a set. `plcmt`, `pos` and `playbackmethod` can each be individually valid and still describe two different formats at once, which the per-value rules cannot see.

| Rule | Severity | Description |
|------|----------|-------------|
| [VAST-4.4-version-attribute](https://vastlint.org/docs/rules/VAST-4.4-version-attribute/) | info | Document declares VAST 4.4, a working-group draft rather than a published spec |
| [VAST-4.4-nonlinear-no-renderable-asset](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-no-renderable-asset/) | warning | `<NonLinear>` has a SIMID interactive file but no renderable fallback asset |
| [VAST-4.4-nonlinear-mediafiles-empty](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-mediafiles-empty/) | error | `<NonLinear>` `<MediaFiles>` contains no renderable or interactive asset |
| [VAST-4.4-nonlinear-simid-iframe](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-simid-iframe/) | info | `<IFrameResource apiFramework="SIMID">` is superseded by `<InteractiveCreativeFile>` in NonLinear |
| [VAST-4.4-nonlinear-video-no-duration](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-video-no-duration/) | warning | `<NonLinear>` delivers video but has no `<Duration>`: quartile tracking cannot fire |
| [VAST-4.4-adcom-extension-unknown-signal](https://vastlint.org/docs/rules/VAST-4.4-adcom-extension-unknown-signal/) | warning | `<Extension ext="adcom">` type is not `plcmt`, `pos`, `playbackmethod` or `attr` |
| [VAST-4.4-adcom-extension-type-mismatch](https://vastlint.org/docs/rules/VAST-4.4-adcom-extension-type-mismatch/) | warning | `<Extension>` type attribute and AdCOM payload element name disagree |
| [VAST-4.4-adcom-signal-not-integer](https://vastlint.org/docs/rules/VAST-4.4-adcom-signal-not-integer/) | error | AdCOM signal payload in `<Extension>` is not an integer |
| [VAST-4.4-adcom-plcmt-value](https://vastlint.org/docs/rules/VAST-4.4-adcom-plcmt-value/) | warning | AdCOM `plcmt` outside the Plcmt Subtypes (Video) range 1–9 |
| [VAST-4.4-adcom-playbackmethod-value](https://vastlint.org/docs/rules/VAST-4.4-adcom-playbackmethod-value/) | warning | AdCOM `playbackmethod` outside the Playback Methods range 1–11 |
| [VAST-4.4-adcom-pos-value](https://vastlint.org/docs/rules/VAST-4.4-adcom-pos-value/) | warning | AdCOM `pos` outside the Placement Positions range 0–17 |
| [VAST-4.4-adcom-pos-format-mismatch](https://vastlint.org/docs/rules/VAST-4.4-adcom-pos-format-mismatch/) | warning | AdCOM `pos` is not a position the declared `plcmt` format supports |
| [VAST-4.4-adcom-playbackmethod-format-mismatch](https://vastlint.org/docs/rules/VAST-4.4-adcom-playbackmethod-format-mismatch/) | warning | AdCOM `playbackmethod` and `plcmt` name different CTV Ad Portfolio formats |
| [VAST-4.4-adcom-attr-not-motion](https://vastlint.org/docs/rules/VAST-4.4-adcom-attr-not-motion/) | info | AdCOM `attr` is not a CTV Ad Portfolio creative attribute (19 QR, 20 alpha, 21–23 motion) |
| [VAST-4.4-qrcode-position-attrs](https://vastlint.org/docs/rules/VAST-4.4-qrcode-position-attrs/) | error | `<QrCodePosition>` requires both `xPosition` and `yPosition` |
| [VAST-4.4-qrcode-position-percent](https://vastlint.org/docs/rules/VAST-4.4-qrcode-position-percent/) | error | `<QrCodePosition>` coordinates must be percentages, not pixels |
| [VAST-4.4-qrcode-size-attr](https://vastlint.org/docs/rules/VAST-4.4-qrcode-size-attr/) | error | `<QrCodeSize>` requires a `size` attribute |
| [VAST-4.4-qrcode-size-percent](https://vastlint.org/docs/rules/VAST-4.4-qrcode-size-percent/) | error | `<QrCodeSize>` `size` must be a percentage |
| [VAST-4.4-qrcode-missing-scan-url](https://vastlint.org/docs/rules/VAST-4.4-qrcode-missing-scan-url/) | warning | QR code geometry declared without a `<QrCodeScanUrl>` destination |
| [VAST-4.4-qrcode-missing-image-url](https://vastlint.org/docs/rules/VAST-4.4-qrcode-missing-image-url/) | warning | QR code geometry declared without a `<QrCodeImageUrl>` image |
| [VAST-3.0-adparameters-xmlencoded-value](https://vastlint.org/docs/rules/VAST-3.0-adparameters-xmlencoded-value/) | warning | `<AdParameters>` xmlEncoded must be a boolean |
| [VAST-2.0-ctv-portfolio-creative-id-required](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-creative-id-required/) | error | `<Extension type="ctv_ad_portfolio">` needs `<CreativeId>` when the ad has multiple creatives |
| [VAST-2.0-ctv-portfolio-creative-id-unmatched](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-creative-id-unmatched/) | error | `<Extension type="ctv_ad_portfolio">` `<CreativeId>` matches no `<Creative>` id in the ad |
| [VAST-2.0-ctv-portfolio-mediafiles-required](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-mediafiles-required/) | error | `<Extension type="ctv_ad_portfolio">` carries no `<MediaFiles>` |
| [VAST-2.0-ctv-portfolio-mediafiles-empty](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-mediafiles-empty/) | error | `<Extension type="ctv_ad_portfolio">` `<MediaFiles>` contains no renderable or interactive asset |
| [VAST-2.0-ctv-portfolio-no-renderable-asset](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-no-renderable-asset/) | warning | `<Extension type="ctv_ad_portfolio">` has a SIMID file but no `<MediaFile>` fallback |
| [VAST-2.0-ctv-portfolio-no-duration](https://vastlint.org/docs/rules/VAST-2.0-ctv-portfolio-no-duration/) | warning | `<Extension type="ctv_ad_portfolio">` delivers a timed asset but has no `<Duration>` |

---

## SIMID rules

| Rule | Severity | Description |
|------|----------|-------------|
| [SIMID-1.0-simid-type-required](https://vastlint.org/docs/rules/SIMID-1.0-simid-type-required/) | error | `<InteractiveCreativeFile apiFramework="SIMID">` must have `type="text/html"` |
| [SIMID-1.0-simid-url-empty](https://vastlint.org/docs/rules/SIMID-1.0-simid-url-empty/) | error | `<InteractiveCreativeFile apiFramework="SIMID">` must contain a non-empty URL |
| [SIMID-1.0-simid-url-https](https://vastlint.org/docs/rules/SIMID-1.0-simid-url-https/) | error | `<InteractiveCreativeFile apiFramework="SIMID">` URL must use HTTPS |
| [SIMID-1.0-simid-variable-duration-value](https://vastlint.org/docs/rules/SIMID-1.0-simid-variable-duration-value/) | warning | `<InteractiveCreativeFile>` `variableDuration` attribute must be `"true"` when present |
| [SIMID-1.0-simid-mediafile-required](https://vastlint.org/docs/rules/SIMID-1.0-simid-mediafile-required/) | error | Linear SIMID ad must include a video/audio `<MediaFile>` alongside the interactive creative |
| [SIMID-1.1-nonlinear-simid-no-iframe](https://vastlint.org/docs/rules/SIMID-1.1-nonlinear-simid-no-iframe/) | error | `<NonLinear apiFramework="SIMID">` must contain an `<IFrameResource>` |
| [SIMID-1.1-iframe-simid-type-required](https://vastlint.org/docs/rules/SIMID-1.1-iframe-simid-type-required/) | warning | `<IFrameResource>` in SIMID `<NonLinear>` should have `type="text/html"` |
| [SIMID-1.1-iframe-simid-url-empty](https://vastlint.org/docs/rules/SIMID-1.1-iframe-simid-url-empty/) | error | `<IFrameResource>` in SIMID `<NonLinear>` must contain a non-empty URL |
| [SIMID-1.1-iframe-simid-url-https](https://vastlint.org/docs/rules/SIMID-1.1-iframe-simid-url-https/) | error | `<IFrameResource>` in SIMID `<NonLinear>` URL must use HTTPS |

---

Full docs, examples, and fix guidance: **[vastlint.org/docs/rules](https://vastlint.org/docs/rules/)**

---

## VMAP 1.0 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [VMAP-1.0-root-version](https://vastlint.org/docs/rules/VMAP-1.0-root-version/) | error | Root `<VMAP>` element must have a version attribute |
| [VMAP-1.0-root-version-value](https://vastlint.org/docs/rules/VMAP-1.0-root-version-value/) | warning | `<VMAP>` version attribute should be `"1.0"` — the only published VMAP version |
| [VMAP-1.0-root-namespace](https://vastlint.org/docs/rules/VMAP-1.0-root-namespace/) | warning | `<VMAP>` should declare the VMAP namespace URI http://www.iab.net/videosuite/vmap |
| [VMAP-1.0-root-unknown-child](https://vastlint.org/docs/rules/VMAP-1.0-root-unknown-child/) | error | `<VMAP>` may only contain `<AdBreak>` and `<Extensions>` elements |
| [VMAP-1.0-adbreak-timeoffset](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-timeoffset/) | error | `<AdBreak>` must have a timeOffset attribute |
| [VMAP-1.0-adbreak-timeoffset-format](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-timeoffset-format/) | error | `<AdBreak>` timeOffset must be hh:mm:ss[.mmm], n%, `"start"`, `"end"`, or #m |
| [VMAP-1.0-adbreak-breaktype](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-breaktype/) | error | `<AdBreak>` must have a breakType attribute |
| [VMAP-1.0-adbreak-breaktype-value](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-breaktype-value/) | error | `<AdBreak>` breakType must be a comma-separated list of linear, nonlinear, or display |
| [VMAP-1.0-adbreak-repeatafter-format](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-repeatafter-format/) | warning | `<AdBreak>` repeatAfter does not match the required hh:mm:ss[.mmm] format |
| [VMAP-1.0-adbreak-unknown-child](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-unknown-child/) | error | `<AdBreak>` may only contain `<AdSource>`, `<TrackingEvents>`, and `<Extensions>` elements |
| [VMAP-1.0-adbreak-multiple-adsource](https://vastlint.org/docs/rules/VMAP-1.0-adbreak-multiple-adsource/) | error | `<AdBreak>` may contain at most one `<AdSource>` element |
| [VMAP-1.0-adsource-bool-attr](https://vastlint.org/docs/rules/VMAP-1.0-adsource-bool-attr/) | warning | `<AdSource>` allowMultipleAds and followRedirects must be `"true"` or `"false"` |
| [VMAP-1.0-adsource-content](https://vastlint.org/docs/rules/VMAP-1.0-adsource-content/) | error | `<AdSource>` must contain exactly one of `<VASTAdData>`, `<AdTagURI>`, or `<CustomAdData>` |
| [VMAP-1.0-adtaguri-empty](https://vastlint.org/docs/rules/VMAP-1.0-adtaguri-empty/) | error | `<AdTagURI>` must contain a URI referencing an ad response |
| [VMAP-1.0-adtaguri-cdata](https://vastlint.org/docs/rules/VMAP-1.0-adtaguri-cdata/) | error | `<AdTagURI>` URI must be contained within a CDATA block |
| [VMAP-1.0-customaddata-cdata](https://vastlint.org/docs/rules/VMAP-1.0-customaddata-cdata/) | error | `<CustomAdData>` data must be contained within a CDATA block |
| [VMAP-1.0-vastaddata-vast-root](https://vastlint.org/docs/rules/VMAP-1.0-vastaddata-vast-root/) | error | `<VASTAdData>` must contain an embedded `<VAST>` element (as XML, not CDATA) |
| [VMAP-1.0-embedded-vast-version](https://vastlint.org/docs/rules/VMAP-1.0-embedded-vast-version/) | info | Embedded VAST is not version 3.0 — VMAP players are only required to support VAST 3.0 |
| [VMAP-1.0-trackingevents-unknown-child](https://vastlint.org/docs/rules/VMAP-1.0-trackingevents-unknown-child/) | error | VMAP `<TrackingEvents>` may only contain `<Tracking>` elements |
| [VMAP-1.0-tracking-event](https://vastlint.org/docs/rules/VMAP-1.0-tracking-event/) | error | VMAP `<Tracking>` must have an event attribute |
| [VMAP-1.0-tracking-event-value](https://vastlint.org/docs/rules/VMAP-1.0-tracking-event-value/) | error | VMAP `<Tracking>` event must be breakStart, breakEnd, or error |
| [VMAP-1.0-error-tracking-macro](https://vastlint.org/docs/rules/VMAP-1.0-error-tracking-macro/) | info | VMAP error tracking URI should include the [ERROR_CODE] macro |
| [VMAP-1.0-tracking-url-empty](https://vastlint.org/docs/rules/VMAP-1.0-tracking-url-empty/) | error | VMAP `<Tracking>` element does not contain a tracking URI |
| [VMAP-1.0-repeatafter-conflict](https://vastlint.org/docs/rules/VMAP-1.0-repeatafter-conflict/) | warning | repeatAfter has no effect when timeOffset is `"start"` or `"end"` |
| [VMAP-1.0-display-break-no-companions](https://vastlint.org/docs/rules/VMAP-1.0-display-break-no-companions/) | info | breakType includes `"display"` but the inline VAST has no `<CompanionAds>` to fill the break |

---

## DAAST 1.0 rules

| Rule | Severity | Description |
|------|----------|-------------|
| [DAAST-1.0-root-version](https://vastlint.org/docs/rules/DAAST-1.0-root-version/) | error | Root `<DAAST>` element must have a version attribute |
| [DAAST-1.0-root-version-value](https://vastlint.org/docs/rules/DAAST-1.0-root-version-value/) | warning | `<DAAST>` version attribute must be a recognised version string (1.0 or 1.1) |
| [DAAST-1.0-root-has-ad-or-error](https://vastlint.org/docs/rules/DAAST-1.0-root-has-ad-or-error/) | error | `<DAAST>` must contain at least one `<Ad>` or `<Error>` |
| [DAAST-1.0-ad-has-inline-or-wrapper](https://vastlint.org/docs/rules/DAAST-1.0-ad-has-inline-or-wrapper/) | error | Each DAAST `<Ad>` must contain exactly one `<InLine>` or `<Wrapper>` |
| [DAAST-1.0-inline-adtitle](https://vastlint.org/docs/rules/DAAST-1.0-inline-adtitle/) | error | DAAST `<InLine>` must contain `<AdTitle>` |
| [DAAST-1.0-inline-impression](https://vastlint.org/docs/rules/DAAST-1.0-inline-impression/) | error | DAAST `<InLine>` must contain at least one `<Impression>` |
| [DAAST-1.0-inline-category](https://vastlint.org/docs/rules/DAAST-1.0-inline-category/) | error | DAAST `<InLine>` must contain `<Category>` (required in DAAST, unlike VAST) |
| [DAAST-1.0-inline-creatives](https://vastlint.org/docs/rules/DAAST-1.0-inline-creatives/) | error | DAAST `<InLine>` must contain `<Creatives>` with at least one `<Creative>` |
| [DAAST-1.0-wrapper-daastadtaguri](https://vastlint.org/docs/rules/DAAST-1.0-wrapper-daastadtaguri/) | error | DAAST `<Wrapper>` must contain `<DAASTAdTagURI>` |
| [DAAST-1.0-wrapper-vast-adtaguri](https://vastlint.org/docs/rules/DAAST-1.0-wrapper-vast-adtaguri/) | warning | `<VASTAdTagURI>` is a VAST element — DAAST wrappers redirect via `<DAASTAdTagURI>` |
| [DAAST-1.0-wrapper-impression](https://vastlint.org/docs/rules/DAAST-1.0-wrapper-impression/) | error | DAAST `<Wrapper>` must contain at least one `<Impression>` |
| [DAAST-1.0-videoclicks-element](https://vastlint.org/docs/rules/DAAST-1.0-videoclicks-element/) | warning | `<VideoClicks>` is a VAST element — DAAST uses `<AdInteractions>` |
| [DAAST-1.0-audiointeractions-renamed](https://vastlint.org/docs/rules/DAAST-1.0-audiointeractions-renamed/) | warning | `<AudioInteractions>` was renamed `<AdInteractions>` in the final DAAST release |
| [DAAST-1.0-linear-duration](https://vastlint.org/docs/rules/DAAST-1.0-linear-duration/) | error | DAAST `<Linear>` must contain `<Duration>` |
| [DAAST-1.0-duration-format](https://vastlint.org/docs/rules/DAAST-1.0-duration-format/) | error | DAAST `<Duration>` value does not match HH:MM:SS[.mmm] format |
| [DAAST-1.0-linear-mediafiles](https://vastlint.org/docs/rules/DAAST-1.0-linear-mediafiles/) | error | DAAST `<Linear>` must contain `<MediaFiles>` with at least one `<MediaFile>` |
| [DAAST-1.0-mediafile-delivery](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-delivery/) | error | DAAST `<MediaFile>` must have a delivery attribute |
| [DAAST-1.0-mediafile-delivery-enum](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-delivery-enum/) | error | DAAST `<MediaFile>` delivery must be `"progressive"` or `"streaming"` |
| [DAAST-1.0-mediafile-type](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-type/) | error | DAAST `<MediaFile>` must have a type attribute |
| [DAAST-1.0-mediafile-audio-type](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-audio-type/) | warning | DAAST `<MediaFile>` type is a video MIME type — DAAST creative is audio |
| [DAAST-1.0-mediafile-id](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-id/) | warning | DAAST `<MediaFile>` should have an id attribute (required by the DAAST XSD) |
| [DAAST-1.0-mediafile-url-empty](https://vastlint.org/docs/rules/DAAST-1.0-mediafile-url-empty/) | error | DAAST `<MediaFile>` does not contain a media URI |
| [DAAST-1.0-tracking-event-value](https://vastlint.org/docs/rules/DAAST-1.0-tracking-event-value/) | error | DAAST `<Tracking>` event is not in the DAAST audio event set |
| [DAAST-1.0-progress-offset](https://vastlint.org/docs/rules/DAAST-1.0-progress-offset/) | error | DAAST `<Tracking event="progress">` requires a valid offset attribute |
| [DAAST-1.0-pricing-model](https://vastlint.org/docs/rules/DAAST-1.0-pricing-model/) | error | DAAST `<Pricing>` is missing the required model attribute |
| [DAAST-1.0-pricing-model-value](https://vastlint.org/docs/rules/DAAST-1.0-pricing-model-value/) | warning | DAAST `<Pricing>` model must be one of cpm, cpc, cpe, cpv, cpo |
| [DAAST-1.0-pricing-currency](https://vastlint.org/docs/rules/DAAST-1.0-pricing-currency/) | error | DAAST `<Pricing>` is missing the required currency attribute |
| [DAAST-1.0-error-url-empty](https://vastlint.org/docs/rules/DAAST-1.0-error-url-empty/) | warning | DAAST `<Error>` element is present but contains no URI |
| [DAAST-1.0-error-tracking-macro](https://vastlint.org/docs/rules/DAAST-1.0-error-tracking-macro/) | info | DAAST `<Error>` URI does not include the [ERRORCODE] macro |

# VAST 4.0 to 4.1 Migration Guide

VAST 4.1 was finalised by the IAB on **November 8, 2018**. It contains several **breaking schema changes**, two major deprecations (VPAID and `conditionalAd`), and new mandatory elements that will cause validation failures if a 4.0 document is simply re-declared as `version="4.1"` without updates.

This guide covers every change and what you need to do to migrate.

---

## Breaking Changes That Require Document Updates

### 1. `<UniversalAdId>`: `idValue` attribute removed, value moves to text content

This is the most common migration breakage. In VAST 4.0, `<UniversalAdId>` used an `idValue` attribute:

```xml
<!-- VAST 4.0 — NOT valid in 4.1 -->
<UniversalAdId idRegistry="ad-id.org" idValue="AB1234"/>
```

In VAST 4.1, the `idValue` attribute was **removed from the schema**. The identifier value is now the element's **text content**:

```xml
<!-- VAST 4.1+ correct syntax -->
<UniversalAdId idRegistry="ad-id.org">AB1234</UniversalAdId>
```

The `idRegistry` attribute remains required in both versions.

**Validation rules:**
- [`VAST-4.0-universaladid-idvalue`](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idvalue/) — `idValue` attribute missing (checked on 4.0 documents only)
- [`VAST-4.1-universaladid-idvalue-removed`](https://vastlint.org/docs/rules/VAST-4.1-universaladid-idvalue-removed/) — `idValue` attribute present on a 4.1+ document
- [`VAST-4.1-universaladid-content`](https://vastlint.org/docs/rules/VAST-4.1-universaladid-content/) — `<UniversalAdId>` has no text content in a 4.1+ document

---

### 2. `<AdServingId>` is now required in `<InLine>`

A new element that **must** appear in every `<InLine>`. It carries a pseudo-unique identifier for this specific ad response instance — not the creative, but this particular serving event. The recommended format is a GUID generated fresh for each response.

```xml
<InLine>
  <AdSystem>My Ad Server</AdSystem>
  <AdServingId>550e8400-e29b-41d4-a716-446655440000</AdServingId>
  <AdTitle>Example Ad</AdTitle>
  ...
</InLine>
```

The `<AdServingId>` is used by measurement and verification vendors to correlate impression and tracking signals across the ad lifecycle. An empty `<AdServingId>` defeats this purpose.

**Validation rules:**
- [`VAST-4.1-adservingid-present`](https://vastlint.org/docs/rules/VAST-4.1-adservingid-present/) — `<AdServingId>` missing from `<InLine>`
- [`VAST-4.1-ad-serving-id-empty`](https://vastlint.org/docs/rules/VAST-4.1-ad-serving-id-empty/) — `<AdServingId>` is present but empty

---

### 3. `<Mezzanine>` gains required attributes

In VAST 4.0, `<Mezzanine>` was typed as `xs:anyURI` — just a bare URL. In VAST 4.1, it became a **full structured element** with four required attributes:

| Attribute | Required | Notes |
|---|---|---|
| `delivery` | yes | `"progressive"` or `"streaming"` |
| `type` | yes | MIME type (e.g. `"video/mp4"`) |
| `width` | yes | Pixel width (use `0` for audio-only) |
| `height` | yes | Pixel height (use `0` for audio-only) |
| `codec` | no | Optional codec identifier |
| `fileSize` | no | File size in bytes |
| `mediaType` | no | `"2D"`, `"3D"`, `"360"` (default `"2D"`) |

```xml
<MediaFiles>
  <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
    <![CDATA[https://example.com/video.mp4]]>
  </MediaFile>
  <Mezzanine delivery="progressive" type="video/mp4" width="3840" height="2160">
    <![CDATA[https://example.com/source.mp4]]>
  </Mezzanine>
</MediaFiles>
```

A `<Mezzanine>` element without `delivery`, `type`, `width`, or `height` is a schema error in 4.1+.

**Validation rules:**
- [`VAST-4.1-mezzanine-delivery`](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-delivery/) — missing `delivery`
- [`VAST-4.1-mezzanine-type`](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-type/) — missing `type`
- [`VAST-4.1-mezzanine-width`](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-width/) — missing `width`
- [`VAST-4.1-mezzanine-height`](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-height/) — missing `height`
- [`VAST-4.1-mezzanine-recommended`](https://vastlint.org/docs/rules/VAST-4.1-mezzanine-recommended/) — no `<Mezzanine>` present (warning; SSAI/CTV servers may reject)

---

## VPAID Deprecated

VPAID is deprecated as of VAST 4.1. This affects two places:

### `apiFramework="VPAID"` on `<MediaFile>`

The `apiFramework` attribute on `<MediaFile>` was already soft-deprecated in 4.0 in favour of `<InteractiveCreativeFile>`. In 4.1 the deprecation is formal: VPAID itself is deprecated.

```xml
<!-- Deprecated — triggers warning -->
<MediaFile delivery="progressive" type="application/javascript" width="640" height="360"
           apiFramework="VPAID">
  <![CDATA[https://example.com/vpaid.js]]>
</MediaFile>

<!-- Preferred pattern if VPAID still needed -->
<InteractiveCreativeFile type="application/javascript" apiFramework="VPAID">
  <![CDATA[https://example.com/vpaid.js]]>
</InteractiveCreativeFile>
```

**Validation rules:**
- [`VAST-4.1-vpaid-apiframework`](https://vastlint.org/docs/rules/VAST-4.1-vpaid-apiframework/) — `apiFramework="VPAID"` present in 4.1+ document
- [`VAST-4.1-vpaid-in-interactive-context`](https://vastlint.org/docs/rules/VAST-4.1-vpaid-in-interactive-context/) — VPAID `<MediaFile>` alongside `<InteractiveCreativeFile>` (zero fill in CTV)

### Why VPAID fails in CTV

VPAID relies on a browser JavaScript execution environment. Connected TV devices do not provide this. A VAST tag with only VPAID creatives will produce **zero fill on all CTV inventory** — the player receives the tag, cannot execute the VPAID unit, and discards the ad. This is one of the most common causes of unexpected revenue shortfalls in programmatic CTV.

---

## `<AdVerifications>` Unified Type

In VAST 4.0, `<AdVerifications>` used separate types for InLine and Wrapper — the Wrapper type was a stripped-down version. In 4.1, a **single unified `Verification_type`** is used for both. This means Wrapper AdVerifications can now carry the full verification payload.

### New `<ExecutableResource>` inside `<Verification>`

4.1 adds `<ExecutableResource>` alongside `<JavaScriptResource>` for non-browser environments (connected TV, mobile native, audio players).

| Attribute | Required | Notes |
|---|---|---|
| `apiFramework` | yes | The verification framework (e.g. `"omid"`) |
| `type` | yes | MIME type identifying the executable format |

```xml
<Verification vendor="company.com-omid">
  <JavaScriptResource apiFramework="omid">
    <![CDATA[https://example.com/omid.js]]>
  </JavaScriptResource>
  <ExecutableResource apiFramework="omid" type="application/x-omid-binary">
    <![CDATA[https://example.com/omid.bin]]>
  </ExecutableResource>
  <VerificationParameters>
    <![CDATA[{"key":"value"}]]>
  </VerificationParameters>
</Verification>
```

**Validation rules:**
- [`VAST-4.1-verification-vendor`](https://vastlint.org/docs/rules/VAST-4.1-verification-vendor/) — `vendor` attribute missing from `<Verification>`
- [`VAST-4.1-verification-no-resource`](https://vastlint.org/docs/rules/VAST-4.1-verification-no-resource/) — `<Verification>` has neither `<JavaScriptResource>` nor `<ExecutableResource>`
- [`VAST-4.1-js-resource-apiframework`](https://vastlint.org/docs/rules/VAST-4.1-js-resource-apiframework/) — `apiFramework` missing from `<JavaScriptResource>`
- [`VAST-4.1-exec-resource-apiframework`](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-apiframework/) — `apiFramework` missing from `<ExecutableResource>`
- [`VAST-4.1-exec-resource-type`](https://vastlint.org/docs/rules/VAST-4.1-exec-resource-type/) — `type` missing from `<ExecutableResource>`

---

## Audio Ad Support: `adType` Attribute

A new optional `adType` attribute on `<Ad>` enables VAST to describe audio ads:

| Value | Meaning |
|---|---|
| `video` | Default. Standard video ad. |
| `audio` | Audio-only ad. Players without audio capability should skip. |
| `hybrid` | Contains both video and audio creative alternatives. |

```xml
<Ad id="1" adType="audio">
  <InLine>
    ...
    <Creatives>
      <Creative>
        <Linear>
          <Duration>00:00:30</Duration>
          <MediaFiles>
            <MediaFile delivery="progressive" type="audio/mpeg" width="0" height="0">
              <![CDATA[https://example.com/audio.mp3]]>
            </MediaFile>
            <Mezzanine delivery="progressive" type="audio/wav" width="0" height="0">
              <![CDATA[https://example.com/source.wav]]>
            </Mezzanine>
          </MediaFiles>
        </Linear>
      </Creative>
    </Creatives>
  </InLine>
</Ad>
```

**Validation rule:**
- [`VAST-4.1-adtype-value`](https://vastlint.org/docs/rules/VAST-4.1-adtype-value/) — `adType` value is not `video`, `audio`, or `hybrid`

---

## `<Companion renderingMode>`

New optional attribute on `<Companion>` controlling display timing relative to the linear ad:

| Value | Meaning |
|---|---|
| `default` | Platform decides when to show the companion |
| `end-card` | Companion displays after the linear ad completes |
| `concurrent` | Companion displays alongside the linear ad |

```xml
<Companion width="300" height="250" renderingMode="end-card">
  ...
</Companion>
```

**Validation rule:**
- [`VAST-4.1-companion-renderingmode-value`](https://vastlint.org/docs/rules/VAST-4.1-companion-renderingmode-value/) — value not in `default`, `end-card`, `concurrent`

---

## Closed Captions: `<ClosedCaptionFiles>`

New optional container inside `<MediaFiles>`. Contains `<ClosedCaptionFile>` elements linking to external caption files.

```xml
<MediaFiles>
  <ClosedCaptionFiles>
    <ClosedCaptionFile type="text/vtt" language="en">
      <![CDATA[https://example.com/captions-en.vtt]]>
    </ClosedCaptionFile>
    <ClosedCaptionFile type="text/vtt" language="es">
      <![CDATA[https://example.com/captions-es.vtt]]>
    </ClosedCaptionFile>
  </ClosedCaptionFiles>
  <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
    <![CDATA[https://example.com/video.mp4]]>
  </MediaFile>
</MediaFiles>
```

---

## New Optional Elements

### `<BlockedAdCategories>` (in Wrapper)

New optional element (repeatable) in `<Wrapper>`. Tells downstream ad servers which content categories to exclude. Should carry an `authority` attribute identifying the taxonomy.

**Validation rule:**
- [`VAST-4.1-blockedadcategories-no-authority`](https://vastlint.org/docs/rules/VAST-4.1-blockedadcategories-no-authority/) — `authority` attribute missing

### `<Expires>` (in InLine)

New optional integer element. Specifies how many seconds this ad response is valid for. After this period, the tag should be re-fetched rather than re-used.

**Validation rule:**
- [`VAST-4.1-expires-integer`](https://vastlint.org/docs/rules/VAST-4.1-expires-integer/) : value is not an integer

### `<InteractiveCreativeFile variableDuration>`

New optional boolean attribute on `<InteractiveCreativeFile>`. Indicates the creative may extend the ad duration beyond the declared `<Duration>` through user interaction (e.g. a SIMID unit that shows additional content when the user taps).

---

## `<Survey>` Deprecated

`<Survey>` is deprecated as of VAST 4.1. New integrations should not use it.

**Validation rule:**
- [`VAST-4.1-survey-deprecated`](https://vastlint.org/docs/rules/VAST-4.1-survey-deprecated/) — `<Survey>` present in a 4.1+ document

---

## Tracking Event Changes

### Removed in 4.1

The 4.1 XSD has a tighter tracking event enum. Some events from 4.0 that had inconsistent support are no longer in the formal enum:

- `acceptInvitationLinear` (was in 3.0/4.0; replaced by `acceptInvitation`)
- `timeSpentViewing`

### Added in 4.1

| Event | Description |
|---|---|
| `loaded` | Creative has loaded and is ready to play |
| `closeLinear` | User skipped or closed the ad before completion |

**Validation rule:**
- [`VAST-4.1-tracking-event-value`](https://vastlint.org/docs/rules/VAST-4.1-tracking-event-value/) — `event` attribute value not in the valid set for this VAST version

---

## `<InteractiveCreativeFile type>` Recommended

Strongly recommended that `<InteractiveCreativeFile>` includes a `type` attribute identifying the MIME type. Without it, players cannot determine how to handle the resource.

**Validation rule:**
- [`VAST-4.1-interactive-creative-type`](https://vastlint.org/docs/rules/VAST-4.1-interactive-creative-type/) — `type` attribute missing

---

## Migration Checklist: VAST 4.0 → 4.1

- [ ] Change `<UniversalAdId idRegistry="..." idValue="ABC">` to `<UniversalAdId idRegistry="...">ABC</UniversalAdId>`
- [ ] Add `<AdServingId>` (GUID) to every `<InLine>`
- [ ] Add required attributes (`delivery`, `type`, `width`, `height`) to any `<Mezzanine>` elements
- [ ] Replace `<MediaFile apiFramework="VPAID">` with `<InteractiveCreativeFile apiFramework="VPAID">` (or ideally move to SIMID)
- [ ] Add `<ExecutableResource>` to `<Verification>` blocks for CTV/non-browser contexts
- [ ] Ensure `<JavaScriptResource>` has `apiFramework` attribute
- [ ] Ensure `<ExecutableResource>` has both `apiFramework` and `type` attributes
- [ ] Add `adType="audio"` to audio ad `<Ad>` elements
- [ ] Set `renderingMode` on `<Companion>` elements used as end-cards
- [ ] Remove `<Survey>` elements or accept the deprecation warning
- [ ] Update `version` attribute on `<VAST>` to `"4.1"`

---

*Validate your VAST 4.1 documents at [vastlint.org](https://vastlint.org). Continue to the [VAST 4.1 to 4.2 migration guide](./vast-4-1-to-4-2.md).*

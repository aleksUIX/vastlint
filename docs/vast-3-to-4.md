# What Changed Between VAST 3.0 and VAST 4.0

VAST 4.0 was published by the IAB in **April 2016**. It is the most structurally significant VAST release since 2.0. It introduced viewability measurement, creative identity, interactive ad support, and broke compatibility with some 3.0 tracking events. A VAST 4.0 document is **not fully backward-compatible** with VAST 3.0 players.

---

## Summary of Breaking Changes

| Change | Impact |
|---|---|
| `fullscreen`/`exitFullscreen` tracking events removed | 3.0 players that fire these get no signal; 4.0 players fire `playerExpand`/`playerCollapse` instead |
| `<ClickThrough>` removed from Wrapper `<VideoClicks>` | Wrapper click-through no longer valid in 4.0 (restored in 4.2) |
| `CompanionClickTracking.id` now required | Previously optional; missing `id` is now a schema error |
| `HTMLResource.xmlEncoded` attribute removed | Present in 3.0 XSD; removed in 4.0 |
| `UniversalAdId` required under every `<Creative>` | New required element — 3.0 documents have none |

---

## New Required Element: `<UniversalAdId>`

The most impactful new addition. Every `<Creative>` in an `<InLine>` must contain at least one `<UniversalAdId>` element. This provides a stable, cross-platform creative identifier independent of the ad server's internal ID.

| Attribute | Required | Notes |
|---|---|---|
| `idRegistry` | yes | Registry URL (e.g. `"ad-id.org"`, `"clearcast.co.uk"`) |
| `idValue` | yes (4.0 only) | The identifier value (**removed in 4.1 — value moves to element text content**) |

```xml
<!-- VAST 4.0 syntax -->
<Creative>
  <UniversalAdId idRegistry="ad-id.org" idValue="AB1234">AB1234</UniversalAdId>
  <Linear>...</Linear>
</Creative>
```

> **Warning:** The `idValue` attribute syntax is 4.0-specific. In VAST 4.1 and later, the value is element text content and `idValue` was removed from the schema. See the [4.0 to 4.1 migration guide](./vast-4-to-4-1.md).

**Validation rules:**
- [`VAST-4.0-universaladid-present`](https://vastlint.org/docs/rules/VAST-4.0-universaladid-present/) — `<UniversalAdId>` missing from `<Creative>`
- [`VAST-4.0-universaladid-idregistry`](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idregistry/) — `idRegistry` attribute missing
- [`VAST-4.0-universaladid-idvalue`](https://vastlint.org/docs/rules/VAST-4.0-universaladid-idvalue/) — `idValue` attribute missing (VAST 4.0 only)

---

## Viewability Measurement: `<ViewableImpression>`

New optional element in both `<InLine>` and `<Wrapper>`. Contains pixel URLs for three viewability states:

| Child element | Fires when |
|---|---|
| `<Viewable>` | Ad meets viewability threshold |
| `<NotViewable>` | Ad does not meet threshold |
| `<ViewUndetermined>` | Viewability could not be measured |

```xml
<ViewableImpression id="vi1">
  <Viewable><![CDATA[https://example.com/viewable]]></Viewable>
  <NotViewable><![CDATA[https://example.com/notviewable]]></NotViewable>
  <ViewUndetermined><![CDATA[https://example.com/undetermined]]></ViewUndetermined>
</ViewableImpression>
```

---

## Third-Party Verification: `<AdVerifications>`

New optional container in both `<InLine>` and `<Wrapper>`. Contains `<Verification>` elements, one per verification vendor. This is the predecessor to the Open Measurement SDK (OMID) integration pattern.

```xml
<AdVerifications>
  <Verification vendor="company.com-omid">
    <JavaScriptResource apiFramework="omid">
      <![CDATA[https://example.com/verify.js]]>
    </JavaScriptResource>
  </Verification>
</AdVerifications>
```

> **Note:** `<AdVerifications>` in 4.0 uses separate Inline and Wrapper types. In 4.1 these were unified into a single type.

---

## Interactive Creative Files: `<InteractiveCreativeFile>`

Replaces the use of `<MediaFile apiFramework="VPAID">` for interactive ads. Lives inside `<MediaFiles>` alongside regular `<MediaFile>` elements.

| Attribute | Required | Notes |
|---|---|---|
| `type` | no | MIME type of the creative |
| `apiFramework` | no | e.g. `"SIMID"`, `"VPAID"` (though VPAID deprecated in 4.1) |

```xml
<MediaFiles>
  <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
    <![CDATA[https://example.com/video.mp4]]>
  </MediaFile>
  <InteractiveCreativeFile type="text/html" apiFramework="SIMID">
    <![CDATA[https://example.com/interactive.html]]>
  </InteractiveCreativeFile>
</MediaFiles>
```

**Validation rules:**
- [`VAST-4.0-interactive-creative-no-api`](https://vastlint.org/docs/rules/VAST-4.0-interactive-creative-no-api/) — `apiFramework` missing from `<InteractiveCreativeFile>`
- [`VAST-4.0-mediafile-apiframework`](https://vastlint.org/docs/rules/VAST-4.0-mediafile-apiframework/) — `<MediaFile apiFramework>` used instead of `<InteractiveCreativeFile>`

---

## Content Categorisation: `<Category>`

New optional element (repeatable) inside `<InLine>`. Declares the ad's content category for brand safety filtering. The `authority` attribute is **required** and should be a URL identifying the taxonomy.

```xml
<Category authority="https://www.iab.com/guidelines/taxonomy">IAB-1</Category>
```

**Validation rule:**
- [`VAST-4.0-category-authority`](https://vastlint.org/docs/rules/VAST-4.0-category-authority/) — `authority` attribute missing

---

## Mezzanine File: `<Mezzanine>`

New optional element inside `<MediaFiles>`. Contains a URI to the raw, high-quality source file. Used by SSAI servers to transcode ad content for different delivery contexts (CTV, OTT, mobile).

In VAST 4.0, `<Mezzanine>` is typed as `xs:anyURI` — it is just a URI with no other required attributes. It gains a full structured type with required attributes in VAST 4.1.

---

## Removed: `fullscreen` and `exitFullscreen` Tracking Events

These two events from VAST 3.0 were **removed** in VAST 4.0 and replaced with `playerExpand` and `playerCollapse`:

| 3.0 event | 4.0 replacement |
|---|---|
| `fullscreen` | `playerExpand` |
| `exitFullscreen` | `playerCollapse` |

A VAST document claiming version `4.0` or higher that contains `fullscreen` or `exitFullscreen` tracking events is using invalid event names for that version.

**Validation rule:**
- [`VAST-4.0-tracking-event-removed`](https://vastlint.org/docs/rules/VAST-4.0-tracking-event-removed/) — `fullscreen`/`exitFullscreen` events present in a 4.0+ document

### New tracking events in 4.0

| Event | Description |
|---|---|
| `playerExpand` | Replaces `fullscreen` |
| `playerCollapse` | Replaces `exitFullscreen` |
| `timeSpentViewing` | Cumulative time the ad was in view |
| `adExpand` | Ad expanded from collapsed state |
| `adCollapse` | Ad collapsed |
| `minimize` | Ad minimised |
| `overlayViewDuration` | Time overlay was in view |
| `otherAdInteraction` | Custom interaction not covered by other events |

---

## Removed: `<ClickThrough>` in Wrapper `<VideoClicks>`

In VAST 4.0, `<ClickThrough>` inside a Wrapper's `<VideoClicks>` was **removed**. Wrappers are redirect containers — the click-through URL should be defined in the final `<InLine>`. This restriction was **re-allowed in VAST 4.2**.

**Validation rule:**
- [`VAST-4.0-wrapper-clickthrough`](https://vastlint.org/docs/rules/VAST-4.0-wrapper-clickthrough/) — `<ClickThrough>` present in Wrapper `<VideoClicks>` for a 4.0/4.1 document

---

## `<VAST>` Root: Both `<Ad>` and `<Error>` Elements

In VAST 4.0, the root `<VAST>` element may contain `<Error>` elements at the root level (not just inside `<Ad>`). However, having both `<Ad>` and root-level `<Error>` elements together is explicitly prohibited in 4.0.

**Validation rule:**
- [`VAST-4.0-wrapper-root-error`](https://vastlint.org/docs/rules/VAST-4.0-wrapper-root-error/) — root `<VAST>` contains both `<Ad>` and `<Error>`

---

## `<CompanionClickTracking id>` Now Required

The `id` attribute on `<CompanionClickTracking>` became **required** in 4.0. It was optional in 3.0.

**Validation rule:**
- [`VAST-4.0-companion-clicktracking-id`](https://vastlint.org/docs/rules/VAST-4.0-companion-clicktracking-id/) — `id` attribute missing from `<CompanionClickTracking>`

---

## `conditionalAd` Attribute (Deprecated in 4.1)

A new optional boolean attribute on `<Ad>`. Indicates the ad is conditional (e.g., depends on user state). Deprecated in VAST 4.1 in favour of `Creative.apiFramework`.

**Validation rule:**
- [`VAST-4.0-conditionalad`](https://vastlint.org/docs/rules/VAST-4.0-conditionalad/) — `conditionalAd` present (deprecated as of 4.1)

---

## New Wrapper Attributes

Three new boolean attributes on the `<Wrapper>` element:

| Attribute | Default | Description |
|---|---|---|
| `followAdditionalWrappers` | `true` | Whether the player should follow additional wrapper redirects |
| `allowMultipleAds` | `false` | Whether multiple ads in the response are acceptable |
| `fallbackOnNoAd` | `false` | Whether to use the next ad if this wrapper chain resolves to nothing |

---

## Structural Changes in 4.0 XSD

These changes affect how the XML schema validates documents, though they align with what was already required in practice:

- **`<MediaFiles>` and `<Duration>` are now formally `minOccurs="1"`** inside `<Linear>` — they were technically optional in the 3.0 XSD but practically required
- **`<Creative>` inside `<Wrapper>`** is now `minOccurs="1"` — Wrapper must have at least one Creative
- **`<InLine Creative>` child order** changed from `xs:sequence` to `xs:all` — child elements can appear in any order
- **Multiple resources of the same type** are now allowed — `StaticResource`, `IFrameResource`, `HTMLResource` can each appear `0..n` times (was choice of 0..1 in 3.0)

---

## Migration Checklist: VAST 3.0 → 4.0

- [ ] Add `<UniversalAdId idRegistry="...">` with `idValue` attribute to every `<Creative>` in `<InLine>`
- [ ] Replace `<MediaFile apiFramework="VPAID">` with `<InteractiveCreativeFile apiFramework="VPAID">` (VPAID deprecated in 4.1)
- [ ] Replace `fullscreen` tracking events with `playerExpand`; replace `exitFullscreen` with `playerCollapse`
- [ ] Remove `<ClickThrough>` from Wrapper `<VideoClicks>` (re-allowed in 4.2)
- [ ] Add `id` attribute to any `<CompanionClickTracking>` elements
- [ ] Add `<Category authority="...">` to categorise the ad (optional but recommended)
- [ ] Add `<AdVerifications>` block if using third-party viewability/verification vendors
- [ ] Update `version` attribute on `<VAST>` to `"4.0"`

---

*Validate your VAST 4.0 documents at [vastlint.org](https://vastlint.org). See the full [rule reference](https://vastlint.org/docs/rules/) for details on every rule, or continue to the [VAST 4.0 to 4.1 migration guide](./vast-4-to-4-1.md).*

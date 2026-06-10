# What Changed Between VAST 4.1 and VAST 4.2

VAST 4.2 was finalised by the IAB in **June 2019**. It is a targeted release with four substantive changes. No existing 4.1 structures were removed or restructured, so migration is straightforward.

---

## Summary

| Change | Impact |
|---|---|
| `<ClickThrough>` re-allowed in Wrapper `<VideoClicks>` | Reverses the 4.0 restriction; wrapper click-through valid again |
| `<IconClickFallbackImages>` and `<IconClickFallbackImage>` added | New icon disclosure fallback for no-HTML environments |
| `interactiveStart` tracking event added | Fires when SIMID creative takes control |
| Error codes 206 and 902 added | New player/SIMID error reporting codes |

---

## 1. `<ClickThrough>` Re-allowed in Wrapper `<VideoClicks>`

VAST 4.0 removed `<ClickThrough>` from Wrapper `<VideoClicks>`, on the reasoning that click-through should be defined in the final `<InLine>`. VAST 4.2 **reverses this decision**, explicitly permitting `<ClickThrough>` in Wrapper VideoClicks again to support use cases where the wrapper itself needs to control the click destination (e.g. a DSP redirecting through multiple layers).

This was achieved in the XSD by collapsing the separate `VideoClicks_Base_type` and `VideoClicks_Inline_type` (4.1) into a single unified `VideoClicks_type` used by both InLine and Wrapper.

**Practical implication:** A VASTlint rule that fires for `<ClickThrough>` in Wrapper on version 4.0 or 4.1 documents should **not** fire on 4.2+ documents.

**Related validation rule:**
- [`VAST-4.0-wrapper-clickthrough`](https://vastlint.org/docs/rules/VAST-4.0-wrapper-clickthrough/) — this rule is version-scoped to 4.0 and 4.1 only; it does not fire on 4.2+ documents

---

## 2. Icon Click Fallback Images

New elements for the AdChoices icon disclosure flow in environments where HTML rendering is unavailable (e.g. some streaming TV surfaces). When a user clicks the AdChoices icon and the player cannot open `<IconClickThrough>` in a browser, it may instead display a static image overlay.

### Structure

The new elements appear inside `<IconClicks>`, alongside the existing `<IconClickThrough>` and `<IconClickTracking>`:

```xml
<IconClicks>
  <IconClickThrough>
    <![CDATA[https://optout.aboutads.info/?c=2&lang=EN]]>
  </IconClickThrough>
  <IconClickFallbackImages>
    <IconClickFallbackImage width="320" height="240">
      <![CDATA[https://example.com/adchoices-disclosure.jpg]]>
    </IconClickFallbackImage>
  </IconClickFallbackImages>
</IconClicks>
```

### `<IconClickFallbackImage>` attributes

| Attribute | Required | Notes |
|---|---|---|
| `width` | recommended | Pixel width of the image |
| `height` | recommended | Pixel height of the image |

Both attributes are optional per the XSD but the spec recommends including them so the player can size the overlay correctly without loading the image first.

**Validation rule:**
- [`VAST-4.2-icon-fallback-image-width-height`](https://vastlint.org/docs/rules/VAST-4.2-icon-fallback-image-width-height/) — `<IconClickFallbackImage>` missing `width` or `height`

---

## 3. `interactiveStart` Tracking Event

A new tracking event added in 4.2 specifically for SIMID (Secure Interactive Media Interface Definition) creatives. Fires when the SIMID interactive creative takes control of the player — i.e., when the interactive experience begins, distinct from when the video starts playing.

```xml
<TrackingEvents>
  <Tracking event="start">https://example.com/track/start</Tracking>
  <Tracking event="interactiveStart">https://example.com/track/interactive-start</Tracking>
</TrackingEvents>
```

This event is not in the VAST 4.1 XSD enum. Using it in a 4.1 document would trigger the tracking event validation rule — use only with `version="4.2"` or later.

---

## 4. New Error Codes

Two new error codes extend the set of values a VAST `<Error>` URI macro (`[ERRORCODE]`) can carry:

| Code | Meaning |
|---|---|
| `206` | Player decided not to play an ad because the ad break was shortened (e.g. a live broadcast break was cut shorter than expected) |
| `902` | General `InteractiveCreativeFile` error — covers SIMID load failures, execution errors, and resource fetch failures |

These are `[ERRORCODE]` macro values, not XML structural changes. They do not affect document linting but are useful context when interpreting error callback traffic.

---

## Multiple `<UniversalAdId>` Explicitly Allowed

The 4.2 spec explicitly documents that `<UniversalAdId>` has `maxOccurs="unbounded"` on `Creative_Inline_type`. While this was technically true from the 4.1 XSD, 4.2 formalises the intent: a creative may carry multiple universal IDs from different registries simultaneously.

```xml
<Creative>
  <UniversalAdId idRegistry="ad-id.org">ADID-12345</UniversalAdId>
  <UniversalAdId idRegistry="clearcast.co.uk">CLR-67890</UniversalAdId>
  <Linear>...</Linear>
</Creative>
```

---

## What Did Not Change

Everything from VAST 4.1 carries forward unchanged:

- `<AdServingId>` required in `<InLine>`
- `<Mezzanine>` typed element with required attributes
- `<ExecutableResource>` in `<Verification>`
- `adType` attribute on `<Ad>`
- `renderingMode` attribute on `<Companion>`
- `<ClosedCaptionFiles>` in `<MediaFiles>`
- Unified `Verification_type`
- VPAID deprecated
- `<Survey>` deprecated

---

## Migration Checklist: VAST 4.1 → 4.2

- [ ] Update `version` attribute on `<VAST>` to `"4.2"`
- [ ] Optionally add `<ClickThrough>` to Wrapper `<VideoClicks>` if your use case requires it (now valid again)
- [ ] Add `<IconClickFallbackImages>` / `<IconClickFallbackImage>` to icon click disclosure flows if serving on CTV or no-HTML surfaces
- [ ] Add `interactiveStart` tracking event to SIMID creative tracking if applicable

---

*Validate your VAST 4.2 documents at [vastlint.org](https://vastlint.org). Continue to the [VAST 4.2 to 4.3 changes](./vast-4-2-to-4-3.md).*

# What Changed Between VAST 4.2 and VAST 4.3

VAST 4.3 was released by the IAB in **December 2022**. It is the smallest VAST release on record. The IAB explicitly noted that the changes "did not change the actual technical spec or XSD, and so did not result in an update to the version" for XSD purposes. The **VAST 4.2 XSD remains the authoritative schema for VAST 4.3 documents.**

There are three meaningful changes.

---

## Summary

| Change | Impact |
|---|---|
| `<InteractiveCreativeFile>` may contain an inline `data:` URI | `data:` URIs are now valid content; do not reject them |
| `[PLAYBACKMETHODS]` macro value `7` added | Live macro context only; no XML lint impact |
| Macros reference moved to separate GitHub document | No XML impact |

---

## 1. `<InteractiveCreativeFile>` Can Contain Inline `data:` URIs

Previously, `<InteractiveCreativeFile>` was expected to contain a URL pointing to the interactive creative resource. VAST 4.3 explicitly allows the content to be an **inline data URI** instead.

### Why this matters

A data URI embeds the resource content directly in the VAST XML rather than requiring a separate HTTP request. For small SIMID creatives, this eliminates one round-trip during the ad load sequence, which matters in time-sensitive environments like live streams and CTV ad breaks.

The spec is explicit: "the recommendation is to make this as small as possible. Once the size of the data URI becomes prohibitively large, use a URL to save the response size of the VAST, itself."

### Format

```xml
<!-- Standard URL (all versions) -->
<InteractiveCreativeFile type="text/html" apiFramework="SIMID">
  <![CDATA[https://example.com/interactive.html]]>
</InteractiveCreativeFile>

<!-- Inline data URI (new in 4.3) -->
<InteractiveCreativeFile type="text/html" apiFramework="SIMID">
  <![CDATA[data:text/html;base64,PCFET0NUWVBFIGh0bWw+...]]>
</InteractiveCreativeFile>
```

### Schema note

The element type remains `xs:anyURI` in the 4.2 XSD. Data URIs are syntactically valid URIs per [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986), so no XSD change was needed. This is a clarification of intent rather than a structural change.

**What validators must do:** Do not flag a `data:` scheme URI inside `<InteractiveCreativeFile>` as an invalid URL. This applies to any validator targeting VAST 4.3.

---

## 2. `[PLAYBACKMETHODS]` Macro Value `7`

A new value was added to the `[PLAYBACKMETHODS]` macro, which is substituted at impression time to describe how the player initiated playback:

| Value | Meaning |
|---|---|
| `1` | Auto-play, sound on |
| `2` | Auto-play, sound off |
| `3` | Click-to-play |
| `4` | Mouse-over |
| `5` | Entering viewport, sound on |
| `6` | Entering viewport, sound off (default for most CTV/display contexts) |
| `7` | **Continuous play** — back-to-back episodes or content without user interaction between them (new in 4.3) |

This is a macro substitution value, not an XML element or attribute. It has no impact on VAST document linting.

---

## 3. Macros Reference Moved to GitHub

The IAB moved the VAST macros reference to a separate, independently maintained GitHub document. The macros themselves did not change (other than value `7` above). This has no impact on XML structure or validation.

---

## `<JavaScriptResource browserOptional>` Recommended

While not new in 4.3, the `browserOptional` attribute on `<JavaScriptResource>` — added in 4.1 — has a corresponding vastlint rule that flags its absence. For CTV/non-browser delivery contexts, this attribute tells the player whether the JavaScript resource is strictly required or whether execution can proceed without it.

```xml
<JavaScriptResource apiFramework="omid" browserOptional="false">
  <![CDATA[https://example.com/omid.js]]>
</JavaScriptResource>
```

**Validation rule:**
- [`VAST-4.3-js-resource-browser-optional`](https://vastlint.org/docs/rules/VAST-4.3-js-resource-browser-optional/) — `browserOptional` attribute missing from `<JavaScriptResource>`

---

## All 4.2 Structures Carry Forward Unchanged

Every rule and structure from VAST 4.2 applies to VAST 4.3 documents without modification:

- `<AdServingId>` required in `<InLine>`
- `<UniversalAdId>` value in text content (not `idValue` attribute)
- `<Mezzanine>` typed element with required `delivery`, `type`, `width`, `height`
- `<ExecutableResource>` in `<Verification>`
- `adType` on `<Ad>`, `renderingMode` on `<Companion>`
- `<ClosedCaptionFiles>` in `<MediaFiles>`
- `<ClickThrough>` allowed in Wrapper `<VideoClicks>` (restored in 4.2)
- `<IconClickFallbackImages>` for icon disclosure
- `interactiveStart` tracking event

---

## Corrections and Clarifications

The 4.3 spec includes a "Corrections & Clarifications" appendix addressing errors in the 4.2 prose document — clarifications to the `HTMLResource` description, `IconClickFallbackImage` references, and the human-readable schema list. None of these changed the XSD or any structural rules.

---

## Migration Checklist: VAST 4.2 → 4.3

The checklist is minimal:

- [ ] Update `version` attribute on `<VAST>` to `"4.3"`
- [ ] If serving SIMID creatives: consider using inline `data:` URIs for small payloads to reduce request latency
- [ ] Ensure `<JavaScriptResource>` has `browserOptional` attribute set for CTV contexts

---

*Validate your VAST 4.3 documents at [vastlint.org](https://vastlint.org). See the [full rule reference](https://vastlint.org/docs/rules/) or go back to the [VAST 4.1 to 4.2 migration guide](./vast-4-1-to-4-2.md).*

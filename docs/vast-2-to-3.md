# What Changed Between VAST 2.0 and VAST 3.0

VAST 3.0 was published by the IAB on **July 19, 2012**. It is a strict superset of VAST 2.0 — nothing was removed, and the document structure is compatible. A VAST 2.0 player can process a VAST 3.0 document if it ignores unknown elements, though it will miss the new features.

---

## Summary

| Category | Change |
|---|---|
| New InLine elements | `Advertiser`, `Pricing` |
| New Linear features | `skipoffset`, skip tracking, `Pricing` |
| New tracking events | `progress`, `skip`, `exitFullscreen`, `acceptInvitationLinear`, `closeLinear` |
| New MediaFile attributes | `minBitrate`, `maxBitrate`, `codec` |
| New ad structure | Icons (`Icons > Icon`), `CreativeExtensions`, `CompanionAds.required` |
| Breaking schema changes | None — 3.0 is fully backward-compatible with 2.0 |

---

## New InLine Elements

### `<Advertiser>`

A new optional text element inside `<InLine>`. Contains the advertiser name, used for competitive exclusion. Players and SSPs can use this to prevent competing brands from serving in adjacent pods.

```xml
<InLine>
  <AdSystem>Acme Ad Server</AdSystem>
  <AdTitle>Example Ad</AdTitle>
  <Advertiser>Acme Corp</Advertiser>  <!-- new in 3.0 -->
  ...
</InLine>
```

### `<Pricing>`

A new optional element in `<InLine>` carrying the declared price for the ad. Has two **required** attributes: `model` and `currency`.

| Attribute | Required | Values |
|---|---|---|
| `model` | yes | `cpm`, `cpc`, `cpe`, `cpv` (lowercase per XSD) |
| `currency` | yes | ISO 4217 three-letter code (e.g. `USD`, `EUR`) |

```xml
<Pricing model="cpm" currency="USD">1.50</Pricing>
```

**Validation rules triggered if malformed:**
- [`VAST-3.0-pricing-model`](https://vastlint.org/docs/rules/VAST-3.0-pricing-model/) — missing `model` attribute
- [`VAST-3.0-pricing-currency`](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency/) — missing `currency` attribute
- [`VAST-3.0-pricing-model-case`](https://vastlint.org/docs/rules/VAST-3.0-pricing-model-case/) — model value should be lowercase
- [`VAST-3.0-pricing-currency-format`](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency-format/) — currency must be a 3-letter ISO 4217 code

---

## Skippable Ads

VAST 3.0 introduced the concept of skippable linear ads via the `skipoffset` attribute on `<Linear>`.

### `skipoffset` attribute

When present, `skipoffset` indicates that the ad is skippable and specifies the time after which the skip control should appear.

**Formats accepted:**
- `HH:MM:SS[.mmm]` — absolute time (e.g. `00:00:05` = skip after 5 seconds)
- `n%` — percentage of duration (e.g. `30%`)

```xml
<Linear skipoffset="00:00:05">
  ...
</Linear>
```

**Validation rules:**
- [`VAST-3.0-skipoffset-format`](https://vastlint.org/docs/rules/VAST-3.0-skipoffset-format/) — value does not match the required format

### `skip` tracking event

Fires when the user activates the skip control. If a `skip` tracking URL is present, the player must also have `skipoffset` set on `<Linear>` — otherwise there is no mechanism to trigger the skip.

**Validation rules:**
- [`VAST-3.0-skip-event-no-skipoffset`](https://vastlint.org/docs/rules/VAST-3.0-skip-event-no-skipoffset/) — `skip` event present but `Linear` has no `skipoffset`

---

## New Tracking Events

| Event | Description |
|---|---|
| `progress` | Fires at a specific offset into playback. **Requires an `offset` attribute.** |
| `skip` | Fires when user skips a skippable ad |
| `exitFullscreen` | Fires when user exits fullscreen |
| `acceptInvitationLinear` | Fires when user accepts an interactive invitation |
| `closeLinear` | Fires when user closes/minimises a linear ad |

### `progress` event and `offset`

The `progress` event is unique in that it **requires** an `offset` attribute specifying the time at which the pixel fires. A `<Tracking event="progress">` without `offset` is invalid per the spec.

Formats: `HH:MM:SS[.mmm]` or `n%`.

```xml
<TrackingEvents>
  <Tracking event="progress" offset="00:00:10">https://example.com/progress</Tracking>
</TrackingEvents>
```

**Validation rules:**
- [`VAST-3.0-progress-offset`](https://vastlint.org/docs/rules/VAST-3.0-progress-offset/) — `offset` attribute missing
- [`VAST-3.0-progress-offset-format`](https://vastlint.org/docs/rules/VAST-3.0-progress-offset-format/) — `offset` value format invalid

---

## New MediaFile Attributes

### `minBitrate` and `maxBitrate`

New optional attributes on `<MediaFile>` for adaptive bitrate streams. When using these, **both must be present together** — specifying only one is invalid.

The existing `bitrate` attribute (single fixed bitrate) must not be used alongside `minBitrate`/`maxBitrate`.

| Attribute | Notes |
|---|---|
| `minBitrate` | Minimum bitrate in Kbps |
| `maxBitrate` | Maximum bitrate in Kbps |
| `codec` | Optional codec identifier (new in 3.0) |

**Validation rules:**
- [`VAST-3.0-minmaxbitrate-pair`](https://vastlint.org/docs/rules/VAST-3.0-minmaxbitrate-pair/) — only one of `minBitrate`/`maxBitrate` present
- [`VAST-3.0-bitrate-conflict`](https://vastlint.org/docs/rules/VAST-3.0-bitrate-conflict/) — both `bitrate` and `minBitrate`/`maxBitrate` present

---

## Icons

VAST 3.0 introduced a full icon system for AdChoices and privacy disclosure. Icons appear inside `<Linear>` as a sibling to `<MediaFiles>`.

```
Linear
  Icons
    Icon [program*, width*, height*, xPosition*, yPosition*, offset, duration, apiFramework]
      StaticResource | IFrameResource | HTMLResource
      IconClicks
        IconClickThrough
        IconClickTracking (0..n)
      IconViewTracking (0..n)
```

### Required `<Icon>` attributes

| Attribute | Required | Notes |
|---|---|---|
| `program` | yes | Identifies the icon program (e.g. `"AdChoices"`) |
| `width` | yes | Pixel width |
| `height` | yes | Pixel height |
| `xPosition` | yes | Horizontal position (`"left"`, `"right"`, or pixel offset) |
| `yPosition` | yes | Vertical position (`"top"`, `"bottom"`, or pixel offset) |
| `offset` | no | Delay before icon appears (same format as `skipoffset`) |
| `duration` | no | How long to display the icon |
| `apiFramework` | no | For interactive icons |

**Validation rules:**
- [`VAST-3.0-icon-program`](https://vastlint.org/docs/rules/VAST-3.0-icon-program/) — missing `program`
- [`VAST-3.0-icon-width`](https://vastlint.org/docs/rules/VAST-3.0-icon-width/) — missing `width`
- [`VAST-3.0-icon-height`](https://vastlint.org/docs/rules/VAST-3.0-icon-height/) — missing `height`
- [`VAST-3.0-icon-xposition`](https://vastlint.org/docs/rules/VAST-3.0-icon-xposition/) — missing `xPosition`
- [`VAST-3.0-icon-yposition`](https://vastlint.org/docs/rules/VAST-3.0-icon-yposition/) — missing `yPosition`
- [`VAST-3.0-icon-resource`](https://vastlint.org/docs/rules/VAST-3.0-icon-resource/) — `<Icon>` has no resource element
- [`VAST-3.0-icon-attrs`](https://vastlint.org/docs/rules/VAST-3.0-icon-attrs/) — multiple missing recommended attributes

---

## `<CreativeExtensions>`

A new element type for per-creative custom extensions, separate from the `<Extensions>` block at the `<InLine>` level. `<CreativeExtensions>` lives inside `<Creative>` and may only contain `<CreativeExtension>` children.

**Validation rule:**
- [`VAST-2.0-creativeextensions-unknown-child`](https://vastlint.org/docs/rules/VAST-2.0-creativeextensions-unknown-child/) — applies to all versions including 3.0+

---

## `<CompanionAds required>`

A new optional `required` attribute on `<CompanionAds>` (inside InLine Creatives) controls the display policy when companions are not available:

| Value | Meaning |
|---|---|
| `"all"` | All companions must be displayed or the ad should not play |
| `"any"` | At least one companion must be displayed |
| `"none"` | Companions are optional (default behaviour) |

**Validation rule:**
- [`VAST-3.0-companion-required-attr`](https://vastlint.org/docs/rules/VAST-3.0-companion-required-attr/) — value is not `all`, `any`, or `none`

---

## Ad Pods

The `sequence` attribute on `<Ad>` was present in 2.0 but was explicitly documented in 3.0 as defining an **Ad Pod** — a sequenced set of ads served together in a single break. When `sequence` is used, all `<Ad>` elements in the response should have it, or none should.

**Validation rule:**
- [`VAST-2.0-ad-sequence`](https://vastlint.org/docs/rules/VAST-2.0-ad-sequence/) — inconsistent use of `sequence` across `<Ad>` elements

---

## What Did Not Change

- Document structure (`VAST > Ad > InLine/Wrapper`) is identical
- All 2.0 required elements (`AdSystem`, `AdTitle`, `Impression`, `Creatives`, `Duration`, `MediaFiles`) remain required with the same rules
- `<Wrapper>` structure is unchanged
- Nothing was removed from 2.0

---

## Relevant New Rules in VASTlint

Rules first enforced against VAST 3.0 documents (not applied to 2.0 documents):

| Rule | Severity |
|---|---|
| [`VAST-3.0-progress-offset`](https://vastlint.org/docs/rules/VAST-3.0-progress-offset/) | error |
| [`VAST-3.0-progress-offset-format`](https://vastlint.org/docs/rules/VAST-3.0-progress-offset-format/) | warning |
| [`VAST-3.0-skipoffset-format`](https://vastlint.org/docs/rules/VAST-3.0-skipoffset-format/) | warning |
| [`VAST-3.0-skip-event-no-skipoffset`](https://vastlint.org/docs/rules/VAST-3.0-skip-event-no-skipoffset/) | warning |
| [`VAST-3.0-minmaxbitrate-pair`](https://vastlint.org/docs/rules/VAST-3.0-minmaxbitrate-pair/) | error |
| [`VAST-3.0-bitrate-conflict`](https://vastlint.org/docs/rules/VAST-3.0-bitrate-conflict/) | warning |
| [`VAST-3.0-icon-program`](https://vastlint.org/docs/rules/VAST-3.0-icon-program/) | error |
| [`VAST-3.0-icon-width`](https://vastlint.org/docs/rules/VAST-3.0-icon-width/) | error |
| [`VAST-3.0-icon-height`](https://vastlint.org/docs/rules/VAST-3.0-icon-height/) | error |
| [`VAST-3.0-icon-xposition`](https://vastlint.org/docs/rules/VAST-3.0-icon-xposition/) | error |
| [`VAST-3.0-icon-yposition`](https://vastlint.org/docs/rules/VAST-3.0-icon-yposition/) | error |
| [`VAST-3.0-icon-resource`](https://vastlint.org/docs/rules/VAST-3.0-icon-resource/) | error |
| [`VAST-3.0-icon-attrs`](https://vastlint.org/docs/rules/VAST-3.0-icon-attrs/) | warning |
| [`VAST-3.0-pricing-model`](https://vastlint.org/docs/rules/VAST-3.0-pricing-model/) | error |
| [`VAST-3.0-pricing-currency`](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency/) | error |
| [`VAST-3.0-pricing-model-case`](https://vastlint.org/docs/rules/VAST-3.0-pricing-model-case/) | warning |
| [`VAST-3.0-pricing-currency-format`](https://vastlint.org/docs/rules/VAST-3.0-pricing-currency-format/) | warning |
| [`VAST-3.0-companion-required-attr`](https://vastlint.org/docs/rules/VAST-3.0-companion-required-attr/) | warning |

---

*Validate your VAST 3.0 documents at [vastlint.org](https://vastlint.org). See the full [rule reference](https://vastlint.org/docs/rules/) for details on every rule.*

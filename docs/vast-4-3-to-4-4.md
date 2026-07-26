# What Changed Between VAST 4.3 and VAST 4.4

VAST 4.4 is still in progress. `vast_4.4.xsd` landed in the IAB VAST repository on **17 July 2026** via PR #57, from a branch named `4.4Development`, and its own schema annotation reads:

> IAB VAST (Video Ad Serving Template), Version 4.4 - DRAFT for working group discussion.

It supports the **CTV Ad Portfolio**, whose signaling guidance *was* finalised, on **22 July 2026**, after a public comment period that closed on 5 June.

One of them is a finished standard and the other can still change, which is most of what you need to know before migrating anything.

| Artifact | Status | What it governs |
|---|---|---|
| CTV Ad Portfolio signaling guidance | **Final** (2026-07-22) | AdCOM enumerations, Native placement types, VAST prose |
| `vast_4.4.xsd` | **Draft** | The XML content model |

If you take one thing from this page: **you do not need to declare `version="4.4"` to use anything on it.** Every VAST example in the final guidance declares `version="4.2"`, which matches the deployed base.

---

## Summary

| Change | Impact |
|---|---|
| `<MediaFiles>` allowed under `<NonLinear>` | Video and cinemagraph assets in Pause, Screensaver, Overlay, Squeezeback, In-Scene |
| `<Duration>` allowed under `<NonLinear>` | Quartile and `overlayViewDuration` tracking for non-linear formats |
| `<Icons>` allowed under `<NonLinearAds>` | Ad-choices disclosure on CTV Ad Portfolio placements |
| `<NonLinearCustomClick>` returns | Absent from every published XSD since 3.0 |
| SIMID moves into NonLinear `<MediaFiles>` | `<IFrameResource apiFramework="SIMID">` deprecated for these formats |
| `<Extension>` carries typed AdCOM signals | `plcmt`, `pos`, `playbackmethod`, `attr` round-tripped into the creative |
| `<CreativeExtension>` carries typed QR elements | Scan URL, image URL, position, size |
| `minSuggestedDuration` retyped | `xs:time` to `vastTime_type`; accepts `mm:ss` |

---

## Why this exists: the CTV Ad Portfolio

Since 2024 the IAB's Ad Format Hero initiative collected hundreds of format submissions and consolidated them into six CTV ad formats that had been running in market for years on bespoke, per-publisher integrations:

| Format | How it transacts |
|---|---|
| Pause | OpenRTB Video object, NonLinear VAST response |
| Screensaver | OpenRTB Video object, NonLinear VAST response |
| Overlay | OpenRTB Video object, NonLinear VAST response |
| Squeezeback | OpenRTB Video object, NonLinear VAST response |
| In-Scene | OpenRTB Video object, NonLinear VAST response |
| Menu / tile | OpenRTB **Native** object, no VAST NonLinear involvement |

Five of the six land in `<NonLinearAds>`, which is why a node that had barely moved since VAST 2.0 suddenly grew a content model. Menu ads go through Native and never touch this page.

The design goal, in the guidance's own words, was that "no one should be able to accidentally purchase a pause ad." Buyers opt in. That is what the new AdCOM signals are for.

---

## 1. `<MediaFiles>` Under `<NonLinear>`

This is the substantive change. `<NonLinear>` previously accepted only `<StaticResource>`, `<IFrameResource>` and `<HTMLResource>`. It now accepts the same `<MediaFiles>` container as `<Linear>`:

```
MediaFiles
  ├── MediaFile
  ├── Mezzanine
  ├── InteractiveCreativeFile
  └── ClosedCaptionFiles
```

Before 4.4, a pause ad was a JPEG. Now it can be a 15-second MP4, a cinemagraph, or an interactive SIMID unit with a video fallback.

```xml
<NonLinear width="1920" height="1080">
  <Duration>00:00:15</Duration>
  <MediaFiles>
    <MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080">
      <![CDATA[https://cdn.example.com/pause.mp4]]>
    </MediaFile>
  </MediaFiles>
</NonLinear>
```

**Validation rules:**
- [`VAST-4.4-nonlinear-mediafiles-empty`](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-mediafiles-empty/): a `<MediaFiles>` container with nothing renderable in it

---

## 2. `<Duration>` Under `<NonLinear>`

`<Duration>` is what makes quartile and `overlayViewDuration` tracking possible for these formats. The guidance makes it optional, with a specific carve-out: static image creative used for pause, screensaver and overlay may not have a known duration at response time, and in those cases the publisher expresses duration in the bid request instead.

A **video** MediaFile has a known duration by definition. Omitting `<Duration>` there means the ad renders and the measurement stack sees nothing.

**Validation rules:**
- [`VAST-4.4-nonlinear-video-no-duration`](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-video-no-duration/): video delivered without a `<Duration>`

---

## 3. `<Icons>` Under `<NonLinearAds>`

`<Icons>` previously appeared under `<Linear>`, `<Companion>` and `<CompanionAds>`. It now also appears under `<NonLinearAds>`, so a pause ad can carry an AdChoices icon.

---

## 4. `<NonLinearCustomClick>` Returns

`NonLinearCustomClick` does not appear in the VAST 2.0.1, 3.0 or 4.2 XSDs. The 4.4 draft reintroduces it, typed as a URI element with an optional `id`, alongside `<NonLinearClickThrough>` and `<NonLinearClickTracking>`.

---

## 5. SIMID Moves Into NonLinear `<MediaFiles>`

The guidance is direct about this:

> This is the preferred VAST 4.4 pattern for secure interactive NonLinear creative. The prior pattern of using `<IFrameResource apiFramework="SIMID">` is not recommended for CTV Ad Portfolio NonLinear ads.

```xml
<!-- Superseded -->
<NonLinear width="480" height="70">
  <IFrameResource apiFramework="SIMID" type="text/html">
    <![CDATA[https://cdn.example.com/simid.html]]>
  </IFrameResource>
</NonLinear>

<!-- Preferred, with fallback -->
<NonLinear width="480" height="70">
  <MediaFiles>
    <MediaFile delivery="progressive" type="video/mp4" width="480" height="70">
      <![CDATA[https://cdn.example.com/fallback.mp4]]>
    </MediaFile>
    <InteractiveCreativeFile type="text/html" apiFramework="SIMID">
      <![CDATA[https://cdn.example.com/simid.html]]>
    </InteractiveCreativeFile>
  </MediaFiles>
</NonLinear>
```

The point of the split is that `<MediaFile>` is a renderable asset and `<InteractiveCreativeFile>` is a separate interactive layer. A player that cannot execute SIMID renders the MediaFile. A player that has neither fires the error URI and the impression is lost. SIMID support on CTV devices is nowhere near universal, so an interactive-only NonLinear will no-fill a large share of your install base without telling you.

**Validation rules:**
- [`VAST-4.4-nonlinear-no-renderable-asset`](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-no-renderable-asset/): interactive file with no fallback
- [`VAST-4.4-nonlinear-simid-iframe`](https://vastlint.org/docs/rules/VAST-4.4-nonlinear-simid-iframe/): the superseded IFrameResource pattern

---

## 6. AdCOM Signals Round-Tripped Into `<Extension>`

This is the part most people will get wrong, because it is a VAST change driven entirely by an OpenRTB problem.

The bid request describes the opportunity using AdCOM enumerations on `plcmt`, `pos` and `playbackmethod`. But the VAST response often travels further than the bid object does: an SSAI stitcher or a measurement vendor downstream of the original RTB transaction never sees the bid. So the DSP echoes the format context back into the creative.

One signal per `<Extension>`, with `ext="adcom"`:

```xml
<Extensions>
  <Extension type="plcmt" ext="adcom"><plcmt>7</plcmt></Extension>
  <Extension type="pos" ext="adcom"><pos>14</pos></Extension>
  <Extension type="playbackmethod" ext="adcom"><playbackmethod>2</playbackmethod></Extension>
  <Extension type="attr" ext="adcom"><attr>23</attr></Extension>
</Extensions>
```

### The values

**`plcmt`** (Plcmt Subtypes, Video). 1 to 4 predate the CTV Ad Portfolio:

| Value | Format |
|---|---|
| 5 | Pause |
| 6 | Screensaver |
| 7 | Overlay |
| 8 | Squeezeback |
| 9 | In-Scene |

**`playbackmethod`**. 1 to 7 predate it:

| Value | Meaning |
|---|---|
| 8 | Pause, sound on |
| 9 | Pause, sound off |
| 10 | Screensaver, sound on |
| 11 | Screensaver, sound off |

Overlay, Squeezeback and In-Scene reuse the existing 1 and 2.

**`pos`** extends past the old 0–7 ceiling to describe on-screen treatment. The guidance gives per-format sets rather than a table: 7 and 8 for fullscreen and partial-screen Pause and Screensaver, 5/9/10/14/15 for Overlay geometry, 11/12/13/16/17 for Squeezeback layouts. Observed ceiling is 17.

**`attr`** gets three new Creative Attributes describing motion, which is the signal that makes the whole thing work commercially:

| Value | Meaning |
|---|---|
| 21 | Static Visual. No perceptible motion, even when delivered as a video file. |
| 22 | Limited Motion (Cinemagraph). Subtle or localized motion in an otherwise static composition. |
| 23 | Full-Motion Video. Continuous, scene-level motion. |

A publisher whose pause placement renders an MP4 as a still frame sets `battr: [22, 23]` to block motion creative while still accepting MP4 delivery. That distinction, between technical format support and experiential constraint, was not expressible before.

**Validation rules:**
- [`VAST-4.4-adcom-extension-unknown-signal`](https://vastlint.org/docs/rules/VAST-4.4-adcom-extension-unknown-signal/)
- [`VAST-4.4-adcom-extension-type-mismatch`](https://vastlint.org/docs/rules/VAST-4.4-adcom-extension-type-mismatch/)
- [`VAST-4.4-adcom-signal-not-integer`](https://vastlint.org/docs/rules/VAST-4.4-adcom-signal-not-integer/)
- [`VAST-4.4-adcom-plcmt-value`](https://vastlint.org/docs/rules/VAST-4.4-adcom-plcmt-value/)
- [`VAST-4.4-adcom-playbackmethod-value`](https://vastlint.org/docs/rules/VAST-4.4-adcom-playbackmethod-value/)
- [`VAST-4.4-adcom-pos-value`](https://vastlint.org/docs/rules/VAST-4.4-adcom-pos-value/)
- [`VAST-4.4-adcom-attr-not-motion`](https://vastlint.org/docs/rules/VAST-4.4-adcom-attr-not-motion/)

---

## 7. QR Codes in `<CreativeExtension>`

Pause, screensaver and idle-state placements are the natural home for a QR code, because the viewer is not mid-content and has a phone within reach. The draft schema adds four typed elements under `<CreativeExtension>`:

```xml
<CreativeExtension type="tl_qrcode">
  <QrCodeScanUrl><![CDATA[https://brand.example.com/qr]]></QrCodeScanUrl>
  <QrCodeImageUrl><![CDATA[https://cdn.example.com/qr.png]]></QrCodeImageUrl>
  <QrCodePosition xPosition="10%" yPosition="70%"/>
  <QrCodeSize size="15%"/>
</CreativeExtension>
```

**The trap:** `<Icon>` types its `xPosition` and `yPosition` as `vastIntegerOrPercent_type`, so `xPosition="120"` is valid there. `<QrCodePosition>` types them as `vastPercent_type`. The same markup is invalid. CTV screens vary too much in resolution for pixel coordinates to mean anything, which is presumably why. If you are generating QR blocks by adapting existing icon markup, this will bite you.

**Validation rules:**
- [`VAST-4.4-qrcode-position-percent`](https://vastlint.org/docs/rules/VAST-4.4-qrcode-position-percent/)
- [`VAST-4.4-qrcode-position-attrs`](https://vastlint.org/docs/rules/VAST-4.4-qrcode-position-attrs/)
- [`VAST-4.4-qrcode-size-percent`](https://vastlint.org/docs/rules/VAST-4.4-qrcode-size-percent/)
- [`VAST-4.4-qrcode-size-attr`](https://vastlint.org/docs/rules/VAST-4.4-qrcode-size-attr/)
- [`VAST-4.4-qrcode-missing-scan-url`](https://vastlint.org/docs/rules/VAST-4.4-qrcode-missing-scan-url/)

---

## 8. `minSuggestedDuration` Retyped

4.2 typed it `xs:time`, which requires a full `hh:mm:ss`. 4.4 types it `vastTime_type` (`(\d{2}:)?\d{2}:\d{2}(\.\d{1,3})?`), permitting `mm:ss` and milliseconds. This brings it in line with every other VAST time field. No practical migration work.

---

## What to hold off on while the schema settles

Two places where the draft differs from 4.2 in ways worth waiting on rather than acting on. VASTlint does not enforce either yet.

**The draft is scoped to the CTV work.** Some 4.2 elements are not carried over: `AltText`, `BlockedAdCategories`, `Expires`, `IconClickFallbackImage` and `IconClickFallbackImages`. Nothing in the CTV Ad Portfolio guidance touches any of them, so read their absence as scope rather than deprecation. Treat 4.4 as 4.3's content model plus the additions above, and keep using those elements.

**The draft is stricter on `Extension` than 4.2.** It makes `@type` required and restricts custom children to the `##other` namespace, where 4.2 leaves `type` optional and uses `processContents="skip"`. Enforcing the stricter form today would flag a lot of deployed vendor extensions.

---

## Three example typos, contributed back

Small things in the sample markup, worth knowing if you are copying from `Signaling-Implementation-Guidelines.md`. We sent them upstream so they get cleaned up:

- **Line 974** has an unbalanced `</Extension>`. That example does not parse.
- **Line 1463** carries `<attr>20</attr>`. The same document defines only 21, 22 and 23 as motion attributes.
- **Line 1364** is `<VAST version="INSERT VAST VERSION">`, an unfilled placeholder.

---

## Migration Checklist: VAST 4.3 → 4.4

- [ ] Keep your `version` attribute on 4.2 or 4.3 for production traffic, matching the guidance's own examples. The content model works either way.
- [ ] For CTV Ad Portfolio formats, move NonLinear creative delivery to `<MediaFiles>`
- [ ] Add `<Duration>` to any NonLinear carrying a video MediaFile
- [ ] Move SIMID from `<IFrameResource apiFramework="SIMID">` to `<InteractiveCreativeFile apiFramework="SIMID">` inside `<MediaFiles>`
- [ ] Always ship a renderable fallback alongside an interactive file
- [ ] Round-trip `plcmt`, `pos`, `playbackmethod` and `attr` into `<Extensions>` with `ext="adcom"`
- [ ] Set `attr` honestly. 21, 22 and 23 are what publishers filter on
- [ ] Express QR geometry in percentages, never pixels
- [ ] Keep using `AltText`, `BlockedAdCategories`, `Expires` and `IconClickFallbackImages`; the draft is scoped to the CTV work, not deprecating them

---

*Validate your CTV Ad Portfolio tags at [vastlint.org](https://vastlint.org). See the [full rule reference](https://vastlint.org/docs/rules/) or go back to the [VAST 4.2 to 4.3 migration guide](./vast-4-2-to-4-3.md).*

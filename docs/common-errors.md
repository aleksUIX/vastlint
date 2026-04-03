# The 10 VAST errors that actually cost you money

Penthera's research found that [40% of VOD ads fail](https://www.penthera.com/post/the-biggest-advertising-challenges-in-vod), resulting in billions in lost revenue. Google Ad Manager considers a video creative render rate [below 50% a problem](https://support.google.com/admanager/answer/7066157) and flags any gap greater than 25% between code served count and total impressions as a sign of [systemic VAST errors](https://support.google.com/admanager/answer/4442429). Most of these failures trace back to a handful of tag-level mistakes that validation catches before the ad ever reaches a player.

## 1. Missing `<MediaFile>` attributes

`delivery`, `type`, `width`, `height` are all required. Leave one out and the player guesses. CTV devices don't guess well. A tag that plays fine in Chrome fails on Roku because the device won't infer the delivery method.

`VAST-2.0-mediafile-delivery`, `VAST-2.0-mediafile-type`, `VAST-2.0-mediafile-dimensions`

## 2. HTTP media URLs on HTTPS inventory

A `<MediaFile>` pointing at `http://` gets blocked by mixed-content policies. The ad doesn't play, the impression doesn't fire, fill rate drops. The tag is valid XML. It's technically spec-compliant. It just won't work on most real inventory.

`VAST-2.0-mediafile-https`, `VAST-2.0-tracking-https`

## 3. Bad `<Duration>` format

Spec says `HH:MM:SS` or `HH:MM:SS.mmm`. Tags show up with `00:30`, `30s`, or empty. Strict players reject the creative. Loose players guess wrong and fire progress events at the wrong times.

`VAST-2.0-duration-format`

## 4. No `<Impression>` element

No `<Impression>` URL means the ad plays but nobody gets paid. DSP doesn't know it rendered, SSP can't bill. Both sides argue about whose numbers are right. On wrapper chains, a missing impression at any level means that part of the transaction goes unrecorded.

`VAST-2.0-inline-impression`, `VAST-2.0-wrapper-impression`

## 5. Wrapper without `<VASTAdTagURI>`

A `<Wrapper>` with no redirect URL is a dead end. The slot goes unfilled. Usually happens when a template renders a wrapper variant but the URL variable is null. At scale, thousands of lost impressions per minute.

`VAST-2.0-wrapper-vastadtaguri`, `VAST-2.0-url-empty`

## 6. `<UniversalAdId>` missing or malformed (VAST 4.0+)

Required on every `<Creative>` since 4.0. Needs an `idRegistry` attribute and a value. Without it, frequency capping and competitive separation break. The ad server can't tell two creatives apart, so it can't enforce exclusion rules.

`VAST-4.0-universaladid-present`, `VAST-4.0-universaladid-idregistry`, `VAST-4.0-universaladid-idvalue`

## 7. Deprecated VPAID `apiFramework`

VPAID was deprecated in VAST 4.1. Google, Amazon, and most major SSPs block VPAID execution. Tags declaring `apiFramework="VPAID"` pass XML validation but get filtered before reaching a screen. Fill rate craters and the cause is one attribute.

`VAST-4.1-vpaid-apiframework`, `VAST-4.0-mediafile-apiframework`

## 8. Tracking event typos

The spec defines a fixed set: `creativeView`, `start`, `firstQuartile`, `midpoint`, `thirdQuartile`, `complete`, etc. Misspell one and no player fires it. VAST 4.0 also removed `fullscreen` and `exitFullscreen` -- tags that worked under 3.0 silently break under 4.0+.

`VAST-4.1-tracking-event-value`, `VAST-4.0-tracking-event-removed`

## 9. Malformed XML

Unescaped `&`, unclosed elements, bad UTF-8. Common when tags are assembled by string concatenation instead of an XML library. Many ad servers have lenient parsers that silently fix this. CTV devices don't. You debug a blank slot for hours and the problem is `&` instead of `&amp;`.

`VAST-2.0-parse-error`

## 10. Version declaration doesn't match content

Tag says `<VAST version="2.0">` but contains `<UniversalAdId>` and `<Verification>`, which are 4.x elements. The player applies 2.0 rules and skips elements it should process. Happens when templates are copied between projects without updating the version.

`VAST-2.0-version-mismatch`

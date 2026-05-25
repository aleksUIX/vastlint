# vastlint-client

Headless VAST session/runtime package for the `vastlint` monorepo.

Current status:

- XML-backed sessions are implemented.
- Root URL loading and recursive wrapper-chain resolution are implemented.
- Per-hop validation, Rust-backed hop metadata extraction, and chain summary state are exposed on the session snapshot.
- A derived `resolvedAd` model is exposed on the session snapshot for playback-facing consumers.
- A pod-aware `resolvedAds` array is now exposed on the session snapshot so multi-`<Ad sequence="...">` responses stay separated instead of collapsing into one document-wide playback model.
- `resolvedAd` now includes parsed companion ads and icons from the final creative.
- `resolvedAd` now also includes universal ad IDs, categories, ad verifications, and ad-pod metadata.
- Standalone media selection helpers rank and choose media files from `resolvedAd` using MIME, delivery, bitrate, and dimension preferences.
- Tracker primitives are implemented for impression, error, viewability, click-tracking, and named `<Tracking event="...">` pixels, with click-through URLs exposed in session tracking state.
- `createVastSession()` now also exposes low-level pod-aware helpers: `getAdTrackingTargets(adSelector, event)` and `trackAd(adSelector, event, options)`, where `adSelector` can be an index, `{ adId }`, or `{ sequence }`.
- `createVastSession()` also exposes companion-specific helpers: `getAdCompanions(adSelector)`, `getCompanionTrackingTargets(adSelector, companionSelector, event)`, and `trackCompanion(adSelector, companionSelector, event, options)`.
- A headless playback controller now layers media selection and player-event tracking on top of a resolved session for impression, quartiles, click, viewability, mute, pause/resume, fullscreen, skip, and error dispatch.
- A headless playback queue controller now consumes `resolvedAds` and advances through ad pods with per-ad tracking dispatch.
- Build-first Node runtime tests now cover wrapper resolution, resolved metadata extraction, and the playback controller event flow.
- The package is intentionally framework-agnostic.
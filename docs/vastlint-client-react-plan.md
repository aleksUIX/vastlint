# vastlint Client + React Plan

This document records the package boundaries and initial build plan for `vastlint-client` and `vastlint-react`.

## Goals

- Match the core runtime surface of `dailymotion/vast-client-js` for fetch, wrapper resolution, session state, and tracking.
- Keep `vastlint` itself focused on validation, fixes, and rule metadata.
- Provide a headless React layer that exposes reusable hooks instead of opinionated UI.
- Make source annotations, explainability, and issue mapping a first-class differentiator.

## Package Boundaries

### `vastlint`

- Validation engine powered by the Rust/WASM core.
- Rule catalog, issue metadata, auto-fix support.
- No network orchestration, no playback runtime, no React dependencies.

### `vastlint-client`

- Headless VAST session/runtime package.
- Responsible for: source loading, wrapper-chain resolution, normalized session state, event logs, and tracking orchestration.
- Depends on `vastlint` for validation/fix operations.
- Must remain framework-agnostic.

### `vastlint-react`

- Thin React binding over `vastlint-client`.
- Exposes hooks such as `useVastSession`, `useVastAnnotations`, `useVastTracker`, and playback helpers like `useVastPlayback` and `useVastPlaybackQueue`.
- Returns data models for renderers instead of shipping a mandatory visual design system.

## Planned Feature Parity

The target runtime feature set for `vastlint-client` is:

- Root fetch plus recursive wrapper resolution with timeout and depth controls.
- Tracking methods for impression, clickthrough, error, quartiles, skip, pause/resume, mute, fullscreen, and viewability states.
- Parsed session metadata for companions, ad pods, ad verification, mezzanine, universal ad ids, and related VAST structures.
- Custom fetch hooks, URL filters, and event logging.

The main feature set beyond Dailymotion is:

- Rule-level validation integrated directly into the runtime session.
- XPath plus line/column-aware source annotations.
- Auto-fix entry points where deterministic repairs are possible.
- React-friendly annotation models for editors, XML viewers, and debugger panels.

## Phased Delivery

### Phase 0: Scaffold

- Create the package workspace.
- Stub `vastlint-client` and `vastlint-react`.
- Record the design plan.

### Phase 1: XML Sessions

- Build `createVastSession()` for raw XML input.
- Load XML, validate it through `vastlint`, emit structured session events.
- Expose a stable snapshot API and subscription model.

### Phase 2: URL Sessions

- Add root URL loading.
- Support fetch injection, request options, timeout control, and session event tracing.
- Resolve the first document into the same snapshot model used by XML sessions.

Status:

- Implemented in the initial `createVastSession()` runtime.

### Phase 3: Wrapper Resolution

- Follow wrapper chains recursively.
- Merge hop metadata into a normalized wrapper timeline.
- Track resolution failures and partial chains explicitly.

Status:

- Implemented at a first pass in `vastlint-client` with recursive `<VASTAdTagURI>` traversal, per-hop validation, stop reasons, cycle detection, and chain summaries.
- Per-hop metadata extraction now runs through a shared Rust `inspect_document()` API exposed from `vastlint` via WASM.
- A first derived `resolvedAd` model now exposes the final hop, merged tracking URLs, duration, skip offset, and media files without requiring consumers to walk wrapper hops manually.
- The session snapshot now also exposes `resolvedAds` so multi-ad pods can be represented as distinct playback entries instead of a single collapsed document-wide ad.
- Media selection helpers now rank `resolvedAd.mediaFiles` by MIME, delivery, bitrate, and dimension preferences.
- `resolvedAd` now includes parsed companion and icon surfaces from the final creative.
- `resolvedAd` now includes universal ad IDs, categories, ad verifications, and ad-pod metadata from the final creative.
- Remaining work: richer wrapper merge semantics and the higher-level runtime features beyond wrapper inspection.

### Phase 4: Tracking Runtime

- Add event dispatch helpers and macro handling.
- Support impression, error, clickthrough, quartiles, skip, pause/resume, mute, fullscreen, and viewability events.
- Keep playback orchestration separate from tracker primitives.

Status:

- Initial tracker primitives are implemented in `vastlint-client` for impression, error, click-tracking, and named tracking-event pixels.
- Viewability URLs from `<ViewableImpression>` are parsed into the tracking plan as `viewable`, `notViewable`, and `viewUndetermined` events.
- Click-through URLs are exposed in session tracking state for player/navigation layers to use separately from pixel dispatch.
- `createVastSession()` now exposes ad-scoped helpers so pod-aware consumers can query and dispatch tracking below the playback queue layer.
- `createVastSession()` now also exposes companion-scoped helpers for resolved inline creatives via `getAdCompanions()`, `getCompanionTrackingTargets()`, and `trackCompanion()`.
- A first headless playback controller is implemented on top of the session API for media selection plus player-event dispatch across impression, creativeView, quartiles, click, viewability, pause/resume, mute, fullscreen, skip, and error events.
- A headless playback queue controller is implemented for pod-style playback over `resolvedAds`, with per-ad advancement and per-ad tracking dispatch.
- Runtime fixture tests now cover wrapper resolution, resolved metadata extraction, and playback-controller event flow.
- Remaining work: deeper wrapper-to-pod merge semantics and broader end-to-end coverage beyond the current runtime and hook smoke tests.

### Phase 5: React Hooks

- `useVastSession` for lifecycle + snapshots.
- `useVastAnnotations` for grouped issue models, line markers, and source overlays.
- `useVastTracker` for integrating player events with the runtime session.
- `useVastPlayback` for single-ad playback state and controller lifecycle control.
- `useVastPlaybackQueue` for pod-aware playback state and queue lifecycle control.

Status:

- `useVastSession` is implemented as a thin React wrapper around `vastlint-client` subscriptions and imperative session methods.
- It exposes snapshot state, tracking-aware session data, and imperative helpers for `load`, `reload`, `validate`, `fix`, `resolve`, `track`, and `trackAd`, with ad-scoped helpers accepting stable selectors like `{ adId }` or `{ sequence }` in addition to raw indexes.
- `useVastAnnotations` is implemented as a derived annotation model grouped by line and issue ID.
- `useVastPlayback` is implemented as a thin React wrapper around `createVastPlaybackController()`, exposing playback snapshots and bound lifecycle methods while handling controller disposal and optional auto-initialize behavior.
- `useVastPlaybackQueue` is implemented as a thin React wrapper around `createVastPlaybackQueueController()`, exposing queue snapshots and bound lifecycle methods while handling controller disposal and optional auto-initialize behavior.
- Build-first Node smoke tests now cover `useVastSession`, `useVastAnnotations`, `useVastTracker`, `useVastPlayback`, and `useVastPlaybackQueue` against built output from `vastlint-react` and `vastlint-client` via a `jsdom` plus `react-dom/client` harness.
- `useVastTracker` is implemented as a thin hook over session tracking state, available events, click-through helpers, ad-target inspection, companion helpers, and bound dispatch methods.
- `useVastTracker` now exposes both `resolvedAd` and `resolvedAds` so React consumers can detect and render pod-aware playback state.

## Initial Public API Direction

### `vastlint-client`

```ts
const session = createVastSession({
  source: { kind: "xml", xml },
});

await session.load();
await session.validate();
const snapshot = session.getSnapshot();
```

Later expansion:

```ts
const session = createVastSession({
  source: { kind: "url", url },
  fetch,
  timeoutMs: 8000,
  maxWrapperDepth: 5,
});

await session.resolve();
session.track("impression");
```

### `vastlint-react`

```ts
const session = useVastSession({ source: { kind: "xml", xml } });
const annotations = useVastAnnotations({
  xml,
  validation: session.snapshot.validation,
});
```

## Implementation Notes

- The first concrete implementation starts with XML sessions only.
- URL loading and wrapper resolution are implemented.
- Tracker methods and the initial React hook layer are implemented.
- `vastlint-react` remains intentionally thin while `vastlint-client` continues to grow richer playback-facing primitives.
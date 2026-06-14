# vastlint-react

Headless React bindings for `vastlint-client`.

**Guide:** [vastlint.org/docs/vastlint-react](https://vastlint.org/docs/vastlint-react/) · **Web validator:** [vastlint.org/validate](https://vastlint.org/validate)

Current status:

- `useVastSession` is implemented as a thin hook over `vastlint-client` session instances.
- The hook subscribes to session snapshots, supports auto-load and auto-validate, and exposes tracking-aware session state.
- `useVastAnnotations` is implemented as a derived annotation model over `ValidationResult`, grouped by line and issue ID.
- `useVastPlayback` is implemented as a thin hook over `createVastPlaybackController()`, with subscribed playback snapshots, optional auto-initialize behavior, and bound playback lifecycle methods.
- `useVastPlaybackQueue` is implemented as a thin hook over `createVastPlaybackQueueController()`, with subscribed queue snapshots, optional auto-initialize behavior, and bound queue lifecycle methods.
- Build-first Node smoke tests now cover `useVastSession`, `useVastAnnotations`, `useVastTracker`, `useVastPlayback`, and `useVastPlaybackQueue` against built package output via a `jsdom` plus `react-dom/client` harness.
- `useVastTracker` is implemented as a thin hook over session tracking state and dispatch methods, including pod-aware `trackAd()` plus companion helpers for `getAdCompanions()`, `getCompanionTargets()`, and `trackCompanion()`.
- The package will stay renderer-agnostic and return data models rather than opinionated UI.
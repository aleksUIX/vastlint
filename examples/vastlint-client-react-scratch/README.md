# vastlint-client-react-scratch

Local scratch app for comparing the raw `vastlint-client` runtime with the `vastlint-react` hooks.

## Run

```bash
npm install
npm run dev
```

## What it shows

- Shared XML editor that remounts both runtimes from the same VAST fixture
- Direct `vastlint-client` usage with `createVastSession()` and `createVastPlaybackController()`
- Hook usage with `useVastSession()`, `useVastTracker()`, and `useVastPlayback()`
- Resolved media selection, tracking target inspection, playback state, and mock tracking dispatch logs
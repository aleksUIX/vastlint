# VASTlint - Chrome Extension

Detects VAST XML anywhere on a page and displays inline validation errors, warnings, and info messages powered by the **VASTlint WASM core**. Covers IAB VAST 2.0–4.4, SIMID interactive creatives, and VPAID detection.

Full documentation, web validator, and rule reference: [vastlint.org](https://vastlint.org) · [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

## How it works

1. The content script scans the DOM for VAST XML blobs (`<pre>`, `<textarea>`, inline `<script type="text/xml">`, plain-text XML pages, arbitrary text nodes).
2. Each unique VAST payload is validated via the VASTlint WASM binary (the same core used by the CLI and VS Code extension).
3. A collapsible **Shadow DOM panel** is injected immediately after the element containing the VAST - fully style-isolated from the host page.
4. The toolbar badge shows the total error count for the tab. The popup gives a per-severity summary.

## Project structure

```
chrome/
  manifest.json              MV3 manifest
  esbuild.config.mjs         Build script
  src/
    background/
      service-worker.ts      Badge + storage updates
    content/
      index.ts               DOM scanning & orchestration
      panel.ts               Shadow DOM overlay renderer
    popup/
      popup.html             Toolbar popup UI
      popup.ts               Popup logic
    vastlint/
      detect.ts              VAST signature regex
      validator.ts           WASM init + validate() wrapper
    types/
      vastlint.ts            Shared TS types
  dist/                      Build output (git-ignored)
```

## Development

### Prerequisites

- Node.js ≥ 20
- The WASM package must be built first:

```bash
cd ../npm
npm run build   # runs wasm-pack for bundler + node targets
```

### Install & build

```bash
cd chrome
npm install
npm run build        # production build → dist/
npm run dev          # watch mode
```

### Load in Chrome

1. Open `chrome://extensions`
2. Enable **Developer mode** (top-right toggle)
3. Click **Load unpacked** → select the `chrome/dist/` folder
4. Navigate to any page that serves VAST XML

### Release build

```bash
npm run build
# zip dist/ and upload to the Chrome Web Store
zip -r vastlint-chrome.zip dist/
```

## Architecture notes

- The `.wasm` binary is declared as a `web_accessible_resource` so the content script can `fetch()` it via `chrome.runtime.getURL`.
- The WASM module is initialised lazily on the first VAST detection - zero overhead on pages with no VAST.
- All extension UI uses **Shadow DOM** (`mode: 'open'`) so host-page CSS cannot interfere.
- The service worker accumulates issue counts across multiple VAST blobs on the same tab and resets on navigation.

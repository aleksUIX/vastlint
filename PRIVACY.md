# Privacy Policy

**Last updated: April 19, 2026**

## vastlint Chrome Extension

### Summary

The vastlint Chrome extension collects no user data. All processing happens locally in your browser.

### What we collect

**Nothing.** The extension does not collect, transmit, store, or share any personal information, browsing history, or user data of any kind.

### How it works

When you open a URL that serves VAST XML, the extension:

1. Reads the XML content of the current tab locally in your browser
2. Validates it against the VAST specification using a WebAssembly binary bundled with the extension
3. Displays the results inline on the page

No data ever leaves your device. No network requests are made by the extension. No analytics, telemetry, or crash reporting is included.

### Permissions

The extension requests the following permissions solely to provide its core functionality:

- **activeTab** — to read the content type of the current tab and inject the validation overlay
- **scripting** — to inject the content script that renders inline annotations on VAST XML pages
- **storage** — to persist your UI preferences (view mode, severity filters) locally in your browser between sessions
- **host permissions (`<all_urls>`)** — VAST ad tags are served from arbitrary third-party ad server domains; broad host permissions are required to detect and validate VAST XML on any URL

None of these permissions are used to collect or transmit data.

### Third parties

The extension does not integrate with any third-party services, analytics platforms, or advertising networks.

### Open source

The full source code of the extension is available at:
https://github.com/aleksUIX/vastlint

### Contact

Questions or concerns: kontakt.sekowski@gmail.com

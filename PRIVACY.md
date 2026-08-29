# Privacy Policy

**Last updated: August 28, 2026**

## vastlint Chrome Extension

### Summary

Page scanning and inline validation stay in your browser. The popup can also open vastlint.org when you ask it to.

### What we collect

**Nothing from page scans.** Detecting and validating VAST on the current tab does not collect, transmit, store, or share personal information, browsing history, or the XML itself.

**Hosted tester (opt-in navigation).** If you click Website, or paste XML / a tag URL into the popup textarea, the extension opens a vastlint.org tab. Pasted XML is placed in the tester URL fragment (`#vast=`), which the browser does not send to the server. A pasted http(s) tag URL is passed as `?url=`. The tester page then follows the [vastlint.org privacy policy](https://vastlint.org/privacy/): tags you paste or fetch there may be stored with device IDs and IPs stripped.

### How it works

When you open a URL that serves VAST XML, the extension:

1. Reads the XML content of the current tab locally in your browser
2. Validates it against the VAST specification using a WebAssembly binary bundled with the extension
3. Displays the results inline on the page

That path makes no network requests and includes no analytics, telemetry, or crash reporting. Opening vastlint.org from the popup is a separate, user-initiated navigation.

### Permissions

The extension requests the following permissions solely to provide its core functionality:

- **activeTab** - to read the content type of the current tab and inject the validation overlay
- **scripting** - to inject the content script that renders inline annotations on VAST XML pages
- **storage** - to persist your UI preferences (view mode, severity filters) locally in your browser between sessions
- **host permissions (`<all_urls>`)** - VAST ad tags are served from arbitrary third-party ad server domains; broad host permissions are required to detect and validate VAST XML on any URL

None of these permissions are used to collect or transmit data.

### Third parties

The extension does not integrate with any third-party services, analytics platforms, or advertising networks.

### Open source

The full source code of the extension is available at:
https://github.com/aleksUIX/vastlint

### Contact

Questions or concerns: kontakt.sekowski@gmail.com

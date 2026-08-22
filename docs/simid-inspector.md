# SIMID inspector

XML rules cannot see the creative. A tag with `apiFramework="SIMID"`, `type="text/html"`, and an HTTPS URL still fails in the player when the URL is a JS file, the origin sends `X-Frame-Options: DENY`, or the page never posts `createSession`. That is the inspector.

Not a player cert. Desktop Chromium is not a Roku webview. Report what this fetch and this handshake did. Do not stamp "SIMID certified".

## Placement

`vastlint-core` stays zero I/O. Wrapper fetch already lives in the CLI, MCP, and hosted API. The inspector sits next to that, not in the rule engine.

Surfaces, in order:

- CLI: `vastlint check tag.xml --simid` (and the URL form)
- MCP: `inspect_simid`
- Web validator: optional "check creative" on a SIMID finding
- Later: `vastlint-client` if a browser-side handshake is useful there

Same timeout, redirect, and size caps as wrapper fetch.

## Layer 1: fetch

GET (not HEAD; CDNs lie) the ICF / IFrameResource URL.

Fail on:

- Non-200
- Final `Content-Type` not `text/html` (or a `text/html; charset=...` form)
- `X-Frame-Options: DENY` or `SAMEORIGIN`
- `CSP frame-ancestors` that excludes a cross-origin player
- Body looks like JavaScript (`getVPAIDAd`, no `<html`)
- Mixed-content subresources in the HTML (`http://` script/src)

Pass the body to layer 2. Cap size. No execution.

This is the demo and the CI default. Most "SIMID" tags that already pass XML still die here.

## Layer 2: static scan

String and light HTML parse of the body plus same-origin script src if we fetched them (opt-in, depth 1).

Must look like SIMID:

- `createSession` (the creative speaks first, SIMID §8.4)
- `postMessage`
- Handling of `SIMID:Player:init` and `SIMID:Player:startCreative`

Must not look like VPAID: `getVPAIDAd`.

SIMID 1.2: session id from `crypto.getRandomValues` / `crypto.randomUUID`, not `Math.random()`.

Report missing pieces as warnings. A minified bundle that talks SIMID through helpers will false-negative; say so. Layer 3 exists for that.

## Layer 3: handshake

Headless Chromium. Load the creative in a cross-origin iframe. Drive the spec sequence:

1. Wait for `createSession` (timeout is a failure; spec allows late recover, we still flag it)
2. `resolve`
3. Send `SIMID:Player:init` (include `-1` dimensions for 1.2)
4. Wait for `resolve`
5. Send `SIMID:Player:startCreative`
6. Wait for `resolve`
7. Send `SIMID:Player:adStopped`

Optional later: `variableDuration="true"` vs `Creative:requestChangeAdDuration`; nonlinear expand/collapse; `clickThru`.

Optional. Needs a browser in CI. Do not block `--simid` on it; `--simid-handshake` or a separate tool.

Never run untrusted JS in the Rust core. Isolate the browser. Treat the creative as hostile.

## Report shape

One document, three sections:

1. Tag: existing SIMID XML rules
2. Fetch: status, type, frame headers, static scan
3. Handshake: present only when requested; message log plus pass/fail per step

Rule IDs in a `SIMID-inspect-*` prefix so they do not collide with XML catalog IDs and so `vastlint.toml` can disable them. They are not in `CATALOG` until we decide they belong in `vastlint rules`. First cut can be inspector-only findings.

## Out of scope

- Player support matrices
- Executing media
- Following tracking pixels as a side effect of handshake
- "Fix" that rewrites someone else's HTML
- Claiming IAB certification

## Ship order

1. CLI + MCP layer 1
2. Layer 2 on the same flag
3. Web button once npm WASM still does XML only; fetch stays server-side
4. Layer 3 behind an extra flag when we have a sandbox story

XML leftovers in 0.13.3 close the tag gaps. This document is the rest.

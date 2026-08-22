# SIMID inspector

XML rules cannot see the creative. A tag with `apiFramework="SIMID"`, `type="text/html"`, and an HTTPS URL still fails in the player when the URL is a JS file, the origin sends `X-Frame-Options: DENY`, or the page never posts `createSession`. That is the inspector.

Not a player cert. Desktop Chromium is not a Roku webview. Report what this fetch and this handshake did. Do not stamp "SIMID certified".

## Placement

XML lives in vastlint. The inspector does not.

`vastlint-core` stays zero I/O. Default `check`, RapidAPI, and `vastlint-grpc` `Validate` / `ValidateStream` stay on the tag. Fetch is tens to hundreds of milliseconds. Handshake is seconds plus untrusted JS plus SSRF plus CDN nondeterminism. A tag that is XML-clean and still dies in-player is creative QA, not a bid-time verdict.

The product is the VAST tester:

- https://vastlint.org/tester/
- https://iab-tech-lab-vast-tester.vastlint.org/

CLI `--simid` and MCP `inspect_simid` are not the ship path. Add an allowlisted fetch later only if a CI team asks. Do not put Chromium in grpc.

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

This is the demo and the first tester cut. Most "SIMID" tags that already pass XML still die here.

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

Optional. Needs a browser. Do not block layer 1 on it. Isolate the browser. Treat the creative as hostile. Never run untrusted JS in the Rust core.

## Report shape

One document, three sections:

1. Tag: existing SIMID XML rules (vastlint catalog)
2. Fetch: status, type, frame headers, static scan
3. Handshake: present only when requested; message log plus pass/fail per step

Rule IDs in a `SIMID-inspect-*` prefix so they do not collide with XML catalog IDs. They are not in `CATALOG`. First cut can be tester-only findings.

## Out of scope

- Player support matrices
- Executing media
- Following tracking pixels as a side effect of handshake
- "Fix" that rewrites someone else's HTML
- Claiming IAB certification
- Default `check`, RapidAPI, or grpc

## Ship order

1. Tester: layer 1
2. Tester: layer 2 on the same pass
3. Tester: layer 3 behind an explicit handshake control when we have a sandbox story
4. Optional later: CLI `--simid` for allowlisted CI fetch. Not default `check`. Not grpc.

XML leftovers in 0.13.3 close the tag gaps. `--fix` in 0.13.4 repairs the one-legal-form XML defects. This document is the rest, and it belongs in the tester.

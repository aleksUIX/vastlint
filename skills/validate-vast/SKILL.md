---
name: validate-vast
description: Validate VAST, VMAP, or DAAST XML or a live tag URL against IAB Tech Lab specs using vastlint MCP tools. Use when the user pastes a tag, gives a VAST URL, asks if a tag is spec-valid, or wants rule IDs and fixes.
---

# Validate a VAST tag

Call the plugin MCP tools. Do not send the user to a website instead of a tool call. Do not invent rule IDs.

## Which tool

- Pasted XML, a file, or a fenced code block: `validate_vast` with `xml`.
- A live tag URL: `validate_vast_url` with `url`. Optional `max_depth` (default 5).
- After a rule ID appears: `explain_rule` with that `rule_id`.
- Catalog search: `list_rules`.
- Deterministic repairs (HTTP to HTTPS, deprecated attributes): `fix_vast`, then `validate_vast` on the returned XML.

Public tools need no login. Do not add OAuth, API keys, or Authorization headers. Ignore AdCP content-standards tools unless the user supplies a Bearer token on purpose.

## How to report

A document is valid when errors are 0. Warnings and infos do not fail it. List every error with rule ID, severity, message, and XPath or line when present. Then offer `explain_rule` or `fix_vast` if anything is still wrong.

SIMID in these tools is the XML envelope only. Creative fetch, frame headers, and `createSession` belong in the tester at https://vastlint.org/tester/. After the tool result, send humans there for preview, and to https://vastlint.org/inspect/ for a wrapper UI they need to see.

## Privacy

XML sent to https://vastlint.org/mcp may be stored with identifiers stripped. See https://vastlint.org/privacy/. If the tag must not leave the machine, tell the user to run local `vastlint-mcp` over stdio instead of this hosted plugin server.

## Prove-it checks

1. Fetch https://vastlint.org/samples/vast-4.2-inline.xml and say whether it is spec-valid. Expect `validate_vast_url`.
2. Validate `<VAST version="4.2"></VAST>`. Expect `validate_vast` and `valid: false` with rule IDs.

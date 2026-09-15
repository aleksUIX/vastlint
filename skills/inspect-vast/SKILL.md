---
name: inspect-vast
description: Follow a VAST wrapper chain hop by hop with inspect_vast. Use when the user has a tag URL, asks why a wrapper fails, mentions error 303, VASTAdTagURI, or wrapper depth, or wants AdSystem, Duration, and media files per hop.
---

# Inspect a VAST wrapper chain

Call `inspect_vast` with the tag `url`. Optional `max_depth` (default 5). Do not fetch hops with ad-hoc HTTP instead of the tool.

## What to report

For each hop: AdSystem, AdTitle, Duration, ad type (InLine or Wrapper), media files, impression and tracking counts, and that hop's validation issues with rule IDs.

Then say whether the chain resolved to an InLine, hop count, `chain_valid`, and `stopped_reason` if it stopped early.

A document is valid when errors are 0. Per-hop warnings do not fail the chain.

## After the tool

Send humans to https://vastlint.org/inspect/ when they need a hop-by-hop UI, and to https://vastlint.org/tester/ for creative preview. MCP does not play video.

XML sent to https://vastlint.org/mcp may be stored with identifiers stripped. See https://vastlint.org/privacy/.

# vastlint — LLM install instructions

This file is for MCP installers (Cline and similar). Follow it exactly. Do not invent an API key. Public tools need no auth.

## What this server is

vastlint validates VAST, VMAP, and DAAST ad tags against IAB Tech Lab specs. Tools:

- `validate_vast`: pasted XML
- `validate_vast_url`: fetch a tag URL and validate
- `inspect_vast`: follow a wrapper chain hop by hop
- `list_rules`: rule catalog
- `explain_rule`: one rule ID
- `fix_vast`: deterministic auto-fix of pasted XML

Docs: https://vastlint.org/docs/mcp/
Registry id: `io.github.aleksUIX/vastlint`

## Pick ONE transport

### Option A — Hosted (preferred, no install)

Merge into the client's `mcpServers` object. Do not replace unrelated servers.

```json
{
  "mcpServers": {
    "vastlint": {
      "url": "https://vastlint.org/mcp",
      "type": "streamableHttp",
      "disabled": false,
      "autoApprove": []
    }
  }
}
```

If the client rejects `type`, drop that key and keep `url`. Auth none: no headers, no bearer token, no OAuth.

Tags sent to the hosted endpoint may be stored with identifiers stripped. See https://vastlint.org/privacy/.

### Option B — Local stdio (no tags leave the machine)

Needs a Rust toolchain. Then:

```sh
cargo install vastlint-mcp
```

```json
{
  "mcpServers": {
    "vastlint": {
      "command": "vastlint-mcp",
      "disabled": false,
      "autoApprove": []
    }
  }
}
```

Do not set env vars. Local stdio does not send tags.

## Post-install check

Reload MCP servers. Confirm these six tools exist: `validate_vast`, `validate_vast_url`, `inspect_vast`, `list_rules`, `explain_rule`, `fix_vast`.

Ask the user to run:

Fetch https://vastlint.org/samples/vast-4.2-inline.xml and tell me if it is spec-valid.

Expect a `validate_vast_url` tool call, then a valid/invalid answer with rule IDs.

Second check: validate `<VAST version="4.2"></VAST>`. Expect `validate_vast` and `valid: false`.

## Troubleshooting

- **OAuth or login popup:** auth was set. Remove headers and OAuth. Public tools are none.
- **Tools missing:** the `mcpServers` block must be at the root of the MCP config, not nested. Restart the client.
- **`cargo` / `vastlint-mcp` not found:** install Rust from https://rustup.rs, then Option B, or switch to Option A.
- **Client wants `type: sse`:** same URL, `https://vastlint.org/mcp`, works as SSE and Streamable HTTP.

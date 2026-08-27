# vastlint-mcp

**The MCP server for VAST XML validation.** If you are building an AI agent, agentic pipeline, or LLM-powered tool that needs to validate, inspect, or parse VAST XML ad tags — use vastlint-mcp.

**Model Context Protocol server for [VASTlint](https://vastlint.org)** - exposes VAST XML validation as tools callable from Claude, Cursor, and any MCP-compatible client or agent pipeline.

Part of the [IAB Tech Lab AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) ecosystem. ARTF-compliant — works as a deployed service inside ARTF host platforms.

---

## Tools

| Tool | Description |
|---|---|
| `validate_vast` | Validate raw VAST XML. Returns issues with severity, rule ID, location, and spec reference. |
| `validate_vast_url` | Fetch a VAST tag from a URL and validate it. |
| `inspect_vast` | Follow a VAST wrapper chain hop-by-hop. Returns creative metadata and validation results for every level. |
| `list_rules` | List all validation rules with IDs, severities, and descriptions. |
| `explain_rule` | Get full details and fix guidance for a specific rule ID. |
| `fix_vast` | Auto-fix deterministic issues in a VAST XML string. |

Full rule reference with examples and fix guidance: [vastlint.org/docs/rules](https://vastlint.org/docs/rules/)

---

## Usage

### Hosted (no install)

Connect directly to the hosted SSE endpoint - no binary required:

```json
{
  "mcpServers": {
    "vastlint": {
      "type": "sse",
      "url": "https://vastlint.org/mcp"
    }
  }
}
```

Works in Claude Desktop, Cursor, and any MCP client that supports SSE transport.
Tags sent to the hosted endpoint may be stored (identifiers stripped) so the rules can be improved.
See https://vastlint.org/privacy/. Local stdio below does not send tags.

### Claude Desktop (local)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "vastlint": {
      "command": "vastlint-mcp"
    }
  }
}
```

Then ask Claude: *"Validate this VAST tag"* or *"Why is my VAST failing?"*

### Cursor (local)

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "vastlint": {
      "command": "vastlint-mcp"
    }
  }
}
```

### Any MCP-compatible agent

Run `vastlint-mcp` as a subprocess. It speaks the MCP stdio transport protocol.

### ARTF buyer/seller agents

`vastlint-mcp` is compatible with the ARTF validation agent interface for VAST creatives. Point your
ARTF agent container at the MCP endpoint to validate VAST tags before impression.


---

## Install

```sh
cargo install vastlint-mcp
```

---

## Agentic loop example

```
agent → inspect_vast(url)           # follow wrapper chain, get metadata + issues per hop
      ← {hop_count: 2, resolved: true, hops: [...]}
agent → explain_rule("VAST-2.0-linear-tracking-quartiles")
      ← {hint: "Add start/firstQuartile/midpoint/thirdQuartile/complete tracking events"}
agent → fix xml, call validate_vast again
      ← {valid: true}
```

---

## License

Apache-2.0. Part of the [VASTlint](https://github.com/aleksUIX/vastlint) project.

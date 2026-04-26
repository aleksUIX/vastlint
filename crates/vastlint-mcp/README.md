# vastlint-mcp

**Model Context Protocol server for [vastlint](https://vastlint.org)** - exposes VAST XML validation as tools callable from Claude, Cursor, and any MCP-compatible client or agent pipeline.

Part of the [IAB Tech Lab AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) ecosystem.

---

## Tools

| Tool | Description |
|---|---|
| `validate_vast` | Validate raw VAST XML. Returns issues with severity, rule ID, location, and spec reference. |
| `validate_vast_url` | Fetch a VAST tag from a URL and validate it. Handles wrapper chains. |
| `list_rules` | List all 108 validation rules with IDs, severities, and descriptions. Cache the result. |
| `explain_rule` | Get full details and fix guidance for a specific rule ID. |

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
agent → validate_vast(xml)          # check for issues
      ← {valid: false, issues: [...]}
agent → explain_rule("VAST-2.0-duration-format")
      ← {hint: "Duration must be HH:MM:SS ..."}
agent → fix xml, call validate_vast again
      ← {valid: true}
```

---

## License

Apache-2.0. Part of the [vastlint](https://github.com/aleksUIX/vastlint) project.

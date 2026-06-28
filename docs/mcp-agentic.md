# VAST validation in agent-based ad delivery - VASTlint MCP

> **TL;DR** - VASTlint ships a native [Model Context Protocol](https://modelcontextprotocol.io) server at `vastlint.org/mcp`. Any MCP-compatible client or agent pipeline can call `validate_vast`, `validate_vast_url`, `explain_rule`, and `fix_vast` as structured tools with no custom integration work. This page covers how to connect and where it fits in the IAB AAMP stack.

---

## Why VAST validation belongs in the pipeline

Malformed VAST tags are a common source of lost impressions, broken tracking, and discrepancies between buyers and sellers. Today most of that is caught - or missed - by ops teams and QA checklists. As creative trafficking moves into automated pipelines, those checks need to move with it.

VASTlint exposes a sub-millisecond validation engine via MCP: call it from a buyer agent before confirming a deal, from an SSAI server before stitching, or from CI before tagging a release. It returns structured JSON with rule IDs, XPath locations, and spec references - enough information to reject, fix, or escalate a creative without a manual review step.

---

## Quick start - connect any MCP client

### Hosted SSE endpoint (no install)

Add to your agent config and you're done. No binary, no Rust toolchain:

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

Works in **Claude Desktop**, **Cursor**, **GitHub Copilot**, and any MCP client that supports SSE transport.

### Claude Desktop

`~/Library/Application Support/Claude/claude_desktop_config.json`:

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

Restart Claude Desktop. You can now ask: *"Validate this VAST tag"*, *"Why is impression tracking firing twice?"*, *"Fix the duration format in this tag"* - Claude will call the tools automatically.

### Cursor

`.cursor/mcp.json` in your project root:

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

### Local stdio install (air-gapped / low-latency)

```sh
cargo install vastlint-mcp
```

```json
{
  "mcpServers": {
    "vastlint": {
      "command": "vastlint-mcp"
    }
  }
}
```

The local binary speaks the MCP stdio transport protocol. Same tools, same JSON schema, zero network round-trips.

### MCP Registry

Listed as **`io.github.aleksUIX/vastlint`** on the [MCP Registry](https://registry.modelcontextprotocol.io). Discoverable by any registry-aware agent runtime.

---

## Available tools

| Tool | When to call it |
|---|---|
| `validate_vast` | Raw XML in, issues array out. Use when you have the tag in memory. |
| `validate_vast_url` | Fetch-and-validate a VAST URL. Follows wrapper chains up to the configured depth. |
| `list_rules` | Returns all 182 rule IDs with severities and descriptions. Cache this - it's static. |
| `explain_rule` | Full details, spec reference, and fix guidance for a specific rule ID. |
| `fix_vast` | Auto-applies all deterministic safe fixes (HTTP→HTTPS upgrades, deprecated attribute removal). Returns patched XML plus a diff of what changed. |

All tools return structured JSON. No string parsing required.

---

## Agentic loop patterns

### Pre-trafficking validation gate

The most common pattern: an agent receives a creative, validates before confirming a deal or inserting into a playlist.

```
buyer-agent → validate_vast_url("https://ad.example.com/tag.xml")
            ← { valid: false, errors: 2, warnings: 1,
                 issues: [
                   { rule: "VAST-4.2-secure-media-url", severity: "error",
                     path: "/VAST/Ad/InLine/Creatives/Creative/Linear/MediaFiles/MediaFile",
                     message: "MediaFile URL must use HTTPS" }
                 ]}
buyer-agent → # reject creative, log rule ID, request fix from seller
```

This is the pattern expected by the **IAB Tech Lab AAMP Buyer Agent SDK** - see [github.com/IABTechLab/buyer-agent](https://github.com/IABTechLab/buyer-agent).

### Fix-and-retry loop

An agent that can mutate the creative before delivery:

```
agent → validate_vast(xml)
      ← { valid: false, issues: [{rule: "VAST-2.0-duration-format", ...}] }
agent → explain_rule("VAST-2.0-duration-format")
      ← { hint: "Duration must be HH:MM:SS.mmm ...", example: "00:00:30.000" }
agent → fix_vast(xml)
      ← { xml: "<VAST ...>...</VAST>", fixes_applied: [...], remaining_issues: [] }
agent → validate_vast(fixed_xml)
      ← { valid: true }
agent → traffic fixed_xml
```

### SSAI pipeline integration

Server-side ad insertion systems can call `validate_vast_url` inline before stitching. At 363 µs for a 17 KB tag (single thread), validation adds less than 0.4% to a typical 100 ms SSAI budget.

### CI/CD creative quality gate

Add vastlint to your deployment pipeline. The MCP server can be invoked from any CI system that supports stdio or HTTP tool calls - or use the [CLI](../README.md#install) or [REST API](../README.md#use-as-a-rest-api) directly.

---

## The agentic advertising standards landscape

The ad industry is converging on interoperable standards for automated ad delivery. Here's where vastlint MCP fits.

### IAB Tech Lab - AAMP (Agentic Advertising Management Protocols)

[AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/) (updated April 2026) is the IAB Tech Lab's umbrella framework for agent-based advertising. It has three pillars:

**1. Agent Foundations - ARTF**

The [Agentic Real Time Framework (ARTF)](https://iabtechlab.com/standards/artf/) defines how agent services operate inside advertising systems, including real-time bidding. ARTF has an MCP interface built into the specification. vastlint-mcp is compatible with the ARTF validation agent interface: deploy it as a container in your ARTF host platform and buyer/seller agents can call VAST validation as a standard service.

**2. Agentic Protocols**

AAMP defines buyer and seller agent SDKs ([Buyer Agent](https://github.com/IABTechLab/buyer-agent), [Seller Agent](https://github.com/IABTechLab/seller-agent)) built on OpenDirect, AdCOM, and the Deals API. Creative validation - confirming that a VAST tag is spec-compliant before a deal is confirmed - fits naturally into the deal flow. vastlint MCP slots in at the creative validation step without changes to the agent protocol.

**3. Trust and Transparency - Agent Registry**

The [IAB Tech Lab Agent Registry](https://iabtechlab.com/introducing-the-iab-tech-lab-agent-registry/) provides identity and disclosure for agents in the ecosystem. Agents calling vastlint-mcp can register as verified participants.

**IAB's position on tooling:** "The key agentic protocols - MCP from Anthropic, and Google's Agent to Agent - perform best when they have utility schemas along with reference implementations to establish context." vastlint MCP is a reference implementation of that: a utility tool with a defined schema that an agent can call and get a consistent, structured result from.

### IAB Tech Lab - ARTF and the Agentic Ad Object (AAO)

The **Agentic Ad Object (AAO)**, derived from [AdCOM](https://iabtechlab.com/standards/adcom-advertising-common-object-model/), is the canonical representation of a creative in the AAMP protocol layer. When an AAO references a VAST tag, `validate_vast_url` provides the spec-compliance signal a buyer or seller agent needs before accepting or confirming the creative.

ARTF enables new use cases in the bid stream - fraud detection, deal activation, identity resolution - all via the same container-and-MCP architecture. VAST validation is the quality check that runs before an impression fires.

### Google - Agent-to-Agent (A2A)

Google's [Agent-to-Agent protocol](https://developers.google.com/workspace/agents) defines how agents discover and call each other over HTTP. vastlint's hosted SSE endpoint (`vastlint.org/mcp`) is consumable via A2A-compatible orchestrators: a campaign management agent can call the VAST validation tool over standard HTTP without any MCP-specific client library.

### Amazon Ads - programmatic and CTV

Amazon's programmatic ecosystem (Fire TV, Prime Video, Amazon DSP) runs VAST-based creatives across CTV and streaming inventory. Buyers trafficking into Amazon inventory through programmatic channels should validate VAST tags before submission - malformed tags result in missed impressions with no fill and no error returned at auction time. vastlint catches structural errors, insecure media URLs, and deprecated attributes before they reach Amazon's ad server.

### OpenRTB and SSAI

Both [OpenRTB](https://iabtechlab.com/standards/openrtb/) bid responses that include VAST XML and SSAI stitchers that resolve VAST wrapper chains benefit from validation. vastlint-mcp's `validate_vast_url` follows wrapper chains and validates each hop, returning per-hop issues - the signal an OpenRTB DSP or SSAI platform needs to decide whether to use a creative.

---

## Integration reference

### Python (MCP client)

```python
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def validate(xml: str):
    server = StdioServerParameters(command="vastlint-mcp")
    async with stdio_client(server) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            result = await session.call_tool("validate_vast", {"xml": xml})
            return result.content[0].text  # JSON string
```

### Node.js (MCP SDK)

```js
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const transport = new StdioClientTransport({ command: "vastlint-mcp" });
const client = new Client({ name: "my-agent", version: "1.0.0" });
await client.connect(transport);

const result = await client.callTool({
  name: "validate_vast_url",
  arguments: { url: "https://ad.example.com/tag.xml", max_depth: 5 }
});
console.log(JSON.parse(result.content[0].text));
```

### HTTP (hosted endpoint, A2A-compatible)

```sh
# SSE endpoint - use any MCP SSE client or direct HTTP
curl -N https://vastlint.org/mcp \
  -H "Accept: text/event-stream"
```

### IAB AAMP Buyer Agent SDK

```python
# In your buyer agent's creative validation step
from buyer_agent import BuyerAgent

agent = BuyerAgent()
agent.add_tool_server("https://vastlint.org/mcp")  # vastlint auto-discovered via MCP

# The agent will call validate_vast_url automatically when evaluating
# a deal that includes a VAST creative URL
result = agent.evaluate_deal(deal_proposal)
```

See [github.com/IABTechLab/buyer-agent](https://github.com/IABTechLab/buyer-agent) for full SDK documentation.

---

## What the tools return

`validate_vast` and `validate_vast_url` return:

```json
{
  "version": "4.2",
  "valid": false,
  "errors": 1,
  "warnings": 2,
  "info": 0,
  "issues": [
    {
      "rule": "VAST-4.2-secure-media-url",
      "severity": "error",
      "path": "/VAST/Ad/InLine/Creatives/Creative/Linear/MediaFiles/MediaFile[1]",
      "line": 42,
      "col": 5,
      "message": "MediaFile URL must use HTTPS in VAST 4.x",
      "spec_ref": "VAST 4.2 §4.11.4"
    }
  ]
}
```

Every issue carries a `rule` ID that maps to `explain_rule`, a `path` in XPath notation, a line/col for editors, and a `spec_ref` pointing to the IAB spec section.

`fix_vast` returns:

```json
{
  "xml": "<VAST version=\"4.2\">...</VAST>",
  "fixes_applied": [
    {
      "rule": "VAST-4.2-secure-media-url",
      "description": "Upgraded HTTP → HTTPS",
      "path": "/VAST/Ad/InLine/.../MediaFile[1]"
    }
  ],
  "remaining_issues": []
}
```

---

## Latency characteristics

| Scenario | Latency |
|---|---|
| Local stdio (`vastlint-mcp`) - 17 KB tag | ~363 µs |
| Local stdio - 44 KB tag | ~2.1 ms |
| Hosted SSE (`vastlint.org/mcp`) - typical round-trip | ~15–40 ms |
| OpenRTB bid cycle budget | 100–300 ms |
| Validation as % of bid budget (local) | < 2.1% |

The local binary is the right choice for latency-sensitive SSAI pipelines and ARTF agent containers close to the bid stream. The hosted endpoint works well for cloud-native pipelines where a few extra milliseconds of network latency is acceptable and you want zero operational overhead.

---

## See also

- [`crates/vastlint-mcp`](../crates/vastlint-mcp/README.md) - full tool schema reference
- [Common errors](common-errors.md) - the VAST mistakes that cost real money
- [Tutorial](tutorial.md) - getting started with VAST validation
- [Rule reference](https://vastlint.org/docs/rules) - all 182 rules with examples and fix instructions
- [IAB Tech Lab AAMP](https://iabtechlab.com/standards/aamp-agentic-advertising-management-protocols/)
- [IAB Tech Lab ARTF](https://iabtechlab.com/standards/artf/)
- [AAMP Buyer Agent SDK](https://github.com/IABTechLab/buyer-agent)
- [AAMP Seller Agent SDK](https://github.com/IABTechLab/seller-agent)
- [MCP Registry listing](https://registry.modelcontextprotocol.io) - `io.github.aleksUIX/vastlint`

# liel MCP server

`liel`'s MCP (Model Context Protocol) support is an optional feature that lets
an AI agent use a `.liel` file as durable graph memory.

It is deliberately decoupled from the Rust core: the core keeps zero added
runtime dependency, and only users who choose `pip install "liel[mcp]"` get
the MCP layer.

## Product stance

For AI integration, **MCP is liel's primary surface**.

- MCP is the standard access layer for reading and writing durable graph memory.
- Skills, prompts, and `/commands` are useful wrappers, but they sit on top of
  the MCP surface rather than replacing it.
- Tool design should prefer **coarse memory actions** over chatty tiny writes,
  so the AI layer stays aligned with liel's single-file, local-first design.

!!! warning "About high-frequency writes"
    Driving `liel` from an AI agent with a commit on every tiny step clashes
    with its design center. Prefer checkpointed writes and grouped operations.
    See **[product trade-offs (DB)](../../design/product-tradeoffs.md#mcp-knowledge-graph)**.

---

## Install

```bash
# Plain install (no MCP, lightweight)
pip install liel

# MCP-enabled install
pip install "liel[mcp]"
```

Requires Python 3.9 or newer.

The optional MCP dependency currently remains broad at `mcp>=1.0`.

---

## Quick start

### 1. Run from the CLI

```bash
# Start with a specific .liel file
liel-mcp --path my.liel

# Auto-discover under the current directory
liel-mcp
```

### 2. Register from your MCP client

Configuration file locations, approval flow, and setup UX vary by client and
change over time. Follow your client's official MCP setup guide, then register
`liel` with a server definition like this:

```json
{
  "mcpServers": {
    "liel": {
      "command": "/path/to/python",
      "args": [
        "-m",
        "liel.mcp",
        "--path",
        "/path/to/your/project.liel"
      ]
    }
  }
}
```

Replace:

- `command` with the Python executable where `liel[mcp]` is installed
- `--path` with the `.liel` file you want the client to use as durable memory

After registering the server, restart your client and confirm that `liel`
appears as connected in its MCP management UI.

### 3. Use programmatically

```python
from liel.mcp import create_server

mcp = create_server("my.liel")
mcp.run()                         # stdio (default)
mcp.run(transport="sse")          # SSE
```

---

## Typical workflow

```text
1. Overview - Start with liel_overview to understand the graph at a glance.
2. Narrow   - Use liel_find to locate candidate nodes.
3. Explore  - Use liel_explore or liel_trace to inspect structure.
4. Explain  - Use liel_map when a Mermaid diagram will help the human.
5. Write    - Use liel_append for guaranteed new records, or liel_merge to reuse existing nodes and idempotent edges.
```

---

## Design

| Layer | What it does | Dependency |
|---|---|---|
| **Core** | Rust / PyO3 read/write, graph algorithms | Zero external dependency |
| **Bridge** | Python wrapper, type conversion, dotted access | Core only |
| **MCP plugin** | FastMCP server, tool definitions, protocol bridging | `mcp>=1.0` |

`import mcp` only happens inside `liel/mcp/`; ordinary `import liel` is
unaffected.

---

## Tools at a glance

The official MCP surface is fixed to these seven tools. New docs, prompts, and
examples should use these names only.

| Tool | Purpose | R/W |
|---|---|---|
| [`liel_overview`](tools.md#liel_overview) | Get the high-level shape of the memory graph | Read |
| [`liel_find`](tools.md#liel_find) | Find nodes by label and exact property match | Read |
| [`liel_explore`](tools.md#liel_explore) | Explore a neighbourhood with BFS and Mermaid output | Read |
| [`liel_trace`](tools.md#liel_trace) | Trace the shortest path between two nodes | Read |
| [`liel_map`](tools.md#liel_map) | Render a chosen subgraph as Mermaid | Read |
| [`liel_append`](tools.md#liel_append) | Append new nodes and edges in one commit | **Write** |
| [`liel_merge`](tools.md#liel_merge) | Reuse or update existing nodes and add idempotent edges in one commit | **Write** |

For full parameter specs, see the **[tools reference](tools.md)**.

---

## AI memory playbook

For prompt snippets and operating rules for Claude, Codex, and Cursor, see
**[AI memory playbook](agent-memory.md)**.

---

## Claude project memory

For a practical `CLAUDE.md` example that uses `liel[mcp]` as durable project
memory, see **[Claude project memory](claude-memory.md)**.

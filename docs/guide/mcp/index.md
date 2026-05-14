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

Configure your LLM client to start the `liel` MCP server. In Claude Code, edit
`.mcp.json` in the project root like this:

```json
{
  "mcpServers": {
    "liel": {
      "type": "stdio",
      "command": "/absolute/path/to/liel-mcp",
      "args": ["--path", "/absolute/path/to/your/project.liel"]
    }
  }
}
```

Use the installed `liel-mcp` executable for `command`, and set `--path` to the
`.liel` file you want the client to use as durable memory. For other LLM/MCP
clients, use the equivalent MCP server setting with the same command and args.

Do not put `mcpServers` in `.claude/settings.json`; that file is for Claude
Code settings such as permissions and environment variables.

For first-time setup, `--path` is the clearest option. If the file does not
exist yet, `liel` creates it on first open. If `--path` is omitted, the server
checks only the startup directory: if no `*.liel` file exists there, it uses
`./memory.liel`; if one exists, it uses that file; if multiple files exist, it
prints the candidates and asks you to register the intended file with `--path`.

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

The official MCP surface is fixed to these ten tools. New docs, prompts, and
examples should use these names only.

| Tool | Purpose | R/W |
|---|---|---|
| [`liel_overview`](tools.md#liel_overview) | Get the high-level shape of the memory graph | Read |
| [`liel_find`](tools.md#liel_find) | Find nodes by label and exact property match | Read |
| [`liel_explore`](tools.md#liel_explore) | Explore a neighbourhood with BFS and Mermaid output | Read |
| [`liel_trace`](tools.md#liel_trace) | Trace the shortest path between two nodes | Read |
| [`liel_map`](tools.md#liel_map) | Render a chosen subgraph as Mermaid | Read |
| [`liel_diff`](tools.md#liel_diff) | Compare two `.liel` files with CLI-compatible JSON | Read |
| [`liel_merge_preview`](tools.md#liel_merge_preview) | Preview a two-file merge without writing output | Read |
| [`liel_manifest`](tools.md#liel_manifest) | Emit deterministic manifest JSON for a `.liel` file | Read |
| [`liel_append`](tools.md#liel_append) | Append new nodes and edges in one commit | **Write** |
| [`liel_merge`](tools.md#liel_merge) | Reuse or update existing nodes and add idempotent edges in one commit | **Write** |

For full parameter specs, see the **[tools reference](tools.md)**.

---

## AI memory playbook

For the recommended LLM memory pattern, prompt snippets, and operating rules
for Claude, Codex, and Cursor, start with **[AI memory playbook](agent-memory.md)**.

---

## Claude setup

For Claude-specific setup notes, see **[Claude setup](claude.md)**.
For a copyable project-instructions sample, see
**[sample `CLAUDE.md`](samples/CLAUDE.md)**.

## End-to-end sample

- [Claude project-memory workflow](claude-workflow.md) — first full ecosystem sample: setup, memory creation, record, trace, and review.

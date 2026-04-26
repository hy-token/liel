# liel

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/hy-token/liel/blob/main/LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/hy-token/liel/ci.yml?branch=main&label=CI)](https://github.com/hy-token/liel/actions/workflows/ci.yml)

**Single-file graph memory for local AI agents.  
Turn LLM interactions into a persistent, traversable knowledge graph - not just text retrieval.**

`liel` is a lightweight local graph memory store for LLM tools, AI agents, and Python applications.
It stores facts, decisions, tasks, files, sources, tool results, and their relationships in one portable `.liel` file.
As a product, `liel` is best understood as a **portable external brain for LLM workflows**, not as a general-purpose graph database.

Use it standalone from Python, or expose the same `.liel` file as an MCP-backed memory layer for tools like Claude.

The core package has **no runtime dependencies**. No external database server, cloud service, or background daemon is required. On supported platforms, `pip install liel` is enough to get started.

MCP integration is optional. Install `liel[mcp]` only when you want to expose a `.liel` memory file to an MCP-capable AI tool.

Under the hood, `liel` uses a Rust-core **property graph storage engine** with a Python-first API and optional MCP integration.
If SQLite is the one-file relational database, `liel` aims to be the one-file **external brain** for relationship-centric AI workflows.
It is not positioned as a full graph database server; it is a minimal, persistent graph substrate for building higher-level memory systems.

> *Etymology: a portmanteau of French* lier *(to connect) and Latin* ligare.

## The Zen of Liel

- One file, any place.
- No server, no waiting.
- Minimal dependencies, simple environments.
- Start small, stay local.

See [Design principles](docs/design/principles.md).

---

## Table of contents

- [Quickstart](#quickstart)
- [Install](#install)
- [What It Is](#what-it-is)
- [How this differs from RAG](#how-this-differs-from-rag)
- [When liel fits](#when-liel-fits)
- [When liel does not fit](#when-liel-does-not-fit)
- [Design trade-offs](#design-trade-offs)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

---

## Quickstart

### 1. LLM memory in one file

Install the core package:

```bash
pip install liel
```

Instead of losing context between sessions, store decisions and relationships as a graph:

```python
import liel

with liel.open("agent-memory.liel") as db:
    task = db.add_node(["Task"], name="Design AI memory system")
    decision = db.add_node(
        ["Decision"],
        content="Use graph memory instead of text-only retrieval",
    )
    source = db.add_node(["Source"], title="Architecture notes")

    db.add_edge(task, "LED_TO", decision)
    db.add_edge(decision, "SUPPORTED_BY", source)
    db.commit()

    for node in db.neighbors(task, edge_label="LED_TO"):
        print(node["content"])
```

Now your AI can recall *why* decisions were made, not just what was said.

### 2. Python property graph

For a minimal graph API example:

```python
import liel

with liel.open(":memory:") as db:
    alice = db.add_node(["Person"], name="Alice")
    bob = db.add_node(["Person"], name="Bob")
    db.add_edge(alice, "KNOWS", bob, since=2020)
    db.commit()

    print(db.neighbors(alice, edge_label="KNOWS")[0]["name"])  # Bob
```

For the Python API, transactions, QueryBuilder, traversal, and examples:

- [Python guide](https://github.com/hy-token/liel/blob/main/docs/guide/connectors/python.md)
- [Quickstart example](https://github.com/hy-token/liel/blob/main/examples/01_quickstart.py)
- [Examples directory](https://github.com/hy-token/liel/tree/main/examples)

### 3. Claude + MCP project memory

`liel[mcp]` exposes one official MCP surface for AI memory:

- `liel_overview`
- `liel_find`
- `liel_explore`
- `liel_trace`
- `liel_map`
- `liel_append`
- `liel_merge`

**Step 1 - Install**

```bash
pip install "liel[mcp]"
```

**Step 2 - Register the MCP server**

Configuration file locations, approval flow, and setup UX differ between MCP
clients and change over time. Follow your client's official MCP setup guide,
then register `liel` with a server definition like this:

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
- `--path` with the `.liel` file you want the AI tool to use as durable memory

After registering the server, restart your MCP client and confirm that `liel`
appears as connected in its MCP management UI.

**Step 3 - Tell Claude to use it**

Add this block to your project's `CLAUDE.md`:

```md
## Project Memory
- Use `liel[mcp]` as the long-term memory store for this project when the MCP server is available.
- At the start of a task, use `liel_overview`, then `liel_find`, then `liel_explore` to restore context before asking the user to repeat it.
- Save only durable information: confirmed decisions, stable preferences, important facts, open questions, and tasks that should survive the session.
- Do not save temporary reasoning, speculative ideas, or verbose logs.
- Reuse existing nodes and link new ones to related nodes. Avoid duplicates.
- Use `liel_append` when you want guaranteed new records, and `liel_merge` when you want to reuse existing nodes or idempotent edges.
- Write at meaningful checkpoints (task complete, decision confirmed, session end) - not on every turn.
```

That's it. Claude will now read and write `agent-memory.liel` autonomously.

For the full setup guide, available tools, and a longer `CLAUDE.md` example:

- [MCP guide](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/index.md)
- [MCP tools reference](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/tools.md)
- [AI memory playbook](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/agent-memory.md)
- [Claude project memory example](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/claude-memory.md)

---

## Install

Install the dependency-free core package:

```bash
pip install liel
```

Install the optional MCP integration only when you want an MCP-capable AI tool to use a `.liel` file as external memory:

```bash
pip install "liel[mcp]"
```

**Platform support**

- OS: Linux, macOS, Windows
- Architecture: x86_64 first, arm64 where practical
- Python: **3.9 or newer**

This installs prebuilt wheels for supported platforms. **Rust is not required** at install time.

If you are contributing or need a source build, see:

- [Contributing guide](https://github.com/hy-token/liel/blob/main/CONTRIBUTING.md)

---

## What It Is

`liel` is a single-file external-brain substrate for local memory and relationship-centric AI workflows.

It is built around a few deliberate choices:

- one portable `.liel` file instead of a server
- explicit graph relationships instead of text-only memory
- local persistence instead of cloud-managed infrastructure
- a small Rust core with a Python-first interface

Internally this is implemented as a property graph, but the product promise is higher-level: durable, inspectable memory that an LLM or local agent can carry between sessions.

For the feature surface and file format:

- [Why liel](https://github.com/hy-token/liel/blob/main/docs/why-liel.md)
- [Feature list](https://github.com/hy-token/liel/blob/main/docs/reference/features.md)
- [Format spec](https://github.com/hy-token/liel/blob/main/docs/reference/format-spec.md)

## How this differs from RAG

RAG retrieves similar text chunks. `liel` stores and traverses relationships between entities, decisions, tasks, sources, files, and tool results.

Use RAG when your main problem is finding relevant passages. Use `liel` when your AI tool needs durable memory that can answer relationship-centric questions like:

- Which decision led to this task?
- What source supported that claim?
- Which files, tool calls, and follow-up tasks are connected?

`liel` is not a retrieval system. It is a persistent memory substrate for local AI workflows.

---

## When liel fits

Use `liel` when:

- you want local AI memory as a file, not a service
- relationships between entities matter
- you want decisions, facts, sources, tasks, and tool outputs to survive across sessions
- you want graph traversal without deploying a separate database server
- you want something easy to copy, back up, inspect, and archive

Common good fits:

- project memory for coding assistants
- local agent memory
- personal or team knowledge graphs
- MCP-backed memory for AI tools
- lightweight provenance-aware tool result stores

Examples and usage patterns:

- [Examples](https://github.com/hy-token/liel/tree/main/examples)
- [Python guide](https://github.com/hy-token/liel/blob/main/docs/guide/connectors/python.md)
- [MCP guide](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/index.md)

---

## When liel does not fit

`liel` is not the right tool when your primary need is:

- semantic similarity search over text
- full-text search or document retrieval as the main access pattern
- very large graph workloads with heavy concurrent writes
- server-style multi-user mutation
- SQL-centric graph querying over an existing relational system

In those cases, a vector database, search engine, PostgreSQL recursive queries, DuckDB graph extensions, or a server-backed graph database may fit better.

More detailed comparisons and non-goals:

- [Product trade-offs](https://github.com/hy-token/liel/blob/main/docs/design/product-tradeoffs.md)

---

## Design trade-offs

`liel` is intentionally narrow. The main trade-offs are:

- single-writer design rather than concurrent peer-to-peer mutation
- page-level WAL for durability, not ultra-high-frequency tiny commits
- no full-text engine, query language, or property index in the current product shape
- Python API and MCP integration first, with the Rust core kept small

That narrowness comes from the product framing: `liel` is trying to be a portable external brain for local AI systems, not a general-purpose graph database platform.

This is what keeps the system simple and portable, but it also defines where it is and is not comfortable to use.

Read these before using `liel` as durable application state:

- [Reliability and failure model](https://github.com/hy-token/liel/blob/main/docs/reference/reliability.md)
- [Product trade-offs](https://github.com/hy-token/liel/blob/main/docs/design/product-tradeoffs.md)

---

## Documentation

The PyPI source distribution is intentionally small and does not include the full documentation tree or example scripts. Use the GitHub repository for:

- [Documentation index](https://github.com/hy-token/liel/blob/main/docs/index.md)
- [Why liel](https://github.com/hy-token/liel/blob/main/docs/why-liel.md)
- [Python guide](https://github.com/hy-token/liel/blob/main/docs/guide/connectors/python.md)
- [MCP guide](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/index.md)
- [Reference](https://github.com/hy-token/liel/blob/main/docs/reference/index.md)
- [Design docs](https://github.com/hy-token/liel/blob/main/docs/design/index.md)
- [Examples](https://github.com/hy-token/liel/tree/main/examples)
- [Example notebooks](https://github.com/hy-token/liel/tree/main/examples/notebooks)

---

## Contributing

Pull requests and issues are welcome. Start here:

- [CONTRIBUTING.md](https://github.com/hy-token/liel/blob/main/CONTRIBUTING.md)

Local checks:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
pytest tests/python/
```

---

## License

[MIT](https://github.com/hy-token/liel/blob/main/LICENSE)

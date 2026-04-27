# Quickstart

This page collects the longer setup paths that used to live in the README: the bundled demo, the Python API, and MCP project memory.

## 1. Run the bundled demo

Install the core package:

```bash
pip install liel
```

Fastest path:

```bash
liel-demo
```

Most portable path, using the same interpreter as `pip`:

```bash
python -m liel.demo
```

The demo ships inside the wheel, so it does not require cloning the repository or downloading example files. It runs without an LLM by default. If you want a local [Ollama](https://ollama.com/) model to phrase the exploration list, start Ollama and run:

```bash
LIEL_DEMO_LLM=1 liel-demo
```

If Ollama is down, the command still finishes using a built-in fallback.

## 2. Store agent memory in one file

Instead of losing context between sessions, store decisions and relationships as a graph:

```python
import liel

with liel.open("agent-memory.liel") as db:
    task = db.add_node(
        ["Task"],
        description="Migrate auth from JWT to server-side sessions",
    )
    question = db.add_node(
        ["OpenQuestion"],
        content="Use Redis or PostgreSQL for the session store?",
    )
    rejected = db.add_node(
        ["RejectedOption"],
        option="Redis",
        reason="Adds another infrastructure dependency",
    )
    decision = db.add_node(
        ["Decision"],
        content="Use a PostgreSQL session table",
    )
    source = db.add_node(["Source"], title="Auth migration notes")

    db.add_edge(task, "RAISED", question)
    db.add_edge(question, "REJECTED", rejected)
    db.add_edge(question, "RESOLVED_BY", decision)
    db.add_edge(decision, "SUPPORTED_BY", source)
    db.commit()

    for node in db.neighbors(question, edge_label="RESOLVED_BY"):
        print(node["content"])
```

Now your AI can recall *why* decisions were made, not just what was said.

## 3. Use the Python property graph API

A minimal in-memory graph:

```python
import liel

with liel.open(":memory:") as db:
    task = db.add_node(["Task"], title="Prepare release")
    file = db.add_node(["File"], path="README.md")
    db.add_edge(task, "TOUCHES", file, reason="clarify positioning")
    db.commit()

    print(db.neighbors(task, edge_label="TOUCHES")[0]["path"])  # README.md
```

For the full Python API, see the [Python guide](connectors/python.md).

## 4. Use Claude or another MCP client

Install the optional MCP integration only when you want an MCP-capable AI tool to use a `.liel` file as memory:

```bash
pip install "liel[mcp]"
```

Register `liel` with your MCP client using a server definition like this:

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

Then add a project-memory policy to your agent instructions. For Claude, see the full [MCP guide](mcp/index.md) and [Claude project memory example](mcp/claude-memory.md).

## Quick limits

- One writer per `.liel` file.
- No semantic/vector search in core.
- `commit()` defines crash-safe boundaries.
- For durability details, read [Reliability and failure model](../reference/reliability.md).

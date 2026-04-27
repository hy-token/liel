# liel

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/hy-token/liel/blob/main/LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/hy-token/liel/ci.yml?branch=main&label=CI)](https://github.com/hy-token/liel/actions/workflows/ci.yml)

The name comes from the French *lier* — to connect, to bind.

**A portable external brain for local AI agents** — one file, structured by relationships.

```bash
pip install liel
liel-demo
```

Runs fully local. No API keys required (LLM optional).

`liel` is a single-file graph memory layer for people using local AI agents while coding. One `.liel` file stores decisions, tasks, sources, files, facts, and the relationships between them, so tools can recall *why* decisions were made, not just what was said.

The core is a small Rust **property graph** engine with **Python (PyO3)** bindings and optional MCP tools. No server, no cloud, no daemon.

## Why Local-First

- **Your code stays on your machine.** No API keys, no telemetry, no cloud round-trips.
- **Works with any LLM.** Local (Ollama, LM Studio) or cloud (Claude, GPT) — only memory stays local.
- **Offline-friendly.** Memory persists across sessions without network access.
- **One file, no lock-in.** Copy, commit, archive, and open with any tool that speaks `.liel`.

## Try It

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

Use it as Claude/Cursor project memory via MCP: see the [MCP guide](docs/guide/mcp/index.md).

## Compared To Mem0 / Letta / Zep

`liel` is intentionally lower-level and local-first. It ships as a single `.liel` file with no server, no API keys, and no required vector index. Relationships are explicit edges you write and traverse, not only facts inferred from chat history.

Mem0, Letta, and Zep may be a better fit when you want a hosted service, a full agent runtime, automatic memory extraction, temporal graph intelligence, dashboards, or production-scale context assembly. `liel` is the smaller substrate: local coding agents and project-adjacent tools that need durable, inspectable graph memory they can copy, commit, archive, and open from Python or MCP.

## The Zen of Liel

- One file, any place.
- No server, no waiting.
- Minimal dependencies, simple environments.
- Start small, stay local.

## Documentation

- [Why liel](docs/why-liel.md) - what it solves and what it does not
- [Quickstart](docs/guide/quickstart.md) - demo, Python, and MCP paths
- [Architecture](docs/design/architecture.md) - system layers and the Mermaid diagram
- [Python guide](docs/guide/connectors/python.md) - API, transactions, traversal
- [MCP guide](docs/guide/mcp/index.md) - Claude and other MCP-capable tools
- [Feature list](docs/reference/features.md) - what is provided at a glance
- [Reliability](docs/reference/reliability.md) - commit semantics, crash recovery, repair
- [Format spec](docs/reference/format-spec.md) - byte-level `.liel` file format
- [Product trade-offs](docs/design/product-tradeoffs.md) - what liel does not do, and why

## Status

`liel` is currently a **Beta** package. The supported contract is the Python-first API plus the single-writer, single-file reliability model. There is no semantic/vector search in core, and `commit()` defines crash-safe boundaries. Breaking changes before `1.0` are tracked in the [changelog](CHANGELOG.md).

## Contributing

Pull requests and issues are welcome. A good first step is to run `liel-demo` and note anything confusing about the output, memory model, or docs.

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Author

Built by Hayato under [`hy-token`](https://github.com/hy-token), a personal namespace for small local-first tools and AI infrastructure experiments.

## License

[MIT](LICENSE)

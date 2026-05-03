# liel

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/hy-token/liel/blob/main/LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/hy-token/liel/ci.yml?label=CI)](https://github.com/hy-token/liel/actions/workflows/ci.yml)
[![PyPI version](https://img.shields.io/pypi/v/liel)](https://pypi.org/project/liel/)

## Git-compatible memory for AI agents.

One local file you can merge, diff, trace, and inspect.

![Parallel merge preview: two agent memories merged with `liel merge --dry-run`](https://raw.githubusercontent.com/hy-token/liel/main/assets/demo/parallel-merge.wsl.gif)

```bash
pip install liel
liel-demo
```

Runs fully local. No API keys required (LLM optional).

`liel` is a single-file graph memory layer for people using local AI agents while coding. One `.liel` file stores decisions, tasks, sources, files, facts, and the relationships between them, so tools can recall *why* decisions were made, not just what was said.

The core is a small Rust **property graph** engine with **Python (PyO3)** bindings and optional MCP tools. No server, no cloud, no daemon.

## Why decisions disappear

Chat turns roll off the context window, but the graph still holds *how* a choice was reached. `liel trace` walks a shortest path so that reasoning stays visible—not just the final answer.

![`liel trace` narrative output (shortest path through decision nodes)](https://raw.githubusercontent.com/hy-token/liel/main/assets/demo/demo-trace.wsl.gif)

The name *liel* comes from the French *lier* — to connect, to bind.

## Three quick demos (~30 seconds each)

Use the **fixed SaaS-style memory** generator (two agents diverge on the same bug/decision graph). From a checkout with `liel` on your `PATH` and Python 3.9+:

```bash
python examples/demo_memory/make_demo_files.py --force
```

Default output: `target/demo-memory/` (`base.liel`, `agent-a.liel`, `agent-b.liel`, `identity-rules.json`).

1. **Parallel merge preview (two agents, one reviewable report)** — the story behind “Git-compatible working memory”:

   ```bash
   liel merge target/demo-memory/agent-a.liel target/demo-memory/agent-b.liel \
     --dry-run --identity-rules target/demo-memory/identity-rules.json \
     --edge-strategy idempotent --format json
   ```

2. **Diff with stable identity (what drifted between branches of memory)**:

   ```bash
   liel diff target/demo-memory/base.liel target/demo-memory/agent-a.liel \
     --identity-rules target/demo-memory/identity-rules.json
   ```

3. **One-file inspect (portable artifact)**:

   ```bash
   liel stats target/demo-memory/base.liel --format json
   ```

For **VHS tapes and GIF outputs**, see [`demos/README.md`](demos/README.md) (English) or [`demos/README.ja.md`](demos/README.ja.md) (Japanese catalog). KPI and posting cadence for these stories live in the maintainer [Phase 4 Marketing Playbook](docs/internal/process/roadmap-phase4-marketing-playbook.ja.md) (Japanese; clone the repo to read it).

## Coding memory helpers (experimental)

Optional thin wrappers in [`python/liel/coding_memory.py`](python/liel/coding_memory.py) for `File` / `Decision` / bug-shaped `Task` nodes — see [`examples/coding_memory/README.md`](examples/coding_memory/README.md) and the [Python guide § Coding memory helpers](docs/guide/connectors/python.md#coding-memory-helpers). Maintainer design (Japanese): [`docs/internal/design/coding-memory.ja.md`](docs/internal/design/coding-memory.ja.md).

## Why Local-First

- **Your code stays on your machine.** No API keys, no telemetry, no cloud round-trips.
- **Works with any LLM.** Local (Ollama, LM Studio) or cloud (Claude, GPT) — only memory stays local.
- **Offline-friendly.** Memory persists across sessions without network access.
- **One file, no lock-in.** Copy, commit, archive, and open with any tool that speaks `.liel`.

## LLM Setup

Use `liel` as project memory through MCP:

```bash
pip install "liel[mcp]"
```

Configure your LLM client to start the `liel` MCP server. In Claude Code, edit
`.mcp.json` in the project root like this:

```json
{
  "mcpServers": {
    "liel": {
      "type": "stdio",
      "command": "/absolute/path/to/liel-mcp",
      "args": ["--path", "/absolute/path/to/agent-memory.liel"]
    }
  }
}
```

Use the installed `liel-mcp` executable for `command`, and set `--path` to the
`.liel` file the AI should use as durable memory. For other LLM/MCP clients,
use the equivalent MCP server setting with the same command and args.

Do not put `mcpServers` in `.claude/settings.json`; that file is for Claude
Code settings such as permissions and environment variables.

For first-time setup, `--path` is the clearest option. If the file does not
exist yet, `liel` creates it on first open. Without `--path`, the server checks
only the startup directory: if no `*.liel` file exists there, it uses
`./memory.liel`; if one exists, it uses that file; if multiple files exist, it
prints the candidates and asks you to register the intended file with `--path`
instead of choosing one silently.

Then add a memory policy to the agent's project instructions. Start with the
[AI memory playbook](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/agent-memory.md),
or use the
[sample `CLAUDE.md`](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/samples/CLAUDE.md) as a longer Claude
template.

## Recommended LLM Memory Pattern

When using `liel` as project memory:

- Always check existing memory before asking the user to repeat context.
- Save only durable, high-signal information: decisions, preferences, tasks,
  sources, and important project facts.
- Do not store temporary reasoning, speculative notes, noisy logs, or every tool result.
- Write at meaningful checkpoints, not every turn.
- Use nodes for entities and edges for relationships.

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

## Compared To Mem0 / Letta / Zep

`liel` is intentionally lower-level and local-first. It ships as a single `.liel` file with no server, no API keys, and no required vector index. Relationships are explicit edges you write and traverse, not only facts inferred from chat history.

Mem0, Letta, and Zep may be a better fit when you want a hosted service, a full agent runtime, automatic memory extraction, temporal graph intelligence, dashboards, or production-scale context assembly. `liel` is the smaller substrate: local coding agents and project-adjacent tools that need durable, inspectable graph memory they can copy, commit, archive, and open from Python or MCP.

## The Zen of Liel

- One file, any place.
- No server, no waiting.
- Minimal dependencies, simple environments.
- Start small, stay local.

## Documentation

- [Why liel](https://github.com/hy-token/liel/blob/main/docs/why-liel.md) - what it solves and what it does not
- [Quickstart](https://github.com/hy-token/liel/blob/main/docs/guide/quickstart.md) - demo, Python, and MCP paths
- [AI memory playbook](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/agent-memory.md) - recommended LLM memory pattern
- [Sample CLAUDE.md](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/samples/CLAUDE.md) - Claude project-instructions template
- [Architecture](https://github.com/hy-token/liel/blob/main/docs/design/architecture.md) - system layers and the Mermaid diagram
- [Python guide](https://github.com/hy-token/liel/blob/main/docs/guide/connectors/python.md) - API, transactions, traversal, [coding memory helpers](https://github.com/hy-token/liel/blob/main/docs/guide/connectors/python.md#coding-memory-helpers)
- [MCP guide](https://github.com/hy-token/liel/blob/main/docs/guide/mcp/index.md) - Claude and other MCP-capable tools
- [Feature list](https://github.com/hy-token/liel/blob/main/docs/reference/features.md) - what is provided at a glance
- [Reliability](https://github.com/hy-token/liel/blob/main/docs/reference/reliability.md) - commit semantics, crash recovery, repair
- [Format spec](https://github.com/hy-token/liel/blob/main/docs/reference/format-spec.md) - byte-level `.liel` file format
- [Product trade-offs](https://github.com/hy-token/liel/blob/main/docs/design/product-tradeoffs.md) - what liel does not do, and why

## Status

`liel` is currently a **Beta** package. The supported contract is the Python-first API plus the single-writer, single-file reliability model. There is no semantic/vector search in core, and `commit()` defines crash-safe boundaries. Breaking changes before `1.0` are tracked in the [changelog](https://github.com/hy-token/liel/blob/main/CHANGELOG.md).

## Contributing

Pull requests and issues are welcome. A good first step is to run `liel-demo` and note anything confusing about the output, memory model, or docs.

See [CONTRIBUTING.md](https://github.com/hy-token/liel/blob/main/CONTRIBUTING.md).

## Author

Built by Hayato under [`hy-token`](https://github.com/hy-token), a personal namespace for small local-first tools and AI infrastructure experiments.

## License

[MIT](https://github.com/hy-token/liel/blob/main/LICENSE)

# liel documentation

**liel** is a single-file, portable external brain for local AI tools, agents, and Python workflows.

It stores facts, decisions, tasks, files, sources, tool results, and their relationships in one portable `.liel` file. The core package has no required runtime dependencies and does not need an external database server, cloud service, or background daemon.

Under the hood, `liel` uses a Rust-core **property graph storage engine** with Python bindings and optional MCP integration.

If you are new to the project, start with **[Why liel](why-liel.md)**.
It explains the problem `liel` is designed for: durable AI memory that keeps relationships, not just retrieved text.

---

## Reading paths

The docs are split by audience, not by importance:

| Audience | Start here | What belongs there |
|---|---|---|
| End users trying the package | [Quickstart](guide/quickstart.md) | Install, demo, first Python and MCP paths |
| Python application users | [Python API](guide/connectors/python.md) | Runtime API, exceptions, transactions, performance notes |
| AI-tool integrators | [MCP guide](guide/mcp/index.md) and [AI memory playbook](guide/mcp/agent-memory.md) | Tool setup and operating rules |
| Connector / format authors | [Format spec](reference/format-spec.md) | Byte layout and compatibility invariants |
| Contributors and reviewers | [Architecture](design/architecture.md) and [Product trade-offs](design/product-tradeoffs.md) | System shape and design rationale |

### Curious first-time reader

Read **[Why liel](why-liel.md)**, then skim **[product trade-offs](design/product-tradeoffs.md)**.
This path explains the memory problem, the single-file shape, and the deliberate limits before you read APIs.

### Trying it from Python

Start with the README, then read the **[Quickstart](guide/quickstart.md)** and **[Python API](guide/connectors/python.md)**.
It covers the API rooted at `liel.open()`, exceptions, transactions, the QueryBuilder, and operational guidance for scans and bulk export.

### Connecting an AI agent such as Claude

Use the **[MCP server](guide/mcp/index.md)**.
Enable it with `pip install "liel[mcp]"` and start it with `liel-mcp --path my.liel`.
The official MCP surface is fixed to seven tools:
`liel_overview`, `liel_find`, `liel_explore`, `liel_trace`, `liel_map`,
`liel_append`, and `liel_merge`.
The exposed tools are documented in the **[Tools reference](guide/mcp/tools.md)**, and practical agent behavior lives in the **[AI memory playbook](guide/mcp/agent-memory.md)**.

### Contributing code

Read **[CONTRIBUTING.md](https://github.com/hy-token/liel/blob/main/CONTRIBUTING.md)**, then use **[architecture overview](design/architecture.md)** for module shape and **[product trade-offs](design/product-tradeoffs.md)** for scope boundaries.

### Understanding the file format

The **[format spec](reference/format-spec.md)** is the canonical file-layout reference.
Read it with **[product trade-offs](design/product-tradeoffs.md)** §6 when you need both byte layout and rationale.

### Building a connector or ecosystem tool in another language

Start with **[format spec](reference/format-spec.md)** for byte layout, **[Python API](guide/connectors/python.md)** for behavior exposed to users, and **[reliability](reference/reliability.md)** for commit and recovery semantics.

### Want a quick overview

- [Why liel](why-liel.md) - problem, before/after, and product positioning
- [Design entry point](design/index.md) - philosophy and trade-offs
- [Behavior and specifications](reference/index.md) - features, reliability, and byte format
- [Feature list](reference/features.md) - what is provided at a glance
- [Reliability and failure model](reference/reliability.md) - commit semantics, crash recovery, and repair guidance
- [Product trade-offs](design/product-tradeoffs.md) - what liel does, what it does not do, and why

---

## Site structure

| Section | Audience |
|---|---|
| **[Why liel](why-liel.md)** | New users deciding whether this solves their AI memory problem |
| **[Guide](guide/index.md)** | Application and tool users using Python or MCP |
| **[Reference](reference/index.md)** | Users and connector authors checking behavior and file compatibility |
| **[Design](design/index.md)** | Anyone reviewing architecture, scope, and trade-offs |
| **[`docs/internal/`](https://github.com/hy-token/liel/blob/main/docs/internal/README.ja.md)** | Maintainers working on implementation, release flow, and internal documentation policy |

Primary sources of truth:

- The byte layout lives in **[format spec](reference/format-spec.md)**.
- The Python exception hierarchy lives in **[Python API](guide/connectors/python.md#exceptions)**.
- The commit/fsync/recovery contract lives in **[reliability](reference/reliability.md)**.
- AI tool operating rules live in **[AI memory playbook](guide/mcp/agent-memory.md)**.
- Product decisions and explicit non-goals live in **[product trade-offs](design/product-tradeoffs.md)**.
- Folder layout and the **document-role SSOT index** (section 7) live in **[documentation taxonomy](https://github.com/hy-token/liel/blob/main/docs/internal/process/documentation-taxonomy.ja.md)** (Japanese; table in §7).

---

## Repository

[github.com/hy-token/liel](https://github.com/hy-token/liel)

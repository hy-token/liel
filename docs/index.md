# liel documentation

**liel** is single-file graph memory for local AI tools, agents, and Python workflows.

It stores facts, decisions, tasks, files, sources, tool results, and their relationships in one portable `.liel` file. The core package has no required runtime dependencies and does not need an external database server, cloud service, or background daemon.

Under the hood, `liel` is a Rust-core embedded **Property Graph database** with Python bindings and optional MCP integration.

---

## Start here

### Python user installing with `pip install liel`

Read the **[Python API](guide/connectors/python.md)**.
It covers the API rooted at `liel.open()`, exceptions, transactions, the QueryBuilder, and operational guidance for scans and bulk export.

### Integrating with an AI agent such as Claude

Use the **[MCP server](guide/mcp/index.md)**.
Enable it with `pip install "liel[mcp]"` and start it with `liel-mcp --path my.liel`.
The official MCP surface is fixed to eight tools:
`liel_overview`, `liel_find`, `liel_explore`, `liel_trace`, `liel_map`,
`liel_remember`, `liel_remember_batch`, and `liel_connect`.
The exposed tools are documented in the **[Tools reference](guide/mcp/tools.md)**, and practical agent behavior lives in the **[AI memory playbook](guide/mcp/agent-memory.md)**.

### Building a connector or ecosystem tool in another language

The **[format spec](reference/format-spec.md)** is the canonical file-layout reference.
For the high-level picture, see the **[architecture overview](design/architecture.md)**.
For frozen scope and deliberate non-goals, see **[product trade-offs](design/product-tradeoffs.md)**.

### Want a quick overview

- [Design entry point](design/index.md) - philosophy and trade-offs
- [Behavior and specifications](reference/index.md) - features, reliability, and byte format
- [Feature list](reference/features.md) - what is provided at a glance
- [Reliability and failure model](reference/reliability.md) - commit semantics, crash recovery, and repair guidance
- [Product trade-offs](design/product-tradeoffs.md) - what liel does, what it does not do, and why

---

## Site structure

| Section | Audience |
|---|---|
| **[Guide](guide/index.md)** | Application and tool users using Python or MCP |
| **[Reference](reference/index.md)** | Users and connector authors checking behavior and file compatibility |
| **[Design](design/index.md)** | Anyone reviewing architecture, scope, and trade-offs |
| **[`docs/internal/`](https://github.com/hy-token/liel/blob/main/docs/internal/README.ja.md)** | Maintainers working on implementation, release flow, and internal documentation policy |

Primary sources of truth:

- The byte layout lives in **[format spec](reference/format-spec.md)**.
- Product decisions and explicit non-goals live in **[product trade-offs](design/product-tradeoffs.md)**.
- Internal documentation policy lives in **[documentation taxonomy](https://github.com/hy-token/liel/blob/main/docs/internal/process/documentation-taxonomy.ja.md)**.

---

## Repository

[github.com/hy-token/liel](https://github.com/hy-token/liel)

# Why liel

AI tools are getting better at reasoning, but they still lose too much useful context between sessions.

Chat logs are long, vector search finds similar text, and project notes drift out of date. What is often missing is durable memory that keeps the relationships between decisions, tasks, files, sources, and tool results.

`liel` exists to make that memory small, local, and explicit.

---

## The problem

Most AI memory systems start from text:

- conversations are saved as transcripts
- documents are split into chunks
- later questions retrieve similar passages

That is useful when the main question is "what text looks relevant?"

It is weaker when the question is relational:

- Which decision led to this task?
- What source supported that claim?
- Which file changed because of this tool result?
- What open questions came out of the last session?
- Why did the agent choose this approach instead of another?

Those answers are not just chunks of text. They are links between things.

---

## Before and after

Without `liel`:

- context disappears into chat history
- decisions are hard to reuse
- sources, files, tasks, and tool outputs are not connected
- an agent has to reconstruct project memory from prose
- memory depends on a hosted service, database server, or ad hoc files

With `liel`:

- decisions, tasks, files, sources, and observations become nodes
- relationships are stored as explicit edges
- context survives across sessions in one `.liel` file
- memory can be traversed, inspected, copied, backed up, and archived
- the same file can be used from Python or exposed through MCP

The goal is not to replace every form of retrieval. The goal is to give AI workflows a durable relationship layer.

---

## Why graph memory

LLM workflows often need to remember *why* something happened, not only *what* was said.

A graph is a natural fit because memory is usually connected:

- a user request leads to a task
- a task produces a decision
- a decision is supported by a source
- a source points to a file
- a file change creates a follow-up task

Storing those links directly makes memory easier to traverse later. Instead of asking an AI tool to infer structure from old transcripts every time, you can persist the structure as part of the work.

`liel` is therefore not just a lightweight graph database example. It is a portable external-brain substrate for relationship-centric AI tools.

---

## Why single-file

Local AI tools often need memory that is easy to adopt, move, and trust.

A single `.liel` file keeps the operational model simple:

- no database server to deploy
- no cloud account required
- no background daemon
- no required runtime dependencies in the core package
- easy copy, backup, inspection, and archival

This matters for agents and local tools because memory should be close to the project it describes. A `.liel` file can live next to source code, notes, experiments, or generated artifacts.

If SQLite is the one-file relational database, `liel` aims to be the one-file graph memory layer for local AI workflows.

---

## Why not just RAG

RAG is valuable when the primary task is retrieving relevant passages from text.

`liel` is for a different access pattern. It stores facts and relationships so tools can traverse memory:

- from task to decision
- from decision to source
- from file to related tool result
- from open question to the context that created it

In practice, these can work together. Use RAG for semantic document lookup. Use `liel` for durable, inspectable relationships that should survive the session.

---

## When this matters

`liel` is useful when an AI tool or application needs to remember:

- project decisions and their rationale
- tasks and follow-up work
- source material and provenance
- files touched during a workflow
- tool results that should remain connected to later actions
- long-running agent state that should stay local

It is intentionally small. It is not a hosted memory platform, a vector database, a full-text search engine, or a server-backed graph database.

That narrowness is the point: `liel` gives higher-level memory systems a durable local graph foundation without forcing them to adopt a heavy database stack.

---

## Where to go next

- Start with the [Python API](guide/connectors/python.md) if you want to use `liel` directly.
- Start with the [MCP guide](guide/mcp/index.md) if you want an AI tool such as Claude to use a `.liel` file as memory.
- Read the [product trade-offs](design/product-tradeoffs.md) if you want to understand the deliberate limits.
- Read the [format spec](reference/format-spec.md) if you want to build compatible tooling.

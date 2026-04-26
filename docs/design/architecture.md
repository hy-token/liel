# Architecture overview

`liel` is a **portable external brain** for LLMs and local AI tools. Internally, that memory is persisted as a **property graph** inside a **single file** (`.liel`). There is no server process; you carry the library and the `.liel` file with the workflow that needs the memory.

In other words, "property graph database" describes the storage engine, while "portable external brain" describes the product promise.

- **Logical model** — how users see nodes, edges, labels, and properties.
- **Physical model** — how those map to disk (pages, fixed-size slots, adjacency lists).
- **Durability** — the role of the WAL in making crashes recoverable.

The byte-level layout (offset table) is consolidated in **[format spec](../reference/format-spec.md)**. The decisions that fix the format and the explicit product trade-offs live in **[product trade-offs](product-tradeoffs.md)**.

---

## Logical model (property graph memory)

- **Node** — `id` (auto-assigned u64), one or more **labels**, optional **properties** (key/value map).
- **Edge** — `id` (auto-assigned), **source** and **target** nodes, **type** (label-equivalent), optional properties.
- Multiple edges of the same type can connect the same pair of nodes.

For the API surface and limits, see **[feature list](../reference/features.md)**.

---

## Physical model: pages and slots

A `.liel` file is treated as a sequence of **fixed 4096-byte pages**. The first 128 bytes of **page 0** are the **file header**; the remaining 3968 bytes of page 0 are currently unused. The **WAL** lives at a fixed location: a 4 MiB reservation starting at **byte offset 4096** (page-aligned). Nodes and edges live in **fixed-size slots** (64 B and 80 B respectively); variable-length data — label strings and properties — lives in separate **property extents**, referenced from the slot by absolute file offset.

This low-level layout matters because the product's portability depends on a single file that is easy to move, copy, back up, and reopen later as the same memory.

Separating fixed-size slots from variable-length payload makes slot arrays scannable at a **constant stride**, which keeps the implementation simple. The numeric details and field layouts are owned exclusively by **[format spec](../reference/format-spec.md)**.

---

## Adjacency lists

Connectivity is represented as **singly linked lists**. Each node carries the ID of its first outgoing edge and its first incoming edge; each edge carries the ID of the next edge that shares the same endpoint (`next`). Traversal does not need an RDB-style JOIN — neighbours are reached at a cost proportional to the degree.

Deleting an edge requires **re-linking** the list to splice the edge out (see `src/graph/edge.rs`).

---

## WAL and commit (how durability works)

Writes are first appended to the **Write-Ahead Log** at page granularity. On **commit** the modified pages are then written to the data file and the WAL is cleared. On startup, any leftover WAL is replayed to restore the data pages.

The order — WAL durable first, then data pages — is mandatory. The byte layout of WAL entries lives in **[format spec §6](../reference/format-spec.md)**.

---

## Query and execution model

`liel` intentionally has no query planner or Cypher-like language. The public
query model is the Python-first API:

- direct ID lookup for `get_node` / `get_edge`
- adjacency-list traversal for `out_edges` / `in_edges` / `neighbors`
- BFS-based traversal for `bfs`, `dfs`, and directed unweighted
  `shortest_path`
- QueryBuilder full scans with optional label prefilter and Python
  `where_` predicate

The in-memory `LabelIndex` is rebuilt on `open()` and maps labels to node IDs to
reduce node scan candidates. It is not a general property index. Arbitrary
property predicates remain scans in the current Beta series.

---

## Consistency and concurrency model

The consistency model is deliberately narrow:

- one writer process owns a `.liel` file at a time
- `open()` starts an implicit transaction
- `commit()` is the durability boundary
- `rollback()` discards uncommitted dirty pages
- `transaction()` provides a commit-on-success / rollback-on-error context
- leftover complete WAL is replayed on the next `open()`
- unsafe double-writer opens fail closed with `AlreadyOpenError`

This is a single-file embedded store, not a server database. Multiple peer
writers, network filesystem semantics, and server-style scheduling are outside
the contract. The operational details live in
**[reliability](../reference/reliability.md)** and
**[single-writer guard](single-writer-guard.md)**.

---

## API contract

The Beta public contract is Python-first. The compatibility surface is
`liel.open`, the documented `GraphDB` methods, `Node` / `Edge`, QueryBuilder,
transaction types, merge reports, and the `GraphDBError` exception hierarchy.

The Rust modules are the implementation boundary for the Python package. They
are kept small and testable, but they are not yet promised as a stable external
Rust API in the same way as the Python package surface.

---

## AI memory contract

The product promise is not "a complete graph database server." It is a durable,
portable graph substrate for AI memory:

- store facts, decisions, tasks, sources, files, and tool results as graph
  records
- preserve relationships and provenance across sessions
- expose the same `.liel` file through Python and the optional MCP server
- support append/merge style memory writes with explicit commit boundaries

`liel` does not provide semantic vector retrieval, embedding search, reasoning
quality guarantees, or multi-agent concurrent writes by itself. Those belong in
the application or agent layer above the storage engine.

---

## Software layers (dependency direction)

Bottom up: **single file** → **storage** (pages, WAL, cache, serialization) → **graph** (CRUD, adjacency lists, traversal) → **query** (QueryBuilder) → **`GraphDB` facade** → **Python bindings**.

The mapping between this layering and the actual Rust modules under `src/` is maintainer-facing and lives outside the public docs site (see `CONTRIBUTING.md` for entry points into the codebase).

---

## Related documents

Start from the **[design overview](index.md)** to navigate design, reference, and guide sections.

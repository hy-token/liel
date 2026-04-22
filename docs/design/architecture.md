# Architecture overview

`liel` is a **property graph** database that persists into a **single file** (`.liel`). There is no server process; you embed the library directly in your application.

- **Logical model** — how users see nodes, edges, labels, and properties.
- **Physical model** — how those map to disk (pages, fixed-size slots, adjacency lists).
- **Durability** — the role of the WAL in making crashes recoverable.

The byte-level layout (offset table) is consolidated in **[format spec](../reference/format-spec.md)**. The decisions that fix the format and the explicit product trade-offs live in **[product trade-offs](product-tradeoffs.md)**.

---

## Logical model (property graph)

- **Node** — `id` (auto-assigned u64), one or more **labels**, optional **properties** (key/value map).
- **Edge** — `id` (auto-assigned), **source** and **target** nodes, **type** (label-equivalent), optional properties.
- Multiple edges of the same type can connect the same pair of nodes.

For the API surface and limits, see **[feature list](../reference/features.md)**.

---

## Physical model: pages and slots

A `.liel` file is treated as a sequence of **fixed 4096-byte pages**. The first 128 bytes of **page 0** are the **file header**; the remaining 3968 bytes of page 0 are currently unused. The **WAL** lives at a fixed location: a 4 MiB reservation starting at **byte offset 4096** (page-aligned). Nodes and edges live in **fixed-size slots** (64 B and 80 B respectively); variable-length data — label strings and properties — lives in separate **property extents**, referenced from the slot by absolute file offset.

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

## Software layers (dependency direction)

Bottom up: **single file** → **storage** (pages, WAL, cache, serialization) → **graph** (CRUD, adjacency lists, traversal) → **query** (QueryBuilder) → **`GraphDB` facade** → **Python bindings**.

The mapping between this layering and the actual Rust modules under `src/` is maintainer-facing and lives outside the public docs site (see `CONTRIBUTING.md` for entry points into the codebase).

---

## Related documents

Start from the **[design overview](index.md)** to navigate design, reference, and guide sections.

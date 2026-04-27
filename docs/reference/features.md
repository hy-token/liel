# Feature list (cheat sheet)

A quick reference of what `liel` provides. For API details, see the [Python API](../guide/connectors/python.md).

The **deliberate non-goals** (with considered alternatives, why-rejected, why-chosen) live in [product trade-offs](../design/product-tradeoffs.md).

!!! note "If your need is outside the use case"
    Highly concurrent servers, Cypher, terabyte-scale, full-text-led workloads — for these, **use a different mature tool**. The README's *When NOT to use* table lists representative alternatives. liel concentrates on **standalone, single-file, dependency-light**.

---

## 1. Product positioning (core value)

| Item | Detail |
|---|---|
| Form factor | Portable external-brain library; no DB server process |
| Persistence | **Single file** `.liel` (or `:memory:`) |
| Model | **Property graph** (nodes and edges with labels and properties) |
| Dependencies | Rust core kept minimal; no external service required at runtime |
| Query | **No Cypher.** Python API and **QueryBuilder** (method chaining) |

---

## 2. Database lifecycle

| Feature | Description |
|---|---|
| `liel.open(path)` | Open a DB by file path or `:memory:` |
| `close()` / context manager | End the session and release resources |
| Crash safety | **Page-level WAL** for commit consistency. `commit()` is the durability boundary and is limited by disk sync latency; batch bulk inserts with `db.transaction()`. The detailed fsync/recovery contract lives in [reliability and failure model](reliability.md); byte layout lives in [format spec §6](format-spec.md#6-wal-write-ahead-log). |

---

## 3. Nodes

| Feature | Description |
|---|---|
| Create | `add_node(labels, **props)` — multiple labels, recursive property values |
| Read | `get_node` / `all_nodes` / `node_count` |
| Update / delete | `update_node` / `delete_node` (also deletes incident edges). If an error occurs while deleting incident edges, the call returns the exception and **dirty pages are left as-is**; without calling `commit()`, a `close()` → `open()` rolls back to the last committed state. `rollback()` has the same effect. |
| Enumerate / search | Via the QueryBuilder (below) |

---

## 4. Edges

| Feature | Description |
|---|---|
| Create | `add_edge(from, label, to, **props)` |
| Idempotent create | `merge_edge` — reuses an existing edge if `(from, label, to, **props)` matches exactly, otherwise creates one (see [product trade-offs §6.6](../design/product-tradeoffs.md)) |
| Read | `get_edge` / `all_edges` / `edge_count` |
| Update / delete | `update_edge` / `delete_edge` (deletion re-links the adjacency lists) |

---

## 5. Adjacency and traversal

| Feature | Description |
|---|---|
| Edge enumeration | `out_edges` / `in_edges` |
| Neighbour nodes | `neighbors` (filterable by edge label) |
| Traversal | `bfs` / `dfs` (with optional max depth) |
| Shortest path | `shortest_path` — **directed, unweighted, minimum-hop** (BFS-based). Edge properties are not used as weights. |
| Bulk read | `all_nodes_as_records` / `all_edges_as_records` (dict-based bulk fetch) |
| Helpers | `degree_stats` (full edge scan to aggregate degrees) / `edges_between` (full edge scan, then filter by endpoint set) |

---

## 6. QueryBuilder (filtered enumeration)

| Target | Example |
|---|---|
| Nodes | `db.nodes().label(...).where_(...).skip(...).limit(...).fetch()`, etc. |
| Edges | `db.edges().label(...).where_(...).count()` / `exists()`, etc. |

---

## 7. Transactions

| Feature | Description |
|---|---|
| Explicit | `commit` / `rollback` (`begin()` is a compatibility **no-op**) |
| Context | `transaction()` |

---

## 8. Maintenance and metadata

| Feature | Description |
|---|---|
| `vacuum()` | Compact the property region (crash-safe via copy-on-write + atomic rename) |
| `clear()` | Fully reset the database to an empty state, discard dirty pages, and reset ID counters |
| `repair_adjacency()` | Rebuild node adjacency heads, degree counters, and edge next-pointers from the live edge set |
| `info()` | Read metadata and statistics |

`vacuum()` on a file-backed database operates as a copy-on-write rewrite ([product-tradeoffs.md §5.6](../design/product-tradeoffs.md)): a sibling `<file>.liel.tmp` is built, fsynced, and atomically renamed over the live file.  **A crash mid-vacuum leaves the original file intact**, and the next `liel.open()` reclaims any leftover `.tmp` on its own.  `:memory:` databases fall back to the original in-place algorithm (there is nothing to crash-corrupt).  The only caveat is that disk usage temporarily peaks at 2× while vacuum runs.
| `merge_from(other, *, node_key=None, edge_strategy="append", on_node_conflict="keep_dst")` | Import all nodes and edges from another `GraphDB`. IDs are remapped automatically (no file-format change). With `node_key`, an existing node can be reused based on a property key; with `edge_strategy="idempotent"`, edges are deduplicated like `merge_edge`. The returned `MergeReport` contains the src→dst ID map and per-class counts. |

---

## 9. Properties and types

The on-disk encoding is a **custom binary format** (no external `serde`). The canonical type tags and byte layout live in [format spec](format-spec.md).

---

## 10. Notes for Python developers

| Item | Detail |
|---|---|
| `liel.__version__` | Matches the installed Python package version |
| Type hints | `python/liel/liel.pyi` |
| Exceptions | `GraphDBError` and its subclasses (`NodeNotFoundError`, …) — see [Python API](../guide/connectors/python.md) |

---

## 11. Practical scale (guidance, not a guarantee)

For everyday use, **a few gigabytes of file size** is typically a comfortable working range. Hardware and workload pattern can change this significantly, so this is **operational guidance, not a numeric guarantee**. Extreme graphs and highly concurrent writes are out of scope.

For scan-heavy and traversal-heavy APIs, see the performance notes embedded in the **[Python guide](../guide/connectors/python.md)**.

---

## 12. Not included (cheat sheet)

The reasoning ("why not, considered alternatives, why rejected, why chosen") is consolidated in **[product trade-offs](../design/product-tradeoffs.md)**. Items only:

- Cypher / custom DSL
- Weighted shortest paths (Dijkstra etc.)
- Undirected-only graph model
- Property index
- Standard reserved metadata keys (creation time, update time, source, session); these may be added later, but no metadata convention is enforced today
- Full-text search and aggregation engines
- Visualization API in core
- JSON export/import in core
- Server mode
- Concurrent mutation by multiple writer processes on the same file (dangerous double-open is rejected with `AlreadyOpenError`)
- WASM / browser distribution
- C FFI / other-language bindings (out of phase)

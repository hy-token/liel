# Product trade-offs (what we do, what we do not, and why)

`liel` is a **portable external brain for LLM workflows** built on top of a single-file graph storage engine. It prioritises **single-file persistence, local portability, and minimal dependencies**. As a result, expectations carried over from a general-purpose or server-style graph database will not always hold.

This page is the canonical record of liel's **deliberate non-goals**, written in a uniform five-point template:

- **Current choice** — what is actually implemented today.
- **Considered alternatives** — options that were dropped or pushed out of phase.
- **Why rejected** — why those alternatives are not in liel.
- **Why chosen** — why the current shape is the one we ship.
- **Trade-off** — what we give up by making this choice.

**Document role:** Canonical **product design** decisions (non-goals and rationale). **Folder layout** in this repo: [documentation taxonomy](../internal/process/documentation-taxonomy.ja.md) sections 1–6; **which document is authoritative for what**: **section 7 (SSOT index)** (Japanese). On-disk contract: [format spec](../reference/format-spec.md). Maintainer build order and backlog index: [internal process index](https://github.com/hy-token/liel/blob/main/docs/internal/process/index.ja.md).

The decisions below — especially everything in §6 — are **frozen**. Anything in §6 affects file-format compatibility and will not change for any reason short of a major version bump with a migration path.

---

## 1. Core value (what liel does)

- Provides a durable, portable external brain for AI tools by persisting graph memory into a **single `.liel` file** or `:memory:`.
- **Rust core + Python bindings (PyO3)**. No DB server process.
- Internally uses a property graph model (nodes/edges with labels and arbitrary properties) as the memory substrate.
- CRUD, adjacency lists, BFS / DFS, unweighted shortest paths, QueryBuilder, `merge_edge`, `vacuum`, explicit `commit` / `rollback`.
- Minimal runtime dependencies. The Rust core depends on essentially `pyo3` only.

---

## 2. Cheat sheet: do / don't

### 2.1 Do

| Area | Coverage |
|---|---|
| Data model | Property graph (multiple labels, nested properties) |
| Storage | Single file `.liel` / `:memory:`, fixed 4 KB pages |
| Durability | Page-level WAL + two-phase fsync on `commit()` |
| Adjacency / traversal | `out_edges` / `in_edges` / `neighbors` / `bfs` / `dfs` |
| Shortest path | `shortest_path` (**directed, unweighted, minimum-hop**, BFS-based) |
| Query | `db.nodes()…fetch()` / `db.edges()…count()` QueryBuilder |
| Transactions | `commit` / `rollback` / `with db.transaction()` |
| Operations | `vacuum` / `clear` / `info` |

### 2.2 Don't

| Area | Not provided | Detail |
|---|---|---|
| Query language | Cypher / custom DSL | §4.1 |
| Shortest path | Weighted (Dijkstra etc.) | §4.2 |
| Traversal | Undirected-only model | §4.3 |
| Index | Property index | §4.4 |
| Search | Full-text / aggregation engines | §4.5 |
| UI | Visualization API in core | §4.6 |
| Interchange | JSON export/import in core | §4.7 |
| Form factor | Server mode | §4.8 |
| Concurrency | Multiple-process writers on the same file | §5.1 |
| WAL | Record granularity | §5.2 |
| Space | Reusing deleted slots | §5.3 |
| I/O | mmap | §5.4 |
| Implementation | External serialization crates | §7.1 |
| Implementation | External LRU / CRC / thiserror | §7.2 |
| Distribution | WASM / browser | §8.1 |
| Distribution | C FFI / other-language bindings | §8.2 |

---

## 3. How to read this page

Each entry uses the five-point template (current / alternatives / rejected / chosen / trade-off). Cross-check against this page when:

- Proposing a new feature — does it conflict with "single file, minimal dependencies"?
- Designing MCP tools or user scripts — read §4.4 / §5.1 / §5.2 to decide on batching.
- Writing a connector for another language — §6 and the [format spec](../reference/format-spec.md) are the canonical references.

---

## 4. Scope and API-level decisions

### 4.1 No Cypher or custom DSL

- **Current choice:** Python API and the QueryBuilder (method chaining) only.
- **Considered alternatives:**
  - (a) A Cypher subset.
  - (b) A custom string query language (DSL).
  - (c) GraphQL-style schema-driven queries.
- **Why rejected:**
  - In-house parser/planner/executor would multiply core size by an order of magnitude and break the "single file, minimal dependencies" position.
  - Conflicts head-on with the policy of avoiding external crates (§7).
  - A query language tends to outlive the engine in spec; for Phase 1's targets (local, research, prototyping) it is overkill.
- **Why chosen:** The Python QueryBuilder covers the typical Cypher uses (label filter, property predicate, skip/limit, count/exists) while letting users keep Python's type completion and `lambda` expressiveness.
- **Trade-off:** Use cases that compose query strings dynamically (BI dashboards etc.) do not fit.

### 4.2 No weighted shortest path (Dijkstra etc.)

- **Current choice:** `shortest_path` is **directed, unweighted, minimum-hop** (BFS-based). Edge properties are not used as weights.
- **Considered alternatives:**
  - (a) Dijkstra (non-negative weights).
  - (b) Bellman-Ford (negative weights allowed).
  - (c) A\* (heuristic).
- **Why rejected:**
  - Letting the API choose "which property is the weight" pulls in type validation, missing-value handling, multi-cast (int / float) conversion, and other surrounding spec.
  - Priority queues and negative-weight handling add core lines that earn little in the target use cases (knowledge graphs, dependency graphs).
- **Why chosen:** "Is there a relation? How many hops away?" is the primary question, and BFS answers it well. Applications that need more can build their own queue on top of `out_edges`.
- **Trade-off:** Distance-, time-, or cost-weighted graph optimization is not a first-class feature.

### 4.3 No undirected-only graph model

- **Current choice:** All edges are directed. `shortest_path` / `bfs` / `dfs` follow out-edges.
- **Considered alternatives:**
  - (a) An `undirected` flag on edges.
  - (b) Separate APIs (`undirected_bfs`, etc.).
  - (c) Add a `direction=in/out/both` parameter to traversal APIs.
- **Why rejected:**
  - (a) and (b) add branches to the adjacency-list code and traversal code; either storage or API has to grow.
  - (c) keeps storage but expands Phase 1 scope. Possible future sub-milestone.
- **Why chosen:** Undirected-like relations are naturally modelled by **two directed edges** (one each way). Storage, API, and meaning stay aligned.
- **Trade-off:** With two directed edges, the application is responsible for storage cost and consistency (e.g. delete both sides).

### 4.4 No property index (today)

- **Current choice:** QueryBuilder does full scans. Cheap meta is available directly (`db.info()`, `node_count()`, `edge_count()`).
- **Considered alternatives:**
  - (a) Hash index on property values.
  - (b) Ordered index (e.g. B+Tree).
  - (c) Per-label secondary index.
- **Why rejected:**
  - Index pages would expand the on-disk format and need a migration story; the cost-benefit does not pay off at the small/medium graph sizes Phase 1 targets.
  - Reconciling index types with the dynamic property format (§6.4) requires extra spec.
- **Why chosen:** For local and small/medium graphs, **full scan + early termination (`exists` / `limit`)** delivers practical latency without adding new core invariants.
- **Trade-off:** Interactive conditional search on huge graphs is not recommended. If it becomes a need, design a sub-milestone with "format v2 + migration".

Performance guidance now lives next to the APIs that trigger it: user-facing load notes belong in the Python guide, and Rust-implementation hot spots belong in the internal Rust module map. This page keeps only the design reason why those full scans exist at all.

### 4.5 No full-text search or aggregation engine

- **Current choice:** Aggregations go through `all_*_as_records` and are computed in Python (e.g. with pandas). Full-text search is out of scope.
- **Considered alternatives:**
  - (a) Bundled inverted index (e.g. tantivy).
  - (b) SQL-style aggregation API (`group_by` / `sum` / `avg`).
- **Why rejected:**
  - (a) requires either an external crate or significant in-house code, conflicting with §7.
  - (b) drags in spec around aggregation types, NULL behaviour, Decimal, etc., duplicating what pandas/NumPy already do.
- **Why chosen:** Concentrating on "store and walk relationships" lets us keep the core thin and delegate numeric work and search to mature Python tools.
- **Trade-off:** Search-led or aggregation-led use cases need a different tool.

### 4.6 No visualization API in the core

- **Current choice:** Neither the Rust core nor the Python package exposes a visualization API. `examples/05_visualization.py` shows a NetworkX + matplotlib integration only.
- **Considered alternatives:**
  - (a) Add `db.to_networkx()` to the core.
  - (b) Expose `db.plot()` with a `matplotlib` dependency.
  - (c) Bundle a Web UI.
- **Why rejected:**
  - Visualization libraries evolve quickly; bundling a dependency drags the core along.
  - Different users want different libraries (pyvis, graph-tool, Cytoscape, …).
- **Why chosen:** `all_nodes_as_records` / `all_edges_as_records` return plain dicts, so users can hand the data to any visualization stack of their choice. The core stays thin.
- **Trade-off:** "One import and a chart appears" is not on offer.

### 4.7 No JSON export/import in the core (Rust)

- **Current choice:** `GraphDB` does not include JSON I/O. Application scripts and `examples/06_export.py` handle it.
- **Considered alternatives:**
  - (a) Implement `GraphDB::export_json` / `import_json` in Rust.
  - (b) Add helpers in the Python package `python/liel/`.
- **Why rejected:**
  - (a) reopens the external-serializer debate and forces a new mapping spec between JSON types and the custom property format (§6.4).
  - Putting it in Rust would also turn JSON spec quirks (numeric handling, map key order) into part of the core contract.
- **Why chosen:** Python's `json` module is enough; option (b) is a possible future addition that does not touch Rust.
- **Trade-off:** Connectors in other languages have to write their own JSON conversion.

### 4.8 No server mode

- **Current choice:** No daemon process. liel is embedded as a library inside an application process.
- **Considered alternatives:**
  - (a) TCP / gRPC server mode (Neo4j-style).
  - (b) HTTP REST wrapper.
- **Why rejected:**
  - Going server-side pulls in authentication, connection management, multi-tenancy, and a new category of requirements.
  - Conflicts with the product positioning ("single file; backup is a copy").
- **Why chosen:** "If you need a server, call liel from MCP / FastAPI / your own server" is enough. The server-side liability is explicitly left to the user.
- **Trade-off:** Unsuitable for systems that assume many concurrent users over a network (also relates to §5.1).

---

## 5. Storage, durability, and concurrency decisions

### 5.1 Multi-process writers on the same file are rejected

- **Current choice:** A `<file>.lock/` directory rejects a second writer process. This is not multi-writer support; dangerous conflicts fail with `AlreadyOpenError`.
- **Considered alternatives:**
  - (a) `fcntl` / Windows `LockFileEx` based file locking.
  - (b) A custom inter-process coordination protocol.
  - (c) SQLite-style WAL + shared memory.
- **Why rejected:**
  - Cross-platform locking has wide OS variance and would break the "pyo3 only" dependency policy (§7.2).
  - Retry-based read/write locks would greatly expand the concurrency contract.
- **Why chosen:** A lock directory uses only the standard library, does not change the on-disk format, and can reclaim stale locks after crashes by checking the owner PID.
- **Trade-off:** Concurrent writes are still not supported. The guarantee is centered on normal local filesystems; network filesystems and sync folders are outside the comfort zone. **Recommended pattern:** when sharing is needed (e.g. an MCP server), centralise to one process and have everyone else write through RPC.

### 5.2 WAL is page-grained (full 4 KB pages)

- **Current choice:** On `commit()`, every modified data page is appended to the WAL **as a full 4 KB**, with the order: WAL fsync → data page write → fsync. After commit, the header `wal_length` resets to zero. The WAL bytes live in a **fixed in-file region** (4 MiB at byte offset 4096; preceded by **page 0**, which is 4096 B and starts with the 128-byte file header).
- **Considered alternatives:**
  - (a) Record-level WAL (log only the changed fields).
  - (b) Double-write (shadow region inside the data file).
  - (c) Keep the WAL and consolidate via checkpoint (SQLite WAL mode).
- **Why rejected:**
  - (a) cuts write volume but complicates recovery and grows bug surface.
  - (c) requires readers to consult the WAL, expanding both code and spec.
- **Why chosen:** "Copy WAL pages back as-is" makes recovery a single straight path with very little room for bugs. WAL bloat is a non-issue at Phase 1's scale.
- **Trade-off:** Even a one-byte change writes 4 KB. **High-frequency tiny commits do not fit.** Recommend "one commit per session" or "commit every N operations or T seconds".

### 5.3 Deleted slots are not reused (monotonic IDs)

- **Current choice:** Node and edge deletion only flips a flag bit; IDs are not reissued. Full reset goes through `db.clear()`.
- **Considered alternatives:**
  - (a) Introduce a freelist and reuse empty slots.
  - (b) Generation-tagged IDs that are safe to reuse.
  - (c) Renumber to consecutive IDs during vacuum.
- **Why rejected:**
  - (a) and (b) require managing the risk that a deleted ID "comes back" as a different entity, which easily breaks application-side persisted references.
  - (c) needs reference repair across the graph and is hard to automate.
- **Why chosen:** Monotonic IDs guarantee "an ID we have ever returned still points to the same thing in the future" — application caches and external joins do not break.
- **Trade-off:** Long-lived workloads with high deletion frequency accumulate dead slots (tens of MB). Operationally, `db.clear()` performs a full reset.

### 5.4 No mmap (use `std::fs` read/write)

- **Current choice:** All I/O goes through `std::fs::File` read / write / seek. The page cache is a hand-written LRU (§7.2).
- **Considered alternatives:**
  - (a) `memmap2` crate.
  - (b) Direct `mmap` / `MapViewOfFile` calls per OS.
- **Why rejected:**
  - Behaviour differs across Windows / macOS / Linux (SIGBUS vs exceptions, shared writes).
  - More complex than the in-house LRU and harder to reason about for fsync semantics.
- **Why chosen:** Explicit read/write is consistent across platforms and makes WAL ordering straightforward to write.
- **Trade-off:** Read-heavy workloads cannot benefit from OS-cache mmap optimization. Possibly revisited later for a read-only path.

### 5.5 Nested transactions are forbidden (re-entering `transaction()` errors, implemented)

- **Current choice:** A transaction is implicitly begun right after `open()`. Explicit transactions are exposed as **Rust-level `GraphDB::transaction()`** returning a `TransactionGuard` and as Python's `with db.transaction()`; both shapes auto-commit on success and auto-rollback on `Drop` / exception.  **Re-entering `transaction()` while one is already active raises `LielError::TransactionError("transaction already active")`** (`TransactionError` on the Python side).  The Rust guard borrows `&mut GraphDB` for its full lifetime so nesting is also rejected by the borrow checker — defence in depth.  As a related rule, **calling `vacuum()` inside an explicit transaction is rejected** with `TransactionError`: vacuum forces an internal `commit()` and would otherwise silently flush the surrounding transaction's staged work.
- **Considered alternatives:**
  - (a) SQL-style savepoints (inner rollback unwinds partially).
  - (b) Flat nesting (only the outer scope commits; inner commits are no-ops).
  - (c) Forbid nesting (re-entry is an immediate error).
- **Why rejected:**
  - (a) would require a savepoint marker in the WAL, conflicting head-on with §5.2's "page-grained, single-path recovery".
  - (b) silently swallows commit-timing bugs, which works against an embedded DB's explicit-durability model.
- **Why chosen:** Given §5.1 (single writer), a single-scope transaction model is sufficient. Forbidding re-entry surfaces caller bugs (confusion about which `rollback` unwinds what) at the first offence.
- **Trade-off:** Library code that wants a narrow commit unit inside a wider caller transaction must be designed around that outer scope. If savepoints ever become necessary, they will be introduced together with a WAL major bump.

### 5.6 Vacuum is crash-safe via copy-on-write + atomic rename (implemented)

- **Current choice:** File-backed `vacuum()` uses **(β) copy-on-write + atomic rename** (`src/graph/vacuum.rs::vacuum_to_tmp_and_rename`).  Writes a sibling `<basename>.liel.tmp` that contains **live node/edge slots together with a compacted property region**, then `commit()` (WAL fsync → data fsync) followed by `atomic_replace` (POSIX `rename` + parent directory fsync, or Windows `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`).  `:memory:` databases fall back to the in-place algorithm because there is no on-disk state to crash-corrupt.
  - **Stale `.liel.tmp` policy:** `open()` **unconditionally `unlink`s** any sibling `.tmp`. Under §5.1's single-writer guarantee, a concurrent vacuum is not a possibility we have to protect against.
  - **Out of disk:** A dedicated free-space pre-check (`statvfs(2)` / `GetDiskFreeSpaceExW`) is **not implemented**.  An earlier draft of this decision called for one, but it conflicts with the ZEN principle "start small, keep it local"; we deliver the same quality through **the OS surfacing `ENOSPC` on first write → the next `open()`'s sweep unconditionally removes the half-finished `.tmp`**.  After a successful rename a crash is a no-op (the new `.liel` is already atomic-replaced), so the only window that can leave a half-finished tmp is the brief stretch before `commit()` — which the open-time sweep already covers.  If real-world feedback eventually demands a pre-check we can add it later via bare FFI (the same pattern as `src/storage/lock.rs`); doing so would not introduce any new external crate.
- **Considered alternatives:**
  - (α) Add a vacuum entry type to the WAL and use the existing WAL as the crash-consistent vehicle.
  - (β) Copy-on-write + atomic rename (above).
  - (γ) Two-phase marker page toggling between old and new slots.
- **Why rejected:**
  - (α) changes the WAL format (major bump) and breaks §5.2's "page-grained, single-path recovery" invariant.
  - (γ) still needs a separate atomic mechanism for slot updates beyond the marker itself.
- **Why chosen:** Does not touch the WAL format. Aligns with the natural embedded-DB idiom of "write a new file next to it and swap in place". A crash before `rename` leaves the old `.liel` intact; cleanup is bounded to `open`'s `.tmp` sweep.
- **Trade-off:** Disk usage temporarily doubles during vacuum. Network filesystems / sync folders still follow §5.1's recommendation (centralise to a single process).
- **Invariant:** Vacuum **preserves node and edge IDs**. `Pager::next_node_id` / `next_edge_id` are unchanged. Application-side caches and external references stay valid across a vacuum. See [format-spec.md §7](../reference/format-spec.md) for the same invariant.
- **Fault-injection mechanism:** Compiling with the `test-fault-injection` Cargo feature enables `src/graph/fault_inject.rs::crash_at`, which reads `LIEL_VACUUM_CRASH_AT=<name>` and `_exit(1)`s at the matching named injection point.  With the feature off (the default for release builds and ordinary `cargo test`), `crash_at` is `#[inline(always)]` no-op the linker drops; release wheels carry zero injection plumbing.  Python tests build with `maturin develop --features pyo3/extension-module,test-fault-injection` and drive the crash via `fork` + `_exit` (see `tests/python/test_vacuum_crash_safety.py`).
- **Implementation order vs B2:** **C1 landed before B2 (the Rust `transaction()` RAII guard).** C1 was the heavier of the two (cross-platform rename, fault injection, recovery tests) with less predictable lead time; B2 is purely an additive API change that can ride on top of the new vacuum and remains the next ticket.

### 5.7 Mutex poison policy is scope-dependent (maintainer-facing)

This entry records an implementation policy for maintainers. End users only
need the Python-facing rule: if a database lock is poisoned, drop the handle and
open a new `GraphDB` connection.

- **Current choice:** Different policies for different scopes.
  - The **`open_files` registry** in `src/db.rs` recovers via `unwrap_or_else(|p| p.into_inner())`.
  - The **`PyGraphDB` inner `Mutex<Option<GraphDB>>`** in `src/python/types.rs` surfaces `PyRuntimeError("...Open a new GraphDB connection.")` whenever the lock is poisoned.
- **Considered alternatives:**
  - (a) Recover everywhere via `into_inner`.
  - (b) Fail everywhere on poison.
  - (c) Split the policy by scope (chosen).
- **Why rejected:**
  - (a) would let CRUD continue after a panic that may have left dirty pages half-updated — a perfect setup for silent corruption.
  - (b) would force the registry's `Drop` to fail-handle as well, complicating teardown for a registry whose worst poisoned outcome is a spurious `AlreadyOpen` warning.
- **Why chosen:** The registry only tracks "is this path in-process open" — a poisoned registry can over-report `AlreadyOpen` but never under-report it, so data integrity is unaffected. The graph mutex, by contrast, may have been held during a CRUD panic, so forcing the caller to reopen is the only safe answer.
- **Trade-off:** Callers see a uniform "drop the handle and reopen" rule, but maintainers must remember the registry behaves differently — the comment in `db.rs::open` documents the split inline.

---

## 6. File-format decisions (F-01…)

The decisions in this section directly affect **on-disk format compatibility**. Changing them requires a major version bump with migration.

The byte-level reference of record is the **[format spec](../reference/format-spec.md)**. Here we record only the rationale and trade-offs.

### 6.1 F-01 Page size is fixed at 4 KB

- **Current choice:** `PAGE_SIZE = 4096`. The header records the value but it is fixed.
- **Considered alternatives:**
  - (a) Switch to 8 KB / 16 KB.
  - (b) Make page size configurable at `open()`.
- **Why rejected:**
  - 4 KB matches SQLite and most RDBMS defaults and the OS page size on x86/x64; no surprise even if we later switch to mmap.
  - Configurable size would put a branch on every offset calculation.
- **Why chosen:** Proven and simple. `file_offset = start + page_index * 4096 + 8 + slot_index * SLOT_SIZE` fits on one line.
- **Trade-off:** None observed in practice.

### 6.2 F-02 NodeSlot does not embed the label string (fixed 64 B)

- **Current choice:** NodeSlot is a fixed 64 B. Label strings live as **blobs in the property extent**; the slot stores `(label_offset, label_length)`.
- **Considered alternatives:**
  - (a) Inline the label string in NodeSlot (short-string optimization).
  - (b) Intern labels into integer IDs (a label dictionary).
- **Why rejected:**
  - (a) requires picking and branching on an inline-size threshold; capping label length is anti-pattern in modern libraries.
  - (b) introduces a separate, heavy spec around dictionary persistence, GC, and synchronization.
- **Why chosen:** With the LRU cache the second-and-later read is essentially free; the API-level performance difference is small while the spec dramatically simplifies.
- **Trade-off:** First label access incurs an extra blob read.

### 6.3 F-03 Out- and in-edge lists are singly linked (EdgeSlot fixed 80 B)

- **Current choice:** Each node carries `first_out_edge` / `first_in_edge`; each edge carries `next_out_edge` / `next_in_edge`. Both directions are singly linked lists.
- **Considered alternatives:**
  - (a) Out-edges only as a singly linked list (full scan for `in_edges`).
  - (b) Doubly linked lists (also store `prev`).
  - (c) Variable-length adjacency arrays inline at the node.
- **Why rejected:**
  - (a) makes `in_edges` and `neighbors(direction="in")` O(|E|), unacceptably slow.
  - (b) requires growing the slot from 64 B to 80 B and adds pointers used only by deletion.
  - (c) introduces variable-length records, complicating page layout.
- **Why chosen:** Both `out_edges` and `in_edges` cost O(degree); slots fit in 80 B; sufficient for the target use cases.
- **Trade-off:** Edge deletion needs O(degree) linear scans on both sides. Not suited for workloads dominated by deletions on very high-degree nodes.

### 6.4 F-04 Properties use a custom binary format (no external serialization crate)

- **Current choice:** A simple "1-byte type tag + value" format implemented in `src/storage/prop_codec.rs`. Types: Null / Bool / Int64 / Float64 / String / List / Map.
- **Considered alternatives:**
  - (a) `serde` + `bincode`.
  - (b) `rmp-serde` (MessagePack).
  - (c) `ciborium` (CBOR).
- **Why rejected:**
  - Tying on-disk compatibility to an external crate's API or maintenance is unacceptable.
  - The set of types becomes hard to control (e.g. `serde`'s Unit Variant or Tuple) and the spec grows.
- **Why chosen:** SQLite, Git, and Redis all use their own formats. The spec fits in 20 lines and the parser in under 100. Zero external dependency means it stays readable in the long run.
- **Trade-off:** When exchanging data with other systems, the on-disk form itself cannot be used directly; conversion to JSON etc. is the user's (or `examples/`'s) responsibility.

### 6.5 F-06 IDs are u64 sequential, with 0 as the NULL sentinel

- **Current choice:** Node and edge IDs are u64. Numbering starts at 1. 0 is reserved as NULL (list terminator / unset).
- **Considered alternatives:**
  - (a) UUID (16 bytes, random).
  - (b) Snowflake-style (time + node ID + sequence).
  - (c) String IDs supplied by the user.
- **Why rejected:**
  - (a) doubles storage and reduces I/O locality.
  - (b) depends on the clock and adds nothing for a single-process single-file system.
  - (c) requires uniqueness handling and variable lengths, which break fixed-size slots.
- **Why chosen:** 8 bytes; O(1) offset arithmetic; an upper bound that is essentially infinite (1.8 × 10¹⁹). Using 0 as NULL turns adjacency-list termination into a single comparison.
- **Trade-off:** If you want domain meaning embedded in the ID (e.g. a timestamp), store it in a property instead.

### 6.6 F-07 Multigraph is unconstrained; `merge_edge` matches `(from, label, to, **props)` exactly

- **Current choice:** Multiple edges of the same label between the same two nodes are allowed. `add_edge` always creates a new edge. `merge_edge` returns an existing edge if one matches `(from, label, to)` **and** every property value, otherwise creates a new one.
- **Considered alternatives:**
  - (a) Enforce `(from, label, to)` uniqueness in the core.
  - (b) Match `merge_edge` on `(from, label, to)` only, ignoring properties.
  - (c) A Cypher-style `ON MATCH` / `ON CREATE` differential update API.
- **Why rejected:**
  - (a) needs a uniqueness check on every write — full scan or a dedicated index (conflicts with §4.4).
  - (b) cannot represent "the same kind of relation at two different points in time".
- **Why chosen:** Holding the same relation at multiple times or in multiple contexts is a legitimate use; per-property idempotency is available through `merge_edge`.
- **Trade-off:** The semantics of duplicate edges is the user's responsibility. If you need a differential update, do it in two steps (`merge_edge` → use the returned `id` with `update_edge`).

---

## 7. Implementation policy decisions

### 7.1 No external serialization crate

- **Current choice:** No `serde` / `rmp-serde` / `ciborium` / `bincode` for property or WAL serialization. Everything is implemented in `src/storage/prop_codec.rs`.
- **Considered alternatives:** Same as §6.4.
- **Why rejected:** Same as §6.4 (do not couple on-disk format to an external dependency).
- **Why chosen:** The spec is small; both Rust and Python sides stay robust.
- **Trade-off:** Connector authors in other languages have to reimplement the encoder/decoder.

### 7.2 No external LRU / CRC / thiserror crates

- **Current choice:** LRU equivalent in `src/storage/cache.rs`, CRC32 in `src/storage/crc32.rs`, error types in `src/error.rs`. The only dependency is `pyo3`.
- **Considered alternatives:**
  - (a) `lru` crate.
  - (b) `crc32fast` crate.
  - (c) `thiserror` crate.
- **Why rejected:**
  - Each implementation is around 50–200 lines. The benefits (speed, maintenance) of pulling in dependencies are outweighed by supply-chain risk, build time, and license-bookkeeping cost.
- **Why chosen:** Reading `Cargo.toml` is enough to understand the core's dependencies. Preserves the "Rust super-minimal dependency" identity of the product.
- **Trade-off:** Use cases that need ultra-fast (SIMD) CRC may underperform vs. an external crate. Not a problem in current workloads.

---

## 8. Distribution and runtime decisions

### 8.1 WASM / browser distribution is out of phase

- **Current choice:** `wasm-bindgen` / `wasm-pack` build targets are not supported.
- **Considered alternatives:**
  - (a) `FetchStorage` (a storage abstraction reading via HTTP Range requests) + read-only WASM.
  - (b) Full-feature WASM build.
- **Why rejected:**
  - The main targets are Python and embedding; browser distribution has lower priority.
  - Writable WASM needs a separate design that connects to IndexedDB or similar.
- **Why chosen:** First stabilise the `pip install liel` experience.
- **Trade-off:** Use cases that want to read `.liel` from Jupyter / Observable / a standalone web app are not covered. Possible future sub-milestone (start from option (a)).

### 8.2 No published C FFI or other-language bindings

- **Current choice:** The public API is Python only (PyO3). We do not ship `liel.h`.
- **Considered alternatives:**
  - (a) Expose a C FFI with `#[no_mangle] extern "C"`.
  - (b) Bundle Node.js / Go bindings.
- **Why rejected:**
  - ABI stability is a separate commitment (symbols, struct layouts, error codes).
  - The Phase 1 core API may still change in the short term.
- **Why chosen:** Once the Python API stabilises, layering a C FFI on top is the realistic path.
- **Trade-off:** Other-language users either go through PyO3 (embedded Python) or wait for the future C FFI.

---

## 9. MCP / AI integration design rationale {#mcp-knowledge-graph}

The canonical operating guidance for AI tools lives in the
[AI memory playbook](../guide/mcp/agent-memory.md). This section records only
the design reason behind that guidance: MCP tool calls multiply quickly, and
committing one fact per tool call would put the workload directly on §5.1
(single writer) and §5.2 (page-grained WAL with fsync).

Therefore MCP-facing integrations should prefer bulk graph writes, stable merge
keys, natural checkpoint commits, and a single owner process for writes. Do not
assume real-time, ultra-high-frequency direct writes against one `.liel` file;
workloads of that shape belong on a server database or dedicated log store.

---

## 10. Relationship to Phase 2 / 3 (summary) {#phase-2-3-relation}

The Phase 2 / 3 lists in the maintainer-facing implementation plan are a **backlog of options to consider**, not a chronological "must finish" checklist. Many items there (Cypher / DSL, property index, WASM, JSON in core) clash directly with the trade-offs above, or fit better in a separate layer.

We do not aim to "do all of Phase 2". Where it survives, it should be redefined into sub-milestones that do not break **single file, minimal dependencies** (e.g. read-only WASM only, property index only, JSON helper at the Python layer only).

Per-item status and the reinterpretation table live in [future-roadmap.ja.md](../internal/process/future-roadmap.ja.md) (single source; not duplicated here). The **local sharing** workstream (e.g. `liel diff` / `merge` conventions) is tracked in [phase2-roadmap.ja.md](../internal/process/phase2-roadmap.ja.md).

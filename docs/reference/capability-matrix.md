# Capability matrix (stakeholder view)

A single-page map of **what `liel` offers today**, grouped by area, **who** it is for, and **why** it matters. Granularity is **user-facing features / commands / tools**.

For API-level detail (methods, limits, non-goals), see the [feature list](features.md). For byte layout and recovery semantics, see the [format spec](format-spec.md) and [reliability](reliability.md). For `liel merge --format json` fields, see [CLI merge report](cli-merge-report.md). For diff/stats/trace/export and exit semantics together, see [CLI JSON inventory](cli-json-inventory.md). For **which document owns which JSON surface** (CLI, MCP, CI, viewer), see [Machine-readable surfaces](json-surfaces.md).

---

## Current capabilities

| Area | Feature / command / tool | Audience | Value |
|------|---------------------------|----------|--------|
| Core | `.liel` single-file storage | Developers; agent tools | Project memory in one file without a server. |
| Core | Property graph model | Developers; agent tools | Facts, tasks, files, sources, decisions with relationships. |
| Core | WAL + `commit()` | Developers; operators | Crash recovery to the last committed boundary. |
| Core | Single-writer guard | Developers; operators | Dangerous concurrent writers on the same file are rejected (`AlreadyOpenError`). |
| Core | `transaction()` | Developers | Batch changes with commit/rollback semantics. |
| Core | `vacuum()` | Operators | Safely compact the file after deletes/updates. |
| Core | `repair_adjacency()` | Operators | Rebuild adjacency lists from live edges when inconsistent. |
| Python API | `liel.open()` | Python developers | Read/write `.liel` from Python. |
| Python API | `add_node` / `add_edge` | Python developers | Minimal API to grow graph memory. |
| Python API | `get_node` / `all_nodes` / `all_edges` | Python developers | Enumerate and fetch stored memory. |
| Python API | `nodes()` / `edges()` QueryBuilder | Python developers | Filter by label and properties. |
| Python API | `out_edges` / `in_edges` / `neighbors` | Python developers | Traverse local relationships quickly. |
| Python API | `bfs` / `dfs` / `shortest_path` | Python developers; agents | Walk impact and dependency paths (shortest path: unweighted, directed, min hops). |
| Python API | `merge_from()` | Python developers | Integrate another `.liel` with ID remapping and merge policies. |
| Python API | `liel.coding_memory` (experimental) | Python developers; agents | Thin `File` / `Decision` / bug-shaped `Task` helpers on `GraphDB` ([Python guide](../guide/connectors/python.md#coding-memory-helpers)). |
| CLI | `liel version` | Users; CI | Check the installed version. |
| CLI | `liel help` | Users | Entry point and per-command help. |
| CLI | `liel diff` | Developers; reviewers; CI | Compare two `.liel` files. |
| CLI | `diff --node-key` | Teams; CI | Compare independent files by stable keys instead of local IDs. |
| CLI | `diff --identity-rules` | Teams; CI | Per-label identity rules for comparison. |
| CLI | Edge multiset diff (`--node-key` / `--identity-rules` paths) | Reviewers; CI | Parallel edges kept as a multiset where applicable (not collapsed to a single logical edge). |
| CLI | `liel merge` | Developers; teams | Merge two `.liel` files into a new output file. |
| CLI | `merge --dry-run` | Reviewers; CI | Preview merge without writing the output file. |
| CLI | `merge --dry-run --fail-on-conflict` | CI | Exit non-zero when the preview is blocked (`can_merge: false`); JSON unchanged. |
| CLI | Merge `conflicts` (JSON / text) | Reviewers; CI | Machine-readable reasons when merge cannot proceed (e.g. missing key, duplicate key). |
| CLI | Merge `warnings` (JSON / text) | Reviewers; CI | See what properties/labels are kept or dropped under merge rules. |
| CLI | `merge --identity-rules` | Teams; CI | Label-specific identity rules for safer merges. |
| CLI | `liel pack` | Developers; sharers | Extract a subgraph by labels into a new `.liel`. |
| CLI | `liel manifest` | Reviewers; CI | Emit a deterministic summary of `.liel` contents. |
| CLI | `liel sign` | Operators; CI | HMAC-sign a manifest with an external key file. |
| CLI | `liel verify` | Operators; CI | Verify a `.liel` against a signature and key file. |
| CLI | `liel stats` | Users; reviewers | Quick node/edge counts and label distribution. |
| CLI | `liel trace` | Users; reviewers; scripts | Shortest path between two node IDs (unweighted, directed); JSON aligns with MCP `liel_trace`. |
| CLI | `liel export` | Developers; tool builders | Export `.liel` to round-trippable JSON. |
| CLI | `liel import` | Developers; tool builders | Rebuild `.liel` from export JSON. |
| MCP | `liel_overview` | LLMs; agents | Overall shape of memory (counts, labels, samples). |
| MCP | `liel_find` | LLMs; agents | Find nodes by label and exact property match. |
| MCP | `liel_explore` | LLMs; agents | BFS neighbourhood with optional Mermaid snippet. |
| MCP | `liel_trace` | LLMs; agents | Shortest path between two node IDs. |
| MCP | `liel_map` | LLMs; humans | Render a subgraph as Mermaid. |
| MCP | `liel_diff` | LLMs; agents; reviewers | Compare two `.liel` files; JSON aligns with CLI `liel diff`. |
| MCP | `liel_merge_preview` | LLMs; agents; reviewers | Two-file merge dry-run; JSON aligns with CLI `liel merge --dry-run`. |
| MCP | `liel_manifest` | LLMs; agents; CI authors | Deterministic manifest JSON (same as CLI `liel manifest`). |
| MCP | `liel_append` | LLMs; agents | Append new nodes/edges in one atomic commit. |
| MCP | `liel_merge` | LLMs; agents | Reuse/update nodes and add edges idempotently in one commit. |
| Docs | Quickstart / README | First-time users | What the tool is for and how to start. |
| Docs | CLI guide | CLI users; CI authors | Command contracts and examples. |
| Docs | MCP guide | Agent integrators | How to run and use tools from LLM clients. |
| Docs | Conventions | Teams; tool builders | Shared habits for labels and provenance. |
| Docs | Reliability docs | Operators | Crash recovery, single-writer guard, commit boundaries. |
| Docs | Format spec | Maintainers; tool builders | On-disk constraints for `.liel`. |
| Docs | Machine-readable surfaces index | CI authors; MCP integrators; tool builders | Single map from automation concern to authoritative JSON docs ([json-surfaces](json-surfaces.md)). |
| Docs | Viewer JSON contract | Tool builders | Stable viewer/dashboard inputs derived from CLI outputs; **do not** parse raw `.liel` bytes in the browser as the primary path ([viewer-json](viewer-json.md)). |
| Docs | Vector hybrid conventions | Teams; integrators | Optional properties when pairing `liel` with external embeddings ([vector-conventions](vector-conventions.md)). |
| Docs | Schema profiles (optional) | Teams; validators | Optional per-label expectations outside core enforcement ([schema-profiles](schema-profiles.md)). |

**In one sentence:** `liel` is not only a lightweight embedded graph store; it is a **single-file graph memory toolkit** to **store, compare, verify, merge, and expose** local AI memory—including from agents via MCP.

---

## Planned / open gaps (examples)

Items below are **not a committed roadmap**; they capture typical **Phase 4+** product and distribution gaps called out in internal playbooks. Priorities change with feedback.

| Area | Feature / tool | Audience | Value |
|------|----------------|----------|--------|
| README | License / PyPI / Python / CI / Docs / Release badges | First-time visitors | Trust signals at a glance. |
| Docs | GitHub Pages (or similar) | Users; teams | Public URL for docs. |
| Viewer | Read-only graph viewer (product/UI) | Reviewers; teams | Browser-friendly “what do we remember?” — **contract docs exist** ([viewer-json](viewer-json.md)); a turnkey viewer app remains optional. |
| API | Higher-level “memory” API | App developers | Graph-free ergonomics beyond `coding_memory` (broader than today’s experimental helper). |
| API | Turnkey IDE / agent preset **package** | Coding-agent users | Pre-built app packaging on top of `liel` (not shipped in core). |

---

## Related documents

| Document | Role |
|----------|------|
| [Feature list](features.md) | Method-level API and core limits |
| [Machine-readable surfaces](json-surfaces.md) | Index of JSON contract owners |
| [Viewer JSON contract](viewer-json.md) | Approved inputs for visualization tools |
| [Command line](../guide/cli.md) | CLI usage |
| [MCP tools](../guide/mcp/tools.md) | MCP parameters and examples |
| Phase 4 marketing playbook | Distribution and messaging strategy (maintainer docs under `docs/internal/process/` in the repository; not shipped on this site) |

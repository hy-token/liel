# CLI JSON and exit codes (inventory)

**Canonical reference** for machine-readable CLI output and process exit codes across commands. **`liel merge` JSON fields** are defined only in [CLI merge report](cli-merge-report.md); this page covers everything else and the cross-command summary.

- Tutorials and flags: [Command line guide](../guide/cli.md).
- How CLI reference pages split responsibility: [Reference index](index.md) (*CLI documentation map*).
- Which automation surface maps to which doc (CI / MCP / viewer): [Machine-readable surfaces](json-surfaces.md).

## Summary (roles, JSON, exit codes)

| Role | Command | `--format json` | Typical success exit | Notes |
|------|---------|-----------------|------------------------|-------|
| Compare two `.liel` files | `liel diff` | Yes | `0` no differences, `1` differences | Not merge-style `can_merge`; see below. |
| Merge preview / write merged file | `liel merge` | Yes | `0` when report prints; `--dry-run --fail-on-conflict` → `1` if blocked | Field definitions: [CLI merge report](cli-merge-report.md). |
| File fingerprint / signing input | `liel manifest` | N/A (stdout **is** JSON) | `0` | Same structural content as signing input; `-o` optional. |
| Round-trip exchange | `liel export`, `liel import` | Import: `--format json` for summary | `0` | Export stdout is JSON. |
| Snapshot counts | `liel stats` | Yes | `0` | Label histograms. |
| Shortest path between nodes | `liel trace` | Yes | `0` | Path query; `path` may be `null`. Aligns with MCP `liel_trace`. |

Process-level failures (`CliError`): exit `2` usage, exit `1` unexpected error — consistent across commands unless noted.

`liel sign` / `liel verify` consume `manifest` bytes; see the command-line guide.

---

## `liel trace --format json`

| Field | Meaning |
|-------|---------|
| `source` | Resolved input `.liel` path. |
| `from_node` / `to_node` | Endpoint IDs from `--from` / `--to`. |
| `edge_label` | Label filter string (`""` when any label is allowed). |
| `path` | Ordered list of node objects (`id`, `labels`, properties), or `null` if no route exists. |
| `path_hop_labels` | Ordered list of edge labels, one per hop along `path` (empty when `path` is `null`). |
| `reasoning_branches` | Out-edges from `--from`: each entry has `edge_label` and `target` (node object), sorted by label then target id. |
| `mermaid` | Flowchart snippet for the path subgraph (empty string when `path` is `null`). |

Always exits `0` when the graph opens and the query completes (including when `path` is `null`).

Text mode (default) prints a **decision narrative** (see command-line guide) when the path includes a `Decision` node; `reasoning_branches` in JSON still supplies the raw out-edges for scripts. `--no-mermaid` omits the Mermaid block only. JSON always includes the fields above.

---

## `liel diff --format json`

Top-level shape:

| Field | Meaning |
|-------|---------|
| `changed` | `true` if any node or edge bucket is non-empty. |
| `left` / `right` | Each has `path`, `nodes` (count), `edges` (count). |
| `nodes` | `added`, `removed`, `changed` — list entries depend on mode (see `identity` inside `nodes` when key-aware). |
| `edges` | Same buckets; key-aware modes attach multiset / identity metadata under `edges.identity`. |

Exit code: `0` if `changed` is false, `1` if true. Identity-rule or `--node-key` violations that fail closed raise usage errors (`2`) before JSON is emitted.

---

## `liel stats --format json`

| Field | Meaning |
|-------|---------|
| `path` | Input `.liel` path. |
| `liel_format` | Format version string from `db.info()`. |
| `file_size` | Bytes on disk. |
| `node_count` / `edge_count` | Live record counts. |
| `node_labels` / `edge_labels` | Label → count maps (sorted keys). |

Always exits `0` on success.

---

## `liel manifest` (stdout or file)

JSON object includes `manifest_version`, `liel_format`, `node_count`, `edge_count`, sorted `nodes` and `edges` arrays with normalized properties (see implementation in `python/liel/cli/manifest.py`). Intended for deterministic review and signing — **not** identical to `export` (different version field and purpose).

---

## `liel export` / `liel import`

**Export** JSON includes `export_version`, `liel_format`, counts, and sorted `nodes` / `edges` (`python/liel/cli/exchange.py`).

**Import** with `--format json` returns:

| Field | Meaning |
|-------|---------|
| `source` / `output` | Paths. |
| `nodes_imported` / `edges_imported` | Counts. |
| `node_id_map` / `edge_id_map` | Source IDs → new IDs in the output file (JSON object keys are stringified integers). |

---

## Related

- [CLI merge report](cli-merge-report.md) — `liel merge --format json` only.
- [Command line](../guide/cli.md) — tutorials and flags.

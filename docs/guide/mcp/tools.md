# MCP tools reference

Parameter specs and usage examples for every tool the `liel` MCP server exposes.

Stability labels for the accepted `1.0` direction:

- Read / inspection tools (`liel_overview`, `liel_find`, `liel_explore`,
  `liel_trace`, `liel_map`, `liel_diff`, `liel_merge_preview`, `liel_manifest`)
  are stable candidates when their arguments map directly to the stable CLI or
  Python surfaces.
- Write tools (`liel_append`, `liel_merge`) remain experimental for `1.0` until
  project write policy, stable-key guidance, and dedupe rules are stronger.

For a **copyable** starting point, use the [project write policy template](agent-memory.md#project-write-policy-template-copy-and-edit) in the AI memory playbook. (Japanese text lives in `agent-memory.ja.md` in this repo; it is excluded from the English Pages build per `mkdocs.yml` exclude rules.)

All tools return a JSON string. On error they always return the same shape:

```json
{ "error": { "code": "slug", "message": "human-readable text" } }
```

### Stable read tools: backing CLI contracts

Stable-candidate read tools are thin wrappers around the same inputs the CLI
accepts today. When tightening behavior, prefer changing **experimental JSON
innards** or adding **compatible fields** before changing documented CLI flags or
stable JSON top-level keys referenced here.

| MCP tool | Primary backing command / JSON |
|----------|----------------------------------|
| `liel_overview` | `liel stats --format json` |
| `liel_find` / `liel_explore` | Graph reads + property filters (same data as CLI/Python record views) |
| `liel_trace` | `liel trace --format json` |
| `liel_map` | Graph structure → Mermaid (shares trace-style node metadata) |
| `liel_diff` | `liel diff --format json` |
| `liel_merge_preview` | `liel merge --dry-run --format json` |
| `liel_manifest` | `liel manifest` (stdout JSON) |

---

## Read tools

### liel_overview

Return the overall shape of the memory graph. This is the best first call after
connecting.

**Parameters:** none

**Returns:**

```json
{
  "node_count": 42,
  "edge_count": 87,
  "node_labels": { "Person": 15, "Module": 10 },
  "edge_labels": { "DEPENDS_ON": 40, "ABOUT": 12 },
  "sample_nodes": [
    { "id": 7, "labels": ["Module"], "name": "auth" }
  ],
  "db_path": "/path/to/project.liel",
  "liel_format": "1.0",
  "file_size": 49152
}
```

`liel_format` and `file_size` match the CLI `liel stats --format json` fields (`version`
and `file_size` from `db.info()`). There is no separate `liel_stats` MCP tool.

### liel_find

Find nodes by label and exact property match. Use it to narrow the search space
before exploring.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `label` | string | `""` | Node label filter. Empty means any label. |
| `where` | string (JSON) | `""` | JSON object of exact-match filters. |
| `limit` | integer | `20` | Max results per page (cap 100). |
| `cursor` | integer | `0` | Pagination offset. |

### liel_explore

Explore the neighbourhood of one node with BFS. Returns nodes, edges, and a
Mermaid diagram.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `node_id` | integer | required | Starting node ID. |
| `max_depth` | integer | `2` | BFS depth cap (max 4). |
| `edge_label` | string | `""` | Traverse only this edge label when set. |

### liel_trace

Trace the shortest path between two nodes.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `from_node` | integer | required | Start node ID. |
| `to_node` | integer | required | End node ID. |
| `edge_label` | string | `""` | Restrict traversal to one edge label. |

### liel_map

Render a chosen subgraph as Mermaid.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `node_ids` | string | `""` | Comma-separated IDs like `"1,3,5"`. |
| `limit` | integer | `30` | Auto-sampling cap when `node_ids` is omitted. |

### liel_diff

Compare **two** `.liel` files on disk. The JSON payload matches CLI
[`liel diff --format json`](../../reference/cli-json-inventory.md) /
[`liel diff`](../cli.md#diff) — same shape as the reference pages for diff.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `left` | string | required | Path to the left/base `.liel` file. |
| `right` | string | required | Path to the right/other `.liel` file. |
| `node_key_json` | string | `""` | Optional JSON array of property names for key-aware diff, e.g. `"[\"path\"]"`. Empty means ID-based diff. |
| `identity_rules_path` | string | `""` | Optional path to identity-rules JSON (same format as CLI `--identity-rules`). Mutually exclusive with `node_key_json`. |

### liel_merge_preview

Preview merging **two** `.liel` files (`merge_from` semantics) **without**
writing an output file. Same JSON as CLI
[`liel merge --dry-run --format json`](../../reference/cli-merge-report.md).

Not the same as **`liel_merge`** below (atomic merge into the already-open graph).

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `left` | string | required | Base file path (copied conceptually first). |
| `right` | string | required | Incoming file path merged into the preview. |
| `output_path` | string | `""` | Optional planned output path string for the report only. |
| `node_key_json` | string | `""` | Optional JSON array of property names (same as CLI `--node-key`). |
| `identity_rules_path` | string | `""` | Optional identity-rules JSON path (same as CLI `--identity-rules`). |
| `edge_strategy` | string | `"append"` | `append` or `idempotent`. |
| `on_node_conflict` | string | `"keep_dst"` | `keep_dst`, `overwrite_from_src`, or `merge_props`. |

### liel_manifest

Return the deterministic manifest **object** for one `.liel` file (sorted nodes
and edges, `manifest_version`, counts). Same content as CLI `liel manifest`
stdout JSON — suitable for signing workflows.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `source` | string | `""` | Path to a `.liel` file. If empty, uses the MCP server’s connected file (`--path`). |

---

## Write tools

**Status:** experimental for `1.0`. Use these tools only with an explicit project
memory policy and stable-key convention; otherwise prefer read tools plus a
human-reviewed CLI/Python write path.

The MCP write tools do not currently add standard metadata properties such as
creation timestamps, update timestamps, or session IDs. A reserved metadata
convention may be added in a future release, but the initial MCP behavior keeps
created graph records exactly to the user-supplied labels, properties, and
edges.

### liel_append

Append new nodes and edges in one atomic commit.

Use this when the AI knows it is recording **new** durable knowledge and does
not need reuse or dedupe behavior.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `nodes` | string (JSON) | `"[]"` | JSON array of new node objects. |
| `edges` | string (JSON) | `"[]"` | JSON array of edge objects. |
| `session` | string | `""` | Reserved for possible future metadata support; currently no properties are added automatically. |

Each node object supports:

```json
{
  "ref": "decision",
  "labels": ["Decision"],
  "props": { "title": "Keep MCP first" }
}
```

Each edge object supports:

```json
{
  "from": "decision",
  "to": 12,
  "label": "ABOUT",
  "props": { "weight": 1 }
}
```

`from` and `to` may reference:

- an existing node ID
- a `ref` created earlier in the same request

**Returns:**

```json
{
  "created_nodes": [
    { "id": 43, "labels": ["Decision"], "title": "Keep MCP first" }
  ],
  "created_edges": [
    { "id": 88, "label": "ABOUT", "from": 43, "to": 12 }
  ],
  "ref_map": { "decision": 43 }
}
```

### liel_merge

Merge nodes and edges in one atomic commit.

Use this when the AI wants to **reuse or update existing nodes** and create
edges idempotently.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `nodes` | string (JSON) | `"[]"` | JSON array of merge node objects. |
| `edges` | string (JSON) | `"[]"` | JSON array of edge objects; edges are added with idempotent semantics. |
| `session` | string | `""` | Reserved for possible future metadata support; currently no properties are added automatically. |

Each node object may use one of these patterns:

1. Update a known node directly:

```json
{
  "id": 7,
  "props": { "owner": "platform" }
}
```

2. Reuse one existing node by exact property match:

```json
{
  "ref": "auth",
  "labels": ["Module"],
  "match": { "path": "src/auth.rs" },
  "props": { "name": "auth", "owner": "platform" }
}
```

If `match` finds exactly one node, that node is reused and its properties are
updated by overlaying `props`. If no node matches, a new node is created from
`labels` and `props`. If multiple nodes match, the tool returns
`ambiguous_match`.

### Match semantics

`liel_merge` is intentionally client-neutral. It does not depend on
`CLAUDE.md`, prompt files, or any single AI tool. The merge contract is:

- `id` updates a known node directly.
- `match` performs exact property matching to find one existing node.
- zero matches means "create a new node".
- more than one match returns `ambiguous_match`.

This means `match` works as an upsert-like operation only when the caller uses
a property set that is stable enough to identify one node.

### Choosing stable keys for `match`

Prefer properties that are naturally unique or close to unique for a label:

- `Module` -> `path`
- `Document` -> `url`
- `Issue` -> `issue_id`
- `Person` -> `email` or `external_id`
- `Task` -> project-specific task ID

Avoid weak keys such as `name` alone unless the dataset really guarantees
uniqueness. If no stable key exists yet, call `liel_find` first and then merge
by `id`.

**Returns:**

```json
{
  "created_nodes": [
    { "id": 44, "labels": ["Decision"], "title": "Auth owned by platform" }
  ],
  "merged_nodes": [
    { "id": 7, "labels": ["Module"], "path": "src/auth.rs", "owner": "platform" }
  ],
  "merged_edges": [
    { "id": 89, "label": "ABOUT", "from": 44, "to": 7 }
  ],
  "ref_map": { "auth": 7 }
}
```

---

## Read-side guidance

Use the read tools in this order unless you already know the graph well:

1. `liel_overview` for the broad picture
2. `liel_find` to narrow candidates
3. `liel_explore` to inspect local structure
4. `liel_trace` when the question is about impact or dependency paths
5. `liel_map` when Mermaid will help explain the result

---

## Write-side guidance

- Use `liel_append` when you want guaranteed new records.
- Use `liel_merge` when you want to reuse existing nodes, update known nodes,
  or create edges without accidental duplication.
- When using `liel_merge` with `match`, prefer stable label-specific keys such
  as `path`, `url`, or `external_id`.
- Batch several related nodes and edges into one call instead of committing one
  tiny fact at a time.

---

## Error handling

This section documents the MCP tool-layer JSON error shape. For Python API
exceptions such as `GraphDBError`, `AlreadyOpenError`, and `MergeError`, see
the [Python guide](../connectors/python.md#exceptions).

Typical stable error codes include:

- `invalid_json`
- `invalid_nodes`
- `invalid_edges`
- `invalid_labels`
- `invalid_node_id`
- `node_not_found`
- `unknown_ref`
- `ambiguous_match`

The `code` field is the stable branch point; the `message` is explanatory text.

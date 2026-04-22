# MCP tools reference

Parameter specs and usage examples for every tool the `liel` MCP server exposes.

All tools return a JSON string. On error they always return the same shape:

```json
{ "error": { "code": "slug", "message": "human-readable text" } }
```

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
  "db_path": "/path/to/project.liel"
}
```

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

---

## Write tools

### liel_append

Append new nodes and edges in one atomic commit.

Use this when the AI knows it is recording **new** durable knowledge and does
not need reuse or dedupe behavior.

**Parameters:**

| Name | Type | Default | Description |
|---|---|---|---|
| `nodes` | string (JSON) | `"[]"` | JSON array of new node objects. |
| `edges` | string (JSON) | `"[]"` | JSON array of edge objects. |
| `session` | string | `""` | Optional `_session` applied to created nodes. |

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
| `session` | string | `""` | Optional `_session` applied only to newly created nodes. |

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

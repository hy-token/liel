# CLI merge report (`liel merge --format json`)

This page documents the **JSON object** printed by `liel merge` when `--format json` is set. The same merge runs produce the [text report](../guide/cli.md#merge) for human review; JSON is the stable, machine-readable contract for CI, MCP, and tools.

`liel` does not decide which memory is “semantically true”. It reports **mechanical** merge results, **structural blockers** (`can_merge: false`), and **review hints** (`warnings`) when policies keep or drop values on reused nodes.

For `1.0`, the stable automation contract is the documented top-level fields,
conflict bucket names, `can_merge`, `warnings`, and key-aware identity metadata.
Diagnostic detail fields may receive compatible additions; consumers should
ignore unknown fields and unknown future `type` values after displaying their
`message`.

---

## Top-level fields

| Field | Type | When present | Meaning |
|------|------|--------------|---------|
| `dry_run` | bool | always | `true` if `--dry-run` was used. |
| `can_merge` | bool | always | `false` if merge cannot proceed under the chosen identity rules (see `conflicts`). `true` if a successful merge (or dry-run simulation) was performed. **Does not mean “no review needed”** when `warnings` is non-empty. |
| `conflicts` | array | always | Empty when `can_merge` is `true`. When `can_merge` is `false`, each item explains a structural reason (missing key, duplicate identity, and so on). |
| `warnings` | array | always | Only key-aware modes (`--node-key` or `--identity-rules`) produce non-empty `warnings`. Review material: property/label differences under `--on-node-conflict`. Omitted or empty in append-only merge with no node key. |
| `output` | string or null | always | Output path string, or `null` for dry-run when `-o` was not given. |
| `nodes_created` | int | `can_merge: true` | Count of new nodes created from the incoming file. |
| `nodes_reused` | int | `can_merge: true` | Count of nodes matched to existing base nodes. |
| `edges_created` | int | `can_merge: true` | Count of new edges appended (or created when not idempotent-reused). |
| `edges_reused` | int | `can_merge: true` | Count of edges treated as idempotent-reused when `--edge-strategy idempotent` applies. |
| `node_id_map` | object | `can_merge: true` | Maps **source** node IDs to **destination** node IDs after merge. JSON object keys are **decimal strings** (JSON does not allow integer object keys). |
| `edge_id_map` | object | `can_merge: true` | Maps **source** edge IDs to **destination** edge IDs. Keys are stringified integers. |

When `can_merge` is `false`, numeric counters and ID maps are zeroed or empty; use `conflicts` for the reason.

---

## Process exit code vs `can_merge`

By default, on success `liel merge` exits **0** even when `can_merge` is `false` or when `warnings` is non-empty: the command produced a valid report. Use **`can_merge` and `warnings` inside the JSON** for automation—or pass **`--dry-run --fail-on-conflict`** so the process exits **`1`** when `can_merge` is `false` or `conflicts` is non-empty (JSON payload unchanged).

| Exit code | Meaning |
|---:|---|
| `0` | Report printed successfully (including blocked previews unless `--fail-on-conflict` applies). |
| `1` | Unexpected failure while merging or previewing (I/O, GraphDB error), **or** `--dry-run --fail-on-conflict` and the preview is blocked (`can_merge: false` / non-empty `conflicts`). |
| `2` | Usage error (`CliError`): missing inputs, invalid flags, refusing overwrite without `--force`, output path equal to an input path, **`--fail-on-conflict` without `--dry-run`**. |

---

## `conflicts[]` items

Each conflict is an object with at least `type` and `message`. Other fields depend on `type`.

| `type` | Additional fields | Meaning |
|--------|--------------------|---------|
| `missing_node_key` | `side`, `node_id`, `missing_keys` | `--node-key` mode: a node on `side` (`"source"` or `"destination"`) lacks a required key property. |
| `duplicate_node_key` | `side`, `identity`, `node_ids` | Two or more nodes on the same side share the same resolved key identity. |
| `ambiguous_destination_node_key` | `identity`, `node_ids` | The base file has more than one node matching a source identity. |
| `unmatched_identity_rule` | `side`, `node_id`, `labels` | `--identity-rules` mode: a source node matches no rule label. |
| `multiple_identity_rules` | `side`, `node_id`, `labels` | A node matches more than one rule label. |
| `missing_identity_rule_key` | `side`, `node_id`, `label`, `missing_keys` | A matched node is missing a property required by its rule. |
| `duplicate_identity_rule` | `side`, `identity`, `node_ids` | Two or more nodes on the same side share the same rule-derived identity. |

New conflict types may appear in minor releases; automation should tolerate unknown `type` values and rely on `message` for display.

---

## `warnings[]` items

Warnings never flip `can_merge` from `true` to `false`. They flag **review-relevant drift**: values or labels that differ between matched nodes and how the active `--on-node-conflict` policy resolves them.

| `type` | Fields | Meaning |
|--------|--------|---------|
| `node_property_conflict` | `identity`, `property`, `destination`, `source`, `policy`, `resolution`, `message` | Same identity on both sides, but the property value differs. `resolution` is one of `source_ignored`, `destination_overwritten`, etc., depending on policy. |
| `node_label_difference` | `identity`, `destination`, `source`, `policy`, `resolution`, `message` | Label sets differ; merge keeps destination labels (`resolution`: `destination_labels_kept`). |

---

## Related

- [CLI JSON inventory](cli-json-inventory.md) — cross-command JSON and exit codes.
- [Command line: Merge](../guide/cli.md#merge) — usage and examples.

## Scope

- Append-oriented merge (no `--node-key` / `--identity-rules`) produces JSON **without** key-aware `warnings` or `conflicts` from preflight; `can_merge` stays `true` unless the merge operation itself fails.
- This document describes the **CLI** JSON shape. Python `GraphDB.merge_from` returns a `MergeReport`; field names align with the CLI payload where applicable.

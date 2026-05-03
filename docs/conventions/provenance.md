# Provenance conventions

`liel` keeps graph structure and properties, but it does not enforce one global
provenance model. This page defines practical conventions for recording where a
fact came from, how it was transformed, and what confidence to assign to it.

The goal is reviewability for local sharing: another person or agent should be
able to trace major nodes and claims back to explicit sources.

## This page decides

- when provenance should be attached
- which minimum fields to keep
- which edges preserve traceability

## This page does not decide

- full vocabulary catalog (see `recommended-labels.md`)
- naming/normalization policy details (see `recommended-labels.md`)

## What provenance should answer

A shared `.liel` file should make these questions easy to answer:

- What source supports this node or claim?
- Which tool or actor created this derived artifact?
- When was it observed, imported, or transformed?
- Is this statement asserted, inferred, or uncertain?

These are conventions above the core engine. They do not change the file format
or require special Rust-side schema support.

## Vocabulary baseline

Use `recommended-labels.md` as the default vocabulary source.
For provenance workflows, the most common labels are:

- `Source`
- `ToolResult`
- `Claim`
- `Decision`
- `Session`
- `Actor`

If your domain needs additional labels, keep them additive and explicit.

## Recommended provenance edges

Prefer explicit edges over free-text phrases:

| Edge label | Meaning |
|---|---|
| `DERIVED_FROM` | Destination content was produced from the source node |
| `SUPPORTS` | Evidence supports a claim or decision |
| `CONTRADICTS` | Evidence conflicts with a claim or decision |
| `CREATED_BY` | Artifact was created by an actor or tool |
| `OBSERVED_IN` | Fact was observed in a specific session, source, or event |
| `DECIDED_IN` | Decision was made in a specific session or source |

Keep directions consistent. For example, use `result -DERIVED_FROM-> source`
across producers instead of mixing both directions.

## Recommended provenance properties

Attach compact, explicit properties so records stay machine-friendly:

| Property | Use for |
|---|---|
| `source_type` | Source category such as `url`, `file`, `issue`, `chat`, `api` |
| `retrieved_at` | RFC 3339 UTC time when source content was fetched |
| `observed_at` | RFC 3339 UTC time when a fact was observed |
| `imported_at` | RFC 3339 UTC time when data entered this `.liel` file |
| `confidence` | Numeric score (for example `0.0..1.0`) if confidence is modeled |
| `method` | How a node was created, such as `manual`, `parser`, `llm_extract` |
| `system` | External system name for IDs, such as `github`, `jira`, `notion` |
| `external_id` | Stable ID from an external system |

Use `path` and `url` from canonicalization conventions for stable identity, not
timestamps or generated internal IDs.

## Minimal provenance pattern

For many pipelines, this small pattern is enough:

1. Represent each evidence item as a `Source` node with stable `url` or `path`.
2. Represent extracted statements as `Claim` nodes.
3. Link `Claim -SUPPORTS-> Decision` or `Source -SUPPORTS-> Claim` as needed.
4. Link generated nodes with `DERIVED_FROM`.
5. Record `retrieved_at` or `observed_at` with RFC 3339 UTC strings.

This preserves traceability without forcing a heavy schema.

## Minimal required fields (operational baseline)

At minimum, keep one of these:

- a `Source` node with `url` or `path`
- a derived node linked by `DERIVED_FROM`

When possible, also keep:

- `retrieved_at` or `observed_at` (RFC 3339 UTC)
- `system` + `external_id` for external-system identity

## Minimal example

1. `Source(path="docs/design/product-tradeoffs.md")`
2. `Claim(text="pack is Tier 2 high priority")`
3. `Source -SUPPORTS-> Claim`
4. `Claim -OBSERVED_IN-> Session`

Even this small pattern is enough to trace evidence later.

## Pre-save checklist (5 items)

1. Evidence node has `url` or `path`  
2. Derived artifact has `DERIVED_FROM`  
3. Claim/decision has `SUPPORTS` or `CONTRADICTS`  
4. `retrieved_at` or `observed_at` exists  
5. External references include `system` + `external_id`  

## Non-goals

- Do not treat provenance conventions as access control or cryptographic trust.
- Do not require semantic deduplication in the core engine.
- Do not rewrite legacy files only to satisfy these conventions.

Use these rules where they improve sharing and review. Existing files remain
valid even if they are partially annotated.


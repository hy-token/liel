# Viewer JSON contract

This page is for **tool builders** (dashboards, static sites, IDE plugins) that
want to show what a `.liel` file contains **without** embedding the Rust core or
parsing the binary format in the browser.

## What not to do

- **Do not** treat the on-disk `.liel` layout as a stable public API for
  JavaScript or WASM readers. The [format spec](format-spec.md) is for
  implementers of the engine and compatibility tooling, not for casual browser
  parsing.

## Approved JSON inputs

Build viewers from **documented CLI outputs** (and the same shapes via MCP
where available):

| Source | Use for | Spec |
|--------|---------|------|
| `liel export` | Round-trip snapshot, rich graph for custom UI | [CLI JSON inventory](cli-json-inventory.md) (`export`); `export_version` |
| `liel stats --format json` | Counts, label histograms, format version, size | [CLI JSON inventory](cli-json-inventory.md) (`stats`) |
| `liel manifest` | Deterministic fingerprint, signing workflows | [CLI JSON inventory](cli-json-inventory.md) (`manifest`); differs from `export` |
| `liel diff --format json` | Two-file comparison views | [CLI JSON inventory](cli-json-inventory.md) (`diff`) |
| `liel merge --dry-run --format json` | Mergeability panels | [CLI merge report](cli-merge-report.md) |

In CI, run these commands and pass JSON forward; in agents, prefer MCP tools that
delegate to the same modules (`liel_diff`, `liel_merge_preview`, `liel_manifest`,
`liel_overview`, …) — see [MCP tools](../guide/mcp/tools.md).

## Suggested pipeline

```text
.liel  →  liel export (or stats / manifest)  →  your UI / static site / report
```

For large graphs, filter at the CLI (`pack`, query APIs in Python) before
visualizing.

## Read-only inspection (no editing)

Reasonable first steps:

1. **Chat / IDE surfaces** — Use MCP tools that already return JSON or Mermaid
   (`liel_map`, `liel_explore`, `liel_trace`) for quick human-readable graphs.
2. **Static HTML** — Generate a page from `liel export` JSON in CI and publish
   as an artifact (same trust model as docs builds).
3. **Future `liel serve` (read-only)** — A small local HTTP server could expose
   stable JSON only; editing and hosted multi-tenant dashboards remain out of
   scope for the core project until explicitly designed.

Lineage-heavy views (decisions, provenance, impact paths) compose from the same
exports plus conventions in [Recommended labels](../conventions/recommended-labels.md)
and [Provenance](../conventions/provenance.md).

## Related

- [Capability matrix](capability-matrix.md) — stakeholder map including MCP and CLI
- [Machine-readable surfaces index](json-surfaces.md) — links to all contract pages

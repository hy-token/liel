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

## Phase 4 E4 minimum profile (careful rollout)

To keep E4 low-risk, build the viewer in gated steps and freeze contracts first.
Implementation may use embedded JS visualization libraries; the contract
boundary stays JSON-only.

### Gate A — Contract freeze

The first viewer release may read only these documented JSON surfaces:

- `liel stats --format json` (overview cards and label counts)
- `liel export` (node/edge table and detail panes)
- `liel trace` output via MCP/CLI JSON (path preview entry points)

Do not add browser-side binary parsing of `.liel` as a shortcut.

### Gate B — Minimal read-only UI

A "minimum done" viewer should provide:

- file summary (counts, labels, format/version hints),
- node/edge list with basic filters,
- drill-down panel for one node/edge,
- optional trace jump (from selected IDs).

Library policy for E4:

- Allowed: embed external JS libraries for table/search/filter and graph rendering.
- Required: keep a thin adapter layer that maps contract JSON to library inputs.
- Not allowed: coupling viewer behavior to `.liel` binary internals or undocumented JSON fields.

### Gate C — Reproducibility and docs

Validate against fixed `examples/demo_memory` data and keep docs/README links to
the viewer entry point in sync with this page.

**Repro steps (Gate C baseline):**

```bash
python examples/demo_memory/make_demo_files.py --force
liel export target/demo-memory/base.liel -o target/demo-memory/base.export.json
```

Open `docs/guide/sample-viewer/app/index.html`, then load
`target/demo-memory/base.export.json`.

**Gate C acceptance checklist:**

- [ ] Viewer opens with bundled trace scenario sample by default.
- [ ] Loading the exported JSON updates summary, node table, edge table, and graph.
- [ ] `docs/guide/cli.md` links to `docs/guide/sample-viewer.md`.
- [ ] This contract page links to the same sample viewer README.

### Gate D — Operations checklist

When changing viewer behavior, run this lightweight checklist:

- [ ] **Contract drift check**: if CLI/MCP JSON fields changed, update
  `cli-json-inventory` / `json-surfaces` and this page together.
- [ ] **Adapter boundary check**: keep mapping logic in the viewer adapter layer;
  do not parse `.liel` bytes or rely on undocumented fields.
- [ ] **Sample smoke check**: open `docs/guide/sample-viewer/app/index.html`,
  load `target/demo-memory/base.export.json`, confirm summary/tables/graph update.
- [ ] **Docs link check**: keep links aligned across
  `docs/guide/cli.md`, this page, and `docs/guide/sample-viewer.md`.
- [ ] **Scale guard check**: preserve node/edge render caps or equivalent guardrails
  so large exports do not freeze the sample UI.

Lineage-heavy views (decisions, provenance, impact paths) compose from the same
exports plus conventions in [Recommended labels](../conventions/recommended-labels.md)
and [Provenance](../conventions/provenance.md).

## Related

- [Capability matrix](capability-matrix.md) — stakeholder map including MCP and CLI
- [Machine-readable surfaces index](json-surfaces.md) — links to all contract pages
- [Sample viewer implementation](../guide/sample-viewer.md) — quick local
  read-only reference UI based on the same JSON contracts

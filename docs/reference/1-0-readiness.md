# 1.0 readiness checklist

This page tracks what must be true before `liel` can be described as a stable
`1.0` substrate for project and agent memory. It is intentionally conservative:
`Ready` means the behavior is documented, tested, and suitable to treat as a
public contract; `Needs work` means useful but not yet frozen; `Deferred` means
explicitly outside the `1.0` promise.

## Status labels

| Label | Meaning |
|-------|---------|
| Ready | Public behavior is documented and has tests or runnable examples that protect it. |
| Needs work | The feature exists or is planned for 1.0, but docs, tests, examples, or compatibility language are incomplete. |
| Deferred | Useful idea, but not part of the 1.0 compatibility promise. |

## Checklist

| Area | Status | Canonical docs | Remaining work before 1.0 |
|------|--------|----------------|---------------------------|
| Python graph API | Ready | [Python connector](../guide/connectors/python.md), [Feature list](features.md) | Stable surface is enumerated in docs and protected by `tests/python/test_public_api_surface.py`, `test_pyi_runtime_consistency.py`, and runtime/stub alignment checks. Helper APIs remain experimental. |
| CLI JSON surfaces | Ready | [CLI JSON inventory](cli-json-inventory.md), [CLI merge report](cli-merge-report.md), [Machine-readable surfaces](json-surfaces.md) | Documented top-level fields, merge buckets, export/import version handling, and manifest signing constraints are covered by Python contract tests. CLI text remains best-effort. |
| MCP tools | Ready | [MCP overview](../guide/mcp/index.md), [Tools reference](../guide/mcp/tools.md), [AI memory playbook](../guide/mcp/agent-memory.md) | Read / inspection tools map back to documented CLI/Python surfaces; write tools remain explicitly experimental with a copyable project policy template and dedupe guidance. |
| CodingMemory helper | Ready | [Feature list](features.md), [Python connector](../guide/connectors/python.md), [example README](https://github.com/hy-token/liel/blob/main/examples/coding_memory/README.md) | The helper is clearly separated as an experimental convention layer rather than a frozen `1.0` core contract. |
| Inspection experience | Ready | [Inspect your memory](../guide/inspect.md), [Sample viewer](../guide/sample-viewer.md), [Viewer JSON contract](viewer-json.md), [Command line](../guide/cli.md) | Keep the fixed fixture, viewer, docs, and contract tests aligned when JSON surfaces change. |
| Reliability model | Ready | [Reliability and failure model](reliability.md), [Operations guide](../guide/operations.md) | Keep release notes explicit when file-format or recovery assumptions change. |
| Backup / verify / repair operations | Ready | [Operations guide](../guide/operations.md), [CI guide](../guide/ci.md) | Smoke the documented commands against a sample memory during release dry-runs. |
| CI examples | Ready | [CI guide](../guide/ci.md), [`examples/github-actions`](https://github.com/hy-token/liel/tree/main/examples/github-actions) | Keep the accepted smoke set aligned across CI guide, operations guide, release procedure, and release evidence template; merge preview stays policy-dependent. |
| On-disk format compatibility | Ready | [Format spec](format-spec.md), [Reliability](reliability.md) | The current reader fails closed on unsupported future versions, tests protect that behavior, and release notes must call out migration/export guidance for breaking format work. |
| Benchmark baseline | Ready | [Benchmarks](benchmarks.md) | Reproducible baseline now covers insert, traversal, export/import, diff, key-aware merge preview, and trace payload generation with recorded 2,000-node results. |
| Agent workflow sample | Ready | [Claude project-memory workflow](../guide/mcp/claude-workflow.md), [MCP AI memory playbook](../guide/mcp/agent-memory.md), [Sample viewer](../guide/sample-viewer.md) | Keep Claude as the first workflow sample; treat it as an example, not a stable agent-runtime contract. |
| Hosted dashboard / editing UI | Deferred | [Product trade-offs](../design/product-tradeoffs.md), [Viewer JSON contract](viewer-json.md) | Keep the viewer read-only; do not add editing or hosted-dashboard commitments for 1.0. |
| Core vector ANN / semantic search | Deferred | [Vector hybrid conventions](vector-conventions.md), [Product trade-offs](../design/product-tradeoffs.md) | Keep vector stores external unless a future design changes the scope explicitly. |
| Server-grade concurrent writes | Deferred | [Single-writer guard](../design/single-writer-guard.md), [Reliability](reliability.md) | Continue to document one writer per `.liel` file; route multiple producers through an owner process. |

## 1.0 label policy

Use a narrow stable surface for 1.0. Everything else should be explicit about
whether it is experimental or deferred.

| Surface | Label | Rationale |
|---------|-------|-----------|
| Python core graph API | Stable | Matches the current Python-first support contract. |
| Reliability model | Stable | Single-file, single-writer, commit-boundary behavior is already documented. |
| Backup / verify / repair operations | Stable | Operations guide and smoke path exist. |
| CLI JSON automation surfaces | Stable | CI, MCP, viewer, and release evidence depend on machine-readable output. |
| CLI text output | Experimental / best-effort | Human-facing wording may evolve. |
| Viewer export contract | Stable | Fixed fixture and contract tests protect the shape. |
| MCP read / inspection tools | Stable candidate | They expose overview, find, explore, trace, diff, merge preview, and manifest workflows. |
| MCP mutation tools | Experimental | Write quality depends on project policy, stable keys, and dedupe discipline. |
| CodingMemory helper | Experimental | Useful convention layer, but not a 1.0 core contract. |
| Claude workflow sample | Example / experimental | Demonstrates integration without becoming an agent-runtime contract. |
| Hosted dashboard, editing UI, core vector ANN, semantic auto-merge, server-grade concurrent writes, non-Python first-party bindings | Deferred | Explicitly outside the 1.0 promise. |

## API and automation surface map

| Surface | 1.0 decision needed | Current reference |
|---------|---------------------|-------------------|
| Python package entry points | Stable names, exception behavior, transaction semantics, and type stubs. | [Python connector](../guide/connectors/python.md) |
| CLI text output | Human-facing; may evolve more freely than JSON, but examples should remain accurate. | [Command line](../guide/cli.md) |
| CLI JSON output | Machine-facing contract; field additions should be compatible and removals should wait for a major boundary. | [CLI JSON inventory](cli-json-inventory.md), [CLI merge report](cli-merge-report.md) |
| MCP tools | Tool names, required arguments, and JSON relationships to CLI surfaces need stable/experimental labels. | [MCP tools reference](../guide/mcp/tools.md) |
| Viewer JSON | Contract for read-only inspection UIs; should be protected by fixtures or contract tests. | [Viewer JSON contract](viewer-json.md) |
| `.liel` file format | Unsupported future formats must fail closed; migrations need release-note visibility. | [Format spec](format-spec.md), [Reliability](reliability.md) |
| Examples and CI workflows | Copyable but not all are part of the core compatibility promise. | [CI guide](../guide/ci.md), [Example Python scripts](../samples/example-scripts.md) |

## Accepted 1.0 decision queue

These are the maintainer-approved 1.0 direction decisions as of 2026-05-12. The
accepted options are intentionally conservative: freeze only the surfaces that
already have users, tests, or release evidence, and keep convenience layers
experimental until they have stronger compatibility pressure.

| Decision | Accepted policy | Deferred alternative | Why / follow-up |
|----------|-----------------|----------------------|-----------------|
| Python graph API stability | Freeze the documented `liel.open`, node/edge/property operations, transaction semantics, exception behavior, and type stubs after one final stub/runtime drift audit. | Mark the whole Python API experimental and ship 1.0 as CLI-first. | Prefer freezing the Python core because it is the primary package surface; follow up with migration notes for any final rename or exception change. |
| `liel export` / `liel import` JSON versioning | Treat `export_version` as a required semantic contract: compatible field additions are allowed within `1.x`; removals, renamed fields, or incompatible meaning changes require a new export version and fail-closed import behavior. | Keep exports as best-effort snapshots and avoid a compatibility promise. | The viewer fixture and import/export workflows depend on this shape; add tests for old-version rejection and forward-compatible ignored fields. |
| Manifest signing payload | Freeze the normalized manifest JSON used for signing, with deterministic key ordering and platform-independent scalar normalization; display-only metadata may evolve separately. | Keep manifests informational and make signatures implementation-specific. | Sign/verify is only useful for release evidence if the signed payload is stable across supported OS/Python combinations; verify this in the OS smoke matrix. |
| `liel merge --format json` schema | Freeze top-level fields, conflict bucket names, `can_merge`, `warnings`, and key-aware identity metadata; mark only diagnostic detail fields as experimental if needed. | Freeze only `can_merge` and `conflicts`, leaving the rest undocumented. | CI needs predictable conflict semantics; add schema examples for blocked, warning-only, and clean previews. |
| MCP read / inspection tools | Label overview/find/explore/trace/diff/merge-preview/manifest tools as stable candidates once their argument names map directly to CLI or Python stable surfaces. | Keep all MCP tools experimental for 1.0. | Read-only tools carry lower risk and make agent workflows useful; document each stable tool's backing CLI/Python contract. |
| MCP mutation tools | Keep mutation tools experimental for 1.0. Require explicit project policy, stable keys, and dedupe guidance before any stable label. | Freeze the current mutation tools together with read tools. | Writes encode project memory policy, not just storage mechanics; avoid promising semantics before dedupe and review guidance are stronger. |
| `CodingMemory` helper conventions | Keep helper method names, labels, and relationship conventions experimental; document them as an opinionated pattern, not the core data model. | Freeze the helper as the primary user-facing API. | This preserves room to adjust labels and ergonomics while keeping the lower-level graph API stable. |
| CLI text output | Document as human-facing best-effort output; examples should stay accurate, but scripts must use `--format json`. | Promise stable text output for common commands. | Stabilizing text output would slow documentation improvements without helping automation. |
| On-disk file format through `1.x` | Promise that the 1.0 reader opens current 1.x-compatible files and fails closed on unsupported future versions; any breaking format change needs changelog visibility and a migration/export path. | Allow silent best-effort opening of future versions. | Fail-closed behavior is safer for project memory and release evidence; add explicit compatibility language to the format spec. |
| `repair_adjacency()` public status | Keep it as a public maintenance API, but freeze only the documented summary fields; reserve detailed diagnostics for compatible additions. | Move it back to internal-only and expose repair only through CLI. | Operators need a programmable repair path; tests should protect the report fields used in docs. |
| Release smoke gate | For 1.0, require `stats --format json`, `manifest`, `verify`, `export`, `import`, and docs build in release evidence; keep merge preview required only for projects that document merge policy. | Require every documented operation, including merge preview, in every release. | This keeps the mandatory gate realistic while still recording evidence for the core operational contract. |
| OS smoke matrix timing | Keep Ubuntu as CI gate now; record Windows/macOS/Linux smoke in release evidence; promote the full matrix to a hard gate only after one clean dry-run on all three OSes. | Make the full OS matrix a hard gate immediately. | The current docs already separate CI from maintainer dry-run; promotion should be based on a successful baseline rather than aspiration. |
| Scale policy | Do not start property indexes, streaming import/export, or format v2 until benchmark baselines identify a concrete bottleneck and target workload. | Begin scale work preemptively before measuring. | Measurements should decide scope; record workload size, command, elapsed time, memory, and file size before choosing an implementation. |

## Not in 1.0 scope

The following are intentionally not required to ship `1.0`:

- Hosted dashboards or browser-side editing of `.liel` files.
- A Cypher-compatible query language or general graph-analytics engine.
- Built-in vector ANN indexes or embedding model management.
- Automatic semantic merge resolution.
- Server-grade concurrent write support.
- First-party bindings for languages beyond the documented Python package.

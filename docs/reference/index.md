# Behavior and specifications

This section describes **how liel behaves today**: feature coverage, reliability, and the byte-level file format. For the rationale behind the scope, see [Design](../design/index.md).

## CLI documentation map

Use **one** authoritative page per concern; do not duplicate JSON field lists across guides.

| Need | Authoritative document |
|------|------------------------|
| Commands, flags, examples | [Command line guide](../guide/cli.md) |
| JSON shapes and exit codes (diff, stats, trace, manifest, export, import, and cross-command notes) | [CLI JSON inventory](cli-json-inventory.md) |
| `liel merge --format json` fields and merge-specific exit semantics | [CLI merge report](cli-merge-report.md) |
| Which document owns which JSON surface (CI, MCP, viewer) | [Machine-readable surfaces](json-surfaces.md) |
| Viewers and dashboards (no raw `.liel` parsing in the browser) | [Viewer JSON contract](viewer-json.md) |
| External vector stores alongside `liel` | [Vector hybrid conventions](vector-conventions.md) |
| Optional per-label validation outside the core | [Schema profiles (optional)](schema-profiles.md) |

## Document scope

| Document | Concern | Read when you need to |
|---|---|---|
| [Capability matrix](capability-matrix.md) | Features, CLI, MCP, docs by audience and value | Stakeholder overview in one table |
| [Feature list](features.md) | Public API and feature coverage | Check what `liel` provides |
| [Reliability and failure model](reliability.md) | Commits, recovery, failure modes, operational assumptions | Use `liel` as durable state |
| [Benchmarks and file size notes](benchmarks.md) | Local benchmark script and practical `.liel` size estimates | Interpret benchmark output or estimate memory-file size |
| [Format spec](format-spec.md) | Byte-level `.liel` file layout | Build compatibility tooling or connectors |
| [CLI merge report](cli-merge-report.md) | `liel merge --format json` payload | CI, MCP, or scripts consuming merge previews |
| [CLI JSON inventory](cli-json-inventory.md) | diff / merge / stats / manifest / export / import JSON and exit codes | Automation across CLI commands |
| [Machine-readable surfaces](json-surfaces.md) | Index of contract owners for CI, MCP, viewers | Pick the right spec page |
| [Viewer JSON contract](viewer-json.md) | Stable inputs for tools that visualize memory | Build dashboards without embedding Rust |
| [Vector hybrid conventions](vector-conventions.md) | Properties that reference external embeddings | Hybrid retrieval setups |
| [Schema profiles (optional)](schema-profiles.md) | Optional label expectations for validators | Team linting, not core enforcement |

This section’s [format spec](format-spec.md) is the public English byte-layout reference. If a parallel maintainer copy exists in the repository for the same concern, keep both aligned in scope when editing.

---

| Document | Content |
|---|---|
| [Capability matrix](capability-matrix.md) | Stakeholder-oriented map (core, Python, CLI, MCP, docs) |
| [Feature list](features.md) | Quick reference of provided functionality |
| [Reliability and failure model](reliability.md) | What committed data means, how crash recovery works, and which failure modes are out of scope |
| [Benchmarks and file size notes](benchmarks.md) | How to read the benchmark script and practical `.liel` size estimates |
| [Format spec](format-spec.md) | Canonical `.liel` byte layout |
| [CLI merge report](cli-merge-report.md) | Stable JSON fields for `liel merge` |
| [CLI JSON inventory](cli-json-inventory.md) | Cross-command JSON and exit semantics |
| [Machine-readable surfaces](json-surfaces.md) | Index of which doc owns which JSON contract |
| [Viewer JSON contract](viewer-json.md) | Approved inputs for visualization tools |
| [Vector hybrid conventions](vector-conventions.md) | Embeddings in external stores |
| [Schema profiles (optional)](schema-profiles.md) | Optional per-label checks |

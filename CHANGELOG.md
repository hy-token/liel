# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- **1.0.0** — Complete multi-OS release evidence, tag `v1.0.0`, switch PyPI `Development Status` to **Production/Stable**, and publish after maintainer sign-off.

## [0.7.0] - 2026-05-14

### Added

- **Release evidence** — Maintainer log for the `v0.7.0` tag is tracked at `docs/internal/process/release-evidence/2026-05-14-v0.7.0.md`, with closeout checks from 2026-05-14 appended.
- **Scale baseline note** — `docs/internal/process/scale-baseline-v0.7.0.ja.md` ties representative workloads to `scripts/bench/bench_python_api.py`, records the current “measure before format/index work” decision, and includes export/import, diff, merge preview, and trace timing.
- **MCP write policy** — Copyable project write policy template in [AI memory playbook](docs/guide/mcp/agent-memory.md#project-write-policy-template-copy-and-edit) (Japanese mirror in `docs/guide/mcp/agent-memory.ja.md`).
- **Release procedure** — On-disk file format breaking-change checklist in `docs/internal/process/release-procedure.ja.md` (section 2.11).
- **1.0 contract tests** — `tests/python/test_manifest_signing_contracts.py` guards manifest/sign `MANIFEST_VERSION` alignment and fail-closed manifest builds when stored floats are not finite JSON (`NaN` / `inf`).
- **Python stable surface** — `test_public_api_surface.py` now covers `GraphDB.repair_adjacency()` and protects the documented repair summary counters.
- **Viewer fixture contract tests** — `test_viewer_fixture_contract.py` keeps the docs/example fixtures and embedded fallback sample aligned.

### Changed

- **Docs** — [Python connector](docs/guide/connectors/python.md) documents the narrow `1.0` Python stable surface; [CLI JSON inventory](docs/reference/cli-json-inventory.md) and [MCP tools](docs/guide/mcp/tools.md) clarify manifest/sign versioning risks and MCP→CLI backing contracts.
- **1.0 readiness** — [1.0 readiness](docs/reference/1-0-readiness.md) now marks the locally closable API, CLI JSON, MCP, CodingMemory, format compatibility, and benchmark work as ready for the final release-evidence pass.
- **CLI JSON contract tests** — Integration tests assert documented top-level fields for `stats` / `trace` / `diff` / `merge` JSON output align with [CLI JSON inventory](docs/reference/cli-json-inventory.md) and [CLI merge report](docs/reference/cli-merge-report.md).
- **Benchmark script** — `scripts/bench/bench_python_api.py` now measures public automation paths (`export`, `import`, `diff`, merge preview, and trace payload generation) in addition to Python API insert/traversal rows.
- **CodingMemory example** — `examples/coding_memory/README.md` now describes the helper as an experimental convention layer rather than a frozen `1.0` contract.

### Fixed

- **MkDocs config** — `mkdocs.yml` nav label for operations mixed multiple `:` tokens in a single YAML scalar; renamed the nav title so `mkdocs build --strict` parses again.
- **pytest** — `pythonpath = ["."]` in `pyproject.toml` so `tests/python/test_bench_python_api.py` can import `scripts.bench` when the repo root is not already on `sys.path` (observed on Windows).
- **Sample viewer fallback** — The read-only viewer preserves the embedded fallback status when browser `file://` fetch restrictions block the checked-in fixture.

### Versioning

- **0.7.0** — Python package and Rust crate **`0.7.0`**. Pre-1.0 release focused on **stability-risk mitigation** (manifest/sign contracts, merge JSON shape, documented Python core + `repair_adjacency`, viewer fixture contract, benchmark baseline, operations/release process docs). **`Development Status` remains `4 - Beta`** until the separate **1.0.0** publish.

## [0.6.4] - 2026-05-05

### Changed

- **Launch docs** — README hero media now ships only the merge and trace GIFs that are included in the public release sync.
- **Public documentation** — Remove public links to maintainer-only `docs/internal` pages and align the MCP overview/playbook with the current 10-tool surface.
- **Viewer JSON contract** — Mark the Phase 4 E4 sample/reference viewer as complete and replace the rollout checklist with maintenance guidance.
- **Release versioning** — `Cargo.toml` is now the package-version source of truth; `pyproject.toml` uses dynamic version metadata, release tooling reads Cargo metadata, and CI examples use a minor-version lower bound.
- **Launch prep docs (internal)** — Add and refine community-posting drafts for Hacker News, Hashnode, and Reddit to support the public launch workflow.

### Versioning

- **0.6.4** — Python package and Rust crate **`0.6.4`**.

## [0.6.3] - 2026-05-05

### Fixed

- **MkDocs / GitHub Pages** — Remove `theme.custom_dir: overrides` so CI no longer fails when **`overrides/`** is missing on a synced checkout (`custom_dir` path must exist). **Google Search Console** `google-site-verification` meta is injected after build by **[`mkdocs_hooks.py`](mkdocs_hooks.py)** (`on_post_build`).

### Changed

- **Docs tooling** — **`requirements-dev.txt`**: **`mkdocs>=1.6`** (hooks). **[`scripts/release/sync_manifest.txt`](scripts/release/sync_manifest.txt)** includes **`mkdocs_hooks.py`** for copying into the public repo. **[`.github/workflows/docs-pages.yml`](.github/workflows/docs-pages.yml)** path filter includes **`mkdocs_hooks.py`**.

### Versioning

- **0.6.3** — Python package and Rust crate **`0.6.3`**.

## [0.6.2] - 2026-05-05

### Added

- **Local coding-agent workflow** — [`docs/internal/process/local-agent-liel-workflow.ja.md`](docs/internal/process/local-agent-liel-workflow.ja.md) describes keeping [`codex-session-memory.liel`](codex-session-memory.liel) in the loop locally (Cursor rule template, MCP, importer). Template [`docs/internal/templates/cursor-rule-liel-memory.mdc`](docs/internal/templates/cursor-rule-liel-memory.mdc) and [`scripts/memory/copy_cursor_liel_rule.ps1`](scripts/memory/copy_cursor_liel_rule.ps1) / [`copy_cursor_liel_rule.sh`](scripts/memory/copy_cursor_liel_rule.sh) install `.cursor/rules/liel-memory.mdc` (gitignored).
- **MkDocs / Search Console** — Homepage **`google-site-verification`** meta for indexing (Material **`overrides/main.html`** + **`theme.custom_dir`** in this release; later revised — see **[Unreleased]**).

### Changed

- **README badges** — [Release](https://shields.io/badges/git-hub-tag) badge uses **`github/v/tag`** (latest git tag) instead of **`github/v/release`**, with link to tags; [README.ja.md](README.ja.md) aligned. PyPI badge continues to reflect the published package version.
- **Project memory importer** — [`scripts/memory/import_project_memory.py`](scripts/memory/import_project_memory.py) adds specification **`spec:docs:readme-and-pages-trust-signals`** and related decisions for README / Pages trust signals.
- **Agent docs** — [`AGENTS.md`](AGENTS.md) and [`CLAUDE.md`](CLAUDE.md) require consulting **`codex-session-memory.liel`** first (MCP or `liel stats` / `liel export` fallback); internal [process index](docs/internal/process/index.ja.md) links the local workflow doc.

### Versioning

- **0.6.2** — Python package and Rust crate **`0.6.2`**.

## [0.6.1] - 2026-05-05

### Changed

- **GitHub Pages** — `Deploy docs to GitHub Pages` runs on **`main` push only** (not `v*` tags), and only when **`docs/**`**, **`mkdocs.yml`**, or **`.github/workflows/docs-pages.yml`** change; **`workflow_dispatch`** still redeploys without path filters. Avoids duplicate deploys and unnecessary runs on code-only commits.
- **Maintainer docs** — [Release procedure](docs/internal/process/release-procedure.ja.md) notes **`github-pages` environment** rules (e.g. deploy from **`main`**, not a tag ref, when branch/tag policies block tag deploys).

### Versioning

- **0.6.1** — Python package and Rust crate **`0.6.1`**.

## [0.6.0] - 2026-05-05

### Added

- **MkDocs sample viewer** — Read-only reference UI under `docs/guide/sample-viewer/` (`app/` static assets) for `liel export` JSON; MkDocs nav entry and links from the [CLI guide](docs/guide/cli.md) and [Viewer JSON contract](docs/reference/viewer-json.md).
- **GitHub Pages** — Workflow **Deploy docs to GitHub Pages** (`.github/workflows/docs-pages.yml`) builds with MkDocs and publishes on push to `main` or `v*` on **`hy-token/liel`** only (enable **Settings → Pages → GitHub Actions** once).

### Changed

- **MkDocs `site_url`** — Set to `https://hy-token.github.io/liel/` for canonical URLs and GitHub Pages; `mkdocs serve` previews under `http://127.0.0.1:8000/liel/` (same path prefix as production). Avoid using `https://github.com/owner/repo` as `site_url` (MkDocs would offset under `/owner/repo/`).

### Chore

- **Git ignore** — `assets/demo/parallel-merge.windows-powershell.ascii` (optional VHS transcript; GIF remains the shipped demo asset).

### Versioning

- **0.6.0** — Python package and Rust crate **`0.6.0`**.

## [0.5.2] - 2026-05-05

### Fixed

- **PyPI project description** - README hero GIFs use absolute `raw.githubusercontent.com` URLs (`hy-token/liel/main/assets/demo/…`) so images render on PyPI; relative paths failed. [`README.md`](README.md) and [`README.ja.md`](README.ja.md).

### Versioning

- **0.5.2** - Python package and Rust crate **`0.5.2`**.

## [0.5.1] - 2026-05-04

### Changed

- **Public release sync** - `scripts/release/sync_manifest.txt` lists only the two README hero GIFs (`assets/demo/parallel-merge.wsl.gif`, `assets/demo/demo-trace.wsl.gif`) for copy into `hy-token/liel`; other `assets/demo/*` and `.ascii` transcripts stay liel-dev-only. Documented in [`scripts/release/README.md`](scripts/release/README.md) and [release procedure](docs/internal/process/release-procedure.ja.md).

### Versioning

- **0.5.1** - Python package and Rust crate **`0.5.1`**.

## [0.5.0] - 2026-05-03

### Added

- **Trace demo graph + narrative output** — `trace-why-postgres.liel` from `make_demo_files.py` includes `trace_prompt` and decision copy for SNS-style demos. `liel trace` **text** mode emphasizes *Decision found* / *Why* / *Key factors* / *Rejected* / *Implemented in* and a short *Path* (no full filesystem path, no raw branch table); JSON still has `reasoning_branches` and `path_hop_labels`. VHS `demo-trace.*` runs a **single** `--no-mermaid` command.

### Phase 4: Wave A (2026-05-03) — review contract and CLI machine-readable base

Completed in [Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md) (items **9** and **11**, stages 1–3): memory-review language; `liel merge --dry-run` text and JSON; and `liel diff` / `liel stats` / `liel manifest` / `liel export` fixed as Git-oriented primitives in the [command-line guide](docs/guide/cli.md) and references. **Contract sources:** [CLI JSON inventory](docs/reference/cli-json-inventory.md), [CLI merge report](docs/reference/cli-merge-report.md). **Scope note:** default merge exit code behavior when `can_merge` is `false` was documented for JSON consumers; see Wave C for optional CI exit mapping.

### Phase 4: Wave B (2026-05-03) — positioning in public docs and `liel trace`

Completed in the same roadmap (backlog item **19** and positioning pass): [Why liel](docs/why-liel.md) and [Design index](docs/design/index.md) (review / diff / merge angle); [design principles](docs/design/principles.md) and [guide index](docs/guide/index.md); **CLI `liel trace`** (`python/liel/cli/trace.py`, JSON aligned with MCP `liel_trace`); [CLI guide](docs/guide/cli.md) / [CLI JSON inventory](docs/reference/cli-json-inventory.md) / [capability matrix](docs/reference/capability-matrix.md) / [documentation taxonomy](docs/internal/process/documentation-taxonomy.ja.md) updates; [marketing playbook](docs/internal/process/roadmap-phase4-marketing-playbook.ja.md) and [GIF task plan](docs/internal/process/phase4-gif-task-plan.ja.md) aligned. README-wide rewrite and outbound hero GIFs remain scheduled for later waves.

### Phase 4: Wave D (2026-05-03) — CodingMemory, Memory API, distribution

Completed in [Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md) (**Wave D 完了**): backlog **4–6**, **14**, **18** — [CodingMemory](docs/internal/design/coding-memory.ja.md), [Focused Memory API](docs/internal/design/memory-api.ja.md), [`python/liel/coding_memory.py`](python/liel/coding_memory.py) (tests + [`examples/coding_memory/`](examples/coding_memory/run_demo.py)), [Python guide § Coding memory helpers](docs/guide/connectors/python.md#coding-memory-helpers), README **Three quick demos** (fixed `demo_memory` + merge / diff / stats), [`demos/README.md`](demos/README.md) scripted trio table, [Show HN draft (JA)](docs/internal/process/show-hn-draft.ja.md), [LangGraph boundary + pseudocode](docs/internal/design/langgraph-liel.ja.md). **Not in core:** official LangGraph adapter package; **Phase 4 program** may still have open items (e.g. turnkey read-only viewer app, embedded README GIF refresh, Pages ops) per roadmap “Phase 4 完了条件”.

### Phase 4: Wave E (in progress) — wrap-up task index and VHS bundle

[Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md) names **Wave E** as the Phase 4 **wrap-up** bucket (viewer product UI, README hero GIFs, Pages, live Show HN, optional portability tape). **VHS:** distribution bundle tapes under `demos/` (merge, diff, stats as text+JSON, trace, export/import, CI `merge --dry-run --fail-on-conflict`), plus [`demos/render_gifs.py`](demos/render_gifs.py) to run **`python demos/render_gifs.py --profile bash|powershell`** and regenerate them in one go. `make_demo_files.py --extras` feeds the CI tape. See [`demos/README.md`](demos/README.md) / [`demos/README.ja.md`](demos/README.ja.md) and [GIF task plan](docs/internal/process/phase4-gif-task-plan.ja.md).

### Phase 4: Wave C (2026-05-03) — CI, MCP, viewer contracts, and conventions

Completed in [Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md) (Wave **C** backlog rows): **`liel merge --dry-run --fail-on-conflict`** (item **10**); MCP **`liel_overview`** enrichment and **`liel_diff` / `liel_merge_preview` / `liel_manifest`** (item **13**); GitHub Actions samples **`liel-memory-check.yml`** (stats) and **`liel-memory-manifest.yml`** (manifest) under `examples/github-actions/` (item **12**); reference pages **[Machine-readable surfaces](docs/reference/json-surfaces.md)**, **[Viewer JSON contract](docs/reference/viewer-json.md)**, **[Vector hybrid conventions](docs/reference/vector-conventions.md)**, and **[Schema profiles (optional)](docs/reference/schema-profiles.md)** with Japanese mirrors and MkDocs nav (items **7, 8, 11, 15, 16, 17**). User guides: [CLI merge report](docs/reference/cli-merge-report.md), [CLI guide](docs/guide/cli.md), [CI guide](docs/guide/ci.md), [MCP tools](docs/guide/mcp/tools.md), [reference index](docs/reference/index.md), [documentation taxonomy](docs/internal/process/documentation-taxonomy.ja.md) §7, [capability matrix](docs/reference/capability-matrix.md), and cross-links from [CLI JSON inventory](docs/reference/cli-json-inventory.md).

### Added

- **`liel trace --no-mermaid`** - Text output can omit the Mermaid diagram (path summary only); VHS `demo-trace` tapes use it for readability. Documented in [CLI guide](docs/guide/cli.md) / [CLI JSON inventory](docs/reference/cli-json-inventory.md); test in `tests/python/test_cli.py`.
- **VHS demo bundle** - `demos/render_gifs.py` runs VHS on six distribution tapes (`--profile bash` or `powershell`): parallel-merge, diff, stats (text then JSON), trace, export/import, and merge `--dry-run --fail-on-conflict`; `make_demo_files.py --extras` supplies the CI pair. Outputs under `assets/demo/*.gif`. Documented in [`demos/README.md`](demos/README.md) / [`demos/README.ja.md`](demos/README.ja.md), [CLI guide](docs/guide/cli.md), and [post-Phase 2 roadmap](docs/internal/process/post-phase2-roadmap.ja.md).
- **CLI `liel trace`** - Shortest-path query from the command line (`python/liel/cli/trace.py`), wired in `liel` CLI help; JSON aligns with MCP `liel_trace`. Documented in [CLI guide](docs/guide/cli.md), [CLI JSON inventory](docs/reference/cli-json-inventory.md), and [capability matrix](docs/reference/capability-matrix.md). Covered by `tests/python/test_cli.py`.
- **Phase 4 GIF task plan** - Maintainer checklist and P0–P3 ordering for VHS
  GIFs linked to Wave milestones (`docs/internal/process/phase4-gif-task-plan.ja.md`),
  indexed from the Phase 4 roadmap, marketing playbook, and `demos/README.md`.
- **CLI JSON inventory** - Cross-command overview of JSON payloads and exit
  codes (`docs/reference/cli-json-inventory.md` and Japanese mirror), linked
  from the reference index and capability matrix.
- **CLI merge report reference** - Documented the stable `liel merge
  --format json` payload (`docs/reference/cli-merge-report.md` and Japanese
  `cli-merge-report.ja.md`), linked from the CLI guide, capability matrix,
  reference index, and MkDocs nav.
- **Workspace agent instructions** - Added root-level `AGENTS.md` so automated
  assistants (including OpenAI Codex, which loads this file from the project
  root) receive consistent guidance to use `codex-session-memory.liel` as the
  default long-term graph memory for work in this repository.
- **MCP playbook** - Updated the Codex-oriented section in
  `docs/guide/mcp/agent-memory.md` and `docs/guide/mcp/agent-memory.ja.md` to
  reference that canonical `.liel` path alongside the existing durable-memory
  workflow.

### Changed

- **English public docs** - User-facing English pages no longer name Japanese-only source files or link to the internal taxonomy by path; use the [reference index](docs/reference/index.md) CLI map and the repository `docs/internal/process/` tree on GitHub for maintainer details.
- **Documentation SSoT** - Clarified CLI doc ownership in [documentation taxonomy](docs/internal/process/documentation-taxonomy.ja.md) §7; deduplicated the
  summary tables in [CLI JSON inventory](docs/reference/cli-json-inventory.md);
  added a **CLI documentation map** to [reference index](docs/reference/index.md) / `index.ja.md`;
  [Phase 4 Git-manageable design](docs/internal/design/phase4-git-manageable-agent-memory.ja.md) now points field-level details to the inventory.
- **Phase 4 roadmap** - Consolidated maintainer planning in [Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md) (Wave A–D completion sections; item **19** `liel trace` in Wave B; Phase 4 program may still list viewer/GIF/Pages wrap-up). Former Phase 4 Operating Plan merged into this file (redirect-only stub remains).
- **Phase 4 maintainer docs** - Merged [Phase 4 Current Operating Plan](docs/internal/process/phase4-current-operating-plan.ja.md)
  into [Roadmap Phase 4](docs/internal/process/roadmap-phase4-automation-ecosystem.ja.md)
  as a single canonical document; added update history; old Operating Plan file
  is redirect-only. Updated cross-links and documentation taxonomy.
- **`liel merge` text output** - Reworked the plain-text merge report for
  clearer headings, counts, conflicts, and structured warning lines; refreshed
  `docs/guide/cli.md` and CLI tests accordingly.
- **`liel export` (`-o` / `--output`)** - After writing the output file, prints a
  short text summary to stdout (source path, output path, node and edge counts).
- **`liel stats` (text mode)** - File sizes use human-readable units (KiB/MiB/…);
  JSON output still reports `file_size` in bytes.
- **README** - Hero layout: headline and subhead before the merge GIF; **Why decisions disappear** section with trace GIF; etymology moved below; PyPI version badge; CI badge uses public [`hy-token/liel`](https://github.com/hy-token/liel) Actions (default branch, no `branch=` query). [`README.ja.md`](README.ja.md) mirrors badges and layout.

### Versioning

- **Stable 0.5.0** - Promoted the Python package from `0.5.0a1` to **`0.5.0`**;
  the Rust crate remains **`0.5.0`** (aligned for this release).

## [0.5.0a1] - 2026-05-02

### Added

- **Phase 4 roadmap split** - Split the post-Phase 2 roadmap into
  phase-specific maintainer docs covering completed Phase 2/3 decisions and
  Phase 4+ planning.
- **Phase 4 productization plan** - Documented the Phase 4 focus on
  Git-compatible agent working memory, CodingMemory, visual inspection, CI/CD
  contracts, MCP/agent integration, vector conventions, and schema profiles.

### Changed

- **Versioning** - Started the Phase 4 alpha line with Python package version
  `0.5.0a1`; the Rust crate moves to the base `0.5.0` version.

## [0.4.0] - 2026-05-02

### Added

- **Phase 3 collaboration semantics** - Added key-aware `liel diff` support
  with repeatable `--node-key`, label-specific `--identity-rules`, and edge
  multiset comparison so independently created `.liel` files can be compared
  without relying on local IDs.
- **Merge previews and rule-aware merge** - Added `liel merge --dry-run`
  previews, `can_merge` / `conflicts` JSON reporting, and label-specific
  `--identity-rules` merge support.
- **Merge warning reports** - Added key-aware merge `warnings` for reused nodes
  whose properties or labels differ under the selected node conflict policy.
- **Phase 3 design notes** - Documented the initial strict identity,
  conflict-report, and merge-preview scope for maintainers.

### Changed

- **Versioning** - Advanced the Rust crate and Python package to version
  `0.4.0`.

## [0.3.0] - 2026-04-30

### Added

- **Phase 2 stable CLI set** - Promoted the local sharing, verification, and
  exchange command set to the stable `0.3.0` line: `diff`, `merge`, `pack`,
  `manifest`, `sign`, `verify`, `stats`, `export`, and `import`.
- **Local sharing workflow** - Completed the Phase 2 workflow for comparing
  `.liel` files, aggregating them safely, extracting shared subsets, verifying
  deterministic manifests, and exchanging graph data through JSON.
- **Maintainer roadmap docs** - Added internal Japanese documentation that
  records Phase 2 completion and separates Phase 3+ work into collaboration,
  automation, scale, and runtime themes.
- **CLI quality gates** - Hardened the Phase 2 CLI test suite with coverage for
  merge options, output overwrite refusal/`--force`, invalid signatures, empty
  key files, and malformed export JSON.

### Changed

- **Versioning** - Removed the Python package alpha suffix and aligned
  `pyproject.toml` with the Rust crate at version `0.3.0`.
- **Phase boundary** - Marked `diff --node-key`, key-aware merge, `filter`,
  MCP expansion, property indexes, WASM, and multi-writer runtime work as
  post-Phase 2 topics.
- **CLI docs clarity** - Split `sign` and `verify` option tables so `--force`
  is documented only for signature output, not verification.

## [0.3.0a18] - 2026-04-30

### Added

- **`liel export` / `liel import`** - Added deterministic JSON export and JSON
  import commands for reconstruction, editing, fixtures, and external tool
  workflows.
- **Export/import tests** - Added coverage for deterministic export JSON,
  unordered import input, ID remapping reports, and missing endpoint rejection.

### Changed

- **Manifest/export boundary** - Documented that `manifest` is the stable
  verification contract while `export` is the reconstruction contract with its
  own `export_version`.
- **Phase 2.2 priority** - Updated the roadmap to prioritize `stats` and
  `export/import`, while leaving `diff --node-key`, `filter`, and `redact` for
  later review.
- **Versioning** - Advanced the Phase 2.2 pre-release line to Python package
  version `0.3.0a18`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a17] - 2026-04-30

### Added

- **`liel stats`** - Added a CLI command that summarizes `.liel` files with
  format, file size, node/edge counts, node label counts, and edge label counts.
- **Stats docs and tests** - Documented `liel stats` and added text, JSON, and
  deterministic label-order tests.

### Changed

- **Versioning** - Advanced the Phase 2.2 pre-release line to Python package
  version `0.3.0a17`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a16] - 2026-04-30

### Added

- **`liel sign` / `liel verify`** - Added external signature commands that sign
  and verify deterministic manifest bytes without modifying `.liel` files.
- **HMAC signature format** - Added a deterministic JSON `.sig` format using
  standard-library `hmac-sha256` as the first shared-secret integrity check.
- **Signature tests** - Added CLI coverage for signing, verification success,
  changed-graph rejection, and JSON verification reports.

### Changed

- **Wave 3 planning** - Documented `hmac-sha256` as the initial no-extra-deps
  signature mode, with public-key algorithms left to future signature versions.
- **CLI docs** - Documented `liel sign` and `liel verify` usage, exit behavior,
  and the shared-secret nature of the initial signature mode.
- **Versioning** - Advanced the Phase 2.1 pre-release line to Python package
  version `0.3.0a16`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a15] - 2026-04-30

### Added

- **`liel manifest`** - Added a CLI command that emits deterministic JSON for a
  `.liel` file, excluding local file names, absolute paths, and generation time
  so the output can be committed to Git and later used as a signing target.
- **Manifest determinism tests** - Added byte-for-byte manifest tests covering
  expected JSON output, repeated generation, file-name independence, and output
  file writing.

### Changed

- **Wave 3 planning** - Documented the separation between `manifest`, future
  `sign`, and future `verify`, keeping signatures outside `.liel` files and
  using exported manifest bytes as the planned signing target.
- **CLI docs** - Documented `liel manifest` usage and the initial deterministic
  JSON rules.
- **Versioning** - Advanced the Phase 2.1 pre-release line to Python package
  version `0.3.0a15`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a14] - 2026-04-30

### Added

- **Canonical verification baseline** - Documented the Wave 2 starting scope
  for canonical-form verification as tests/docs-first, explicitly avoiding
  byte-level file-format canonicalization for now.
- **Pack determinism tests** - Added pytest coverage that locks down
  `liel pack` label normalization and stable `node_id_map` ordering as the
  first concrete canonical-form checks.

### Changed

- **Versioning** - Advanced the Phase 2.1 pre-release line to Python package
  version `0.3.0a14`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a13] - 2026-04-30

### Added

- **Provenance conventions** - Added `docs/conventions/provenance.md` and
  `docs/conventions/provenance.ja.md` with a minimal traceability workflow for
  source links, derivation edges, and observation metadata.
- **Japanese conventions set** - Added Japanese companion pages for conventions:
  `index.ja.md`, `recommended-labels.ja.md`, and `canonicalization.ja.md`.

### Changed

- **Conventions information architecture** - Simplified the conventions docs to
  a lightweight flow (`recommended-labels` then `provenance`) and moved
  canonicalization guidance into the recommended-labels page, keeping
  canonicalization as a compatibility pointer.
- **MCP writing guidance** - Updated `CLAUDE.md` to recommend provenance-first
  writes when using `liel[mcp]` (stable source keys, derivation/support edges,
  and RFC 3339 UTC timestamps).
- **Versioning** - Advanced the Phase 2.1 pre-release line to Python package
  version `0.3.0a13`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a12] - 2026-04-29

### Added

- **`liel pack`** - Added a CLI command that extracts nodes matching explicit
  labels into a new `.liel` file and copies only edges whose endpoints are both
  included.
- **Pack docs and tests** - Documented `liel pack` usage in the CLI guide and
  added focused CLI coverage for help, JSON reporting, in-place output
  rejection, and real `.liel` extraction.

### Changed

- **Versioning** - Advanced the Phase 2.1 pre-release line to Python package
  version `0.3.0a12`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a11] - 2026-04-29

### Fixed

- **Windows Rust CI** - Add Python's `libs` directory to MSVC's `LIB` path
  before `cargo test`, fixing Windows linker failures that could not find
  `python3.lib`.

### Changed

- **Python lint CI** - Include the CLI smoke helper script in the ruff check
  and format gates.
- **Python matrix cache** - Stop restoring the shared Cargo `target` directory
  in Python CI jobs so Windows/PyO3 builds do not reuse artifacts from another
  Python version or job.
- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a11`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a10] - 2026-04-29

### Added

- **CLI smoke source data** - Moved the CLI smoke `.liel` fixture data into
  tracked CSV files under `examples/cli_smoke_data/`, while keeping generated
  `.liel` files ignored under `target/cli-smoke/`.

### Changed

- **CLI smoke generator** - Updated `examples/09_cli_smoke_files.py` to build
  fixed smoke files from CSV source data instead of hard-coded in-script graph
  definitions.
- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a10`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a9] - 2026-04-29

### Added

- **CLI smoke files** - Added `examples/09_cli_smoke_files.py` to generate
  ignored `.liel` files for manual `liel diff` and `liel merge` smoke tests.
- **CLI docs** - Documented how to generate the smoke files from a source
  checkout.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a9`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a8] - 2026-04-29

### Added

- **`liel help`** - Added an explicit help subcommand for top-level help and
  command-specific help such as `liel help diff` and `liel help merge`.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a8`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a7] - 2026-04-29

### Added

- **CLI identity helpers** - Centralized current ID-based record matching for
  CLI diff and node-key normalization for CLI merge in a shared identity module.
- **ID diff specification** - Documented the current ID-based diff rule in the
  CLI guide as the single public place for how record IDs are paired and
  reported.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a7`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a6] - 2026-04-29

### Added

- **Phase 2 growth strategy** - Documented the staged strategy for independent
  `.liel` file merge and diff: append / ID-based first, then explicit
  key-aware behavior, then manifest-aware rules, with fuzzy matching limited to
  review candidates.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a6`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a5] - 2026-04-29

### Added

- **CLI guide** - Added public documentation for the `liel` console script,
  including `version`, `diff`, `merge`, exit codes, merge options, and links to
  the sharing conventions.

### Changed

- **Docs navigation** - Added the CLI guide to the `Use liel` navigation and
  documentation indexes.
- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a5`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a4] - 2026-04-29

### Added

- **`liel merge`** - Added a Python CLI command that copies a base `.liel`
  file to a new output and merges another `.liel` file into it through the
  existing `GraphDB.merge_from` API.
- **Merge options** - Added `--node-key`, `--edge-strategy`,
  `--on-node-conflict`, `--force`, and text/JSON report output for the merge
  command.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a4`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a3] - 2026-04-29

### Added

- **`liel diff`** - Added a read-only Python CLI command that compares two
  `.liel` files by mechanical node and edge records, with text and JSON output.

### Changed

- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a3`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a2] - 2026-04-29

### Added

- **Sharing conventions** - Added public conventions docs for
  canonicalization habits and recommended labels, keeping Phase 2 sharing
  guidance above the schemaless core.

### Changed

- **Docs navigation** - Added a Conventions section to the MkDocs navigation
  and main documentation index.
- **Versioning** - Advanced the Phase 2.0 pre-release line to Python package
  version `0.3.0a2`; the Rust crate remains at the base `0.3.0` version.

## [0.3.0a1] - 2026-04-29

### Added

- **General CLI shell** - Added the `liel` console script and
  `python -m liel.cli` entry point with a minimal `liel version` command.
- **CLI I/O policy foundation** - Added shared helpers for text/JSON output,
  user-facing CLI errors, exit codes, and output overwrite protection so Phase
  2 commands can share one behavior surface.

### Changed

- **Versioning** - Started the Phase 2.0 pre-release line with Python package
  version `0.3.0a1` while keeping the Rust crate at the base `0.3.0` version.

## [0.2.13] - 2026-04-29

### Fixed

- **README links** - Converted README document links to GitHub absolute URLs so
  links from the PyPI project page do not resolve under `pypi.org`.

## [0.2.12] - 2026-04-29

### Changed

- **LLM setup docs** - Documented Claude Code setup through project-level
  `.mcp.json`, simplified other LLM/MCP client guidance to the same
  `command` / `args` shape, and moved the recommended memory pattern into the
  README entry path.
- **Benchmark output** - Show item counts for each benchmark row and include the
  generated `.liel` file size.
- **Reference docs** - Added benchmark interpretation and practical `.liel` file
  size notes.

### Fixed

- **MCP startup** - When `--path` is omitted, the server now checks only the
  startup directory: it uses `./memory.liel` when no direct `*.liel` file exists,
  uses the single direct candidate when exactly one exists, and reports direct
  candidates when multiple files are present instead of silently picking one.

## [0.2.11] - 2026-04-29

### Changed

- **LLM memory docs** (`0b0feb5`) - Promoted the recommended LLM memory pattern
  into the README and MkDocs navigation; split Claude guidance into a
  Claude-specific setup page and a copyable `samples/CLAUDE.md`; clarified the
  role of the AI memory playbook as the general operating pattern.

## [0.2.10] - 2026-04-29

### Changed

- **Public English docs** — Neutral wording for maintainer-only repository links
  (`architecture`, `product-tradeoffs`, `index`, `reference/index`, `format-spec`
  intro); `docs/index` “internal” row points at the `docs/internal/` tree instead
  of a single README. Aligns with `mkdocs build --strict` (no `internal/**` in the
  built site) without locale-specific “site” framing in prose.
- **Phase 2 roadmap (JA)** — Open question on conventions rephrased to avoid
  language/region–specific options.

## [0.2.9] - 2026-04-28

### Added

- **Phase 2 maintainer roadmap** (`cd2c19b`) — `docs/internal/process/phase2-roadmap.ja.md` for local sharing and aggregation: proposed `liel` CLI shell, conventions, `liel diff` / `liel merge`, later `pack` / provenance / signing, and explicit non-goals.

### Changed

- **README / Quickstart docs** (`0bf952d`) — Shortened the English and
  Japanese READMEs into a compact entry point: tagline, install/demo command,
  one agent-memory code example, Mem0/Letta/Zep comparison, Zen, status, and
  links into docs. Moved the longer demo, Python, and MCP setup paths into
  `docs/guide/quickstart.md` and `docs/guide/quickstart.ja.md`, and added the
  quickstart page to the MkDocs navigation.
- **Architecture docs** (`0bf952d`) — Moved the Mermaid layer diagram out of the
  README and into `docs/design/architecture.md` /
  `docs/design/architecture.ja.md`, keeping README short while preserving the
  system-at-a-glance view.
- **README local-first introduction** (`6108aba`) — Reframed the English and
  Japanese README opening around liel as a portable external brain for local AI
  agents, and added a short local-first section covering code locality, LLM
  flexibility, offline use, and single-file portability.
- **Documentation ownership / cross-link cleanup** — Clarified reader paths and
  primary sources of truth in `docs/index.md` / `docs/index.ja.md`: byte layout
  belongs to the format spec, Python exceptions to the Python guide,
  commit/fsync/recovery semantics to reliability, AI-tool operating rules to
  the AI memory playbook, and product decisions to product trade-offs.
- **Reference/design docs** — Reduced repeated `commit()` / fsync explanation
  in feature and architecture pages by linking to the reliability contract, and
  linked Beta/breaking-change guidance to the changelog.
- **Product trade-offs** — Marked the mutex poison policy as maintainer-facing
  and compressed the MCP/AI integration section so it records rationale while
  delegating operating rules to the AI memory playbook.
- **Python and MCP docs** — Expanded the Python exception hierarchy with
  `AlreadyOpenError`, `MergeError`, and `CapacityExceededError`; documented why
  `get_node()` / `get_edge()` return `None` while mutation methods raise typed
  not-found errors; and separated MCP JSON error codes from Python exceptions.
- **Documentation SSOT** (`6ce2a18`) — Section 7 (document roles index) in
  `documentation-taxonomy.ja.md`; “document role” blurbs on architecture and
  product trade-offs (EN/JA); Phase 2/3 backlog text deduplicated with pointers
  to `future-roadmap.ja.md`, `phase2-roadmap.ja.md`, and `product-tradeoffs`
  §10; `CLAUDE.md` and `docs/index` paths updated; `internal/process/index.ja.md`
  table clarified.

### Fixed

- **Internal doc links** — `docs/internal/rust-modules.ja.md` related-document
  links are repo-relative (no local absolute paths). Stray
  `../../documentation-taxonomy.ja.md` targets corrected to
  `docs/internal/process/documentation-taxonomy.ja.md`.

## [0.2.8] - 2026-04-27

### Changed

- **README** — Positioning as a *structured memory layer* for people using local AI
  while coding; Architecture (Mermaid) and entry points; Etymology, scope, and
  Quickstart copy with an updated `liel-demo` output example.
- **Bundled demo** (`liel-demo` / `python -m liel.demo`) — Suggestions and
  exploration order are **graph-derived**: dual `SUGGESTS` traversals
  (preference and place) with **overlap-first** topic ordering, explicit
  **Graph Traversal** and **Graph Inputs** sections, and readable `why` lines.
  Example uses Silicon Valley + Palo Alto; optional Ollama still only rephrases
  the exploration list.
- **Python connector guide** — `update_node` example uses `city="Austin"` for
  locale-neutral sample data (`python.md` / `python.ja.md`).

### Added

- `examples/08_demo.py` — Module docstring explaining checkout vs installed wheel.
- `tests/python/test_demo.py` — Assertions for traversal blocks, graph-derived
  suggestion line, and exploration provenance.

### Fixed

- **Bundled demo** — Console output is ASCII-only so `python -m liel.demo` does
  not fail with `UnicodeEncodeError` on Windows (e.g. cp932). README Quickstart
  example text matches the printed output.

## [0.2.7] - 2026-04-26

### Changed

- Restore the publish workflow to GitHub's release-tag dropdown flow instead
  of requiring a manually typed `release_tag` input.
- Clarify the Beta contract across release planning, reliability, format, and
  public documentation so the `Development Status :: 4 - Beta` classifier
  matches the documented support surface.
- Align MCP tool lists and single-writer documentation references with the
  implemented API and design notes.

## [0.2.6] - 2026-04-26

### Fixed

- Restore all-wheel downloads for publish workflow metadata validation while
  keeping install-smoke checks scoped to each runner's own wheel artifact.

## [0.2.5] - 2026-04-26

### Fixed

- Download only the current runner's wheel artifact during publish workflow
  install-smoke checks, preventing pip from trying to install incompatible
  wheels built for other operating systems or architectures.

## [0.2.4] - 2026-04-26

### Changed

- Publish workflow now runs from `main` with an explicit `release_tag` input
  and checks out that tag for builds, avoiding GitHub UI tag-selector issues
  while preserving tag/version validation.

## [0.2.3] - 2026-04-26

### Changed

- Replace long local CI command blocks in the release procedure with
  environment-specific runner scripts, avoiding terminal paste corruption and
  ensuring package smoke tests start from a clean `dist/`.
- Treat invalid Unix owner PIDs as stale in writer-lock recovery, fixing the
  Linux/macOS `stale_owner_is_reclaimed` test when the fake PID exceeds
  `pid_t`'s signed range.
- Ignore Dependabot lower-bound bumps for maintainer-only Python tooling
  because local CI already installs the latest versions allowed by
  `requirements-dev.txt`.

## [0.2.2] - 2026-04-26

### Fixed

- Ignore Dependabot PyO3 version bumps until the Python bindings are migrated
  deliberately, preventing automated PRs from reintroducing the PyO3 0.28
  release-build failure fixed in `0.2.1`.

## [0.2.1] - 2026-04-26

### Fixed

- Pin PyO3 to `0.24.2` so release builds do not resolve to newer PyO3 APIs
  before the bindings have been migrated. This fixes CI failures where
  `PyObject` was no longer available through the prelude and `#[pymethods]`
  return conversion became ambiguous during wheel builds.

## [0.2.0] - 2026-04-26

This batch implements the design improvements collected on the
`prioritize-report-improvements` branch and documented under
[product-tradeoffs §5.5–§5.7](docs/design/product-tradeoffs.md) and
[graphdb-implementation 0.3 計画](docs/internal/process/graphdb-implementation.ja.md).

### Added

- **Crash-safe `vacuum()`** for file-backed databases (product-tradeoffs §5.6).
  Vacuum now writes a sibling `<basename>.liel.tmp`, fsyncs it, and atomically
  renames it over the live file using POSIX `rename(2)` plus a parent-directory
  fsync on Unix and `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` on Windows.
  A crash before the rename leaves the original file intact; the next
  `liel.open()` unconditionally removes any leftover `.tmp`.  `:memory:`
  databases continue to use the original in-place algorithm because there is
  no on-disk state to crash-corrupt.
- **`GraphDB::transaction()` RAII guard** in the Rust core (product-tradeoffs
  §5.5).  Returns a `TransactionGuard` that auto-rolls-back on drop and
  implements `Deref`/`DerefMut` so every `GraphDB` method is callable directly
  through the guard for the duration of the transaction.
- **No-nesting rule** for explicit transactions, surfaced in both Rust
  (`LielError::TransactionError("transaction already active")`) and Python
  (`liel.TransactionError`).  `with db.transaction(): with db.transaction(): …`
  now fails fast at the inner `__enter__`.
- **`vacuum()` is rejected inside an explicit transaction** with
  `TransactionError`.  Vacuum's internal forced commit would otherwise silently
  flush the surrounding transaction's staged work — the opposite of what an
  explicit transaction is for.
- **`QueryBuilder.exists()` short-circuits** on the first surviving record in
  both the Rust `QueryBuilder` / `EdgeQueryBuilder` and the Python
  `NodeQuery` / `EdgeQuery` wrappers.  Any caller-supplied `limit` is
  overridden to 1.
- **`liel._BUILT_WITH_FAULT_INJECTION`** module attribute on the native
  extension lets the new `tests/python/test_vacuum_crash_safety.py` harness
  skip itself when the wheel was built without the `test-fault-injection`
  Cargo feature.
- **`test-fault-injection` Cargo feature** exposing the four named injection
  points (`BEFORE_TMP_OPEN`, `AFTER_TMP_WRITES`, `AFTER_TMP_FSYNC`,
  `AFTER_RENAME`) inside `vacuum`, gated entirely behind the feature so
  release builds carry zero injection plumbing.
- **`src/storage/atomic_rename.rs`** with cross-platform `atomic_replace`,
  `src/graph/fault_inject.rs` with the gated `crash_at` helper, and
  stale-tmp cleanup folded into `GraphDB::open` so a previous vacuum's
  abandoned `.tmp` is unconditionally removed on the next open.
- New WAL fault-injection tests for undersized `entry_length` and wrong
  `data_length`, plus 11 unit tests for the parser helpers introduced when
  `parse_wal_entries` was split into `read_entry_length` / `verify_entry_crc`
  / `decode_entry_body`.

### Changed

- Release sync now derives the public repository commit message from the
  matching `CHANGELOG.md` version section and can create the local `vX.Y.Z`
  tag alongside the sync commit, while leaving all pushes as explicit manual
  steps.
- `format-spec.{ja,md}` now ships **§7 Adjacency-list and vacuum invariants**
  documenting the head-insert order contract and the
  ID-stability invariant across vacuum.
- The Python `.pyi` stub documents the reverse-insertion order contract on
  `out_edges` / `in_edges` / `neighbors` / `bfs` / `dfs`, the no-nesting rule
  on `Transaction`, and the short-circuit semantics of `NodeQuery.exists`
  / `EdgeQuery.exists`.
- WAL layout constants (`PAGE_HEADER_SIZE`, `MIN_ENTRY_LEN`,
  `OP_WRITE_MIN_LEN`, individual field-size constants) replace the magic
  numbers that previously appeared inline.
- `prop_to_py`'s bool conversion is centralised in `bool_to_pyobject` so a
  future pyo3 upgrade has one place to absorb a `Borrowed` ↔ `Bound` shift.

### Fixed

- Adjacency-list traversal order is now contractually documented as
  reverse-insertion (rather than implicit on the head-insert layout),
  matching what callers have been observing all along.

### Documentation

- `product-tradeoffs.{ja,md}` adds §5.5 (transaction nesting forbidden),
  §5.6 (vacuum crash-safety + the deliberate decision not to ship a
  free-space pre-check; see the ZEN-aligned reasoning), and §5.7
  (Mutex poison policy is scope-dependent).
- `graphdb-implementation.ja.md` records a "0.3 計画（順序固定）" section
  with both C1 (vacuum CoW) and B2 (transaction RAII) ticked as ✅ 実装済み.
- `features.{ja,md}` drops the previous "vacuum is not crash-safe" warning
  now that the rewrite has landed.

### Compatibility / verification

- The Unix vacuum path is end-to-end verified via fork + `_exit` crash
  injection (see `tests/python/test_vacuum_crash_safety.py`); the Windows
  `MoveFileExW` path is shipped as written but has **not been verified on a
  real NTFS filesystem** in CI yet — `os.fork` is unavailable on Windows so
  the crash-safety harness skips itself there.  Anyone running vacuum on
  Windows for the first time should plan to manually mirror the Linux harness
  to confirm the contract.
- Cargo dependency surface is unchanged: production deps remain
  `pyo3` only, dev deps remain `tempfile` only.  All new platform code
  (atomic rename, fault-injection exit, stale-tmp cleanup) uses bare
  `extern "C"` / `extern "system"` declarations the same way
  `src/storage/lock.rs` already did.

## [0.1.1] - 2026-04-24

First post-`0.1.0` update. This release tightens graph-memory correctness, reshapes the MCP surface around AI-memory workflows, and rewrites the product docs around `liel` as a portable external brain for LLMs.

### Added

- English and Japanese `Why liel` pages focused on the AI-memory / external-brain use case.
- Design-principles pages and a corresponding "Zen of Liel" section in the README.
- Dedicated MCP guides for:
  - AI memory operating rules
  - Claude project memory setup
  - updated MCP overview and tools reference
- A new Python MCP surface test suite covering the published tool contract.
- A WAL recovery regression test that exercises replay of extent-index pages and verifies cache invalidation after recovery.
- Self-loop regression tests for edge insertion, traversal, deletion, and node cascade deletion.
- A node-slot regression test asserting that empty labels and empty properties persist as `(offset=0, length=0)`.
- Notebook examples under `examples/notebooks/`, plus updated example docs and test-work notebook material.

### Fixed

- Self-loop edges now update both incoming and outgoing adjacency heads correctly, remain visible through `out_edges()` and `in_edges()`, and delete cleanly.
- `GraphDB::delete_node` now avoids double-deleting self-loop edges during cascade deletion.
- `Pager::open` now clears stale cached pages after WAL recovery before rebuilding extent chains, fixing recovery correctness when extent-index pages were replayed from the WAL.
- `FileHeader::from_bytes` no longer runs a duplicate dead-code checksum verification block.
- `CapacityExceeded` errors now report the correct unit for each resource kind, including bytes for property storage.
- MCP write operations no longer inject default metadata automatically.
- MCP FastMCP metadata now uses the correct description / instructions shape for the current integration path.
- Example scripts and notebooks were updated for current API compatibility.

### Changed

- Empty labels and empty property maps now skip unnecessary encoding work and persist as `(offset=0, length=0)` where appropriate.
- Product-facing documentation now positions `liel` as a portable external brain for LLM workflows, while keeping the property-graph storage engine as the implementation model.
- The public MCP surface was redesigned around AI-memory workflows:
  - tool names were renamed away from graph-database terminology
  - the surface was consolidated into seven public tools
  - setup guidance, merge guidance, and Claude/Codex-oriented usage docs were rewritten accordingly
- The README was substantially restructured around:
  - LLM memory in one file
  - optional MCP integration
  - product fit / non-fit guidance
  - portable external-brain positioning
- The Japanese README and top-level documentation were refreshed and brought back into alignment with the current product story.
- Notebook examples were moved under `examples/notebooks/` from the old top-level `notebooks/` location.

## [0.1.0] - 2026-04-21

First public Beta PyPI release.

### Added

- `CHANGELOG.md` for release notes.
- `acquire_graph_lock` in Python bindings: poisoned mutexes raise `RuntimeError` with a clear message instead of panicking.
- A reproducible benchmark entry point at `scripts/bench/bench_python_api.py` plus a reference page documenting how to run it.
- Rust-side WAL fault-injection tests covering torn writes mid-payload, trailing garbage past a valid commit, oversized `entry_length` corruption, and a corrupted commit marker following a valid write entry.
- A `tests/python/test_pyi_runtime_consistency.py` integration test that parses `python/liel/liel.pyi` with `ast` and cross-checks every public class member against the live runtime so stub drift is caught in CI.

### Changed

- **MCP `liel_query` response shape** changed from a bare JSON array (`[{...}, ...]`) to a paginated envelope (`{"nodes": [...], "next_cursor": int|null}`). A `cursor` parameter (default `0`) was added to page through large result sets. The previous shape was never part of a published release.
- **Version** `0.0.1` -> **`0.1.0`** in `Cargo.toml` and `pyproject.toml`.
- PyPI **Development Status** classifier remains **Beta** (`Development Status :: 4 - Beta`).
- WAL commit: missing dirty page is reported as `TransactionError` instead of `expect`/`panic`.
- LRU cache: avoid `unwrap` when reordering deque entries.
- CI: `cargo clippy` runs with **`-D warnings`** on Ubuntu.
- The main CI workflow now includes an Ubuntu wheel build/install smoke test in addition to the release-only package checks.
- Public Python docs and type stubs now state the single-writer-per-file constraint more explicitly and align the `shortest_path` documentation with the directed implementation.
- `liel.__version__` is now sourced from `importlib.metadata` (with a fallback to the value baked into the native extension) so the reported version always matches the published distribution.
- pytest no longer forces a shared repo-local `--basetemp` by default; Windows-specific temp path overrides remain opt-in locally and explicit in CI.
- Internal maintainer policy now formalizes the `private dev repo + public release repo + squash sync` workflow.
- Python crash-recovery tests now cover committed WAL replay and CRC-corrupted committed WAL rollback from the binding layer.
- Documentation and release instructions now align with the current stable-release workflow.

### Maintainer checklist after merge

1. Confirm [GitHub Actions](https://github.com/hy-token/liel/actions) is green on `main`.
2. Tag: `git tag v0.1.0 && git push origin v0.1.0`
3. Create a **GitHub Release** (notes can point to this file).
4. Publish wheels: on the public repo, **Actions -> Build and publish Python package**, run with `target=testpypi` first, then `target=pypi` once the TestPyPI install passes a clean-venv `import liel`.
5. Optional: `maturin build --release` -> `twine check dist/*` -> install the `.whl` in a clean venv and run `import liel`.

[0.1.0]: https://github.com/hy-token/liel/releases/tag/v0.1.0
[0.1.1]: https://github.com/hy-token/liel/releases/tag/v0.1.1
[0.2.0]: https://github.com/hy-token/liel/releases/tag/v0.2.0
[0.2.8]: https://github.com/hy-token/liel/releases/tag/v0.2.8
[0.2.9]: https://github.com/hy-token/liel/releases/tag/v0.2.9
[0.2.10]: https://github.com/hy-token/liel/releases/tag/v0.2.10
[0.4.0]: https://github.com/hy-token/liel/releases/tag/v0.4.0
[0.6.1]: https://github.com/hy-token/liel/releases/tag/v0.6.1
[0.6.2]: https://github.com/hy-token/liel/releases/tag/v0.6.2
[0.6.3]: https://github.com/hy-token/liel/releases/tag/v0.6.3
[0.7.0]: https://github.com/hy-token/liel/releases/tag/v0.7.0
[0.6.4]: https://github.com/hy-token/liel/releases/tag/v0.6.4
[0.6.0]: https://github.com/hy-token/liel/releases/tag/v0.6.0
[0.5.2]: https://github.com/hy-token/liel/releases/tag/v0.5.2
[0.5.1]: https://github.com/hy-token/liel/releases/tag/v0.5.1
[0.5.0]: https://github.com/hy-token/liel/releases/tag/v0.5.0
[0.5.0a1]: https://github.com/hy-token/liel/releases/tag/v0.5.0a1

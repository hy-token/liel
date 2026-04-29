# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

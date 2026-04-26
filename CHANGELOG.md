# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

First public stable PyPI release.

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

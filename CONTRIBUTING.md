# Contributing to liel

Thank you for your interest in contributing to liel!

## Setup

### Prerequisites

- Rust 1.75+: https://rustup.rs
- Python 3.9+
- C compiler (`build-essential` on Linux, Xcode CLT on macOS)

```bash
git clone https://github.com/hy-token/liel.git
cd liel

python3 -m venv .venv
source .venv/bin/activate

pip install maturin pytest
maturin develop
```

## Running Tests

```bash
# Rust unit tests
cargo test

# Python integration tests
pytest tests/python/

# Both
cargo test && maturin develop && pytest tests/python/
```

All tests must pass before submitting a PR.

### Windows note

If your machine has restrictive permissions on the default temporary directory,
point pytest at a writable repo-local temp directory explicitly:

```bash
python -m pytest tests/python -q --basetemp ./target/pytest-temp
```

CI already does this on Windows, so local failures around `%TEMP%` usually
indicate an environment-specific permission issue rather than a product bug.

## Pre-commit hooks (optional but recommended)

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, and `ruff check` on every PR. To catch the same issues locally before you commit, install the pre-commit hooks:

```bash
pip install pre-commit
pre-commit install
```

Hooks then run automatically on `git commit`. To run them on demand against the full repo:

```bash
pre-commit run --all-files
```

The hook configuration lives in [`.pre-commit-config.yaml`](.pre-commit-config.yaml).

## Documentation site (MkDocs)

Preview locally:

```bash
pip install -r requirements-dev.txt
mkdocs serve
```

Then open `http://127.0.0.1:8000`. The docs site is **not auto-deployed to GitHub Pages** at the moment (GitHub Free cannot restrict Pages access). Read the docs as Markdown directly under `docs/`, or build locally with `mkdocs build --strict`.

## Design Constraints

Before making changes, read [`docs/design/product-tradeoffs.md`](docs/design/product-tradeoffs.md) (which records rejected alternatives and rationale) and [`docs/reference/complexity.md`](docs/reference/complexity.md) (operation complexity, based on the actual implementation). Additional architecture and byte-level format references are being translated to English; until they land, ask in an issue if you need clarification.

The following are **frozen** — changes break file format compatibility:

| Item | Value |
|------|-------|
| Page size | 4096 bytes |
| NodeSlot size | 64 bytes |
| EdgeSlot size | 80 bytes |
| Adjacency list | singly-linked list (head insert) |
| Property encoding | custom binary (no external crates) |
| ID scheme | u64, 0 = NULL sentinel, 1-based |
| WAL granularity | page-level (4KB) |

**Allowed dependencies**: match root `Cargo.toml` — currently **`pyo3` only** (`tempfile` is dev-only). CRC, LRU cache, and errors are implemented in-tree; do not add crates for those without discussion.  
Do not add new Cargo dependencies without discussion.

## Pull Request Guidelines

1. One logical change per PR
2. Add tests for new behaviour in `tests/python/` and/or `src/` (`#[cfg(test)]`)
3. Keep commit messages in English, concise, imperative mood
4. If your change affects the on-disk format or the public Python API, update [`docs/design/product-tradeoffs.md`](docs/design/product-tradeoffs.md) and any affected docs under `docs/`

## File Structure

```
src/
  storage/   — pager, WAL, serializer, prop_codec, LRU cache
  graph/     — node/edge CRUD, adjacency list, BFS/DFS/shortest_path
  query/     — QueryBuilder (NodeQuery / EdgeQuery)
  python/    — PyO3 bindings (PyGraphDB, PyNode, PyEdge, …)
tests/
  python/    — pytest integration tests
examples/   — runnable sample scripts
docs/
  design/        — product-tradeoffs.md (and more, in English, coming)
  reference/     — complexity.md (and more, in English, coming)
  guide/         — user-facing guides (connectors, MCP, …)
```

## Reporting Issues

Please open an issue at https://github.com/hy-token/liel/issues with:

- liel version (`python -c "import liel; print(liel.open(':memory:').info())"`)
- Python version, OS
- Minimal reproducible example

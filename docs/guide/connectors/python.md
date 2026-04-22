# liel Python API

The Python API available with `pip install liel`.
For the file format and storage internals, see **[format spec](../../reference/format-spec.md)** and **[architecture overview](../../design/architecture.md)**.

---

## Install

```bash
pip install liel
```

Requires Python 3.9 or newer. No runtime dependencies.

**For maintainers:** to build the extension from source or to publish to PyPI, follow the procedures bundled with the development repository. End users only need `pip install` — Rust and `requirements-dev.txt` are not required.

**Write-safety note:** a single `.liel` file should have exactly one writer process. If you need shared writes, route them through one service or worker process and let other processes read from the committed file.

The runtime detects double-open and raises `liel.AlreadyOpenError` (a subclass of `liel.GraphDBError`). Within one process this is handled by an in-process registry; across processes it is handled by a `<file>.lock/` directory. The `with liel.open(...) as db:` block releases the writer slot on exit, so re-opening the same path in a subsequent block is fine.

---

## JSON import / export {#json-import-export}

**The Rust core `GraphDB` does not expose JSON methods.** The on-disk format is the binary described in [format spec](../../reference/format-spec.md); interchange with other systems is intended to happen through the **Python API layer**.

- **Reference implementations today:** [`examples/06_export.py`](https://github.com/hy-token/liel/blob/main/examples/06_export.py) and [`examples/03_bulk_import.py`](https://github.com/hy-token/liel/blob/main/examples/03_bulk_import.py) read and write nodes and edges through dicts.
- **Possible future direction:** add pure-Python helpers under `python/liel/` (built on top of `all_nodes_as_records` / `all_edges_as_records`), without growing the file format or the Rust API.

---

## Quick start

```python
import liel

db = liel.open("my.liel")
alice = db.add_node(["Person"], name="Alice", age=30)
bob   = db.add_node(["Person"], name="Bob",   age=25)
db.add_edge(alice, "KNOWS", bob, since=2024)
db.commit()
db.close()

# Reopen and confirm persistence
db = liel.open("my.liel")
friends = db.neighbors(alice, edge_label="KNOWS")
print(friends[0]["name"])  # "Bob"
```

---

## Module attributes and functions

| Name | Description |
|---|---|
| `liel.__version__` | Matches the installed Python package version, including PEP 440 pre-release suffixes |
| `liel.open(path, **kwargs)` | Open a file or `:memory:` and return a `GraphDB` |

```python
# Open a file (created if it does not exist)
db = liel.open("my.liel")

# In-memory (for testing)
db = liel.open(":memory:")

# Context manager (close is automatic)
with liel.open("my.liel") as db:
    ...
```

---

## Class reference

### GraphDB

#### Node operations

```python
# Add (returns a Node)
alice = db.add_node(labels=["Person"], name="Alice", age=30)
bob   = db.add_node(labels=["Person", "Employee"], name="Bob", age=25)
acme  = db.add_node(labels=["Company"], name="Acme Corp", founded=1990)

# Look up by ID (returns None if missing)
node = db.get_node(alice.id)

# Property access
print(alice.id)           # 1
print(alice.labels)       # ["Person"]
print(alice["name"])      # "Alice"
print(alice.properties)   # {"name": "Alice", "age": 30}

# Update properties (replaces the node's property map)
db.update_node(alice.id, age=31, city="Tokyo")

# Delete (also deletes incident edges)
db.delete_node(alice.id)
```

#### Edge operations

```python
# Add (always creates a new edge; duplicates allowed)
e1 = db.add_edge(alice, "KNOWS", bob, since=2020, strength=0.9)
e2 = db.add_edge(alice, "WORKS_AT", acme, role="Engineer", since=2019)

# Merge (returns the existing edge if (from, label, to, **props) matches
# exactly; otherwise creates a new one)
e3 = db.merge_edge(alice, "KNOWS", bob, since=2020)

# Read
edge = db.get_edge(e1.id)
print(edge.label)      # "KNOWS"
print(edge.from_node)  # alice.id
print(edge["since"])   # 2020

# Update
db.update_edge(e1.id, strength=1.0)

# Delete
db.delete_edge(e1.id)
```

#### Adjacency queries

```python
# Outgoing edges
out = db.out_edges(alice)
# -> [Edge(id=1, label="KNOWS", ...), Edge(id=2, label="WORKS_AT", ...)]

# Incoming edges
inc = db.in_edges(bob)

# Filter by label
knows_edges = db.out_edges(alice, label="KNOWS")

# Neighbouring nodes
friends   = db.neighbors(alice, edge_label="KNOWS")                   # outgoing (default)
followers = db.neighbors(alice, edge_label="KNOWS", direction="in")
all_adj   = db.neighbors(alice, direction="both")
```

#### Traversal

```python
# BFS (returns [(Node, depth), ...])
results = db.bfs(alice, max_depth=3)
for node, depth in results:
    print(f"{'  ' * depth}{node['name']} (depth={depth})")

# DFS
results = db.dfs(alice, max_depth=3)

# Minimum-hop directed path (BFS, unweighted; returns [Node, ...] or None)
path = db.shortest_path(alice, bob)
if path:
    print(" -> ".join(n["name"] for n in path))

# Restrict to one edge label (only out-edges of that label are followed)
path = db.shortest_path(alice, bob, edge_label="KNOWS")
```

**Performance notes:**

- `bfs()`, `dfs()`, and `shortest_path()` expand over the reachable subgraph, so high-degree hubs and large `max_depth` values can grow work quickly.
- `shortest_path()` is an unweighted BFS over **out-edges only**. It does not use edge properties as weights and does not traverse incoming edges.

#### Utilities

```python
# Statistics
print(db.node_count())    # 42
print(db.edge_count())    # 87

# Wipe everything and reset IDs (use to rebuild the graph from scratch)
db.clear()  # node_count=0, edge_count=0, next_id=1

# Rebuild adjacency metadata from the live edge set
report = db.repair_adjacency()
# -> {"nodes_rewritten": 42, "edges_relinked": 87}

# Enumerate everything (returned as a list)
for node in db.all_nodes():
    print(node)

for edge in db.all_edges():
    print(edge)

# Bulk dict fetch (fewer PyO3 objects; suitable for DataFrames)
rows = db.all_nodes_as_records()
rows = db.all_edges_as_records()

# Degree statistics (internally scans all edges once)
deg = db.degree_stats()  # { node_id: (out_degree, in_degree) }

# Edges whose both endpoints are in the given set (internally scans all edges)
sub = db.edges_between({alice.id, bob.id})

# Database info
info = db.info()
# -> {"version": "1.0", "node_count": 42, "edge_count": 87, "file_size": 12288}
```

**Heavy operations to watch:**

- `all_nodes()`, `all_edges()`, `all_nodes_as_records()`, and `all_edges_as_records()` scan the full allocated node or edge ID range.
- `degree_stats()` currently scans all edges once internally.
- `edges_between(...)` builds an ID set and still scans all edges.
- `out_edges()` / `in_edges()` walk the full adjacency list for that node; label filters do not skip the walk itself.
- `merge_edge()` is linear in the source node's out-degree because it searches existing outgoing edges for an exact match first.

**JSON conversion:** `GraphDB` does **not** have `export_json` / `import_json`. See [`examples/06_export.py`](https://github.com/hy-token/liel/blob/main/examples/06_export.py) and [`examples/03_bulk_import.py`](https://github.com/hy-token/liel/blob/main/examples/03_bulk_import.py).

**Corruption guidance:** if `liel.CorruptedFileError` says an adjacency list is inconsistent or cyclic, stop writing to that `.liel` file, copy it somewhere safe, and run `db.repair_adjacency()`. That repair pass rebuilds node head pointers, degree counters, and edge next-pointers from the live edge slots. If repair fails because a live edge points to a missing or deleted node, the damage is beyond adjacency metadata alone and the safest path is restore-or-salvage into a fresh database.

**Combining two databases:** `GraphDB.merge_from` imports all nodes and edges from another DB. IDs are remapped automatically (no file-format change).

```python
dst = liel.open("aggregated.liel")
src = liel.open("batch-2025-04.liel")

# Default: every src node and edge is appended as new
report = dst.merge_from(src)

# Identify existing nodes by a property key (e.g. unify users with the same email)
report = dst.merge_from(
    src,
    node_key=["email"],
    edge_strategy="idempotent",        # exact (from, label, to, props) match is reused
    on_node_conflict="overwrite_from_src",
)
print(report.nodes_created, report.nodes_reused)
print(report.edges_created, report.edges_reused)

dst.commit()                           # merge_from does not commit
```

`MergeReport` carries `node_id_map: dict[src_id, dst_id]`, `edge_id_map`, and per-class counts. If a property listed in `node_key` is missing on a src node, `liel.MergeError` is raised.

---

### Node

```python
class Node:
    id: int
    labels: list[str]
    properties: dict[str, Any]

    def __getitem__(self, key: str) -> Any: ...
    def __repr__(self) -> str: ...
```

### Edge

```python
class Edge:
    id: int
    label: str
    from_node: int
    to_node: int
    properties: dict[str, Any]

    def __getitem__(self, key: str) -> Any: ...
    def __repr__(self) -> str: ...
```

### GraphDB (canonical signatures live in `python/liel/liel.pyi`)

A summary of the main methods. Full signatures and docstrings are in **`python/liel/liel.pyi`** (`export_json` / `import_json` **do not exist**).

```python
class GraphDB:
    def add_node(self, labels: list[str] = [], **properties) -> Node: ...
    def get_node(self, node_id: int) -> Node | None: ...
    def update_node(self, node_id: int, **properties) -> None: ...
    def delete_node(self, node: int | Node) -> None: ...
    def add_edge(self, from_node: int | Node, label: str, to_node: int | Node, **properties) -> Edge: ...
    def merge_edge(self, from_node: int | Node, label: str, to_node: int | Node, **properties) -> Edge: ...
    def get_edge(self, edge_id: int) -> Edge | None: ...
    def update_edge(self, edge_id: int, **properties) -> None: ...
    def delete_edge(self, edge: int | Edge) -> None: ...
    def out_edges(self, node: int | Node, label: str | None = None) -> list[Edge]: ...
    def in_edges(self, node: int | Node, label: str | None = None) -> list[Edge]: ...
    def neighbors(self, node: int | Node, edge_label: str | None = None, direction: str = "out") -> list[Node]: ...
    def bfs(self, start: int | Node, max_depth: int) -> list[tuple[Node, int]]: ...
    def dfs(self, start: int | Node, max_depth: int) -> list[tuple[Node, int]]: ...
    def shortest_path(self, start: int | Node, goal: int | Node, edge_label: str | None = None) -> list[Node] | None: ...
    def nodes(self) -> NodeQuery: ...
    def edges(self) -> EdgeQuery: ...
    def all_nodes(self) -> list[Node]: ...
    def all_edges(self) -> list[Edge]: ...
    def all_nodes_as_records(self) -> list[dict]: ...
    def all_edges_as_records(self) -> list[dict]: ...
    def degree_stats(self) -> dict[int, tuple[int, int]]: ...
    def edges_between(self, node_ids: Any) -> list[dict]: ...
    def node_count(self) -> int: ...
    def edge_count(self) -> int: ...
    def begin(self) -> None: ...  # currently a no-op
    def commit(self) -> None: ...
    def rollback(self) -> None: ...
    def transaction(self) -> ContextManager: ...
    def info(self) -> dict: ...
    def vacuum(self) -> None: ...
    def clear(self) -> None: ...
    def repair_adjacency(self) -> dict[str, int]: ...
    def close(self) -> None: ...
    def __enter__(self) -> GraphDB: ...
    def __exit__(self, *args) -> None: ...
```

### NodeQuery / EdgeQuery

The method is named **`where_`** (to avoid collision with Python's `where` reserved-ish usage). Canonical reference: `liel.pyi`.

```python
class NodeQuery:
    def label(self, label: str) -> NodeQuery: ...
    def where_(self, predicate: Callable[[Node], bool]) -> NodeQuery: ...
    def limit(self, n: int) -> NodeQuery: ...
    def skip(self, n: int) -> NodeQuery: ...
    def fetch(self) -> list[Node]: ...
    def exists(self) -> bool: ...
    def count(self) -> int: ...

class EdgeQuery:
    def label(self, label: str) -> EdgeQuery: ...
    def where_(self, predicate: Callable[[Edge], bool]) -> EdgeQuery: ...
    def limit(self, n: int) -> EdgeQuery: ...
    def skip(self, n: int) -> EdgeQuery: ...
    def fetch(self) -> list[Edge]: ...
    def exists(self) -> bool: ...
    def count(self) -> int: ...
```

#### Examples

```python
# Filter nodes
results = (
    db.nodes()
      .label("Person")
      .where_(lambda n: n["age"] > 20)
      .limit(10)
      .fetch()
)

# Filter edges
edges = (
    db.edges()
      .label("KNOWS")
      .where_(lambda e: e["since"] >= 2020)
      .fetch()
)

# Existence check / count
exists = db.nodes().label("Person").where_(lambda n: n["name"] == "Alice").exists()
count  = db.nodes().label("Person").count()
```

**Query performance notes:**

- Label filters are applied on the Rust side first, so `label(...)` is the cheapest way to narrow candidates.
- `where_(...)` runs a Python predicate for each surviving candidate, so large candidate sets can become expensive even when the predicate itself is simple.
- `count()` and `exists()` currently materialize matching results rather than using a streaming counter or first-match short circuit.

---

## Transactions

A transaction is implicitly active right after `open()`. `begin()` is a **no-op** in the current implementation (kept for compatibility).

```python
# Explicit transaction (begin is optional)
try:
    alice = db.add_node(labels=["Person"], name="Alice")
    db.add_edge(alice, "KNOWS", bob)
    db.commit()
except Exception:
    db.rollback()
    raise

# Context manager (recommended)
with db.transaction():
    alice = db.add_node(labels=["Person"], name="Alice")
    db.add_edge(alice, "KNOWS", bob)
# normal exit -> auto commit; exception -> auto rollback
```

**Crash safety:** if the process exits without `commit()`, the next `open()` finds no Commit entry in the WAL, the changes are discarded, and the database returns to the state immediately after the last commit.

---

## Exceptions

```python
liel.GraphDBError         # base class
liel.NodeNotFoundError    # node does not exist
liel.EdgeNotFoundError    # edge does not exist
liel.CorruptedFileError   # file is corrupted
liel.TransactionError     # transaction violation
OSError                  # file I/O error from the operating system
```

```python
try:
    node = db.get_node(9999)  # -> None (does not raise)
    db.delete_node(9999)      # -> NodeNotFoundError
except liel.NodeNotFoundError as e:
    print(e)
```

---

## Limitations

- **No concurrent writes:** keep one writer process per `.liel` file. A second writer open is rejected with `AlreadyOpenError`; this protects the file, but it does not make concurrent mutation supported. Multiple readers are fine when coordinated outside the writer path.
- **No query language:** Python API only. A DSL is a candidate for Phase 2 or later.
- **u64 auto-assigned node / edge IDs:** user-supplied IDs are not allowed.
- **Property size:** up to 64 MB per entry.
- **Index:** adjacency lists only. A property index is a candidate for Phase 3 or later.

---

## Build and packaging (for developers)

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.5,<2.0"]
build-backend = "maturin"

[project]
name = "liel"
version = "0.1.0a1"
requires-python = ">=3.9"

[tool.maturin]
features = ["pyo3/extension-module"]
python-source = "python"
```

### Build commands

The Python dev dependencies (`maturin`, `pytest`) are listed in **`requirements-dev.txt`** at the repository root.

```bash
pip install -r requirements-dev.txt

# Editable install (development build)
maturin develop

# Release build
maturin build --release

# Tests
cargo test
pytest tests/python/
```

---

## Python integration tests (pytest)

```python
# tests/python/test_basic.py
def test_add_and_get_node(db):
    node = db.add_node(labels=["Person"], name="Alice")
    fetched = db.get_node(node.id)
    assert fetched["name"] == "Alice"
    assert "Person" in fetched.labels

def test_traversal(db):
    a = db.add_node(labels=["X"])
    b = db.add_node(labels=["X"])
    c = db.add_node(labels=["X"])
    db.add_edge(a, "LINK", b)
    db.add_edge(b, "LINK", c)
    results = db.bfs(a, max_depth=2)
    assert len(results) == 2

def test_persistence(tmp_path):
    path = tmp_path / "test.liel"
    with liel.open(str(path)) as db:
        db.add_node(labels=["Person"], name="Alice")
    with liel.open(str(path)) as db:
        results = db.nodes().label("Person").fetch()
        assert results[0]["name"] == "Alice"

def test_crash_recovery(tmp_path):
    # Test startup from a state where the WAL is non-empty
    ...
```

---

## Type stubs

`python/liel/liel.pyi` carries type stubs that match the PyO3-generated signatures. IDE completion uses this file (PEP 561 — the stub filename matches the compiled `liel.liel` module).

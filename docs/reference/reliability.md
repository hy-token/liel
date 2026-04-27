# Reliability and failure model

This page is the reference for what `liel` takes responsibility for, and where
the failure model intentionally stops.

`liel` is not a server database. Its durability path is designed around a narrow
model: **one writer process writing one local `.liel` file, with explicit
`commit()` calls**.

For the feature catalogue, see [feature list](features.md). For the byte-level
format, see [format spec](format-spec.md). For why this scope is
intentional, see [product trade-offs](../design/product-tradeoffs.md).

---

## 1. Short version

Use `liel` when this model fits:

- One process owns writes to a `.liel` file.
- Writes are grouped and finalized with `commit()` or `with db.transaction():`.
- The file lives on a normal local filesystem.
- If the process crashes, reopening the file should return to the last
  committed state or replay a complete WAL.

Do not rely on `liel` for:

- Multiple processes mutating the same `.liel` file at the same time.
- Server-style concurrent writer workloads.
- Filesystems that do not preserve ordinary write and fsync ordering.
- Keeping changes that were never committed.

---

## 2. What is guaranteed

### 2.1 Committed data is the unit of durability

`commit()` is the durability boundary. Before `commit()`, changes belong to the
current `GraphDB` session and are not guaranteed to survive a process crash.
After a successful `commit()`, those changes are intended to survive crash and
reopen.

Recommended pattern:

```python
with db.transaction():
    db.add_node(["Task"], title="write docs")
    db.add_node(["Decision"], text="keep the API small")
```

For larger imports, commit in explicit batches sized for the WAL reservation:

```python
pending = 0
for row in rows:
    db.add_node(["Record"], **row)
    pending += 1
    if pending >= 20_000:
        db.commit()
        pending = 0
if pending:
    db.commit()
```

### 2.2 Interrupted commits are recovered on open

`liel` uses a fixed-location, page-level WAL. On commit, modified pages are
written to the WAL, the WAL is fsynced, pages are copied to their canonical
locations, and the data file is fsynced.

If the process stops while the WAL is still live, the next open replays complete
WAL entries back into the data file. The byte layout is specified in
[format spec §6](format-spec.md#6-wal-write-ahead-log).

### 2.3 Double-open inside one process is rejected

Within one Python process, opening the same `.liel` path twice for writing
returns `AlreadyOpenError`.

```python
db1 = liel.open("memory.liel")
db2 = liel.open("memory.liel")  # AlreadyOpenError
```

Close the previous handle, or let its `with` block exit, before reopening.

### 2.4 Corrupt files fail closed

Files that cannot be interpreted safely should not be read silently.

Examples:

- Invalid magic bytes.
- Header checksum mismatch.
- Unsupported layout metadata or format version.
- Truncated files.
- WAL entries that fail validation.
- Invalid property, slot, or extent references.

These surface as `GraphDBError` subclasses such as `CorruptedFileError`.

---

## 3. What is not guaranteed

### 3.1 Multi-process writers are rejected

When another process tries to open the same `.liel` file for writing, `liel`
uses a `<file>.lock/` directory to reject the second writer. This is not
concurrent write support; it is fail-closed protection against file corruption.

Recommended pattern:

- Put one process in charge of writes to a `.liel` file.
- If several tools or agents need to update it, route writes through an MCP
  server, application service, job queue, or another owner process.
- Do not let each producer open and mutate the same `.liel` file directly.

If a writer process crashes and leaves `.lock/` behind, the next `open()` checks
the PID stored in `owner.json`. If the owner is clearly dead, `liel` reclaims the
stale lock with `rename -> delete -> recreate`. If the owner is alive or cannot
be checked safely, `open()` fails with `AlreadyOpenError`.

### 3.2 Uncommitted changes may be lost

If the process exits before `commit()`, those changes are outside the durability
contract. Reopening the file returns to the last committed state.

Use `rollback()` when you want to discard in-session changes explicitly.

### 3.3 Network and sync filesystems are outside the comfort zone

`liel` relies on the local filesystem preserving ordinary write and fsync
ordering. Cloud sync folders, network filesystems, virtual filesystems, and
aggressive antivirus or backup tools can change the real failure behavior.

For durable application state, prefer a normal local disk. Copy the `.liel` file
for backup only after the writer has closed or after coordinating explicitly
with the writer process.

### 3.4 WAL capacity is finite

The current WAL reservation is fixed at 4 MiB. A single transaction that would
exceed that reservation returns a transaction error before it can be committed.

Split very large imports into batches. This is intentional: it keeps recovery
small and predictable.

---

## 4. Failure modes table

| Failure | Expected behavior |
|---|---|
| Process exits before `commit()` | Uncommitted changes are lost; reopen returns to the last committed state |
| Process exits before WAL write | Reopen returns to the last committed state |
| Process exits after WAL fsync but before data-page apply | Next open replays the WAL |
| Process exits while applying data pages | Next open replays the remaining WAL |
| WAL entry is truncated or fails CRC | Recovery accepts complete entries only; invalid tail is not applied |
| Header checksum mismatch | `CorruptedFileError` |
| Unsupported format version | Explicit compatibility error or `CorruptedFileError` |
| Same file opened twice in one process | `AlreadyOpenError` |
| Same file opened by two writer processes | Lock directory rejects the second writer with `AlreadyOpenError` |
| `.lock/` remains after writer crash | Next `open()` reclaims it if the owner PID is dead |
| Transaction exceeds WAL reservation | Transaction error; retry with smaller batches |

---

## 5. Operational recommendations

- Keep one writer process per `.liel` file.
- Group bulk inserts inside `with db.transaction():` or explicit batch commits.
- Avoid very frequent tiny commits.
- Do not write directly to `.liel` files inside cloud sync folders.
- Copy a `.liel` file for backup only after the writer has closed, or after
  coordinating with the writer process.
- For MCP integration, route writes through one MCP server or owner process
  rather than letting multiple clients open the file directly.

---

## 6. Compatibility policy

### 6.1 API compatibility

`liel` is published as `Development Status :: 4 - Beta` during the current
`0.x` series. The Beta compatibility surface is Python-first:

- `liel.open`
- `GraphDB` lifecycle, CRUD, traversal, transaction, QueryBuilder, merge,
  maintenance, and metadata methods documented in the Python guide
- `Node`, `Edge`, `NodeQuery`, `EdgeQuery`, `Transaction`, and `MergeReport`
- the main exception classes under `GraphDBError`

Breaking changes may still happen before `1.0`, but changes to this surface
should be recorded in the [changelog](https://github.com/hy-token/liel/blob/main/CHANGELOG.md) with migration notes.
Rust internals and helper APIs that are not documented in the Python guide may
change more freely.

### 6.2 On-disk format compatibility

The canonical `.liel` file format lives in [format spec](format-spec.md).
During `0.x`, breaking format changes may happen, but they must be explicit:
the format version should be checked, unsupported future formats should fail
closed, and [release notes](https://github.com/hy-token/liel/blob/main/CHANGELOG.md) should say whether existing
files remain readable or need migration.

An older `liel` should not silently read an unknown future format version.
Unsupported formats fail closed.

---

## 7. How to describe reliability

Good short description:

> `liel` is a portable external-brain store built on a graph engine that uses
> a page-level WAL to recover committed writes under a single-writer,
> single-file, local-filesystem model.

Avoid saying:

> `liel` is a server-grade, fully concurrent database.

The accurate claim is narrower and stronger: `liel` is not trying to make
concurrent writers safe; it is trying to keep local single-file graph memory
recoverable and easy to reason about.

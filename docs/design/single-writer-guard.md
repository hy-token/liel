# Single-writer guard

`liel` is designed for one writer process per `.liel` file. The guard exists to
fail closed before two independent pagers can write the same WAL/header region.

This is not multi-writer concurrency control. It is a safety boundary for the
single-file, single-writer durability model described in
[reliability](../reference/reliability.md) and
[product trade-offs §5.1](product-tradeoffs.md#51-multi-process-writers-on-the-same-file-are-rejected).

## Invariants

- At most one live writer owns a file path at a time.
- `:memory:` databases are independent and are not registered.
- A second writer attempt returns `AlreadyOpenError`.
- A stale lock left by a crashed owner may be reclaimed only when the recorded
  owner process is clearly dead.
- If liveness cannot be determined safely, opening fails closed.

## Mechanism

The Rust core uses two layers:

- An in-process registry keyed by the canonical database path.
- A cross-process `<file>.lock/` directory containing `owner.json` for stale
  owner diagnostics.

Directory creation is the lock acquisition primitive because it is available in
the standard library and works across the supported platforms without adding a
runtime dependency.

## Recovery

If `<file>.lock/owner.json` points to a dead process, `open()` renames and
removes the stale lock directory, then retries acquisition. The retry count is
bounded so races with another process cannot spin forever.

The lock directory contains no graph data. Removing a confirmed stale lock does
not change the `.liel` file; it only allows the next single writer to open it.

## Non-goals

- Concurrent peer writers.
- Read/write lock scheduling.
- Network filesystem guarantees.
- Using the lock as a substitute for application-level write ownership.

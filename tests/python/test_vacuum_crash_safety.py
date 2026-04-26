"""Vacuum crash-safety tests for the 0.3 copy-on-write rewrite.

Drives the `LIEL_VACUUM_CRASH_AT` injection points exposed by
`src/graph/fault_inject.rs`.  Each test forks a worker that opens the
database, sets the env var to a chosen injection point, and calls
`db.vacuum()`.  The worker is expected to terminate via `os._exit(1)`
inside the Rust extension before vacuum completes, leaving the on-disk
state in whatever shape that injection point produces.  The parent then
re-opens the same path and asserts:

  * data is in either the pre-vacuum or post-vacuum state — never a
    half-finished mix,
  * `<basename>.liel.tmp` is absent after the parent's `open()` (the
    stale-tmp sweep cleans it up),
  * IDs allocated before vacuum still resolve, and the next allocator
    ID is monotonic.

The whole module is skipped unless the wheel was built with
``--features test-fault-injection``.  Without that flag,
``liel._BUILT_WITH_FAULT_INJECTION`` is False and the injection hooks
are no-ops, so the worker would not actually crash and the test would
hang or pass trivially.
"""

from __future__ import annotations

import os
import sys

import pytest

import liel
import liel.liel as _liel_native

# `_BUILT_WITH_FAULT_INJECTION` lives on the native module (`liel.liel`),
# not on the Python wrapper that selectively re-exports public surface.
if not getattr(_liel_native, "_BUILT_WITH_FAULT_INJECTION", False):
    pytest.skip(
        "liel was built without `test-fault-injection`; rebuild with "
        "`maturin develop --features pyo3/extension-module,test-fault-injection` "
        "to run these tests.",
        allow_module_level=True,
    )

if sys.platform == "win32":
    pytest.skip(
        "vacuum crash-safety harness uses os.fork(); skipped on Windows.",
        allow_module_level=True,
    )


# Injection-point names — must stay in sync with crash_at(...) calls in
# `src/graph/vacuum.rs`.  Adding a new point: extend this list and add a
# matching test case below.
INJECTION_POINTS = [
    "BEFORE_TMP_OPEN",
    "AFTER_TMP_WRITES",
    "AFTER_TMP_FSYNC",
    "AFTER_RENAME",
]


def _seed_database(path: str) -> tuple[int, int, int]:
    """Populate `path` with three nodes (one deleted), one edge, then
    commit and close.  Returns (alice_id, bob_id, edge_id)."""
    db = liel.open(path)
    try:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Person"], name="Bob")
        carol = db.add_node(["Person"], name="Carol")
        edge = db.add_edge(alice, "KNOWS", bob, since=2020)
        db.commit()
        db.delete_node(carol)
        db.commit()
        return alice.id, bob.id, edge.id
    finally:
        db.close()


def _vacuum_with_injected_crash(path: str, crash_at: str) -> int:
    """Run `db.vacuum()` in a child process that crashes at the named
    injection point.  Returns the child's exit status as reported by
    `os.waitpid` so callers can sanity-check the crash actually fired."""
    pid = os.fork()
    if pid == 0:
        # Child — never returns.
        try:
            os.environ["LIEL_VACUUM_CRASH_AT"] = crash_at
            db = liel.open(path)
            db.vacuum()
            # If we reach here, the injection point did NOT trigger.
            # Distinguish from a normal crash so the parent's assertion
            # can fail meaningfully instead of silently passing.
            os._exit(99)
        except SystemExit:
            raise
        except BaseException:
            # Any other exception: report a unique exit code so the
            # parent does not misread it as a controlled crash.
            os._exit(98)
    _, status = os.waitpid(pid, 0)
    return status


@pytest.mark.parametrize("injection_point", INJECTION_POINTS)
def test_vacuum_survives_crash_at_each_injection_point(tmp_path, injection_point):
    """For every injection point, the database must reopen successfully
    after a crash and surface either the pre-vacuum or post-vacuum
    state — never a half-finished mix and never a leftover `.tmp`.
    """
    db_path = str(tmp_path / "crash.liel")
    tmp_sibling = db_path + ".tmp"

    alice_id, bob_id, edge_id = _seed_database(db_path)

    # Crash the worker at the chosen point.
    status = _vacuum_with_injected_crash(db_path, injection_point)
    # `os._exit(1)` from the injection hook produces WEXITSTATUS == 1.
    # The hook fires *unconditionally* once the env var matches; if the
    # child exited any other way it means the injection point was not
    # reached, which is a real bug we want surfaced.
    assert os.WIFEXITED(status), f"child did not exit cleanly: status={status}"
    assert os.WEXITSTATUS(status) == 1, (
        f"injection point {injection_point} did not fire — "
        f"child exit code was {os.WEXITSTATUS(status)}"
    )

    # Reopen.  This must succeed; the open-time sweep removes any
    # leftover `.tmp` from a partial vacuum.
    assert not os.path.exists(tmp_sibling) or injection_point in {
        "AFTER_TMP_WRITES",
        "AFTER_TMP_FSYNC",
    }, "before reopen, .tmp should only exist if we crashed mid/post-write"
    db = liel.open(db_path)
    try:
        assert not os.path.exists(tmp_sibling), (
            f"after open(), the stale-tmp sweep should have removed {tmp_sibling}"
        )

        # Alice and Bob were committed before the crash.  They must
        # always be visible regardless of where the crash happened.
        alice = db.get_node(alice_id)
        bob = db.get_node(bob_id)
        edge = db.get_edge(edge_id)
        assert alice is not None, "alice survived a pre-vacuum commit"
        assert bob is not None, "bob survived a pre-vacuum commit"
        assert edge is not None, "edge survived a pre-vacuum commit"
        assert alice["name"] == "Alice"
        assert edge["since"] == 2020

        # Next allocated ID is strictly greater than every existing ID
        # (carol's ID was 3; vacuum either kept the counter at 4 or did
        # not run — either way, the next allocation lands at 4 or
        # higher).
        new_node = db.add_node([], scratch=True)
        assert new_node.id >= 4, (
            f"id counter regressed after crash at {injection_point}: "
            f"got {new_node.id}, expected >= 4"
        )
    finally:
        db.close()


def test_vacuum_after_a_partial_crash_then_full_run(tmp_path):
    """After a crash, a subsequent successful `vacuum()` must still
    produce a clean, fully-compacted state.  Guards against any
    sticky in-memory residue from the failed run."""
    db_path = str(tmp_path / "retry.liel")
    alice_id, _bob_id, _edge_id = _seed_database(db_path)

    status = _vacuum_with_injected_crash(db_path, "AFTER_TMP_WRITES")
    assert os.WIFEXITED(status) and os.WEXITSTATUS(status) == 1

    db = liel.open(db_path)
    try:
        # Second vacuum should succeed normally.
        db.vacuum()
        assert not os.path.exists(db_path + ".tmp")
        # Pre-vacuum data still present.
        assert db.get_node(alice_id) is not None
    finally:
        db.close()

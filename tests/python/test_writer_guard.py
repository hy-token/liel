"""Single-writer guard tests.

These tests verify that ``liel.open`` rejects a second writer handle on the
same ``.liel`` file and surfaces the error to Python as
:class:`liel.AlreadyOpenError`. The Rust core combines an in-process registry
with a cross-process ``.lock/`` directory; see
``docs/design/single-writer-guard.ja.md``.
"""

from __future__ import annotations

import gc
import os
import subprocess
import sys
import textwrap
import time
from pathlib import Path

import pytest

import liel

REPO_ROOT = Path(__file__).resolve().parents[2]


def test_alreadyopen_error_is_a_graphdberror():
    assert issubclass(liel.AlreadyOpenError, liel.GraphDBError)


def test_second_open_on_same_path_raises(tmp_path):
    db_path = str(tmp_path / "guard.liel")

    first = liel.open(db_path)
    try:
        with pytest.raises(liel.AlreadyOpenError) as excinfo:
            liel.open(db_path)
        assert "already open" in str(excinfo.value).lower()
    finally:
        # Drop the first handle and force GC so the registry slot is
        # released before pytest tears the temp directory down.
        del first
        gc.collect()


def test_open_after_close_succeeds(tmp_path):
    db_path = str(tmp_path / "reopen.liel")

    with liel.open(db_path) as db:
        db.add_node(["Marker"], n=1)
        db.commit()

    # Force GC so the previous handle releases its registry slot before the
    # next open attempt; CPython usually does this synchronously, but we
    # do not want the test to be flaky on alternative implementations.
    gc.collect()

    with liel.open(db_path) as db2:
        nodes = db2.nodes().label("Marker").fetch()
        assert len(nodes) == 1


def test_path_normalisation_treats_aliases_as_same_file(tmp_path):
    """A trailing ``/.`` segment must not slip past the registry."""
    db_path = tmp_path / "alias.liel"
    canonical = str(db_path)
    aliased = str(tmp_path / "." / "alias.liel")

    first = liel.open(canonical)
    try:
        with pytest.raises(liel.AlreadyOpenError):
            liel.open(aliased)
    finally:
        del first
        gc.collect()


def test_memory_db_is_not_subject_to_guard():
    a = liel.open(":memory:")
    b = liel.open(":memory:")
    try:
        # Each in-memory database is its own buffer, so they must be
        # independent — no AlreadyOpenError.
        a.add_node(["A"], i=1)
        b.add_node(["A"], i=2)
        a.commit()
        b.commit()
        assert a.node_count() == 1
        assert b.node_count() == 1
    finally:
        a.close()
        b.close()


def test_child_process_holding_file_blocks_parent_open(tmp_path):
    db_path = tmp_path / "cross-process.liel"
    child_code = textwrap.dedent(
        """
        import pathlib
        import sys
        import time

        import liel

        db = liel.open(sys.argv[1])
        pathlib.Path(sys.argv[2]).write_text("ready", encoding="utf-8")
        try:
            time.sleep(30)
        finally:
            db.close()
        """
    )
    ready_path = tmp_path / "ready.txt"
    env = child_env()

    child = subprocess.Popen(
        [sys.executable, "-c", child_code, str(db_path), str(ready_path)], env=env
    )
    try:
        for _ in range(100):
            if ready_path.exists():
                break
            if child.poll() is not None:
                pytest.fail(f"child exited before opening database: {child.returncode}")
            time.sleep(0.05)
        else:
            pytest.fail("child did not report ready")

        with pytest.raises(liel.AlreadyOpenError):
            liel.open(str(db_path))
    finally:
        child.terminate()
        child.wait(timeout=5)


def test_stale_child_lock_is_reclaimed_after_crash(tmp_path):
    db_path = tmp_path / "stale-process.liel"
    child_code = textwrap.dedent(
        """
        import os
        import sys

        import liel

        db = liel.open(sys.argv[1])
        db.add_node(["Marker"], value=1)
        db.commit()
        os._exit(7)
        """
    )

    child = subprocess.run([sys.executable, "-c", child_code, str(db_path)], check=False)
    assert child.returncode == 7
    assert db_path.with_name(db_path.name + ".lock").exists()

    with liel.open(str(db_path)) as db:
        assert db.node_count() == 1

    assert not db_path.with_name(db_path.name + ".lock").exists()


def child_env():
    env = dict(os.environ)
    paths = [str(REPO_ROOT), str(REPO_ROOT / "python")]
    if env.get("PYTHONPATH"):
        paths.append(env["PYTHONPATH"])
    env["PYTHONPATH"] = os.pathsep.join(paths)
    return env

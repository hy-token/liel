import pytest

import liel


@pytest.fixture
def db(tmp_path):
    """Open a fresh file-backed database in a temporary directory.

    The database file is created at ``tmp_path/test.liel`` and closed
    automatically after the test finishes.  Data written without an explicit
    ``commit()`` call is discarded on close.
    """
    d = liel.open(str(tmp_path / "test.liel"))
    yield d
    d.close()


@pytest.fixture
def mem_db():
    """Open a fresh in-memory database that is never written to disk.

    Useful for tests that do not need persistence across re-opens and that
    benefit from faster I/O.  The database is closed after the test finishes.
    """
    d = liel.open(":memory:")
    yield d
    d.close()


@pytest.fixture
def populated_query_graph():
    """Per-test in-memory graph for QueryBuilder tests (``test_phase2_query_*``).

    Function-scoped: each test receives a freshly populated graph, so tests can
    mutate it without affecting one another.

    3 Person nodes, 1 Company node, and 3 edges (2 KNOWS, 1 LIKES).

    Yields ``(db, alice, bob, carol)``.
    """
    d = liel.open(":memory:")
    alice = d.add_node(["Person"], name="Alice", age=30)
    bob = d.add_node(["Person"], name="Bob", age=19)
    carol = d.add_node(["Person"], name="Carol", age=42)
    d.add_node(["Company"], name="Acme")

    d.add_edge(alice, "KNOWS", bob, since=2022)
    d.add_edge(alice, "KNOWS", carol, since=2018)
    d.add_edge(bob, "LIKES", carol, score=5)
    yield d, alice, bob, carol
    d.close()

"""Tests that cover features described in the specification but not exercised
by the milestone test files.  This module fills in coverage gaps for the
neighbors() direction parameter, the .properties accessor on Node and Edge,
the GraphDBError exception hierarchy, the with db.transaction() context
manager, and the begin() no-op behaviour.

Historically this file defined a local ``db`` fixture backed by
``:memory:`` that silently shadowed the file-backed ``db`` fixture in
conftest.py, which made it unclear which storage each test actually
exercised.  The local fixture now explicitly delegates to ``mem_db``
(the in-memory fixture in conftest.py), so the dependency shows up in
the test signature dependency graph and there is no risk of accidentally
picking up the file-backed variant from another file.
"""

import pytest

import liel


@pytest.fixture
def db(mem_db):
    """Alias for the conftest ``mem_db`` fixture.

    Makes the in-memory backing explicit by inheriting from ``mem_db``
    instead of calling ``liel.open(":memory:")`` locally.  Test bodies keep
    using the familiar ``db`` name.
    """
    return mem_db


@pytest.fixture
def graph(db):
    """Builds a small directed graph used by the neighbors() direction tests.

    Graph topology::

        a --KNOWS-->  b
        a --LIKES-->  c
        d --FOLLOWS-> a

    Returns the database handle and the four node objects as a tuple
    ``(db, a, b, c, d)``.
    """
    a = db.add_node(["Person"], name="Alice", age=30)
    b = db.add_node(["Person"], name="Bob", age=25)
    c = db.add_node(["Person"], name="Carol", age=40)
    d = db.add_node(["Person"], name="Dave", age=20)
    db.add_edge(a, "KNOWS", b)
    db.add_edge(a, "LIKES", c)
    db.add_edge(d, "FOLLOWS", a)
    return db, a, b, c, d


# ── neighbors() direction parameter ──────────────────────────────────────────


def test_neighbors_out_default(graph):
    """neighbors() with no direction argument must default to outgoing edges.

    Node a has two outgoing edges (to b and c), so neighbors(a) must return
    exactly {b, c}.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a)
    ids = {n.id for n in ns}
    assert ids == {b.id, c.id}


def test_neighbors_out_explicit(graph):
    """neighbors(direction='out') must return only nodes reachable via outgoing edges.

    Explicitly passing direction='out' must produce the same result as the
    default — only the two nodes that a points to.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a, direction="out")
    ids = {n.id for n in ns}
    assert ids == {b.id, c.id}


def test_neighbors_in(graph):
    """neighbors(direction='in') must return only nodes that have an edge pointing to the given node.

    Node a is the target of one incoming edge (from d via FOLLOWS), so
    neighbors(a, direction='in') must return exactly {d}.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a, direction="in")
    ids = {n.id for n in ns}
    assert ids == {d.id}


def test_neighbors_both(graph):
    """neighbors(direction='both') must return nodes reachable via edges in either direction.

    Node a has two outgoing edges (to b, c) and one incoming edge (from d), so
    neighbors(a, direction='both') must return exactly {b, c, d}.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a, direction="both")
    ids = {n.id for n in ns}
    assert ids == {b.id, c.id, d.id}


def test_neighbors_with_label_filter(graph):
    """neighbors() with edge_label must filter to only edges with that label.

    Node a has edges labelled KNOWS (to b) and LIKES (to c).  Filtering by
    edge_label='KNOWS' must return only b.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a, edge_label="KNOWS")
    assert len(ns) == 1
    assert ns[0].id == b.id


def test_neighbors_in_with_label_filter(graph):
    """neighbors(direction='in') with edge_label must filter incoming edges by label.

    Node a has one incoming FOLLOWS edge from d.  Filtering with
    edge_label='FOLLOWS' and direction='in' must return only d.
    """
    db, a, b, c, d = graph
    ns = db.neighbors(a, edge_label="FOLLOWS", direction="in")
    assert len(ns) == 1
    assert ns[0].id == d.id


def test_neighbors_both_no_label_filter(graph):
    """neighbors(direction='both') without a label filter must include all adjacent nodes.

    Node b has one incoming edge (KNOWS from a).  neighbors(b, direction='both')
    must include a in the result set.
    """
    db, a, b, c, d = graph
    # b has one incoming edge: KNOWS from a
    ns = db.neighbors(b, direction="both")
    ids = {n.id for n in ns}
    assert a.id in ids


def test_neighbors_invalid_direction(db):
    """neighbors() must raise ValueError for an unrecognised direction string.

    Passing an invalid direction such as 'sideways' must raise ValueError with
    a message that contains 'invalid direction'.
    """
    n = db.add_node([])
    with pytest.raises(ValueError, match="invalid direction"):
        db.neighbors(n, direction="sideways")


def test_neighbors_no_edges(db):
    """neighbors() on an isolated node must return an empty list for all directions.

    A node with no edges must produce an empty result regardless of whether the
    direction is 'out', 'in', or 'both'.
    """
    n = db.add_node([])
    assert db.neighbors(n) == []
    assert db.neighbors(n, direction="in") == []
    assert db.neighbors(n, direction="both") == []


# ── Node.properties / Edge.properties ────────────────────────────────────────


def test_node_properties_dict(db):
    """Node.properties must return a dict containing all properties stored at creation time.

    The dict must include all key/value pairs passed to add_node() with their
    correct types preserved.
    """
    n = db.add_node(["Person"], name="Alice", age=30, active=True)
    props = n.properties
    assert isinstance(props, dict)
    assert props["name"] == "Alice"
    assert props["age"] == 30
    assert props["active"] is True


def test_node_properties_empty(db):
    """Node.properties must return an empty dict when no properties were stored.

    A node created with no keyword arguments must have an empty properties dict,
    not None or a missing attribute.
    """
    n = db.add_node([])
    assert n.properties == {}


def test_edge_properties_dict(db):
    """Edge.properties must return a dict containing all properties stored at creation time.

    The dict must include all key/value pairs passed to add_edge() with their
    correct types preserved.
    """
    a = db.add_node([])
    b = db.add_node([])
    e = db.add_edge(a, "KNOWS", b, since=2020, weight=0.9)
    props = e.properties
    assert isinstance(props, dict)
    assert props["since"] == 2020
    assert abs(props["weight"] - 0.9) < 1e-9


def test_edge_properties_empty(db):
    """Edge.properties must return an empty dict when no properties were stored.

    An edge created without keyword arguments must have an empty properties dict.
    """
    a = db.add_node([])
    b = db.add_node([])
    e = db.add_edge(a, "L", b)
    assert e.properties == {}


def test_get_node_properties_dict(db):
    """The .properties accessor must work on a Node retrieved via get_node().

    Nodes returned by get_node() must expose the same .properties dict as nodes
    returned by add_node().
    """
    n = db.add_node(["X"], val=42)
    fetched = db.get_node(n.id)
    assert fetched.properties["val"] == 42


# ── GraphDBError exception hierarchy ─────────────────────────────────────────


def test_graphdb_error_is_base(db):
    """NodeNotFoundError must be catchable as GraphDBError (the base exception type).

    Any code that catches GraphDBError must implicitly handle all of its
    specialised subclasses, including NodeNotFoundError.
    """
    with pytest.raises(liel.GraphDBError):
        db.delete_node(9999)


def test_node_not_found_is_graphdb_error(db):
    """NodeNotFoundError must be a subclass of GraphDBError."""
    assert issubclass(liel.NodeNotFoundError, liel.GraphDBError)


def test_edge_not_found_is_graphdb_error(db):
    """EdgeNotFoundError must be a subclass of GraphDBError."""
    assert issubclass(liel.EdgeNotFoundError, liel.GraphDBError)


def test_corrupted_file_is_graphdb_error():
    """CorruptedFileError must be a subclass of GraphDBError."""
    assert issubclass(liel.CorruptedFileError, liel.GraphDBError)


def test_transaction_error_is_graphdb_error():
    """TransactionError must be a subclass of GraphDBError."""
    assert issubclass(liel.TransactionError, liel.GraphDBError)


# ── with db.transaction() ─────────────────────────────────────────────────────


def test_transaction_auto_commit(tmp_path):
    """Exiting a with db.transaction() block normally must auto-commit all changes.

    Any writes made inside the transaction block must be persisted to disk when
    the block exits without an exception, and must survive a close/re-open cycle.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        with db.transaction():
            db.add_node(["Person"], name="Alice")
        # Auto-committed by the transaction context manager — data must persist.

    with liel.open(path) as db:
        assert db.node_count() == 1
        assert db.get_node(1)["name"] == "Alice"


def test_transaction_auto_rollback_on_exception(tmp_path):
    """An exception inside with db.transaction() must trigger an automatic rollback.

    Any writes made inside the failed transaction block must be discarded.
    The database must remain in the state it was in before the block was entered.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        db.add_node(["Person"], name="Before")
        db.commit()

        try:
            with db.transaction():
                db.add_node(["Person"], name="During")
                raise ValueError("simulated error")
        except ValueError:
            pass

        # The node added inside the failed transaction must have been rolled back.
        assert db.node_count() == 1

    with liel.open(path) as db:
        assert db.node_count() == 1


def test_transaction_exception_propagates(db):
    """Exceptions raised inside with db.transaction() must not be suppressed.

    The transaction context manager must re-raise any exception that propagates
    out of the block, even after performing the rollback.
    """
    with pytest.raises(RuntimeError, match="boom"):
        with db.transaction():
            raise RuntimeError("boom")


def test_transaction_context_manager_returns_self(db):
    """with db.transaction() as txn must bind a non-None transaction object.

    The __enter__ method must return a truthy value so that code using
    ``as txn`` has a usable reference (though the exact type is not prescribed).
    """
    with db.transaction() as txn:
        assert txn is not None


def test_nested_operations_in_transaction(db):
    """Multiple writes inside a single transaction must all be committed atomically.

    Creating two nodes and one edge within a single transaction block must result
    in all three objects being present after the block exits successfully.
    """
    with db.transaction():
        a = db.add_node(["A"], x=1)
        b = db.add_node(["B"], y=2)
        db.add_edge(a, "LINK", b)

    assert db.node_count() == 2
    assert db.edge_count() == 1


def test_nested_transaction_raises_transaction_error(db):
    """Re-entering ``db.transaction()`` while one is already active must raise
    :class:`TransactionError` at the inner ``__enter__`` (product-tradeoffs §5.5).

    Nesting was rejected in favour of (i) "forbid re-entry" because it lets
    caller bugs surface immediately instead of silently swallowing the
    inner scope's commit.
    """
    with db.transaction():
        with pytest.raises(liel.TransactionError, match="already active"):
            with db.transaction():
                # Should never reach here.
                db.add_node(["NeverWritten"])


def test_consecutive_transactions_work_after_commit(db):
    """The flag must clear on commit — back-to-back ``with`` blocks must both
    succeed without raising :class:`TransactionError`.
    """
    with db.transaction():
        db.add_node(["First"], n=1)

    with db.transaction():
        db.add_node(["Second"], n=2)

    assert db.node_count() == 2


def test_consecutive_transactions_work_after_rollback(db):
    """The flag must clear on rollback too, even when the rollback was
    triggered by an exception escaping the ``with`` block."""
    try:
        with db.transaction():
            db.add_node(["WillBeRolledBack"])
            raise RuntimeError("intentional rollback trigger")
    except RuntimeError:
        pass

    # Subsequent transactions are unaffected.
    with db.transaction():
        db.add_node(["AfterRollback"])

    assert db.node_count() == 1


def test_unentered_transaction_does_not_block_future_calls(db):
    """Constructing a transaction without entering its ``with`` block must
    not leave the explicit-transaction flag stuck on True."""
    _stray = db.transaction()  # never entered, never used
    del _stray  # explicit GC hint; the flag must already be unset

    # If the flag had been set in `db.transaction()` instead of in
    # `__enter__`, this next block would raise TransactionError.
    with db.transaction():
        db.add_node(["AfterStray"])


def test_vacuum_inside_transaction_raises_transaction_error(db):
    """Calling ``db.vacuum()`` inside an explicit transaction must fail
    with :class:`TransactionError` so the user does not silently flush
    work they meant to keep transactional (product-tradeoffs §5.5/§5.6).

    The rejected `vacuum()` must NOT clear the transaction-active flag,
    so the surrounding `with` block is still in control of the commit.
    """
    with db.transaction():
        db.add_node(["StillInsideTx"])
        with pytest.raises(liel.TransactionError, match="explicit transaction"):
            db.vacuum()
        # The transaction continues normally; commit happens at __exit__.
        db.add_node(["AlsoCommittedTogether"])

    # Both nodes were committed together by the surrounding transaction;
    # the rejected vacuum did no work.
    assert db.node_count() == 2


# ── begin() ───────────────────────────────────────────────────────────────────


def test_begin_is_no_op(db):
    """begin() called when a transaction is already active must behave as a no-op.

    Calling begin() explicitly on a freshly opened database (which already has
    an implicit transaction) must not interfere with subsequent writes or commits.
    """
    db.begin()
    n = db.add_node([], name="Alice")
    db.commit()
    assert db.get_node(n.id)["name"] == "Alice"

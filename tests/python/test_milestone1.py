import liel


def test_open_creates_file(tmp_path):
    """Opening a new path should create a .liel file on disk.

    After calling ``liel.open()`` on a path that does not yet exist and then
    closing the handle, the file must be present in the filesystem.
    """
    db = liel.open(str(tmp_path / "test.liel"))
    db.close()
    assert (tmp_path / "test.liel").exists()


def test_open_memory():
    """The special path \":memory:\" must open without error and close cleanly.

    In-memory databases are never written to disk and exist only for the
    lifetime of the GraphDB handle.
    """
    db = liel.open(":memory:")
    db.close()


def test_context_manager(tmp_path):
    """GraphDB must work as a context manager and close without raising.

    Using ``with liel.open(...) as db`` should give a valid database handle
    inside the block and close the file cleanly on exit.
    """
    with liel.open(str(tmp_path / "test.liel")) as db:
        db.add_node(["X"])
    # The with block must exit without raising an exception.


def test_add_and_get_node(db):
    """add_node() must return a Node with ID 1, and get_node() must retrieve it.

    The first node added to an empty database should receive ID 1.  All
    properties and labels passed to add_node() must be readable back via
    get_node() using both attribute access and item access.
    """
    node = db.add_node(["Person"], name="Alice", age=30)
    assert node.id == 1
    fetched = db.get_node(node.id)
    assert fetched["name"] == "Alice"
    assert fetched["age"] == 30
    assert "Person" in fetched.labels


def test_node_id_increments(db):
    """Each successive add_node() call must assign a strictly incrementing ID.

    The second node's ID must be exactly one greater than the first node's ID,
    confirming that IDs are assigned sequentially from 1.
    """
    a = db.add_node([])
    b = db.add_node([])
    assert b.id == a.id + 1


def test_get_nonexistent_node(db):
    """get_node() must return None for an ID that has never been assigned.

    Requesting a node with an ID that does not exist (e.g. 9999) should
    return None rather than raise an exception.
    """
    assert db.get_node(9999) is None


def test_add_and_get_edge(db):
    """add_edge() must create a directed edge retrievable by get_edge().

    The edge returned by get_edge() must carry the same label and properties
    that were supplied to add_edge().
    """
    alice = db.add_node(["Person"])
    bob = db.add_node(["Person"])
    edge = db.add_edge(alice, "KNOWS", bob, since=2024)
    fetched = db.get_edge(edge.id)
    assert fetched.label == "KNOWS"
    assert fetched["since"] == 2024


def test_out_edges(db):
    """out_edges() must return all edges originating from the given node.

    When two edges are added from the same source node, out_edges() on that
    node must return exactly two edges.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    carol = db.add_node([])
    db.add_edge(alice, "KNOWS", bob)
    db.add_edge(alice, "KNOWS", carol)
    edges = db.out_edges(alice)
    assert len(edges) == 2


def test_out_edges_label_filter(db):
    """out_edges(label=...) must filter by edge label.

    When a node has edges with different labels, passing label="KNOWS" to
    out_edges() must return only the KNOWS edges and exclude all others.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    db.add_edge(alice, "KNOWS", bob)
    db.add_edge(alice, "LIKES", bob)
    knows = db.out_edges(alice, label="KNOWS")
    assert len(knows) == 1
    assert knows[0].label == "KNOWS"


def test_neighbors(db):
    """neighbors() must return the set of nodes reachable via outgoing edges.

    With two outgoing KNOWS edges from alice, neighbors(alice, "KNOWS") must
    return the two target nodes.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    carol = db.add_node([])
    db.add_edge(alice, "KNOWS", bob)
    db.add_edge(alice, "KNOWS", carol)
    neighbors = db.neighbors(alice, "KNOWS")
    assert len(neighbors) == 2


# NOTE: The legacy milestone-1 tests `test_commit_persists` and
# `test_no_commit_no_persist` were removed in favour of their stricter
# equivalents `test_data_survives_reopen` and
# `test_uncommitted_data_lost_on_reopen` in test_persistence.py, which
# additionally verify edges, neighbour lookups, and the exact pre-commit
# state. Keeping two slightly different copies of the same assertion was
# confusing and made it unclear which file was the source of truth for
# persistence behaviour.

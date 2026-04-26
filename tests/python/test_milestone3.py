def test_update_node(db):
    """update_node() must replace existing node properties with the new values.

    After calling update_node() with a new value for an existing property,
    get_node() must return the updated value rather than the original one.
    """
    alice = db.add_node(["Person"], age=30)
    db.update_node(alice.id, age=31)
    assert db.get_node(alice.id)["age"] == 31


def test_update_edge(db):
    """update_edge() must replace existing edge properties with the new values.

    After calling update_edge() with a new weight value, get_edge() must
    return the updated weight rather than the original one.
    """
    a = db.add_node([])
    b = db.add_node([])
    edge = db.add_edge(a, "L", b, weight=1.0)
    db.update_edge(edge.id, weight=2.0)
    assert db.get_edge(edge.id)["weight"] == 2.0


def test_merge_edge_creates_if_not_exists(db):
    """merge_edge() must create a new edge when no matching edge exists.

    Calling merge_edge() between two nodes for the first time must result in
    exactly one edge being present in the database.
    """
    a = db.add_node([])
    b = db.add_node([])
    db.merge_edge(a, "KNOWS", b)
    assert db.edge_count() == 1


def test_merge_edge_returns_existing(db):
    """merge_edge() must return the same edge when called twice with identical arguments.

    If an edge with the same (from, label, to) and no properties already exists,
    a second merge_edge() call must return that edge rather than creating a new one.
    The edge count must remain 1.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b)
    e2 = db.merge_edge(a, "KNOWS", b)
    assert e1.id == e2.id
    assert db.edge_count() == 1


def test_shortest_path(db):
    """shortest_path() must return the full ordered node list for an existing path.

    Given the chain a -> b -> c, the shortest path from a to c must contain
    exactly three nodes: [a, b, c].
    """
    a = db.add_node([])
    b = db.add_node([])
    c = db.add_node([])
    db.add_edge(a, "L", b)
    db.add_edge(b, "L", c)
    path = db.shortest_path(a, c)
    assert path is not None
    assert len(path) == 3


def test_shortest_path_none(db):
    """shortest_path() must return None when no path connects the two nodes.

    When two nodes exist but have no edges between them, shortest_path() must
    return None rather than an empty list or raise an exception.
    """
    a = db.add_node([])
    b = db.add_node([])
    assert db.shortest_path(a, b) is None


def test_clear(db):
    """clear() must remove all nodes and edges from the database.

    After calling clear() on a database that contains nodes, both node_count()
    and edge_count() must return 0.
    """
    db.add_node([])
    db.add_node([])
    db.clear()
    assert db.node_count() == 0
    assert db.edge_count() == 0


def test_clear_resets_id(db):
    """clear() must reset the ID counter so the next node receives ID 1.

    After clearing the database, the first node added must have ID 1, confirming
    that the ID sequence was fully reset rather than continued from the previous
    highest ID.
    """
    db.add_node([])
    db.clear()
    node = db.add_node([])
    assert node.id == 1  # ID counter is reset to 1 after clear()


def test_info(db):
    """info() must return a dict containing the required metadata keys.

    The dictionary returned by info() must at minimum contain the keys
    "version", "node_count", "edge_count", and "file_size".
    """
    info = db.info()
    assert "version" in info
    assert "node_count" in info
    assert "edge_count" in info
    assert "file_size" in info


def test_repair_adjacency_exists_and_returns_counts(db):
    """repair_adjacency() must be exposed and return numeric repair counters."""
    a = db.add_node([])
    b = db.add_node([])
    db.add_edge(a, "R", b)
    db.commit()

    report = db.repair_adjacency()
    assert isinstance(report, dict)
    assert "nodes_rewritten" in report
    assert "edges_relinked" in report
    assert report["nodes_rewritten"] >= 2
    assert report["edges_relinked"] >= 1

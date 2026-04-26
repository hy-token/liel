import liel


def test_data_survives_reopen(tmp_path):
    """Data written and committed must be fully readable after closing and reopening the file.

    This test verifies that nodes, edges, properties, and adjacency relationships
    are correctly persisted to disk and can be retrieved in a fresh database
    session opened against the same file path.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Person"], name="Bob")
        db.add_edge(alice, "KNOWS", bob)
        db.commit()

    with liel.open(path) as db:
        assert db.node_count() == 2
        assert db.edge_count() == 1
        node = db.get_node(alice.id)
        assert node["name"] == "Alice"
        neighbors = db.neighbors(alice, "KNOWS")
        assert neighbors[0]["name"] == "Bob"


def test_uncommitted_data_lost_on_reopen(tmp_path):
    """Uncommitted data must not appear after closing and reopening the database.

    If the database is closed without calling commit(), all in-flight changes
    must be discarded.  On the next open, the database must be in the same
    state it was before the uncommitted session began.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        db.add_node(["Person"], name="Alice")
        # Deliberately omitting commit() — changes must not be persisted.

    with liel.open(path) as db:
        assert db.node_count() == 0


def test_multiple_commits(tmp_path):
    """Multiple sequential commits within one session must each persist their changes.

    After two separate commit() calls in a single session, both nodes created
    before the respective commits must survive a re-open.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        db.add_node([], name="A")
        db.commit()
        db.add_node([], name="B")
        db.commit()

    with liel.open(path) as db:
        assert db.node_count() == 2


def test_rollback(db):
    """rollback() must discard all changes made since the last commit.

    After adding a node and then calling rollback(), the database must appear
    empty — as if add_node() was never called.
    """
    db.add_node([], name="A")
    db.rollback()
    assert db.node_count() == 0


def test_large_data(tmp_path):
    """Writing and reading 10,000 nodes must complete without error or data loss.

    This test exercises the storage layer at scale: all 10,000 nodes are written
    in a single session, committed to disk, and then verified by node_count()
    after a fresh re-open.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        for i in range(10000):
            db.add_node(["X"], index=i)
        db.commit()

    with liel.open(path) as db:
        assert db.node_count() == 10000

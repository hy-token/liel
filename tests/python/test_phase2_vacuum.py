"""Phase 2: tests for the vacuum() operation.

vacuum() compacts the property storage section of a liel database file by
rewriting all live property blobs into fresh property extents and dropping any
orphaned blobs left behind by previous delete_node(), delete_edge(), or
update_node() / update_edge() calls.  Under the extent-chained allocator the
on-disk file size does not shrink as a result — vacuum reclaims *logical*
space (future appends reuse the compacted region) rather than physical bytes.
These tests therefore check that live data survives vacuum unchanged and that
the node/edge counts, labels, and properties read back consistently.
"""

import liel


def test_vacuum_empty_db(db):
    """vacuum() on an empty database must complete without raising any error.

    Calling vacuum() when no nodes or edges have ever been added must be a
    harmless no-op — the database must remain valid and openable afterward.
    """
    db.vacuum()


def test_vacuum_no_delete(db):
    """vacuum() on a database with no deletions must preserve all live data.

    When vacuum() is called after committing nodes and edges but without
    deleting any of them, all data must remain intact and fully queryable.
    """
    a = db.add_node(["Person"], name="Alice")
    b = db.add_node(["Person"], name="Bob")
    db.add_edge(a, "KNOWS", b, since=2020)
    db.commit()
    db.vacuum()
    assert db.node_count() == 2
    assert db.edge_count() == 1
    assert db.get_node(a.id)["name"] == "Alice"
    assert db.get_node(b.id)["name"] == "Bob"
    neighbors = db.neighbors(a, "KNOWS")
    assert len(neighbors) == 1
    assert neighbors[0]["name"] == "Bob"


def test_vacuum_reclaims_space(tmp_path):
    """vacuum() must keep the 20 surviving nodes intact after a mass delete.

    200 nodes with 500-byte string properties are created and committed.  After
    deleting 90% of them (180 nodes) and calling vacuum(), re-opening the file
    must still show exactly the 20 survivors with their properties readable.
    File size is not asserted: the extent-chained allocator rewrites live blobs
    into fresh extents, so physical bytes may remain flat even though the live
    set shrank dramatically.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        nodes = [db.add_node([], data="x" * 500) for _ in range(200)]
        db.commit()

        for n in nodes[:180]:  # Delete 90% of nodes to create orphaned property blobs
            db.delete_node(n.id)
        db.commit()
        db.vacuum()

    with liel.open(path) as db:
        assert db.node_count() == 20
        # All survivors keep the original 500-byte payload.
        for n in nodes[180:]:
            assert db.get_node(n.id)["data"] == "x" * 500


def test_vacuum_preserves_active_data(tmp_path):
    """vacuum() must not corrupt any live node or edge data.

    A mixed scenario is used: two permanent nodes and one edge are created
    alongside a temporary node.  After deleting the temporary node and running
    vacuum(), all properties and adjacency relationships on the remaining nodes
    and edge must be exactly as stored before vacuum was called.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        a = db.add_node(["Person"], name="Alice", age=30)
        b = db.add_node(["Person"], name="Bob", age=25)
        e = db.add_edge(a, "KNOWS", b, since=2020)
        c = db.add_node(["Temp"], dummy=True)  # Will be deleted before vacuum
        db.commit()
        db.delete_node(c.id)
        db.commit()
        db.vacuum()

    with liel.open(path) as db:
        assert db.node_count() == 2
        assert db.edge_count() == 1

        node_a = db.get_node(a.id)
        assert node_a["name"] == "Alice"
        assert node_a["age"] == 30
        assert "Person" in node_a.labels

        node_b = db.get_node(b.id)
        assert node_b["name"] == "Bob"

        edge = db.get_edge(e.id)
        assert edge["since"] == 2020
        assert edge.label == "KNOWS"

        neighbors = db.neighbors(a, "KNOWS")
        assert len(neighbors) == 1
        assert neighbors[0]["name"] == "Bob"


def test_vacuum_after_updates(tmp_path):
    """vacuum() must preserve the latest property value after repeated updates.

    Each call to update_node() replaces the property blob for a node, leaving
    the old blob orphaned in the property extents.  After 20 successive
    updates, vacuum() must rewrite live properties into fresh extents and the
    final value (version=19) must survive re-open.  File size is not asserted:
    the extent-chained allocator may keep the physical file the same size even
    though the logical live set is tiny — future appends will reuse the
    compacted region.
    """
    path = str(tmp_path / "test.liel")
    with liel.open(path) as db:
        n = db.add_node([], data="x" * 500)
        db.commit()
        # Each update orphans the previous property blob, causing logical bloat.
        for i in range(20):
            db.update_node(n.id, data="x" * 500, version=i)
        db.commit()
        db.vacuum()

    with liel.open(path) as db:
        # The last update set version=19 — this value must survive vacuum.
        assert db.get_node(n.id)["version"] == 19
        assert db.get_node(n.id)["data"] == "x" * 500

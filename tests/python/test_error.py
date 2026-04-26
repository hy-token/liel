import pytest

import liel


def test_node_not_found_on_delete(db):
    """delete_node() must raise NodeNotFoundError when the node ID does not exist.

    Attempting to delete a node with an ID that was never assigned (e.g. 9999)
    must raise NodeNotFoundError rather than silently doing nothing.
    """
    with pytest.raises(liel.NodeNotFoundError):
        db.delete_node(9999)


def test_edge_not_found_on_delete(db):
    """delete_edge() must raise EdgeNotFoundError when the edge ID does not exist.

    Attempting to delete an edge with an ID that was never assigned (e.g. 9999)
    must raise EdgeNotFoundError rather than silently doing nothing.
    """
    with pytest.raises(liel.EdgeNotFoundError):
        db.delete_edge(9999)


def test_add_edge_invalid_from_node(db):
    """add_edge() must raise NodeNotFoundError when the source node does not exist.

    If the from_node ID supplied to add_edge() does not refer to an existing
    node, NodeNotFoundError must be raised before any edge is written.
    """
    bob = db.add_node([])
    with pytest.raises(liel.NodeNotFoundError):
        db.add_edge(9999, "KNOWS", bob)


def test_add_edge_invalid_to_node(db):
    """add_edge() must raise NodeNotFoundError when the target node does not exist.

    If the to_node ID supplied to add_edge() does not refer to an existing node,
    NodeNotFoundError must be raised before any edge is written.
    """
    alice = db.add_node([])
    with pytest.raises(liel.NodeNotFoundError):
        db.add_edge(alice, "KNOWS", 9999)


def test_open_invalid_file(tmp_path):
    """liel.open() must raise CorruptedFileError when the file is not a valid liel database.

    Opening a file that does not begin with the LIEL magic bytes must raise
    CorruptedFileError immediately rather than returning a partially
    initialised handle.
    """
    invalid_path = tmp_path / "invalid.liel"
    invalid_path.write_bytes(b"not-a-liel-file")
    with pytest.raises(liel.CorruptedFileError):
        liel.open(str(invalid_path))


def test_update_node_missing_id_raises_node_not_found(db):
    """update_node() must raise NodeNotFoundError when the node id does not exist.

    update_*() shares the same not-found contract as delete_*(): no silent
    create-or-update behaviour. Catching NodeNotFoundError lets the caller
    decide whether to fall back to add_node().
    """
    with pytest.raises(liel.NodeNotFoundError):
        db.update_node(9999, name="Ghost")


def test_update_edge_missing_id_raises_edge_not_found(db):
    """update_edge() must raise EdgeNotFoundError when the edge id does not exist."""
    with pytest.raises(liel.EdgeNotFoundError):
        db.update_edge(9999, weight=1.0)


def test_capacity_exceeded_error_is_exposed():
    """CapacityExceededError must be importable from `liel` and subclass GraphDBError.

    Triggering this exception at runtime requires allocating beyond the
    file-format's per-kind extent cap (MAX_EXTENTS_PER_KIND * NODES_PER_EXTENT,
    on the order of 10**11 nodes) or constructing a corrupt header, neither of
    which is reasonable inside a unit test. We therefore validate the symbol
    is reachable and correctly typed so user code can write
    `except liel.CapacityExceededError:` with confidence.
    """
    assert hasattr(liel, "CapacityExceededError")
    assert issubclass(liel.CapacityExceededError, liel.GraphDBError)
    assert issubclass(liel.CapacityExceededError, Exception)
    assert liel.CapacityExceededError is not liel.GraphDBError

"""Phase 2: tests for the merge_edge() property-matching semantics.

merge_edge() is designed for idempotent graph construction.  It returns an
existing edge when one is found with matching (from_node, label, to_node,
properties), and creates a new edge when no such match exists.  These tests
verify that the matching logic correctly handles identical properties, differing
properties, partially mismatched properties, and differing labels.
"""


def test_merge_edge_no_props_unchanged(db):
    """merge_edge() with no properties must return the same edge on repeated calls.

    This confirms that the Phase 1 no-property merge behaviour is preserved:
    two calls with the same (from, label, to) and no properties must yield the
    same edge ID and keep the edge count at 1.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b)
    e2 = db.merge_edge(a, "KNOWS", b)
    assert e1.id == e2.id
    assert db.edge_count() == 1


def test_merge_edge_same_props_returns_existing(db):
    """merge_edge() with identical properties must return the existing edge.

    When an edge already exists with the same label and exactly matching
    property values, a second merge_edge() call must return that same edge
    rather than creating a duplicate.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b, since=2020)
    e2 = db.merge_edge(a, "KNOWS", b, since=2020)
    assert e1.id == e2.id
    assert db.edge_count() == 1


def test_merge_edge_different_props_creates_new(db):
    """merge_edge() with different property values must create a new edge.

    When the supplied properties differ from those on an existing edge (even if
    the label and endpoints match), a new edge must be created, resulting in an
    edge count of 2.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b, since=2020)
    e2 = db.merge_edge(a, "KNOWS", b, since=2021)
    assert e1.id != e2.id
    assert db.edge_count() == 2


def test_merge_edge_partial_prop_mismatch(db):
    """merge_edge() must create a new edge even when only one property value differs.

    A partial property mismatch (same keys, but one value different) is
    sufficient to consider the edges distinct, so a new edge must be created.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b, since=2020, weight=1.0)
    e2 = db.merge_edge(a, "KNOWS", b, since=2020, weight=2.0)
    assert e1.id != e2.id


def test_merge_edge_props_vs_no_props(db):
    """merge_edge() must treat an edge with properties as distinct from one without.

    An edge with no properties and an edge with at least one property are
    considered different even when the label and endpoints match, so two edges
    must exist after calling merge_edge() with and without properties.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b)
    e2 = db.merge_edge(a, "KNOWS", b, since=2020)
    assert e1.id != e2.id
    assert db.edge_count() == 2


def test_merge_edge_different_labels_creates_new(db):
    """merge_edge() must always create a new edge when the label differs.

    Even if the endpoints and properties are identical, two edges with different
    labels are distinct.  A second call with a different label must create a
    new edge rather than returning the first.
    """
    a = db.add_node([])
    b = db.add_node([])
    e1 = db.merge_edge(a, "KNOWS", b)
    e2 = db.merge_edge(a, "LIKES", b)
    assert e1.id != e2.id

"""Smoke tests for bulk record helpers (dict projections and graph metrics).

These paths live in ``src/python/types.rs`` and are easy to break during refactors;
they are intentionally narrow: shape of dicts, degree tallies, and ``edges_between``
filter semantics — not full CRUD (covered elsewhere).
"""

import pytest

import liel


@pytest.fixture
def db_triangle():
    """Three nodes in a triangle: a->b, b->c, c->a (one directed cycle)."""
    d = liel.open(":memory:")
    a = d.add_node(["N"], name="a")
    b = d.add_node(["N"], name="b")
    c = d.add_node(["N"], name="c")
    d.add_edge(a, "E", b)
    d.add_edge(b, "E", c)
    d.add_edge(c, "E", a)
    d.commit()
    yield d, a, b, c
    d.close()


def test_all_nodes_as_records_shape(db_triangle):
    db, a, b, c = db_triangle
    rows = db.all_nodes_as_records()
    assert len(rows) == 3
    ids = {r["id"] for r in rows}
    assert ids == {a.id, b.id, c.id}
    by_id = {r["id"]: r for r in rows}
    assert "N" in by_id[a.id]["labels"]
    assert by_id[a.id]["name"] == "a"


def test_all_edges_as_records_shape(db_triangle):
    db, a, b, c = db_triangle
    rows = db.all_edges_as_records()
    assert len(rows) == 3
    for r in rows:
        assert set(r.keys()) >= {"id", "label", "from_node", "to_node"}
        assert r["label"] == "E"
    endpoints = {(r["from_node"], r["to_node"]) for r in rows}
    assert (a.id, b.id) in endpoints
    assert (b.id, c.id) in endpoints
    assert (c.id, a.id) in endpoints


def test_degree_stats_matches_edge_endpoints(db_triangle):
    db, a, b, c = db_triangle
    stats = db.degree_stats()
    # Triangle: each node has out_degree 1 and in_degree 1
    assert stats[a.id] == (1, 1)
    assert stats[b.id] == (1, 1)
    assert stats[c.id] == (1, 1)


def test_edges_between_filters_to_subgraph(db_triangle):
    db, a, b, _c = db_triangle
    # Only the a-b edge has both endpoints in {a, b}
    rows = db.edges_between({a.id, b.id})
    assert len(rows) == 1
    assert rows[0]["from_node"] == a.id
    assert rows[0]["to_node"] == b.id

"""Phase 2: QueryBuilder edge queries (``db.edges()``).

Uses ``populated_query_graph`` from ``conftest.py`` (same graph as node query tests).
"""


def test_edges_fetch_all(populated_query_graph):
    """db.edges().fetch() returns all edges (2 KNOWS + 1 LIKES = 3)."""
    db, *_ = populated_query_graph
    edges = db.edges().fetch()
    assert len(edges) == 3


def test_edges_label_filter(populated_query_graph):
    """db.edges().label('KNOWS') returns only KNOWS edges."""
    db, *_ = populated_query_graph
    edges = db.edges().label("KNOWS").fetch()
    assert len(edges) == 2
    assert all(e.label == "KNOWS" for e in edges)


def test_edges_where(populated_query_graph):
    """db.edges().where_() filters by a Python predicate."""
    db, *_ = populated_query_graph
    edges = db.edges().label("KNOWS").where_(lambda e: e["since"] >= 2020).fetch()
    assert len(edges) == 1
    assert edges[0]["since"] == 2022


def test_edges_count(populated_query_graph):
    db, *_ = populated_query_graph
    assert db.edges().label("KNOWS").count() == 2
    assert db.edges().count() == 3


def test_edges_exists(populated_query_graph):
    db, *_ = populated_query_graph
    assert db.edges().label("LIKES").exists()
    assert not db.edges().label("HATES").exists()


def test_edges_exists_short_circuits_predicate(populated_query_graph):
    """edges().exists() must stop invoking the predicate after the first match."""
    db, *_ = populated_query_graph
    call_count = 0

    def count_calls(_):
        nonlocal call_count
        call_count += 1
        return True

    found = db.edges().label("KNOWS").where_(count_calls).exists()
    assert found
    assert call_count == 1, f"predicate invoked {call_count} times, expected exactly 1"


def test_edges_skip_limit(populated_query_graph):
    db, *_ = populated_query_graph
    edges = db.edges().label("KNOWS").skip(1).limit(1).fetch()
    assert len(edges) == 1

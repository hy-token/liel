"""Phase 2: QueryBuilder node queries (``db.nodes()``).

Chained methods return new query objects; execution happens on fetch(), count(),
or exists().  Shared graph fixture: ``populated_query_graph`` in ``conftest.py``.
"""


def test_nodes_fetch_all(populated_query_graph):
    """db.nodes().fetch() returns every node regardless of label (4 nodes)."""
    db, *_ = populated_query_graph
    nodes = db.nodes().fetch()
    assert len(nodes) == 4


def test_nodes_label_filter(populated_query_graph):
    """db.nodes().label('Person').fetch() returns only Person nodes (3)."""
    db, *_ = populated_query_graph
    nodes = db.nodes().label("Person").fetch()
    assert len(nodes) == 3
    assert all("Person" in n.labels for n in nodes)


def test_nodes_where(populated_query_graph):
    """db.nodes().where_() applies a Python predicate after label filter."""
    db, *_ = populated_query_graph
    nodes = db.nodes().label("Person").where_(lambda n: n["age"] >= 30).fetch()
    names = {n["name"] for n in nodes}
    assert names == {"Alice", "Carol"}


def test_nodes_limit(populated_query_graph):
    """db.nodes().limit(n) caps result size."""
    db, *_ = populated_query_graph
    nodes = db.nodes().label("Person").limit(2).fetch()
    assert len(nodes) == 2


def test_nodes_skip(populated_query_graph):
    """db.nodes().skip(n) skips the first n matching nodes."""
    db, *_ = populated_query_graph
    all_persons = db.nodes().label("Person").fetch()
    skipped = db.nodes().label("Person").skip(2).fetch()
    assert len(skipped) == len(all_persons) - 2


def test_nodes_skip_limit(populated_query_graph):
    """skip() and limit() combine for offset-style pagination."""
    db, *_ = populated_query_graph
    nodes = db.nodes().label("Person").skip(1).limit(1).fetch()
    assert len(nodes) == 1


def test_nodes_count(populated_query_graph):
    """db.nodes().count() matches fetch length for the same filters."""
    db, *_ = populated_query_graph
    assert db.nodes().label("Person").count() == 3
    assert db.nodes().count() == 4


def test_nodes_exists_true(populated_query_graph):
    db, *_ = populated_query_graph
    assert db.nodes().label("Person").where_(lambda n: n["name"] == "Alice").exists()


def test_nodes_exists_false(populated_query_graph):
    db, *_ = populated_query_graph
    assert not db.nodes().label("Person").where_(lambda n: n["name"] == "Zara").exists()


def test_nodes_exists_short_circuits_predicate(populated_query_graph):
    """exists() must stop invoking the predicate after the first match.

    Guards against regressing to the old implementation that collected every
    matching node before testing emptiness.
    """
    db, *_ = populated_query_graph
    call_count = 0

    def count_calls(n):
        nonlocal call_count
        call_count += 1
        return True

    found = db.nodes().label("Person").where_(count_calls).exists()
    assert found
    assert call_count == 1, f"predicate invoked {call_count} times, expected exactly 1"


def test_nodes_empty_db(mem_db):
    """On an empty DB, fetch is [], count is 0, exists is False."""
    assert mem_db.nodes().fetch() == []
    assert mem_db.nodes().count() == 0
    assert not mem_db.nodes().exists()


def test_query_chaining_is_immutable(populated_query_graph):
    """Chained where_() must not mutate the base query object."""
    db, *_ = populated_query_graph
    base = db.nodes().label("Person")
    filtered = base.where_(lambda n: n["age"] > 25)
    assert base.count() == 3
    assert filtered.count() == 2

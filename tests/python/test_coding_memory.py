from __future__ import annotations

import liel
from liel.coding_memory import (
    find_file_node_id,
    link,
    record_bug,
    record_decision,
    record_file,
)


def test_record_file_dedupes_by_path():
    db = liel.open(":memory:")
    with db.transaction():
        a = record_file(db, "src/lib.rs", role="core")
        b = record_file(db, "src/lib.rs", role="ignored")
    assert a == b
    assert find_file_node_id(db, "src/lib.rs") == a
    db.close()


def test_record_decision_and_record_bug():
    db = liel.open(":memory:")
    with db.transaction():
        d = record_decision(db, "Use 4K pages", area="storage")
        t = record_bug(db, "Overflow", severity="S0")
    assert d != t
    n = db.get_node(d)
    assert n is not None
    assert "Decision" in n.labels
    assert n["title"] == "Use 4K pages"
    m = db.get_node(t)
    assert m is not None
    assert "Task" in m.labels
    assert m["task_kind"] == "bug"
    db.close()


def test_link_creates_edge():
    db = liel.open(":memory:")
    with db.transaction():
        f = record_file(db, "a.py")
        d = record_decision(db, "Refactor")
        eid = link(db, f, "RELATES_TO", d, note="context")
    edges = db.all_edges()
    assert len(edges) == 1
    assert edges[0].id == eid
    assert edges[0].label == "RELATES_TO"
    db.close()

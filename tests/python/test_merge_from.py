"""Tests for :meth:`GraphDB.merge_from` — combining two databases into one.

The Rust side already has unit tests for the core merge policies (see
``src/graph/merge.rs``).  These tests focus on the Python binding: keyword
arguments, :class:`MergeReport` fields, error types, and the file ↔ memory
combinations that the Python users will actually run.
"""

import pytest

import liel

# ── append (default) ─────────────────────────────────────────────────────────


def test_merge_from_default_appends_nodes_and_edges():
    dst = liel.open(":memory:")
    a = dst.add_node(["P"], name="DstA")
    b = dst.add_node(["P"], name="DstB")
    dst.add_edge(a, "E", b)

    src = liel.open(":memory:")
    x = src.add_node(["P"], name="SrcX")
    y = src.add_node(["P"], name="SrcY")
    src.add_edge(x, "E", y)

    report = dst.merge_from(src)

    assert dst.node_count() == 4
    assert dst.edge_count() == 2
    assert report.nodes_created == 2
    assert report.nodes_reused == 0
    assert report.edges_created == 1
    assert report.edges_reused == 0
    assert set(report.node_id_map.keys()) == {x.id, y.id}
    assert set(report.edge_id_map.keys()) == {1}  # only src edge id


def test_merge_from_remaps_colliding_ids(tmp_path):
    # Both databases start with NodeId(1); the merged edge must connect the
    # remapped pair, not the destination's original NodeId(1).
    dst_path = tmp_path / "dst.liel"
    src_path = tmp_path / "src.liel"

    with liel.open(str(dst_path)) as dst, liel.open(str(src_path)) as src:
        original = dst.add_node([], label="dst-only")
        dst.commit()

        s1 = src.add_node([], tag="S1")
        s2 = src.add_node([], tag="S2")
        edge = src.add_edge(s1, "L", s2)
        src.commit()

        report = dst.merge_from(src)
        dst.commit()

        assert s1.id == 1 and s2.id == 2
        new_edge_id = report.edge_id_map[edge.id]
        e = dst.get_edge(new_edge_id)
        assert e.from_node != original.id
        assert e.from_node == report.node_id_map[s1.id]
        assert e.to_node == report.node_id_map[s2.id]


# ── node_key identity ────────────────────────────────────────────────────────


def test_merge_from_node_key_reuses_matching_nodes():
    dst = liel.open(":memory:")
    existing = dst.add_node(["User"], email="alice@example.com", name="Alice")

    src = liel.open(":memory:")
    src.add_node(["User"], email="alice@example.com", name="Alice (src)")
    src.add_node(["User"], email="bob@example.com", name="Bob")

    report = dst.merge_from(src, node_key=["email"])

    assert dst.node_count() == 2
    assert report.nodes_reused == 1
    assert report.nodes_created == 1
    # keep_dst default preserves the destination's name property.
    n = dst.get_node(existing.id)
    assert n["name"] == "Alice"


def test_merge_from_missing_node_key_raises_merge_error():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    src.add_node(["User"], name="no-email-here")

    with pytest.raises(liel.MergeError):
        dst.merge_from(src, node_key=["email"])


def test_merge_from_empty_node_key_raises_value_error():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    with pytest.raises(ValueError):
        dst.merge_from(src, node_key=[])


# ── edge_strategy ────────────────────────────────────────────────────────────


def test_merge_from_idempotent_is_stable_on_second_run():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    a = src.add_node([], tag="A")
    b = src.add_node([], tag="B")
    src.add_edge(a, "R", b)

    first = dst.merge_from(src, node_key=["tag"], edge_strategy="idempotent")
    assert first.nodes_created == 2
    assert first.edges_created == 1
    assert first.edges_reused == 0

    second = dst.merge_from(src, node_key=["tag"], edge_strategy="idempotent")
    assert second.nodes_reused == 2
    assert second.nodes_created == 0
    assert second.edges_created == 0
    assert second.edges_reused == 1

    # Database should still hold exactly the original 2 nodes and 1 edge.
    assert dst.node_count() == 2
    assert dst.edge_count() == 1


def test_merge_from_append_duplicates_edges_on_repeat():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    a = src.add_node([], tag="A")
    b = src.add_node([], tag="B")
    src.add_edge(a, "R", b)

    dst.merge_from(src, node_key=["tag"], edge_strategy="append")
    dst.merge_from(src, node_key=["tag"], edge_strategy="append")
    assert dst.node_count() == 2
    assert dst.edge_count() == 2


# ── on_node_conflict ─────────────────────────────────────────────────────────


def test_merge_from_overwrite_from_src_overlays_props():
    dst = liel.open(":memory:")
    existing = dst.add_node([], email="x@example.com", name="OLD")
    src = liel.open(":memory:")
    src.add_node([], email="x@example.com", name="NEW", age=42)

    dst.merge_from(
        src,
        node_key=["email"],
        on_node_conflict="overwrite_from_src",
    )
    n = dst.get_node(existing.id)
    assert n["name"] == "NEW"
    assert n["age"] == 42


def test_merge_from_merge_props_fills_only_missing_keys():
    dst = liel.open(":memory:")
    existing = dst.add_node([], email="x@example.com", name="DST")
    src = liel.open(":memory:")
    src.add_node([], email="x@example.com", name="SRC", age=7)

    dst.merge_from(src, node_key=["email"], on_node_conflict="merge_props")
    n = dst.get_node(existing.id)
    assert n["name"] == "DST"  # dst wins on collision
    assert n["age"] == 7  # src-only key filled in


def test_merge_from_keep_dst_is_default():
    dst = liel.open(":memory:")
    existing = dst.add_node([], email="x@example.com", name="ORIG")
    src = liel.open(":memory:")
    src.add_node([], email="x@example.com", name="CHANGED")

    dst.merge_from(src, node_key=["email"])
    n = dst.get_node(existing.id)
    assert n["name"] == "ORIG"


# ── validation ───────────────────────────────────────────────────────────────


def test_merge_from_same_db_raises():
    db = liel.open(":memory:")
    with pytest.raises(ValueError):
        db.merge_from(db)


def test_merge_from_bad_edge_strategy_raises():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    with pytest.raises(ValueError):
        dst.merge_from(src, edge_strategy="bogus")


def test_merge_from_bad_conflict_mode_raises():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    with pytest.raises(ValueError):
        dst.merge_from(src, on_node_conflict="nope")


# ── persistence (file ↔ file) ────────────────────────────────────────────────


def test_merge_from_file_to_file_persists_after_reopen(tmp_path):
    dst_path = tmp_path / "dst.liel"
    src_path = tmp_path / "src.liel"

    with liel.open(str(src_path)) as src:
        s1 = src.add_node(["P"], name="S1")
        s2 = src.add_node(["P"], name="S2")
        src.add_edge(s1, "L", s2)
        src.commit()

    with liel.open(str(dst_path)) as dst, liel.open(str(src_path)) as src:
        report = dst.merge_from(src)
        assert report.nodes_created == 2
        assert report.edges_created == 1
        dst.commit()

    with liel.open(str(dst_path)) as dst:
        assert dst.node_count() == 2
        assert dst.edge_count() == 1


def test_merge_report_repr_mentions_counters():
    dst = liel.open(":memory:")
    src = liel.open(":memory:")
    src.add_node([], tag="only")
    report = dst.merge_from(src)
    text = repr(report)
    assert "MergeReport" in text
    assert "nodes_created=1" in text

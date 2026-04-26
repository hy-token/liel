"""Tests for the MCP tool implementations (_do_* functions in server.py)."""

from __future__ import annotations

import json

import pytest

import liel
from liel.mcp.server import (
    _do_append,
    _do_explore,
    _do_inspect,
    _do_merge,
    _do_mermaid,
    _do_path,
    _do_query,
)


@pytest.fixture
def empty_db():
    db = liel.open(":memory:")
    yield db
    db.close()


@pytest.fixture
def simple_db():
    """Three Person nodes + one Company node, two KNOWS edges."""
    db = liel.open(":memory:")
    alice = db.add_node(["Person"], name="Alice", age=30)
    bob = db.add_node(["Person"], name="Bob", age=19)
    carol = db.add_node(["Person"], name="Carol", age=42)
    acme = db.add_node(["Company"], name="Acme")
    db.add_edge(alice, "KNOWS", bob)
    db.add_edge(alice, "KNOWS", carol)
    db.commit()
    yield db, alice, bob, carol, acme
    db.close()


class TestDoInspect:
    def test_empty_graph(self, empty_db):
        result = json.loads(_do_inspect(empty_db, ":memory:"))
        assert result["node_count"] == 0
        assert result["edge_count"] == 0
        assert result["node_labels"] == {}
        assert result["edge_labels"] == {}
        assert result["sample_nodes"] == []
        assert result["db_path"] == ":memory:"

    def test_populated_graph_counts(self, simple_db):
        db, *_ = simple_db
        result = json.loads(_do_inspect(db, "/fake.liel"))
        assert result["node_count"] == 4
        assert result["edge_count"] == 2
        assert result["node_labels"]["Person"] == 3
        assert result["node_labels"]["Company"] == 1
        assert result["edge_labels"]["KNOWS"] == 2

    def test_sample_nodes_highest_degree_first(self, simple_db):
        db, alice, *_ = simple_db
        result = json.loads(_do_inspect(db, ":memory:"))
        assert result["sample_nodes"][0]["id"] == alice.id


class TestDoQuery:
    def test_empty_db_returns_empty(self, empty_db):
        result = json.loads(_do_query(empty_db))
        assert result["nodes"] == []
        assert result["next_cursor"] is None

    def test_label_filter(self, simple_db):
        db, *_ = simple_db
        result = json.loads(_do_query(db, label="Person"))
        assert len(result["nodes"]) == 3
        assert all("Person" in node["labels"] for node in result["nodes"])

    def test_where_filter(self, simple_db):
        db, alice, *_ = simple_db
        result = json.loads(_do_query(db, where='{"name": "Alice"}'))
        assert len(result["nodes"]) == 1
        assert result["nodes"][0]["id"] == alice.id

    def test_where_invalid_json(self, empty_db):
        result = json.loads(_do_query(empty_db, where="not json"))
        assert result["error"]["code"] == "invalid_json"

    def test_pagination_full_page(self, simple_db):
        db, *_ = simple_db
        result = json.loads(_do_query(db, limit=2, cursor=0))
        assert len(result["nodes"]) == 2
        assert result["next_cursor"] == 2


class TestDoExplore:
    def test_invalid_node_id_zero(self, empty_db):
        result = json.loads(_do_explore(empty_db, node_id=0))
        assert result["error"]["code"] == "invalid_node_id"

    def test_node_not_found(self, empty_db):
        result = json.loads(_do_explore(empty_db, node_id=999))
        assert result["error"]["code"] == "node_not_found"

    def test_bfs_returns_neighbours(self, simple_db):
        db, alice, bob, carol, _ = simple_db
        result = json.loads(_do_explore(db, node_id=alice.id))
        ids = {n["id"] for n in result["nodes"]}
        assert alice.id in ids
        assert bob.id in ids
        assert carol.id in ids

    def test_mermaid_present(self, simple_db):
        db, alice, *_ = simple_db
        result = json.loads(_do_explore(db, node_id=alice.id))
        assert result["mermaid"].startswith("graph LR")


class TestDoPath:
    def test_invalid_from_node(self, empty_db):
        result = json.loads(_do_path(empty_db, from_node=0, to_node=1))
        assert result["error"]["code"] == "invalid_node_id"

    def test_no_path_returns_null(self, simple_db):
        db, alice, _, _, acme = simple_db
        result = json.loads(_do_path(db, from_node=alice.id, to_node=acme.id))
        assert result["path"] is None

    def test_direct_edge_path(self, simple_db):
        db, alice, bob, *_ = simple_db
        result = json.loads(_do_path(db, from_node=alice.id, to_node=bob.id))
        ids = [n["id"] for n in result["path"]]
        assert ids[0] == alice.id
        assert ids[-1] == bob.id


class TestDoMermaid:
    def test_empty_db_no_nodes_message(self, empty_db):
        result = json.loads(_do_mermaid(empty_db))
        assert "No nodes found" in result["mermaid"]

    def test_explicit_node_ids(self, simple_db):
        db, alice, bob, *_ = simple_db
        result = json.loads(_do_mermaid(db, node_ids=f"{alice.id},{bob.id}"))
        diagram = result["mermaid"]
        assert f"N{alice.id}" in diagram
        assert f"N{bob.id}" in diagram

    def test_invalid_node_ids_non_integer(self, empty_db):
        result = json.loads(_do_mermaid(empty_db, node_ids="abc"))
        assert result["error"]["code"] == "invalid_node_ids"


class TestDoAppend:
    def test_invalid_nodes_json(self, empty_db):
        result = json.loads(_do_append(empty_db, nodes="not json"))
        assert result["error"]["code"] == "invalid_json"

    def test_creates_multiple_nodes_and_edges(self, empty_db):
        payload_nodes = json.dumps(
            [
                {"ref": "decision", "labels": ["Decision"], "props": {"title": "Keep MCP first"}},
                {"ref": "task", "labels": ["Task"], "props": {"title": "Add merge writes"}},
            ]
        )
        payload_edges = json.dumps(
            [
                {"from": "decision", "to": "task", "label": "LEADS_TO"},
            ]
        )

        result = json.loads(
            _do_append(empty_db, nodes=payload_nodes, edges=payload_edges, session="sess_1")
        )
        assert len(result["created_nodes"]) == 2
        assert len(result["created_edges"]) == 1
        assert result["ref_map"]["decision"] > 0
        assert all("_session" not in node for node in result["created_nodes"])
        assert all("_created_at" not in node for node in result["created_nodes"])

    def test_missing_target_rolls_back_everything(self, empty_db):
        payload_edges = json.dumps([{"from": 1, "to": 9999, "label": "ABOUT"}])
        result = json.loads(_do_append(empty_db, edges=payload_edges))
        assert result["error"]["code"] in {"node_not_found", "invalid_edge_endpoint"}
        assert empty_db.node_count() == 0
        assert empty_db.edge_count() == 0

    def test_unknown_ref_rolls_back(self, empty_db):
        payload_nodes = json.dumps([{"ref": "note", "labels": ["Note"], "props": {"title": "x"}}])
        payload_edges = json.dumps([{"from": "note", "to": "missing", "label": "ABOUT"}])
        result = json.loads(_do_append(empty_db, nodes=payload_nodes, edges=payload_edges))
        assert result["error"]["code"] == "unknown_ref"
        assert empty_db.node_count() == 0


class TestDoMerge:
    def test_invalid_nodes_json(self, empty_db):
        result = json.loads(_do_merge(empty_db, nodes="not json"))
        assert result["error"]["code"] == "invalid_json"

    def test_reuses_existing_node_by_match_and_updates_props(self, simple_db):
        db, alice, bob, *_ = simple_db
        payload_nodes = json.dumps(
            [
                {
                    "ref": "alice",
                    "labels": ["Person"],
                    "match": {"name": "Alice"},
                    "props": {"role": "Lead"},
                },
                {
                    "ref": "note",
                    "labels": ["Decision"],
                    "props": {"title": "Alice owns auth"},
                },
            ]
        )
        payload_edges = json.dumps(
            [
                {"from": "note", "to": "alice", "label": "ABOUT"},
                {"from": "alice", "to": bob.id, "label": "KNOWS"},
            ]
        )

        result = json.loads(_do_merge(db, nodes=payload_nodes, edges=payload_edges))
        assert len(result["created_nodes"]) == 1
        assert len(result["merged_nodes"]) == 1
        assert len(result["merged_edges"]) == 2

        refreshed = db.get_node(alice.id)
        assert refreshed is not None
        assert refreshed.properties["role"] == "Lead"

    def test_creates_when_no_match_exists(self, empty_db):
        payload_nodes = json.dumps(
            [
                {
                    "ref": "module",
                    "labels": ["Module"],
                    "match": {"path": "src/auth.rs"},
                    "props": {"path": "src/auth.rs", "name": "auth"},
                }
            ]
        )
        result = json.loads(_do_merge(empty_db, nodes=payload_nodes))
        assert len(result["created_nodes"]) == 1
        assert result["merged_nodes"] == []

    def test_ambiguous_match_returns_error(self, empty_db):
        empty_db.add_node(["Person"], name="Alice")
        empty_db.add_node(["Person"], name="Alice")
        payload_nodes = json.dumps(
            [
                {
                    "labels": ["Person"],
                    "match": {"name": "Alice"},
                    "props": {"role": "Lead"},
                }
            ]
        )
        result = json.loads(_do_merge(empty_db, nodes=payload_nodes))
        assert result["error"]["code"] == "ambiguous_match"

    def test_direct_id_updates_existing_node(self, simple_db):
        db, alice, *_ = simple_db
        payload_nodes = json.dumps([{"id": alice.id, "props": {"team": "core"}}])
        result = json.loads(_do_merge(db, nodes=payload_nodes))
        assert len(result["merged_nodes"]) == 1
        refreshed = db.get_node(alice.id)
        assert refreshed is not None
        assert refreshed.properties["team"] == "core"

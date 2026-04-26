"""Public Python API smoke tests.

These tests protect the contract between the runtime extension module,
``python/liel/__init__.py``, and the user-facing type stub
``python/liel/liel.pyi``. They intentionally check only the stable,
high-level surface that users import directly.
"""

from __future__ import annotations

import liel


def test_module_exports_expected_symbols():
    expected = {
        "__version__",
        "open",
        "GraphDB",
        "Node",
        "Edge",
        "Transaction",
        "NodeQuery",
        "EdgeQuery",
        "MergeReport",
        "GraphDBError",
        "NodeNotFoundError",
        "EdgeNotFoundError",
        "CorruptedFileError",
        "TransactionError",
        "MergeError",
        "CapacityExceededError",
        "AlreadyOpenError",
    }

    assert set(liel.__all__) == expected
    for name in expected:
        assert hasattr(liel, name), f"liel is missing public symbol {name!r}"


def test_runtime_classes_expose_stubbed_methods():
    graph_methods = {
        "close",
        "add_node",
        "get_node",
        "update_node",
        "delete_node",
        "add_edge",
        "get_edge",
        "update_edge",
        "delete_edge",
        "merge_edge",
        "out_edges",
        "in_edges",
        "neighbors",
        "bfs",
        "dfs",
        "shortest_path",
        "all_nodes",
        "all_edges",
        "all_nodes_as_records",
        "all_edges_as_records",
        "degree_stats",
        "edges_between",
        "node_count",
        "edge_count",
        "nodes",
        "edges",
        "begin",
        "commit",
        "rollback",
        "transaction",
        "merge_from",
        "vacuum",
        "clear",
        "info",
    }
    node_methods = {"get", "keys"}
    edge_methods = {"get", "keys"}
    query_methods = {"label", "where_", "skip", "limit", "fetch", "count", "exists"}
    merge_report_attrs = {
        "node_id_map",
        "edge_id_map",
        "nodes_created",
        "nodes_reused",
        "edges_created",
        "edges_reused",
    }

    with liel.open(":memory:") as db:
        node = db.add_node(["Person"], name="Alice")
        edge = db.add_edge(node, "SELF", node, since=2026)
        node_query = db.nodes()
        edge_query = db.edges()
        merge_report = db.merge_from(liel.open(":memory:"))

        for name in graph_methods:
            assert hasattr(db, name), f"GraphDB is missing method {name!r}"
        for name in node_methods:
            assert hasattr(node, name), f"Node is missing method {name!r}"
        for name in edge_methods:
            assert hasattr(edge, name), f"Edge is missing method {name!r}"
        for name in query_methods:
            assert hasattr(node_query, name), f"NodeQuery is missing method {name!r}"
            assert hasattr(edge_query, name), f"EdgeQuery is missing method {name!r}"
        for name in merge_report_attrs:
            assert hasattr(merge_report, name), f"MergeReport is missing attribute {name!r}"

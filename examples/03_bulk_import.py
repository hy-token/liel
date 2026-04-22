"""
liel Bulk Import - insert public data in one transaction.

What you'll learn:
  - Reading a public JSON dataset with the standard library
  - Mapping external ids into liel node ids
  - Importing all nodes and edges inside a single transaction

The demo uses the Les Miserables character co-appearance graph from
vega-datasets:
https://raw.githubusercontent.com/vega/vega-datasets/main/data/miserables.json
"""

from __future__ import annotations

import csv
import json
import pathlib
import urllib.error
import urllib.request
from typing import Any

import liel

PUBLIC_GRAPH_URL = (
    "https://raw.githubusercontent.com/vega/vega-datasets/main/data/miserables.json"
)


def import_nodes_csv(db: liel.GraphDB, path: str) -> dict[str, int]:
    """Bulk-import nodes from a CSV file.

    CSV format: id, labels, then any property columns.
    The labels column may contain one or more labels separated by "|".
    """
    id_map: dict[str, int] = {}
    with open(path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        with db.transaction():
            for row in reader:
                labels = [label.strip() for label in row.pop("labels").split("|")]
                ext_id = row.pop("id")
                props = {k: _coerce(v) for k, v in row.items() if v != ""}
                node = db.add_node(labels, **props)
                id_map[ext_id] = node.id
    return id_map


def import_edges_csv(db: liel.GraphDB, path: str, id_map: dict[str, int]) -> None:
    """Bulk-import edges from a CSV file.

    CSV format: from_id, label, to_id, then any property columns.
    Pass the return value of import_nodes_csv as id_map.
    """
    with open(path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        with db.transaction():
            for row in reader:
                from_id = id_map[row.pop("from_id")]
                to_id = id_map[row.pop("to_id")]
                label = row.pop("label")
                props = {k: _coerce(v) for k, v in row.items() if v != ""}
                db.add_edge(from_id, label, to_id, **props)


def import_json(db: liel.GraphDB, path: str) -> dict[int, int]:
    """Bulk-import a graph exported by examples/06_export.py.

    JSON format:
        {"nodes": [{"id": int, "labels": [...], "properties": {...}}],
         "edges": [{"from_node": int, "label": str, "to_node": int, "properties": {...}}]}
    """
    data = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    id_map: dict[int, int] = {}

    with db.transaction():
        for n in data["nodes"]:
            node = db.add_node(n["labels"], **n.get("properties", {}))
            id_map[n["id"]] = node.id

        for e in data["edges"]:
            db.add_edge(
                id_map[e["from_node"]],
                e["label"],
                id_map[e["to_node"]],
                **e.get("properties", {}),
            )

    return id_map


def download_json(url: str) -> dict[str, Any]:
    """Download a small public JSON dataset."""
    request = urllib.request.Request(url, headers={"User-Agent": "liel-example/1.0"})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, TimeoutError) as exc:
        raise RuntimeError(f"Could not download public dataset: {url}") from exc


def import_miserables_graph(db: liel.GraphDB, data: dict[str, Any]) -> list[int]:
    """Import the Les Miserables co-appearance graph.

    The source data stores edge endpoints as zero-based indexes into the node
    array, so the import keeps a small index-to-node-id map.
    """
    node_ids: list[int] = []
    with db.transaction():
        for index, record in enumerate(data["nodes"]):
            node = db.add_node(
                ["Character"],
                source_index=index,
                name=record["name"],
                group=record["group"],
            )
            node_ids.append(node.id)

        for record in data["links"]:
            db.add_edge(
                node_ids[record["source"]],
                "APPEARS_WITH",
                node_ids[record["target"]],
                weight=record["value"],
            )
    return node_ids


def _coerce(value: str):
    """Convert a CSV string to a simple Python scalar when possible."""
    try:
        return int(value)
    except ValueError:
        pass
    try:
        return float(value)
    except ValueError:
        pass
    if value.lower() == "true":
        return True
    if value.lower() == "false":
        return False
    return value


def demo() -> None:
    print(f"Downloading public graph data:\n  {PUBLIC_GRAPH_URL}")
    data = download_json(PUBLIC_GRAPH_URL)

    with liel.open(":memory:") as db:
        node_ids = import_miserables_graph(db, data)
        print(f"Imported: {db.node_count()} characters, {db.edge_count()} co-appearances")

        valjean = db.get_node(node_ids[11])
        neighbors = db.neighbors(valjean, edge_label="APPEARS_WITH")
        strongest = sorted(neighbors, key=lambda n: n.get("name"))[:5]

        print(f"\nExample query: characters connected from {valjean['name']}")
        for node in strongest:
            print(f"  - {node['name']} (group {node['group']})")


if __name__ == "__main__":
    demo()

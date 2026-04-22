"""
liel Export - Write graph to JSON / CSV files

For importing from JSON / CSV, see 03_bulk_import.py.
Requires: standard library only
"""

import csv
import json
import pathlib
import tempfile

import liel


def _prepare_output_path(path: str) -> pathlib.Path:
    out_path = pathlib.Path(path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    return out_path


def export_json(db: liel.GraphDB, path: str) -> None:
    """Export the entire graph to a JSON file."""
    out_path = _prepare_output_path(path)
    data = {
        "nodes": [
            {
                "id": r["id"],
                "labels": r["labels"],
                "properties": {k: v for k, v in r.items() if k not in ("id", "labels")},
            }
            for r in db.all_nodes_as_records()
        ],
        "edges": [
            {
                "id": r["id"],
                "label": r["label"],
                "from_node": r["from_node"],
                "to_node": r["to_node"],
                "properties": {
                    k: v for k, v in r.items() if k not in ("id", "label", "from_node", "to_node")
                },
            }
            for r in db.all_edges_as_records()
        ],
    }
    out_path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"[export_json] {len(data['nodes'])} nodes, {len(data['edges'])} edges -> {out_path}")


def export_nodes_csv(db: liel.GraphDB, path: str) -> None:
    """Export all nodes to CSV. The labels column is '|' separated."""
    out_path = _prepare_output_path(path)
    records = db.all_nodes_as_records()
    if not records:
        return
    prop_keys = sorted({k for r in records for k in r if k not in ("id", "labels")})
    fieldnames = ["id", "labels"] + prop_keys

    with out_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in records:
            row = {"id": r["id"], "labels": "|".join(r["labels"])}
            for k in prop_keys:
                row[k] = r.get(k)
            writer.writerow(row)

    print(f"[export_nodes_csv] {len(records)} nodes -> {out_path}")


def export_edges_csv(db: liel.GraphDB, path: str) -> None:
    """Export all edges to CSV."""
    out_path = _prepare_output_path(path)
    records = db.all_edges_as_records()
    if not records:
        return
    meta = ("id", "label", "from_node", "to_node")
    prop_keys = sorted({k for r in records for k in r if k not in meta})
    fieldnames = list(meta) + prop_keys

    with out_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in records:
            writer.writerow({k: r.get(k) for k in fieldnames})

    print(f"[export_edges_csv] {len(records)} edges -> {out_path}")


def demo() -> None:
    db = liel.open(":memory:")
    alice = db.add_node(["Person"], name="Alice", age=30)
    bob = db.add_node(["Person"], name="Bob", age=25)
    carol = db.add_node(["Person"], name="Carol", age=35)
    db.add_edge(alice, "FOLLOWS", bob, since=2020)
    db.add_edge(bob, "FOLLOWS", carol, since=2021)
    db.commit()

    out_dir = pathlib.Path(tempfile.gettempdir()) / "liel-export-demo"
    graph_json = out_dir / "graph.json"
    nodes_csv = out_dir / "nodes.csv"
    edges_csv = out_dir / "edges.csv"

    export_json(db, str(graph_json))
    export_nodes_csv(db, str(nodes_csv))
    export_edges_csv(db, str(edges_csv))

    print("\n--- nodes.csv ---")
    print(nodes_csv.read_text(encoding="utf-8"))
    print("--- edges.csv ---")
    print(edges_csv.read_text(encoding="utf-8"))


if __name__ == "__main__":
    demo()

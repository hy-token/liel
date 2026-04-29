"""
Generate small `.liel` files for manual CLI smoke testing.

Run from a checkout after `maturin develop`:

    python examples/09_cli_smoke_files.py --force

The source data lives in `examples/cli_smoke_data/*.csv`. Generated `.liel`
files are written under `target/cli-smoke/` by default and are ignored by git.
"""

from __future__ import annotations

import argparse
import csv
import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import liel

DATA_DIR = Path(__file__).with_name("cli_smoke_data")


@dataclass(frozen=True)
class NodeSpec:
    ref: str
    labels: list[str]
    properties: dict[str, Any]


@dataclass(frozen=True)
class EdgeSpec:
    from_ref: str
    label: str
    to_ref: str
    properties: dict[str, Any]


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate .liel files for CLI smoke tests.")
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("target/cli-smoke"),
        help="Output directory for generated .liel files.",
    )
    parser.add_argument("--data", type=Path, default=DATA_DIR, help="CSV source data directory.")
    parser.add_argument(
        "--force", action="store_true", help="Overwrite an existing output directory."
    )
    parser.add_argument(
        "--clean-locks",
        action="store_true",
        help="Remove generated .liel.lock directories from the output directory and exit.",
    )
    args = parser.parse_args()

    if args.clean_locks:
        _clean_locks(args.out)
        print(f"Removed generated lock directories under {args.out}")
        return

    if args.out.exists():
        if not args.force:
            raise SystemExit(f"{args.out} already exists; pass --force to overwrite it")
        shutil.rmtree(args.out)
    args.out.mkdir(parents=True)

    nodes_by_graph = _load_nodes(args.data / "nodes.csv")
    edges_by_graph = _load_edges(args.data / "edges.csv")
    for graph_name, nodes in nodes_by_graph.items():
        _write_graph(args.out / f"{graph_name}.liel", nodes, edges_by_graph.get(graph_name, []))

    print(f"Generated CLI smoke files in {args.out}")
    print(f"Source CSV: {args.data}")
    print()
    print("Try:")
    print(f"  liel diff {args.out / 'same-left.liel'} {args.out / 'same-right.liel'}")
    print(f"  liel diff {args.out / 'changed-left.liel'} {args.out / 'changed-right.liel'}")
    print(
        "  liel merge "
        f"{args.out / 'merge-base.liel'} {args.out / 'merge-incoming.liel'} "
        f"-o {args.out / 'merged.liel'} --node-key path --edge-strategy idempotent"
    )
    print()
    print("If lock warnings appear after manual CLI runs:")
    print(f"  python examples/09_cli_smoke_files.py --out {args.out} --clean-locks")


def _load_nodes(path: Path) -> dict[str, list[NodeSpec]]:
    nodes: dict[str, list[tuple[int, NodeSpec]]] = {}
    for row in _read_csv(path):
        graph = row["graph"]
        spec = NodeSpec(
            ref=row["ref"],
            labels=_split_labels(row["labels"]),
            properties=_decode_properties(row["properties_json"]),
        )
        nodes.setdefault(graph, []).append((int(row["order"]), spec))
    return {graph: [spec for _, spec in sorted(specs)] for graph, specs in nodes.items()}


def _load_edges(path: Path) -> dict[str, list[EdgeSpec]]:
    edges: dict[str, list[tuple[int, EdgeSpec]]] = {}
    for row in _read_csv(path):
        graph = row["graph"]
        spec = EdgeSpec(
            from_ref=row["from_ref"],
            label=row["label"],
            to_ref=row["to_ref"],
            properties=_decode_properties(row["properties_json"]),
        )
        edges.setdefault(graph, []).append((int(row["order"]), spec))
    return {graph: [spec for _, spec in sorted(specs)] for graph, specs in edges.items()}


def _read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def _split_labels(value: str) -> list[str]:
    return [label for label in value.split(";") if label]


def _decode_properties(value: str) -> dict[str, Any]:
    parsed = json.loads(value or "{}")
    if not isinstance(parsed, dict):
        raise ValueError(f"properties_json must decode to an object: {value}")
    return parsed


def _write_graph(path: Path, nodes: list[NodeSpec], edges: list[EdgeSpec]) -> None:
    _remove_generated_file(path)
    with liel.open(str(path)) as db:
        created = {spec.ref: db.add_node(spec.labels, **spec.properties) for spec in nodes}
        for spec in edges:
            db.add_edge(created[spec.from_ref], spec.label, created[spec.to_ref], **spec.properties)
        db.commit()
    _remove_generated_lock(path)


def _remove_generated_file(path: Path) -> None:
    path.unlink(missing_ok=True)
    _remove_generated_lock(path)


def _remove_generated_lock(path: Path) -> None:
    shutil.rmtree(Path(str(path) + ".lock"), ignore_errors=True)


def _clean_locks(out: Path) -> None:
    if not out.exists():
        return
    for lock_dir in out.glob("*.liel.lock"):
        shutil.rmtree(lock_dir, ignore_errors=True)


if __name__ == "__main__":
    main()

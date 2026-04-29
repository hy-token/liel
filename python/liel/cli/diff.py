from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import liel

from .common import EXIT_ERROR, EXIT_OK, CliError, emit_json, emit_text, require_existing_file
from .identity import diff_records_by_id, records_by_id

EXIT_DIFFERENT = 1

_NODE_META_KEYS = {"id", "labels"}
_EDGE_META_KEYS = {"id", "label", "from_node", "to_node"}


def run(args: argparse.Namespace) -> int:
    report = diff_files(args.left, args.right)
    if args.format == "json":
        emit_json(report)
    else:
        emit_text(format_text(report))
    return EXIT_DIFFERENT if report["changed"] else EXIT_OK


def diff_files(left_path: str | Path, right_path: str | Path) -> dict[str, Any]:
    left = _snapshot(require_existing_file(left_path))
    right = _snapshot(require_existing_file(right_path))

    node_diff = diff_records_by_id(left["nodes_by_id"], right["nodes_by_id"])
    edge_diff = diff_records_by_id(left["edges_by_id"], right["edges_by_id"])

    changed = any(node_diff[key] or edge_diff[key] for key in ("added", "removed", "changed"))
    return {
        "changed": changed,
        "left": {
            "path": str(left["path"]),
            "nodes": left["node_count"],
            "edges": left["edge_count"],
        },
        "right": {
            "path": str(right["path"]),
            "nodes": right["node_count"],
            "edges": right["edge_count"],
        },
        "nodes": node_diff,
        "edges": edge_diff,
    }


def format_text(report: dict[str, Any]) -> str:
    if not report["changed"]:
        return "No differences."

    nodes = report["nodes"]
    edges = report["edges"]
    lines = [
        f"Nodes: +{len(nodes['added'])} -{len(nodes['removed'])} ~{len(nodes['changed'])}",
        f"Edges: +{len(edges['added'])} -{len(edges['removed'])} ~{len(edges['changed'])}",
    ]
    lines.extend(_detail_lines("node", nodes))
    lines.extend(_detail_lines("edge", edges))
    return "\n".join(lines)


def _detail_lines(kind: str, diff: dict[str, list[int]]) -> list[str]:
    lines: list[str] = []
    for key in ("added", "removed", "changed"):
        ids = diff[key]
        if ids:
            lines.append(f"{kind} {key}: {', '.join(str(item) for item in ids)}")
    return lines


def _snapshot(path: Path) -> dict[str, Any]:
    try:
        with liel.open(str(path)) as db:
            nodes = [_normalize_node(record) for record in db.all_nodes_as_records()]
            edges = [_normalize_edge(record) for record in db.all_edges_as_records()]
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"failed to read {path}: {exc}", EXIT_ERROR) from exc

    return {
        "path": path,
        "node_count": len(nodes),
        "edge_count": len(edges),
        "nodes_by_id": records_by_id(nodes),
        "edges_by_id": records_by_id(edges),
    }


def _normalize_node(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": record["id"],
        "labels": sorted(record.get("labels", [])),
        "properties": {key: record[key] for key in sorted(record) if key not in _NODE_META_KEYS},
    }


def _normalize_edge(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": record["id"],
        "label": record["label"],
        "from_node": record["from_node"],
        "to_node": record["to_node"],
        "properties": {key: record[key] for key in sorted(record) if key not in _EDGE_META_KEYS},
    }

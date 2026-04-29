from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
from typing import Any

import liel

from .common import EXIT_ERROR, EXIT_OK, CliError, emit_json, emit_text, require_existing_file


def run(args: argparse.Namespace) -> int:
    payload = stats_file(args.source)
    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_text(payload))
    return EXIT_OK


def stats_file(source_path: str | Path) -> dict[str, Any]:
    source = require_existing_file(source_path)
    try:
        with liel.open(str(source)) as db:
            info = db.info()
            nodes = db.all_nodes_as_records()
            edges = db.all_edges_as_records()
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"stats failed: {exc}", EXIT_ERROR) from exc

    node_labels = Counter(label for node in nodes for label in node.get("labels", []))
    edge_labels = Counter(edge["label"] for edge in edges)
    return {
        "path": str(source),
        "liel_format": info["version"],
        "file_size": info["file_size"],
        "node_count": len(nodes),
        "edge_count": len(edges),
        "node_labels": dict(sorted(node_labels.items())),
        "edge_labels": dict(sorted(edge_labels.items())),
    }


def format_text(payload: dict[str, Any]) -> str:
    lines = [
        f"File: {payload['path']}",
        f"Format: {payload['liel_format']}",
        f"File size: {payload['file_size']} bytes",
        f"Nodes: {payload['node_count']}",
        f"Edges: {payload['edge_count']}",
        "Node labels:",
    ]
    lines.extend(_label_lines(payload["node_labels"]))
    lines.append("Edge labels:")
    lines.extend(_label_lines(payload["edge_labels"]))
    return "\n".join(lines)


def _label_lines(labels: dict[str, int]) -> list[str]:
    if not labels:
        return ["  (none)"]
    return [f"  {label}: {count}" for label, count in labels.items()]

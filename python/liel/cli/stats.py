from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
from typing import Any

import liel

from .common import EXIT_ERROR, EXIT_OK, CliError, emit_json, emit_text, require_existing_file


def _human_file_size(num_bytes: int) -> str:
    """Binary IEC units for text output; JSON still uses raw ``file_size`` bytes."""
    if num_bytes < 0:
        raise ValueError("file_size must be non-negative")
    if num_bytes < 1024:
        return f"{num_bytes} bytes"
    size = float(num_bytes)
    units = ("KiB", "MiB", "GiB", "TiB")
    for i, unit in enumerate(units):
        size /= 1024.0
        if size < 1024.0 or i == len(units) - 1:
            rounded = f"{size:.2f}".rstrip("0").rstrip(".")
            return f"{rounded} {unit}"


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
        f"File size: {_human_file_size(int(payload['file_size']))}",
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

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import liel

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    CliError,
    emit_text,
    refuse_overwrite,
    require_existing_file,
)

MANIFEST_VERSION = 1
_NODE_META_KEYS = {"id", "labels"}
_EDGE_META_KEYS = {"id", "label", "from_node", "to_node"}


def run(args: argparse.Namespace) -> int:
    manifest_bytes = build_manifest_bytes(args.source)
    if args.output is None:
        emit_text(manifest_bytes.decode("utf-8").rstrip("\n"))
    else:
        output = refuse_overwrite(args.output, force=args.force)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(manifest_bytes)
    return EXIT_OK


def build_manifest_bytes(source_path: str | Path) -> bytes:
    payload = build_manifest(source_path)
    try:
        text = json.dumps(
            payload,
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
            separators=(",", ": "),
            allow_nan=False,
        )
    except ValueError as exc:
        raise CliError(f"manifest serialization failed: {exc}", EXIT_ERROR) from exc
    return f"{text}\n".encode()


def build_manifest(source_path: str | Path) -> dict[str, Any]:
    source = require_existing_file(source_path)
    try:
        with liel.open(str(source)) as db:
            info = db.info()
            nodes = [_normalize_node(record) for record in db.all_nodes_as_records()]
            edges = [_normalize_edge(record) for record in db.all_edges_as_records()]
    except (OSError, ValueError, liel.GraphDBError) as exc:
        raise CliError(f"manifest failed: {exc}", EXIT_ERROR) from exc

    nodes.sort(key=lambda record: record["id"])
    edges.sort(key=lambda record: record["id"])
    return {
        "edge_count": len(edges),
        "edges": edges,
        "liel_format": info["version"],
        "manifest_version": MANIFEST_VERSION,
        "node_count": len(nodes),
        "nodes": nodes,
    }


def _normalize_node(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": record["id"],
        "labels": sorted(record.get("labels", [])),
        "properties": {key: record[key] for key in sorted(record) if key not in _NODE_META_KEYS},
    }


def _normalize_edge(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "from_node": record["from_node"],
        "id": record["id"],
        "label": record["label"],
        "properties": {key: record[key] for key in sorted(record) if key not in _EDGE_META_KEYS},
        "to_node": record["to_node"],
    }

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any

import liel

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    EXIT_USAGE,
    CliError,
    emit_json,
    emit_text,
    refuse_overwrite,
    require_existing_file,
)

EXPORT_VERSION = 1
_NODE_META_KEYS = {"id", "labels"}
_EDGE_META_KEYS = {"id", "label", "from_node", "to_node"}


def run_export(args: argparse.Namespace) -> int:
    export_bytes = build_export_bytes(args.source)
    if args.output is None:
        emit_text(export_bytes.decode().rstrip("\n"))
    else:
        output = refuse_overwrite(args.output, force=args.force)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(export_bytes)
    return EXIT_OK


def run_import(args: argparse.Namespace) -> int:
    payload = import_file(args.source, args.output, force=args.force)
    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_import_text(payload))
    return EXIT_OK


def build_export_bytes(source_path: str | Path) -> bytes:
    payload = build_export(source_path)
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
        raise CliError(f"export serialization failed: {exc}", EXIT_ERROR) from exc
    return f"{text}\n".encode()


def build_export(source_path: str | Path) -> dict[str, Any]:
    source = require_existing_file(source_path)
    try:
        with liel.open(str(source)) as db:
            info = db.info()
            nodes = [_normalize_node(record) for record in db.all_nodes_as_records()]
            edges = [_normalize_edge(record) for record in db.all_edges_as_records()]
    except (OSError, ValueError, liel.GraphDBError) as exc:
        raise CliError(f"export failed: {exc}", EXIT_ERROR) from exc

    nodes.sort(key=lambda record: record["id"])
    edges.sort(key=lambda record: record["id"])
    return {
        "edge_count": len(edges),
        "edges": edges,
        "export_version": EXPORT_VERSION,
        "liel_format": info["version"],
        "node_count": len(nodes),
        "nodes": nodes,
    }


def import_file(
    source_path: str | Path,
    output_path: str | Path,
    *,
    force: bool = False,
) -> dict[str, Any]:
    source = require_existing_file(source_path)
    output = refuse_overwrite(output_path, force=force)
    payload = _load_export(source)
    nodes = sorted(payload["nodes"], key=lambda record: record["id"])
    edges = sorted(payload["edges"], key=lambda record: record["id"])
    node_id_map: dict[int, int] = {}
    edge_id_map: dict[int, int] = {}

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        _remove_created_output(output)
        with liel.open(str(output)) as db:
            for record in nodes:
                created = db.add_node(record["labels"], **record["properties"])
                node_id_map[record["id"]] = created.id
            for record in edges:
                _require_edge_endpoints(record, node_id_map)
                created = db.add_edge(
                    node_id_map[record["from_node"]],
                    record["label"],
                    node_id_map[record["to_node"]],
                    **record["properties"],
                )
                edge_id_map[record["id"]] = created.id
            db.commit()
    except CliError:
        _remove_created_output(output)
        raise
    except (OSError, ValueError, liel.GraphDBError) as exc:
        _remove_created_output(output)
        raise CliError(f"import failed: {exc}", EXIT_ERROR) from exc

    return {
        "source": str(source),
        "output": str(output),
        "nodes_imported": len(node_id_map),
        "edges_imported": len(edge_id_map),
        "node_id_map": node_id_map,
        "edge_id_map": edge_id_map,
    }


def format_import_text(payload: dict[str, Any]) -> str:
    return "\n".join(
        [
            f"Imported {payload['source']} into {payload['output']}",
            f"Nodes: {payload['nodes_imported']}",
            f"Edges: {payload['edges_imported']}",
        ]
    )


def _load_export(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CliError(f"failed to read export JSON {path}: {exc}", EXIT_ERROR) from exc
    _validate_export_payload(payload)
    return payload


def _validate_export_payload(payload: Any) -> None:
    if not isinstance(payload, dict):
        raise CliError("export JSON must contain an object", EXIT_USAGE)
    if payload.get("export_version") != EXPORT_VERSION:
        raise CliError(f"unsupported export_version: {payload.get('export_version')}", EXIT_USAGE)
    nodes = payload.get("nodes")
    edges = payload.get("edges")
    if not isinstance(nodes, list) or not isinstance(edges, list):
        raise CliError("export JSON must contain nodes and edges arrays", EXIT_USAGE)
    for record in nodes:
        _validate_node_record(record)
    for record in edges:
        _validate_edge_record(record)


def _validate_node_record(record: Any) -> None:
    if not isinstance(record, dict):
        raise CliError("node records must be objects", EXIT_USAGE)
    if not isinstance(record.get("id"), int):
        raise CliError("node id must be an integer", EXIT_USAGE)
    if not isinstance(record.get("labels"), list) or not all(
        isinstance(label, str) for label in record.get("labels", [])
    ):
        raise CliError("node labels must be strings", EXIT_USAGE)
    if not isinstance(record.get("properties"), dict):
        raise CliError("node properties must be an object", EXIT_USAGE)


def _validate_edge_record(record: Any) -> None:
    if not isinstance(record, dict):
        raise CliError("edge records must be objects", EXIT_USAGE)
    if not isinstance(record.get("id"), int):
        raise CliError("edge id must be an integer", EXIT_USAGE)
    if not isinstance(record.get("from_node"), int) or not isinstance(record.get("to_node"), int):
        raise CliError("edge endpoints must be integers", EXIT_USAGE)
    if not isinstance(record.get("label"), str):
        raise CliError("edge label must be a string", EXIT_USAGE)
    if not isinstance(record.get("properties"), dict):
        raise CliError("edge properties must be an object", EXIT_USAGE)


def _require_edge_endpoints(record: dict[str, Any], node_id_map: dict[int, int]) -> None:
    if record["from_node"] not in node_id_map or record["to_node"] not in node_id_map:
        raise CliError(f"edge {record['id']} references a missing node", EXIT_USAGE)


def _remove_created_output(path: Path) -> None:
    try:
        path.unlink(missing_ok=True)
        shutil.rmtree(Path(str(path) + ".lock"), ignore_errors=True)
    except OSError:
        pass


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

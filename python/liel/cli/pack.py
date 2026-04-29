from __future__ import annotations

import argparse
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

_NODE_META_KEYS = {"id", "labels"}
_EDGE_META_KEYS = {"id", "label", "from_node", "to_node"}


def run(args: argparse.Namespace) -> int:
    payload = pack_file(
        args.source,
        args.output,
        include_labels=args.include_labels,
        force=args.force,
    )
    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_text(payload))
    return EXIT_OK


def pack_file(
    source_path: str | Path,
    output_path: str | Path,
    *,
    include_labels: str | list[str],
    force: bool = False,
) -> dict[str, Any]:
    source = require_existing_file(source_path)
    output = refuse_overwrite(output_path, force=force)
    _reject_in_place_output(source, output)
    labels = _parse_include_labels(include_labels)

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        _remove_created_output(output)
        with liel.open(str(source)) as src, liel.open(str(output)) as dst:
            source_edges = src.edge_count()
            selected_nodes = _selected_nodes(src, labels)
            node_id_map = _copy_nodes(dst, selected_nodes)
            copied_edges = _copy_edges(src, dst, set(node_id_map), node_id_map)
            dst.commit()
    except (OSError, ValueError, liel.GraphDBError) as exc:
        _remove_created_output(output)
        raise CliError(f"pack failed: {exc}", EXIT_ERROR) from exc

    return {
        "source": str(source),
        "output": str(output),
        "include_labels": labels,
        "source_nodes": len(selected_nodes["all_nodes"]),
        "source_edges": source_edges,
        "nodes_packed": len(node_id_map),
        "edges_packed": copied_edges,
        "node_id_map": node_id_map,
    }


def format_text(payload: dict[str, Any]) -> str:
    labels = ", ".join(payload["include_labels"])
    return "\n".join(
        [
            f"Packed {payload['source']} into {payload['output']}",
            f"Labels: {labels}",
            f"Nodes: {payload['nodes_packed']} of {payload['source_nodes']}",
            f"Edges: {payload['edges_packed']} of {payload['source_edges']}",
        ]
    )


def _parse_include_labels(value: str | list[str]) -> list[str]:
    values = [value] if isinstance(value, str) else value
    labels = [label.strip() for item in values for label in item.split(",") if label.strip()]
    if not labels:
        raise CliError("pack requires at least one --include-labels value", EXIT_USAGE)
    return sorted(dict.fromkeys(labels))


def _selected_nodes(db: liel.GraphDB, labels: list[str]) -> dict[str, Any]:
    records = db.all_nodes_as_records()
    wanted = set(labels)
    nodes = [record for record in records if wanted.intersection(record.get("labels", []))]
    return {"all_nodes": records, "selected": sorted(nodes, key=lambda record: record["id"])}


def _copy_nodes(dst: liel.GraphDB, nodes: dict[str, Any]) -> dict[int, int]:
    node_id_map: dict[int, int] = {}
    for record in nodes["selected"]:
        created = dst.add_node(record.get("labels", []), **_node_properties(record))
        node_id_map[record["id"]] = created.id
    return node_id_map


def _copy_edges(
    src: liel.GraphDB,
    dst: liel.GraphDB,
    selected_ids: set[int],
    node_id_map: dict[int, int],
) -> int:
    copied = 0
    for record in sorted(src.edges_between(selected_ids), key=lambda item: item["id"]):
        dst.add_edge(
            node_id_map[record["from_node"]],
            record["label"],
            node_id_map[record["to_node"]],
            **_edge_properties(record),
        )
        copied += 1
    return copied


def _node_properties(record: dict[str, Any]) -> dict[str, Any]:
    return {key: record[key] for key in record if key not in _NODE_META_KEYS}


def _edge_properties(record: dict[str, Any]) -> dict[str, Any]:
    return {key: record[key] for key in record if key not in _EDGE_META_KEYS}


def _reject_in_place_output(source: Path, output: Path) -> None:
    if output.resolve() == source.resolve():
        raise CliError("pack output must be different from the input file", EXIT_USAGE)


def _remove_created_output(path: Path) -> None:
    try:
        path.unlink(missing_ok=True)
        shutil.rmtree(Path(str(path) + ".lock"), ignore_errors=True)
    except OSError:
        pass

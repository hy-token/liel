from __future__ import annotations

import argparse
from collections import Counter
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
    require_existing_file,
)
from .identity import (
    diff_records_by_id,
    identity_from_rules,
    identity_string,
    load_identity_rules,
    records_by_id,
)

EXIT_DIFFERENT = 1

_NODE_META_KEYS = {"id", "labels"}
_EDGE_META_KEYS = {"id", "label", "from_node", "to_node"}


def run(args: argparse.Namespace) -> int:
    report = diff_files(
        args.left,
        args.right,
        node_key=args.node_key,
        identity_rules=args.identity_rules,
    )
    if args.format == "json":
        emit_json(report)
    else:
        emit_text(format_text(report))
    return EXIT_DIFFERENT if report["changed"] else EXIT_OK


def diff_files(
    left_path: str | Path,
    right_path: str | Path,
    *,
    node_key: list[str] | None = None,
    identity_rules: str | Path | None = None,
) -> dict[str, Any]:
    if node_key and identity_rules is not None:
        raise CliError("--node-key and --identity-rules cannot be used together", EXIT_USAGE)

    left = _snapshot(require_existing_file(left_path))
    right = _snapshot(require_existing_file(right_path))

    if identity_rules is not None:
        rules = load_identity_rules(identity_rules)
        node_diff, left_identities, right_identities = _diff_nodes_by_rules(
            left["nodes"], right["nodes"], rules
        )
        edge_diff = _diff_edges_by_identity(
            left["edges"],
            right["edges"],
            left_identities,
            right_identities,
            identity={"mode": "identity_rules_edge_multiset", "rules": rules},
        )
    elif node_key:
        node_diff = _diff_nodes_by_key(left["nodes"], right["nodes"], node_key)
        edge_diff = _diff_edges_by_node_key(left, right, node_key)
    else:
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


def _detail_lines(kind: str, diff: dict[str, Any]) -> list[str]:
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
        "nodes": nodes,
        "edges": edges,
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


def _diff_nodes_by_key(
    left_nodes: list[dict[str, Any]],
    right_nodes: list[dict[str, Any]],
    node_key: list[str],
) -> dict[str, Any]:
    left = _records_by_node_key(left_nodes, node_key, side="left")
    right = _records_by_node_key(right_nodes, node_key, side="right")
    left_keys = set(left)
    right_keys = set(right)
    shared = left_keys & right_keys
    return {
        "identity": {"mode": "node_key", "keys": node_key},
        "added": sorted(right_keys - left_keys),
        "removed": sorted(left_keys - right_keys),
        "changed": sorted(
            key
            for key in shared
            if _node_compare_record(left[key]) != _node_compare_record(right[key])
        ),
    }


def _diff_edges_by_node_key(
    left: dict[str, Any],
    right: dict[str, Any],
    node_key: list[str],
) -> dict[str, Any]:
    left_nodes = _records_by_id_key(left["nodes"], node_key, side="left")
    right_nodes = _records_by_id_key(right["nodes"], node_key, side="right")
    return _diff_edges_by_identity(
        left["edges"],
        right["edges"],
        left_nodes,
        right_nodes,
        identity={"mode": "node_key_edge_multiset", "node_keys": node_key},
    )


def _diff_nodes_by_rules(
    left_nodes: list[dict[str, Any]],
    right_nodes: list[dict[str, Any]],
    rules: dict[str, list[str]],
) -> tuple[dict[str, Any], dict[int, str], dict[int, str]]:
    left = _records_by_identity_rules(left_nodes, rules, side="left")
    right = _records_by_identity_rules(right_nodes, rules, side="right")
    left_records, left_identities = left
    right_records, right_identities = right
    left_keys = set(left_records)
    right_keys = set(right_records)
    shared = left_keys & right_keys
    diff = {
        "identity": {"mode": "identity_rules", "rules": rules},
        "added": sorted(right_keys - left_keys),
        "removed": sorted(left_keys - right_keys),
        "changed": sorted(
            key
            for key in shared
            if _node_compare_record(left_records[key]) != _node_compare_record(right_records[key])
        ),
    }
    return diff, left_identities, right_identities


def _diff_edges_by_identity(
    left_edges_raw: list[dict[str, Any]],
    right_edges_raw: list[dict[str, Any]],
    left_nodes: dict[int, str],
    right_nodes: dict[int, str],
    *,
    identity: dict[str, Any],
) -> dict[str, Any]:
    left_edges = _edge_multiset_by_node_key(left_edges_raw, left_nodes)
    right_edges = _edge_multiset_by_node_key(right_edges_raw, right_nodes)
    return {
        "identity": identity,
        "added": _counter_delta(right_edges, left_edges),
        "removed": _counter_delta(left_edges, right_edges),
        "changed": [],
    }


def _records_by_node_key(
    records: list[dict[str, Any]], node_key: list[str], *, side: str
) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for record in records:
        key = _node_identity(record, node_key, side=side)
        if key in indexed:
            raise CliError(
                f"{side} node identity is not unique for --node-key: {key}",
                EXIT_USAGE,
            )
        indexed[key] = record
    return indexed


def _records_by_id_key(
    records: list[dict[str, Any]], node_key: list[str], *, side: str
) -> dict[int, str]:
    return {record["id"]: _node_identity(record, node_key, side=side) for record in records}


def _records_by_identity_rules(
    records: list[dict[str, Any]],
    rules: dict[str, list[str]],
    *,
    side: str,
) -> tuple[dict[str, dict[str, Any]], dict[int, str]]:
    by_identity: dict[str, dict[str, Any]] = {}
    by_id: dict[int, str] = {}
    for record in records:
        identity = identity_from_rules(record, rules, side=side)
        if identity in by_identity:
            raise CliError(
                f"{side} node identity is not unique for --identity-rules: {identity}",
                EXIT_USAGE,
            )
        by_identity[identity] = record
        by_id[record["id"]] = identity
    return by_identity, by_id


def _node_identity(record: dict[str, Any], node_key: list[str], *, side: str) -> str:
    properties = record["properties"]
    for key in node_key:
        if key not in properties:
            raise CliError(
                f"{side} node {record['id']} is missing --node-key property: {key}",
                EXIT_USAGE,
            )
    return identity_string(properties, node_key)


def _node_compare_record(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "labels": record["labels"],
        "properties": record["properties"],
    }


def _edge_multiset_by_node_key(
    edges: list[dict[str, Any]], node_id_keys: dict[int, str]
) -> Counter[str]:
    return Counter(_edge_identity(edge, node_id_keys) for edge in edges)


def _edge_identity(edge: dict[str, Any], node_id_keys: dict[int, str]) -> str:
    from_key = node_id_keys[edge["from_node"]]
    to_key = node_id_keys[edge["to_node"]]
    props = identity_string(edge["properties"], sorted(edge["properties"]))
    prop_part = f" {props}" if props else ""
    return f"{from_key} -[{edge['label']}{prop_part}]-> {to_key}"


def _counter_delta(left: Counter[str], right: Counter[str]) -> list[str]:
    delta = left - right
    return sorted(key for key, count in delta.items() for _ in range(count))

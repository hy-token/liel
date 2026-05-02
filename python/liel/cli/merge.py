from __future__ import annotations

import argparse
import shutil
import tempfile
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
from .identity import (
    identity_string,
    load_identity_rules,
    normalize_node_key,
    record_properties,
)


def run(args: argparse.Namespace) -> int:
    payload = merge_files(
        args.left,
        args.right,
        args.output,
        dry_run=args.dry_run,
        force=args.force,
        node_key=args.node_key,
        identity_rules=args.identity_rules,
        edge_strategy=args.edge_strategy,
        on_node_conflict=args.on_node_conflict,
    )
    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_text(payload))
    return EXIT_OK


def merge_files(
    left_path: str | Path,
    right_path: str | Path,
    output_path: str | Path | None,
    *,
    dry_run: bool = False,
    force: bool = False,
    node_key: list[str] | None = None,
    identity_rules: str | Path | None = None,
    edge_strategy: str = "append",
    on_node_conflict: str = "keep_dst",
) -> dict[str, Any]:
    if node_key and identity_rules is not None:
        raise CliError("--node-key and --identity-rules cannot be used together", EXIT_USAGE)
    if not dry_run and output_path is None:
        raise CliError("merge output is required unless --dry-run is set", EXIT_USAGE)

    left = require_existing_file(left_path)
    right = require_existing_file(right_path)
    rules = load_identity_rules(identity_rules) if identity_rules is not None else None

    if dry_run:
        if rules is not None:
            return _preview_identity_rules_merge(
                left,
                right,
                output_path,
                rules=rules,
                edge_strategy=edge_strategy,
                on_node_conflict=on_node_conflict,
            )
        return _preview_merge(
            left,
            right,
            output_path,
            node_key=node_key,
            edge_strategy=edge_strategy,
            on_node_conflict=on_node_conflict,
        )

    output = refuse_overwrite(output_path, force=force)
    _reject_in_place_output(left, right, output)

    if rules is not None:
        return _merge_identity_rules_to_output(
            left,
            right,
            output,
            rules=rules,
            edge_strategy=edge_strategy,
            on_node_conflict=on_node_conflict,
        )

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(left, output)
        with liel.open(str(output)) as dst, liel.open(str(right)) as src:
            report = dst.merge_from(
                src,
                node_key=normalize_node_key(node_key),
                edge_strategy=edge_strategy,
                on_node_conflict=on_node_conflict,
            )
            dst.commit()
    except (OSError, ValueError, liel.GraphDBError) as exc:
        _remove_created_output(output)
        raise CliError(f"merge failed: {exc}", EXIT_ERROR) from exc

    return _report_payload(report, output)


def format_text(payload: dict[str, Any]) -> str:
    if payload.get("dry_run"):
        target = payload["output"] if payload["output"] is not None else "(no output path)"
        heading = f"Dry-run merge preview for {target}"
    else:
        heading = f"Merged into {payload['output']}"
    lines = [
        heading,
        f"Can merge: {'yes' if payload.get('can_merge', True) else 'no'}",
        f"Nodes: +{payload['nodes_created']} reused {payload['nodes_reused']}",
        f"Edges: +{payload['edges_created']} reused {payload['edges_reused']}",
    ]
    conflicts = payload.get("conflicts", [])
    if conflicts:
        lines.append(f"Conflicts: {len(conflicts)}")
        lines.extend(_conflict_text_lines(conflicts))
    warnings = payload.get("warnings", [])
    if warnings:
        lines.append(f"Warnings: {len(warnings)}")
        lines.extend(_warning_text_lines(warnings[:10]))
        if len(warnings) > 10:
            lines.append(f"- ... {len(warnings) - 10} more warning(s)")
    return "\n".join(lines)


def _reject_in_place_output(left: Path, right: Path, output: Path) -> None:
    resolved_output = output.resolve()
    if resolved_output in {left.resolve(), right.resolve()}:
        raise CliError("merge output must be different from both input files", EXIT_USAGE)


def _remove_created_output(path: Path) -> None:
    try:
        path.unlink(missing_ok=True)
    except OSError:
        pass


def _preview_merge(
    left: Path,
    right: Path,
    output_path: str | Path | None,
    *,
    node_key: list[str] | None,
    edge_strategy: str,
    on_node_conflict: str,
) -> dict[str, Any]:
    try:
        normalized_node_key = normalize_node_key(node_key)
        conflicts, warnings = _preflight_report(
            left,
            right,
            normalized_node_key,
            on_node_conflict=on_node_conflict,
        )
        if conflicts:
            return _conflict_payload(output_path, conflicts)

        with tempfile.TemporaryDirectory(prefix="liel-merge-dry-run-", dir=left.parent) as tmp:
            temp_output = Path(tmp) / "preview.liel"
            shutil.copyfile(left, temp_output)
            with liel.open(str(temp_output)) as dst, liel.open(str(right)) as src:
                report = dst.merge_from(
                    src,
                    node_key=normalized_node_key,
                    edge_strategy=edge_strategy,
                    on_node_conflict=on_node_conflict,
                )
        return _report_payload(
            report,
            Path(output_path) if output_path is not None else None,
            dry_run=True,
            warnings=warnings,
        )
    except (OSError, ValueError, liel.GraphDBError) as exc:
        raise CliError(f"merge preview failed: {exc}", EXIT_ERROR) from exc


def _preview_identity_rules_merge(
    left: Path,
    right: Path,
    output_path: str | Path | None,
    *,
    rules: dict[str, list[str]],
    edge_strategy: str,
    on_node_conflict: str,
) -> dict[str, Any]:
    try:
        conflicts, warnings = _identity_rule_preflight_report(
            left,
            right,
            rules,
            on_node_conflict=on_node_conflict,
        )
        if conflicts:
            return _conflict_payload(output_path, conflicts)

        with tempfile.TemporaryDirectory(prefix="liel-merge-dry-run-", dir=left.parent) as tmp:
            temp_output = Path(tmp) / "preview.liel"
            shutil.copyfile(left, temp_output)
            with liel.open(str(temp_output)) as dst, liel.open(str(right)) as src:
                payload = _merge_open_identity_rules(
                    dst,
                    src,
                    rules=rules,
                    edge_strategy=edge_strategy,
                    on_node_conflict=on_node_conflict,
                )
        payload["dry_run"] = True
        payload["output"] = str(output_path) if output_path is not None else None
        payload["warnings"] = warnings
        return payload
    except (OSError, ValueError, liel.GraphDBError) as exc:
        raise CliError(f"merge preview failed: {exc}", EXIT_ERROR) from exc


def _merge_identity_rules_to_output(
    left: Path,
    right: Path,
    output: Path,
    *,
    rules: dict[str, list[str]],
    edge_strategy: str,
    on_node_conflict: str,
) -> dict[str, Any]:
    conflicts, warnings = _identity_rule_preflight_report(
        left,
        right,
        rules,
        on_node_conflict=on_node_conflict,
    )
    if conflicts:
        raise CliError(
            f"merge conflicts: {len(conflicts)} conflict(s); rerun with --dry-run --format json",
            EXIT_USAGE,
        )

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(left, output)
        with liel.open(str(output)) as dst, liel.open(str(right)) as src:
            payload = _merge_open_identity_rules(
                dst,
                src,
                rules=rules,
                edge_strategy=edge_strategy,
                on_node_conflict=on_node_conflict,
            )
            dst.commit()
        payload["output"] = str(output)
        payload["warnings"] = warnings
        return payload
    except (OSError, ValueError, liel.GraphDBError) as exc:
        _remove_created_output(output)
        raise CliError(f"merge failed: {exc}", EXIT_ERROR) from exc


def _merge_open_identity_rules(
    dst: liel.GraphDB,
    src: liel.GraphDB,
    *,
    rules: dict[str, list[str]],
    edge_strategy: str,
    on_node_conflict: str,
) -> dict[str, Any]:
    src_nodes = src.all_nodes_as_records()
    src_edges = src.all_edges_as_records()
    dst_nodes = dst.all_nodes_as_records()
    dst_index = _index_identity_rule_records(
        dst_nodes,
        rules,
        side="destination",
        require_match=False,
    )

    node_id_map: dict[int, int] = {}
    edge_id_map: dict[int, int] = {}
    nodes_created = 0
    nodes_reused = 0
    edges_created = 0
    edges_reused = 0

    for src_record in src_nodes:
        identity, _ = _identity_rule_for_record(
            src_record,
            rules,
            side="source",
            require_match=True,
        )
        if identity in dst_index:
            dst_record = dst_index[identity][0]
            dst_id = dst_record["id"]
            _apply_node_conflict(
                dst,
                dst_id,
                dst_record,
                src_record,
                on_node_conflict=on_node_conflict,
            )
            nodes_reused += 1
        else:
            node = dst.add_node(src_record.get("labels", []), **record_properties(src_record))
            dst_id = node.id
            nodes_created += 1
        node_id_map[src_record["id"]] = dst_id

    for src_edge in src_edges:
        from_id = node_id_map[src_edge["from_node"]]
        to_id = node_id_map[src_edge["to_node"]]
        props = _edge_properties(src_edge)
        if edge_strategy == "append":
            edge = dst.add_edge(from_id, src_edge["label"], to_id, **props)
            edges_created += 1
        elif edge_strategy == "idempotent":
            before = dst.edge_count()
            edge = dst.merge_edge(from_id, src_edge["label"], to_id, **props)
            if dst.edge_count() > before:
                edges_created += 1
            else:
                edges_reused += 1
        else:
            raise ValueError("merge_from: invalid edge_strategy")
        edge_id_map[src_edge["id"]] = edge.id

    return {
        "dry_run": False,
        "can_merge": True,
        "conflicts": [],
        "warnings": [],
        "output": None,
        "nodes_created": nodes_created,
        "nodes_reused": nodes_reused,
        "edges_created": edges_created,
        "edges_reused": edges_reused,
        "node_id_map": node_id_map,
        "edge_id_map": edge_id_map,
    }


def _report_payload(
    report: liel.MergeReport,
    output: Path | None,
    *,
    dry_run: bool = False,
    warnings: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    return {
        "dry_run": dry_run,
        "can_merge": True,
        "conflicts": [],
        "warnings": warnings or [],
        "output": str(output) if output is not None else None,
        "nodes_created": report.nodes_created,
        "nodes_reused": report.nodes_reused,
        "edges_created": report.edges_created,
        "edges_reused": report.edges_reused,
        "node_id_map": report.node_id_map,
        "edge_id_map": report.edge_id_map,
    }


def _conflict_payload(
    output_path: str | Path | None, conflicts: list[dict[str, Any]]
) -> dict[str, Any]:
    return {
        "dry_run": True,
        "can_merge": False,
        "conflicts": conflicts,
        "warnings": [],
        "output": str(output_path) if output_path is not None else None,
        "nodes_created": 0,
        "nodes_reused": 0,
        "edges_created": 0,
        "edges_reused": 0,
        "node_id_map": {},
        "edge_id_map": {},
    }


def _preflight_report(
    left: Path,
    right: Path,
    node_key: list[str] | None,
    *,
    on_node_conflict: str,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    if node_key is None:
        return [], []

    try:
        with liel.open(str(left)) as dst, liel.open(str(right)) as src:
            dst_nodes = dst.all_nodes_as_records()
            src_nodes = src.all_nodes_as_records()
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"merge preview failed: {exc}", EXIT_ERROR) from exc

    conflicts: list[dict[str, Any]] = []
    warnings: list[dict[str, Any]] = []
    src_index = _index_nodes_for_preflight(src_nodes, node_key, side="source", conflicts=conflicts)
    dst_index = _index_nodes_for_preflight(
        dst_nodes,
        node_key,
        side="destination",
        conflicts=conflicts,
        missing_is_conflict=False,
    )

    src_keys = {key for key, ids in src_index.items() if len(ids) == 1}
    for key in sorted(src_keys):
        dst_records = dst_index.get(key, [])
        if len(dst_records) > 1:
            conflicts.append(
                {
                    "type": "ambiguous_destination_node_key",
                    "identity": key,
                    "node_ids": [record["id"] for record in dst_records],
                    "message": f"destination has multiple nodes matching source identity {key}",
                }
            )
        elif len(dst_records) == 1:
            src_record = src_index[key][0]
            dst_record = dst_records[0]
            warnings.extend(
                _node_conflict_warnings(
                    key,
                    dst_record,
                    src_record,
                    on_node_conflict=on_node_conflict,
                )
            )
    return conflicts, warnings


def _identity_rule_preflight_report(
    left: Path,
    right: Path,
    rules: dict[str, list[str]],
    *,
    on_node_conflict: str,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    try:
        with liel.open(str(left)) as dst, liel.open(str(right)) as src:
            dst_nodes = dst.all_nodes_as_records()
            src_nodes = src.all_nodes_as_records()
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"merge preview failed: {exc}", EXIT_ERROR) from exc

    conflicts: list[dict[str, Any]] = []
    warnings: list[dict[str, Any]] = []
    src_index = _index_identity_rule_records(
        src_nodes,
        rules,
        side="source",
        require_match=True,
        conflicts=conflicts,
    )
    dst_index = _index_identity_rule_records(
        dst_nodes,
        rules,
        side="destination",
        require_match=False,
        conflicts=conflicts,
    )

    src_keys = {key for key, records in src_index.items() if len(records) == 1}
    for key in sorted(src_keys):
        dst_records = dst_index.get(key, [])
        if len(dst_records) == 1:
            warnings.extend(
                _node_conflict_warnings(
                    key,
                    dst_records[0],
                    src_index[key][0],
                    on_node_conflict=on_node_conflict,
                )
            )
    return conflicts, warnings


def _index_identity_rule_records(
    nodes: list[dict[str, Any]],
    rules: dict[str, list[str]],
    *,
    side: str,
    require_match: bool,
    conflicts: list[dict[str, Any]] | None = None,
) -> dict[str, list[dict[str, Any]]]:
    indexed: dict[str, list[dict[str, Any]]] = {}
    for record in nodes:
        identity, conflict = _identity_rule_for_record(
            record,
            rules,
            side=side,
            require_match=require_match,
        )
        if conflict is not None:
            if conflicts is not None:
                conflicts.append(conflict)
            continue
        if identity is not None:
            indexed.setdefault(identity, []).append(record)

    for identity, records in sorted(indexed.items()):
        if len(records) > 1 and conflicts is not None:
            node_ids = [record["id"] for record in records]
            conflicts.append(
                {
                    "type": "duplicate_identity_rule",
                    "side": side,
                    "identity": identity,
                    "node_ids": node_ids,
                    "message": f"{side} identity rule is not unique: {identity}",
                }
            )
    return indexed


def _identity_rule_for_record(
    record: dict[str, Any],
    rules: dict[str, list[str]],
    *,
    side: str,
    require_match: bool,
) -> tuple[str | None, dict[str, Any] | None]:
    matched_labels = [label for label in sorted(record.get("labels", [])) if label in rules]
    node_id = record["id"]
    if not matched_labels:
        if require_match:
            return None, {
                "type": "unmatched_identity_rule",
                "side": side,
                "node_id": node_id,
                "labels": record.get("labels", []),
                "message": f"{side} node {node_id} does not match any identity rule label",
            }
        return None, None
    if len(matched_labels) > 1:
        return None, {
            "type": "multiple_identity_rules",
            "side": side,
            "node_id": node_id,
            "labels": matched_labels,
            "message": f"{side} node {node_id} matches multiple identity rule labels: {', '.join(matched_labels)}",
        }

    label = matched_labels[0]
    keys = rules[label]
    properties = record_properties(record)
    missing = [key for key in keys if key not in properties]
    if missing:
        return None, {
            "type": "missing_identity_rule_key",
            "side": side,
            "node_id": node_id,
            "label": label,
            "missing_keys": missing,
            "message": f"{side} node {node_id} is missing identity rule key: {label}.{', '.join(missing)}",
        }
    return f"{label}:{identity_string(properties, keys)}", None


def _index_nodes_for_preflight(
    nodes: list[dict[str, Any]],
    node_key: list[str],
    *,
    side: str,
    conflicts: list[dict[str, Any]],
    missing_is_conflict: bool = True,
) -> dict[str, list[dict[str, Any]]]:
    indexed: dict[str, list[dict[str, Any]]] = {}
    for record in nodes:
        node_id = record["id"]
        properties = record_properties(record)
        missing = [key for key in node_key if key not in properties]
        if missing:
            if missing_is_conflict:
                conflicts.append(
                    {
                        "type": "missing_node_key",
                        "side": side,
                        "node_id": node_id,
                        "missing_keys": missing,
                        "message": f"{side} node {node_id} is missing node key: {', '.join(missing)}",
                    }
                )
            continue
        identity = identity_string(properties, node_key)
        indexed.setdefault(identity, []).append(record)

    for identity, records in sorted(indexed.items()):
        if len(records) > 1:
            conflicts.append(
                {
                    "type": "duplicate_node_key",
                    "side": side,
                    "identity": identity,
                    "node_ids": [record["id"] for record in records],
                    "message": f"{side} node key is not unique: {identity}",
                }
            )
    return indexed


def _apply_node_conflict(
    dst: liel.GraphDB,
    dst_id: int,
    dst_record: dict[str, Any],
    src_record: dict[str, Any],
    *,
    on_node_conflict: str,
) -> None:
    if on_node_conflict == "keep_dst":
        return
    if on_node_conflict == "overwrite_from_src":
        dst.update_node(dst_id, **record_properties(src_record))
        return
    if on_node_conflict == "merge_props":
        merged = record_properties(dst_record)
        for key, value in record_properties(src_record).items():
            merged.setdefault(key, value)
        dst.update_node(dst_id, **merged)
        return
    raise ValueError("merge_from: invalid on_node_conflict")


def _node_conflict_warnings(
    identity: str,
    dst_record: dict[str, Any],
    src_record: dict[str, Any],
    *,
    on_node_conflict: str,
) -> list[dict[str, Any]]:
    warnings: list[dict[str, Any]] = []
    dst_props = record_properties(dst_record)
    src_props = record_properties(src_record)
    for key in sorted(set(dst_props) & set(src_props)):
        dst_value = dst_props[key]
        src_value = src_props[key]
        if dst_value == src_value:
            continue
        if on_node_conflict == "keep_dst":
            resolution = "source_ignored"
        elif on_node_conflict == "overwrite_from_src":
            resolution = "destination_overwritten"
        elif on_node_conflict == "merge_props":
            resolution = "source_ignored"
        else:
            raise ValueError("merge_from: invalid on_node_conflict")
        warnings.append(
            {
                "type": "node_property_conflict",
                "identity": identity,
                "property": key,
                "destination": dst_value,
                "source": src_value,
                "policy": on_node_conflict,
                "resolution": resolution,
                "message": (
                    f"{identity} property {key!r} differs; {resolution} by {on_node_conflict}"
                ),
            }
        )

    dst_labels = sorted(dst_record.get("labels", []))
    src_labels = sorted(src_record.get("labels", []))
    if dst_labels != src_labels:
        warnings.append(
            {
                "type": "node_label_difference",
                "identity": identity,
                "destination": dst_labels,
                "source": src_labels,
                "policy": on_node_conflict,
                "resolution": "destination_labels_kept",
                "message": f"{identity} labels differ; destination labels are kept",
            }
        )
    return warnings


def _edge_properties(record: dict[str, Any]) -> dict[str, Any]:
    return {
        key: record[key]
        for key in sorted(record)
        if key not in {"id", "label", "from_node", "to_node"}
    }


def _conflict_text_lines(conflicts: list[dict[str, Any]]) -> list[str]:
    lines: list[str] = []
    for conflict in conflicts:
        lines.append(f"- {conflict['type']}: {conflict['message']}")
    return lines


def _warning_text_lines(warnings: list[dict[str, Any]]) -> list[str]:
    lines: list[str] = []
    for warning in warnings:
        lines.append(f"- {warning['type']}: {warning['message']}")
    return lines

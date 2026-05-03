"""Shortest-path CLI (`liel trace`) — aligns with MCP `liel_trace` / `GraphDB.shortest_path`."""

from __future__ import annotations

import argparse
import re
from typing import Any

import liel

from .common import EXIT_OK, EXIT_USAGE, CliError, emit_json, emit_text, require_existing_file


class _EdgeLite:
    __slots__ = ("id", "label", "from_node", "to_node", "properties")

    _RESERVED = ("id", "label", "from_node", "to_node")

    def __init__(self, record: dict[str, Any]) -> None:
        self.id = record["id"]
        self.label = record["label"]
        self.from_node = record["from_node"]
        self.to_node = record["to_node"]
        self.properties = {k: v for k, v in record.items() if k not in self._RESERVED}


def _node_to_dict(node: Any) -> dict[str, Any]:
    return {"id": node.id, "labels": list(node.labels), **node.properties}


def _node_title(node: dict[str, Any]) -> str:
    props = {k: v for k, v in node.items() if k not in ("id", "labels")}
    if "title" in props:
        return str(props["title"])
    if "path" in props:
        return str(props["path"])
    if "name" in props:
        return str(props["name"])
    labels = node.get("labels") or []
    if labels:
        return f"{'/'.join(str(x) for x in labels)} #{node['id']}"
    return f"node {node['id']}"


def _node_kind(node: dict[str, Any]) -> str:
    labels = node.get("labels") or []
    if not labels:
        return "node"
    if len(labels) == 1:
        return str(labels[0])
    return "/".join(str(x) for x in labels)


def _path_row_title(node: dict[str, Any]) -> str:
    """One-line label for collapsed path (prefer file path for File nodes)."""
    if "File" in (node.get("labels") or []) and node.get("path"):
        return str(node["path"])
    return _node_title(node)


def _first_decision_on_path(path: list[dict[str, Any]]) -> dict[str, Any] | None:
    for n in path:
        if "Decision" in (n.get("labels") or []):
            return n
    return None


def _why_bullets(decision: dict[str, Any]) -> list[str]:
    raw = decision.get("reason")
    if raw is None or raw == "":
        return []
    parts = [p.strip() for p in re.split(r"\s*;\s*", str(raw)) if p.strip()]
    return parts


def _key_factor_line(tgt: dict[str, Any], *, on_path_option: bool) -> str:
    """One line for Key factors: incident → compliance → technical (on-path Option)."""
    kf = tgt.get("key_factor")
    if isinstance(kf, str) and kf.strip():
        return kf.strip()
    labs = tgt.get("labels") or []
    if on_path_option and "Option" in labs:
        summ = tgt.get("summary")
        if isinstance(summ, str) and " and " in summ:
            return summ.split(" and ", 1)[0].strip()
    return _node_title(tgt)


def _rejected_alternative_line(tgt: dict[str, Any]) -> str:
    title = _node_title(tgt)
    short = title.removeprefix("Use ").strip() if title.startswith("Use ") else title
    note = tgt.get("rejection_note")
    if isinstance(note, str) and note.strip():
        return f"{short} ({note.strip()})"
    return short


def _narrative_key_factors_and_rejected(
    branches: list[dict[str, Any]], path_ids: set[int]
) -> tuple[list[str], list[str]]:
    """Key factors: LEARNED_FROM, then CONSTRAINED_BY, then chosen Option (by target id)."""
    learned: list[tuple[int, str]] = []
    constrained: list[tuple[int, str]] = []
    onpath_option: list[tuple[int, str]] = []
    rejected: list[str] = []

    for b in branches:
        label = b["edge_label"]
        tgt = b["target"]
        tid = int(tgt["id"])
        labs = tgt.get("labels") or []

        if label == "LEARNED_FROM":
            learned.append((tid, _key_factor_line(tgt, on_path_option=False)))
        elif label == "CONSTRAINED_BY":
            constrained.append((tid, _key_factor_line(tgt, on_path_option=False)))
        elif label == "CONSIDERED" and "Option" in labs:
            if tid in path_ids:
                onpath_option.append((tid, _key_factor_line(tgt, on_path_option=True)))
            else:
                rejected.append(_rejected_alternative_line(tgt))

    learned.sort(key=lambda t: t[0])
    constrained.sort(key=lambda t: t[0])
    onpath_option.sort(key=lambda t: t[0])
    factors = [line for _, line in learned + constrained + onpath_option]
    return factors, rejected


def _collect_reasoning_branches(db: Any, from_id: int) -> list[dict[str, Any]]:
    edges = sorted(db.out_edges(from_id), key=lambda e: (e.label, e.to_node))
    rows: list[dict[str, Any]] = []
    for e in edges:
        tgt = db.get_node(e.to_node)
        if tgt is None:
            continue
        rows.append({"edge_label": e.label, "target": _node_to_dict(tgt)})
    return rows


def _path_hop_labels(db: Any, node_path: list[Any], edge_filter: str | None) -> list[str]:
    hops: list[str] = []
    for i in range(len(node_path) - 1):
        fr_id = node_path[i].id
        to_id = node_path[i + 1].id
        label = "?"
        for edge in db.out_edges(fr_id):
            if edge.to_node != to_id:
                continue
            if edge_filter and edge.label != edge_filter:
                continue
            label = edge.label
            break
        hops.append(label)
    return hops


def _to_mermaid(nodes: list[Any], edges: list[_EdgeLite]) -> str:
    lines = ["graph LR"]
    for n in nodes:
        label_part = "/".join(n.labels) if n.labels else str(n.id)
        name = n.properties.get("name") or n.properties.get("title") or ""
        display = f"{label_part}:{name}" if name else label_part
        lines.append(f'    N{n.id}["{display}"]')
    for e in edges:
        lines.append(f"    N{e.from_node} -->|{e.label}| N{e.to_node}")
    return "\n".join(lines)


def build_trace_payload(
    db: Any,
    *,
    from_node: int,
    to_node: int,
    edge_label: str,
    source_path: str,
) -> dict[str, Any]:
    el = edge_label if edge_label else None
    node_path = db.shortest_path(from_node, to_node, el)

    base: dict[str, Any] = {
        "source": source_path,
        "from_node": from_node,
        "to_node": to_node,
        "edge_label": edge_label,
    }

    branches = _collect_reasoning_branches(db, from_node)

    if node_path is None:
        out = dict(base)
        out["path"] = None
        out["mermaid"] = ""
        out["reasoning_branches"] = branches
        out["path_hop_labels"] = []
        return out

    path_ids = {n.id for n in node_path}
    edge_records = db.edges_between(path_ids)
    if el:
        edge_records = [e for e in edge_records if e.get("label") == el]

    edges = [_EdgeLite(r) for r in edge_records]
    out = dict(base)
    out["path"] = [_node_to_dict(n) for n in node_path]
    out["path_hop_labels"] = _path_hop_labels(db, node_path, el)
    out["reasoning_branches"] = branches
    out["mermaid"] = _to_mermaid(node_path, edges)
    return out


def format_text(payload: dict[str, Any], *, include_mermaid: bool = True) -> str:
    el = payload.get("edge_label") or ""
    path = payload.get("path")
    branches = payload.get("reasoning_branches") or []
    lines: list[str] = []

    if path is None:
        lines.append("Trace: (no path found)")
        if el:
            lines.append(f"Edge label filter: {el}")
        lines.append("")
        lines.append("No route under the given constraints.")
        return "\n".join(lines)

    start, end = path[0], path[-1]
    hook = start.get("trace_prompt")
    if isinstance(hook, str) and hook.strip():
        lines.append(f"Trace: {hook.strip()}")
    else:
        lines.append(f"Trace: {_node_title(start)} → {_node_title(end)}")
    if el:
        lines.append(f"Edge label filter: {el}")
    lines.append("")

    path_ids = {int(n["id"]) for n in path}
    decision = _first_decision_on_path(path)

    if decision:
        lines.append("Decision found:")
        lines.append(_node_title(decision))
        lines.append("")
        bullets = _why_bullets(decision)
        if bullets:
            lines.append("Why:")
            for b in bullets:
                lines.append(f"  ✓ {b}")
            lines.append("")

    factors, rejected_opts = _narrative_key_factors_and_rejected(branches, path_ids)
    if factors:
        lines.append("Key factors:")
        for t in factors:
            lines.append(f"  ✓ {t}")
        lines.append("")

    if rejected_opts:
        lines.append("Rejected:")
        for t in rejected_opts:
            lines.append(f"  ✗ {t}")
        lines.append("")

    if "File" in (end.get("labels") or []):
        lines.append("Implemented in:")
        lines.append(f"  {_path_row_title(end)}")
        lines.append("")

    lines.append("Path:")
    for i, n in enumerate(path):
        prefix = "  " if i == 0 else "  → "
        lines.append(f"{prefix}{_path_row_title(n)}")

    if include_mermaid and payload.get("mermaid"):
        lines.append("")
        lines.append("Mermaid:")
        lines.append(payload["mermaid"])
    return "\n".join(lines)


def run(args: argparse.Namespace) -> int:
    source = require_existing_file(args.source)
    if args.from_node <= 0 or args.to_node <= 0:
        raise CliError("'--from' and '--to' must be positive node IDs", EXIT_USAGE)

    try:
        with liel.open(str(source)) as db:
            payload = build_trace_payload(
                db,
                from_node=args.from_node,
                to_node=args.to_node,
                edge_label=args.edge_label or "",
                source_path=str(source.resolve()),
            )
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"trace failed: {exc}", 1) from exc

    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_text(payload, include_mermaid=not args.no_mermaid))
    return EXIT_OK

"""MCP server for liel - exposes the graph as AI-readable tools.

Primary tools:
  liel_overview  - schema / memory overview (labels, stats)
  liel_find      - find nodes by label / property filter
  liel_explore   - BFS neighbourhood around a node
  liel_trace     - shortest path between two nodes
  liel_map       - render a subgraph as a Mermaid diagram
  liel_append    - append nodes and edges in one atomic commit
  liel_merge     - merge nodes and edges in one atomic commit

Every tool returns a JSON string. Success responses are arbitrary payloads
specific to the tool; failures always share the same shape:

    {"error": {"code": "<slug>", "message": "<human-readable text>"}}
"""

from __future__ import annotations

import atexit
import json
import os
import pathlib
from collections.abc import Iterable
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import liel as _liel_types  # noqa: F401


_QUERY_LIMIT_MAX = 100
_QUERY_LIMIT_DEFAULT = 20
_EXPLORE_DEPTH_MAX = 4
_EXPLORE_DEPTH_DEFAULT = 2
_EXPLORE_NODES_MAX = 50
_MERMAID_LIMIT_MAX = 100
_MERMAID_LIMIT_DEFAULT = 30
_INSPECT_SAMPLE_SIZE = 5


def _err(code: str, message: str) -> str:
    return json.dumps(
        {"error": {"code": code, "message": message}},
        ensure_ascii=False,
    )


def _clamp(value: int, *, lo: int, hi: int) -> int:
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value


def _require_mcp() -> Any:
    try:
        from mcp.server.fastmcp import FastMCP  # type: ignore[import]

        return FastMCP
    except ImportError as exc:
        raise ImportError(
            'liel MCP support requires the mcp package.\nInstall it with:  pip install "liel[mcp]"'
        ) from exc


def _discover_liel_file() -> str:
    cwd = pathlib.Path.cwd()
    for p in sorted(cwd.rglob("*.liel")):
        return str(p)
    raise FileNotFoundError(
        "No .liel file found under the current directory. Pass --path explicitly."
    )


def _node_to_dict(node: Any) -> dict[str, Any]:
    return {"id": node.id, "labels": node.labels, **node.properties}


def _edge_to_dict(edge: Any) -> dict[str, Any]:
    return {
        "id": edge.id,
        "label": edge.label,
        "from": edge.from_node,
        "to": edge.to_node,
        **edge.properties,
    }


class _EdgeRecord:
    __slots__ = ("id", "label", "from_node", "to_node", "properties")

    _RESERVED = ("id", "label", "from_node", "to_node")

    def __init__(self, record: dict[str, Any]) -> None:
        self.id = record["id"]
        self.label = record["label"]
        self.from_node = record["from_node"]
        self.to_node = record["to_node"]
        self.properties = {k: v for k, v in record.items() if k not in self._RESERVED}


def _records_to_edges(records: Iterable[dict[str, Any]]) -> list[_EdgeRecord]:
    return [_EdgeRecord(r) for r in records]


def _to_mermaid(nodes: list[Any], edges: list[Any]) -> str:
    lines = ["graph LR"]
    for n in nodes:
        label_part = "/".join(n.labels) if n.labels else str(n.id)
        name = n.properties.get("name") or n.properties.get("title") or ""
        display = f"{label_part}:{name}" if name else label_part
        lines.append(f'    N{n.id}["{display}"]')
    for e in edges:
        lines.append(f"    N{e.from_node} -->|{e.label}| N{e.to_node}")
    return "\n".join(lines)


def _do_inspect(db: Any, path: str) -> str:
    all_nodes = db.all_nodes()
    all_edges = db.all_edges()

    node_labels: dict[str, int] = {}
    for n in all_nodes:
        for lbl in n.labels:
            node_labels[lbl] = node_labels.get(lbl, 0) + 1

    edge_labels: dict[str, int] = {}
    for e in all_edges:
        edge_labels[e.label] = edge_labels.get(e.label, 0) + 1

    stats = db.degree_stats()
    if stats:
        top_ids = sorted(stats, key=lambda nid: sum(stats[nid]), reverse=True)
        top_ids = top_ids[:_INSPECT_SAMPLE_SIZE]
        sample_nodes = [db.get_node(nid) for nid in top_ids]
        samples = [_node_to_dict(n) for n in sample_nodes if n is not None]
    else:
        samples = [_node_to_dict(n) for n in all_nodes[:_INSPECT_SAMPLE_SIZE]]

    result = {
        "node_count": len(all_nodes),
        "edge_count": len(all_edges),
        "node_labels": node_labels,
        "edge_labels": edge_labels,
        "sample_nodes": samples,
        "db_path": path,
    }
    return json.dumps(result, ensure_ascii=False, default=str)


def _do_query(
    db: Any,
    label: str = "",
    where: str = "",
    limit: int = _QUERY_LIMIT_DEFAULT,
    cursor: int = 0,
) -> str:
    limit = _clamp(limit, lo=1, hi=_QUERY_LIMIT_MAX)
    cursor = max(0, cursor)

    filters: dict[str, Any] = {}
    if where:
        try:
            parsed = json.loads(where)
        except json.JSONDecodeError as exc:
            return _err("invalid_json", f"'where' is not valid JSON: {exc}")
        if not isinstance(parsed, dict):
            return _err(
                "invalid_where",
                "'where' must decode to a JSON object of equality filters",
            )
        filters = parsed

    q = db.nodes()
    if label:
        q = q.label(label)
    if filters:
        q = q.where_(lambda n, f=filters: all(n.get(k) == v for k, v in f.items()))
    q = q.skip(cursor).limit(limit)

    results = [_node_to_dict(n) for n in q.fetch()]
    next_cursor: int | None = cursor + len(results) if len(results) == limit else None
    return json.dumps(
        {"nodes": results, "next_cursor": next_cursor},
        ensure_ascii=False,
        default=str,
    )


def _do_explore(
    db: Any,
    node_id: int,
    max_depth: int = _EXPLORE_DEPTH_DEFAULT,
    edge_label: str = "",
) -> str:
    if not isinstance(node_id, int) or node_id <= 0:
        return _err("invalid_node_id", "'node_id' must be a positive integer")
    max_depth = _clamp(max_depth, lo=0, hi=_EXPLORE_DEPTH_MAX)
    el = edge_label if edge_label else None

    center_node = db.get_node(node_id)
    if center_node is None:
        return _err("node_not_found", f"Node {node_id} not found")

    bfs_results = db.bfs(node_id, max_depth)
    reachable_nodes = [center_node] + [n for n, _ in bfs_results]

    truncated = False
    if len(reachable_nodes) > _EXPLORE_NODES_MAX:
        reachable_nodes = reachable_nodes[:_EXPLORE_NODES_MAX]
        truncated = True

    reachable_ids = {n.id for n in reachable_nodes}
    edge_records = db.edges_between(reachable_ids)
    if el:
        edge_records = [e for e in edge_records if e.get("label") == el]

    edges = _records_to_edges(edge_records)

    result = {
        "center": _node_to_dict(center_node),
        "nodes": [_node_to_dict(n) for n in reachable_nodes],
        "edges": [_edge_to_dict(e) for e in edges],
        "mermaid": _to_mermaid(reachable_nodes, edges),
        "truncated": truncated,
    }
    return json.dumps(result, ensure_ascii=False, default=str)


def _do_path(
    db: Any,
    from_node: int,
    to_node: int,
    edge_label: str = "",
) -> str:
    if not isinstance(from_node, int) or from_node <= 0:
        return _err("invalid_node_id", "'from_node' must be a positive integer")
    if not isinstance(to_node, int) or to_node <= 0:
        return _err("invalid_node_id", "'to_node' must be a positive integer")

    el = edge_label if edge_label else None
    node_path = db.shortest_path(from_node, to_node, el)

    if node_path is None:
        return json.dumps({"path": None})

    path_ids = {n.id for n in node_path}
    edge_records = db.edges_between(path_ids)
    if el:
        edge_records = [e for e in edge_records if e.get("label") == el]

    edges = _records_to_edges(edge_records)

    result = {
        "path": [_node_to_dict(n) for n in node_path],
        "mermaid": _to_mermaid(node_path, edges),
    }
    return json.dumps(result, ensure_ascii=False, default=str)


def _do_mermaid(
    db: Any,
    node_ids: str = "",
    limit: int = _MERMAID_LIMIT_DEFAULT,
) -> str:
    limit = _clamp(limit, lo=1, hi=_MERMAID_LIMIT_MAX)

    if node_ids.strip():
        try:
            ids = {int(x.strip()) for x in node_ids.split(",") if x.strip()}
        except ValueError:
            return _err(
                "invalid_node_ids",
                "'node_ids' must be comma-separated positive integers",
            )
        if any(i <= 0 for i in ids):
            return _err(
                "invalid_node_ids",
                "'node_ids' must be comma-separated positive integers",
            )
        nodes = [db.get_node(i) for i in ids]
        nodes = [n for n in nodes if n is not None]
    else:
        nodes = db.all_nodes()[:limit]

    if not nodes:
        return json.dumps({"mermaid": "graph LR\n    empty[No nodes found]"}, ensure_ascii=False)

    node_set = {n.id for n in nodes}
    edge_records = db.edges_between(node_set)
    edges = _records_to_edges(edge_records)
    return json.dumps({"mermaid": _to_mermaid(nodes, edges)}, ensure_ascii=False)


def _parse_label_list(raw_labels: Any) -> list[str]:
    if isinstance(raw_labels, str):
        return [label.strip() for label in raw_labels.split(",") if label.strip()]
    if isinstance(raw_labels, list):
        labels: list[str] = []
        for label in raw_labels:
            if not isinstance(label, str):
                return []
            stripped = label.strip()
            if stripped:
                labels.append(stripped)
        return labels
    return []


def _parse_json_array(
    raw: str, *, field: str, item_name: str
) -> tuple[list[Any] | None, str | None]:
    try:
        parsed = json.loads(raw) if raw.strip() else []
    except json.JSONDecodeError as exc:
        return None, _err("invalid_json", f"'{field}' is not valid JSON: {exc}")
    if not isinstance(parsed, list):
        return None, _err(
            f"invalid_{field}",
            f"'{field}' must decode to a JSON array of {item_name}",
        )
    return parsed, None


def _normalize_props(
    raw_props: Any, *, field_name: str
) -> tuple[dict[str, Any] | None, str | None]:
    if raw_props is None:
        return {}, None
    if not isinstance(raw_props, dict):
        return None, _err("invalid_props", f"'{field_name}' must be an object")
    return dict(raw_props), None


def _apply_creation_defaults(properties: dict[str, Any], _session: str) -> dict[str, Any]:
    return dict(properties)


def _resolve_node_pointer(
    value: Any,
    *,
    ref_map: dict[str, int],
    field_name: str,
) -> tuple[int | None, str | None]:
    if isinstance(value, int) and value > 0:
        return value, None
    if isinstance(value, str) and value.strip():
        ref = value.strip()
        if ref not in ref_map:
            return None, _err(
                "unknown_ref",
                f"Unknown node reference '{ref}' in '{field_name}'",
            )
        return ref_map[ref], None
    return None, _err(
        "invalid_edge_endpoint",
        f"'{field_name}' must be a positive integer node id or a known ref string",
    )


def _find_existing_node(
    db: Any,
    *,
    labels: list[str],
    match: dict[str, Any],
) -> tuple[Any | None, str | None]:
    def _matches(node: Any) -> bool:
        if labels and not all(label in node.labels for label in labels):
            return False
        return all(node.get(k) == v for k, v in match.items())

    matches = db.nodes().where_(_matches).limit(2).fetch()
    if len(matches) > 1:
        return None, _err(
            "ambiguous_match",
            "Multiple nodes matched the requested merge criteria",
        )
    return (matches[0] if matches else None), None


def _parse_edge_items(raw_edges: str) -> tuple[list[dict[str, Any]] | None, str | None]:
    parsed, parse_error = _parse_json_array(raw_edges, field="edges", item_name="edge objects")
    if parse_error is not None:
        return None, parse_error
    assert parsed is not None

    edges: list[dict[str, Any]] = []
    for idx, item in enumerate(parsed):
        if not isinstance(item, dict):
            return None, _err("invalid_edges", f"edges[{idx}] must be an object")
        label = item.get("label")
        if not isinstance(label, str) or not label.strip():
            return None, _err("invalid_edges", f"edges[{idx}].label must be a non-empty string")
        props, props_error = _normalize_props(
            item.get("props", {}), field_name=f"edges[{idx}].props"
        )
        if props_error is not None:
            return None, props_error
        assert props is not None
        edges.append(
            {
                "from": item.get("from"),
                "to": item.get("to"),
                "label": label.strip(),
                "props": props,
            }
        )
    return edges, None


def _create_edge_batch(
    db: Any,
    *,
    parsed_edges: list[dict[str, Any]],
    ref_map: dict[str, int],
    dedupe: bool,
) -> tuple[list[dict[str, Any]] | None, str | None]:
    created_edges: list[dict[str, Any]] = []
    for idx, item in enumerate(parsed_edges):
        from_id, from_error = _resolve_node_pointer(
            item["from"], ref_map=ref_map, field_name=f"edges[{idx}].from"
        )
        if from_error is not None:
            return None, from_error
        to_id, to_error = _resolve_node_pointer(
            item["to"], ref_map=ref_map, field_name=f"edges[{idx}].to"
        )
        if to_error is not None:
            return None, to_error
        assert from_id is not None and to_id is not None

        src = db.get_node(from_id)
        dst = db.get_node(to_id)
        if src is None or dst is None:
            missing = from_id if src is None else to_id
            return None, _err(
                "node_not_found",
                f"Target node {missing} not found; rolled back",
            )

        if dedupe:
            edge = db.merge_edge(src, item["label"], dst, **item["props"])
        else:
            edge = db.add_edge(src, item["label"], dst, **item["props"])
        created_edges.append(_edge_to_dict(edge))
    return created_edges, None


def _do_append(
    db: Any,
    nodes: str = "[]",
    edges: str = "[]",
    session: str = "",
) -> str:
    parsed_nodes, node_parse_error = _parse_json_array(
        nodes,
        field="nodes",
        item_name="node objects",
    )
    if node_parse_error is not None:
        return node_parse_error
    parsed_edges, edge_parse_error = _parse_edge_items(edges)
    if edge_parse_error is not None:
        return edge_parse_error
    assert parsed_nodes is not None and parsed_edges is not None

    created_nodes: list[dict[str, Any]] = []
    ref_map: dict[str, int] = {}

    for idx, item in enumerate(parsed_nodes):
        if not isinstance(item, dict):
            db.rollback()
            return _err("invalid_nodes", f"nodes[{idx}] must be an object")

        labels = _parse_label_list(item.get("labels", []))
        if not labels:
            db.rollback()
            return _err("invalid_labels", f"nodes[{idx}].labels must not be empty")

        props, props_error = _normalize_props(
            item.get("props", {}), field_name=f"nodes[{idx}].props"
        )
        if props_error is not None:
            db.rollback()
            return props_error
        assert props is not None

        ref = item.get("ref", "")
        if ref:
            if not isinstance(ref, str) or not ref.strip():
                db.rollback()
                return _err("invalid_ref", f"nodes[{idx}].ref must be a non-empty string")
            ref = ref.strip()
            if ref in ref_map:
                db.rollback()
                return _err("duplicate_ref", f"nodes[{idx}].ref '{ref}' is duplicated")

        new_node = db.add_node(labels, **_apply_creation_defaults(props, session))
        created_nodes.append(_node_to_dict(new_node))
        if ref:
            ref_map[ref] = new_node.id

    created_edges, create_edge_error = _create_edge_batch(
        db,
        parsed_edges=parsed_edges,
        ref_map=ref_map,
        dedupe=False,
    )
    if create_edge_error is not None:
        db.rollback()
        return create_edge_error
    assert created_edges is not None

    db.commit()
    return json.dumps(
        {
            "created_nodes": created_nodes,
            "created_edges": created_edges,
            "ref_map": ref_map,
        },
        ensure_ascii=False,
        default=str,
    )


def _do_merge(
    db: Any,
    nodes: str = "[]",
    edges: str = "[]",
    session: str = "",
) -> str:
    parsed_nodes, node_parse_error = _parse_json_array(
        nodes,
        field="nodes",
        item_name="node objects",
    )
    if node_parse_error is not None:
        return node_parse_error
    parsed_edges, edge_parse_error = _parse_edge_items(edges)
    if edge_parse_error is not None:
        return edge_parse_error
    assert parsed_nodes is not None and parsed_edges is not None

    created_nodes: list[dict[str, Any]] = []
    reused_nodes: list[dict[str, Any]] = []
    ref_map: dict[str, int] = {}

    for idx, item in enumerate(parsed_nodes):
        if not isinstance(item, dict):
            db.rollback()
            return _err("invalid_nodes", f"nodes[{idx}] must be an object")

        ref = item.get("ref", "")
        if ref:
            if not isinstance(ref, str) or not ref.strip():
                db.rollback()
                return _err("invalid_ref", f"nodes[{idx}].ref must be a non-empty string")
            ref = ref.strip()
            if ref in ref_map:
                db.rollback()
                return _err("duplicate_ref", f"nodes[{idx}].ref '{ref}' is duplicated")

        props, props_error = _normalize_props(
            item.get("props", {}), field_name=f"nodes[{idx}].props"
        )
        if props_error is not None:
            db.rollback()
            return props_error
        match, match_error = _normalize_props(
            item.get("match", {}), field_name=f"nodes[{idx}].match"
        )
        if match_error is not None:
            db.rollback()
            return match_error
        assert props is not None and match is not None

        labels = _parse_label_list(item.get("labels", []))
        direct_id = item.get("id")

        if direct_id is not None:
            if not isinstance(direct_id, int) or direct_id <= 0:
                db.rollback()
                return _err("invalid_node_id", f"nodes[{idx}].id must be a positive integer")
            existing = db.get_node(direct_id)
            if existing is None:
                db.rollback()
                return _err("node_not_found", f"Target node {direct_id} not found; rolled back")
        elif match:
            existing, find_error = _find_existing_node(db, labels=labels, match=match)
            if find_error is not None:
                db.rollback()
                return find_error
        else:
            existing = None

        if existing is None:
            if not labels:
                db.rollback()
                return _err(
                    "invalid_labels",
                    f"nodes[{idx}].labels must not be empty when merge creates a new node",
                )
            new_node = db.add_node(labels, **_apply_creation_defaults(props, session))
            created_nodes.append(_node_to_dict(new_node))
            resolved = new_node
        else:
            merged_props = dict(existing.properties)
            merged_props.update(props)
            db.update_node(existing.id, **merged_props)
            resolved = db.get_node(existing.id)
            assert resolved is not None
            reused_nodes.append(_node_to_dict(resolved))

        if ref:
            ref_map[ref] = resolved.id

    merged_edges, create_edge_error = _create_edge_batch(
        db,
        parsed_edges=parsed_edges,
        ref_map=ref_map,
        dedupe=True,
    )
    if create_edge_error is not None:
        db.rollback()
        return create_edge_error
    assert merged_edges is not None

    db.commit()
    return json.dumps(
        {
            "created_nodes": created_nodes,
            "merged_nodes": reused_nodes,
            "merged_edges": merged_edges,
            "ref_map": ref_map,
        },
        ensure_ascii=False,
        default=str,
    )


def create_server(path: str | None = None) -> Any:
    FastMCP = _require_mcp()

    if path is None:
        path = _discover_liel_file()

    import liel  # noqa: PLC0415

    db = liel.open(path)

    def _cleanup() -> None:
        try:
            db.close()
        except Exception as exc:
            import sys
            import traceback

            print(
                f"liel: warning: db.close() raised {exc!r} during atexit cleanup",
                file=sys.stderr,
            )
            traceback.print_exception(exc, file=sys.stderr)

    atexit.register(_cleanup)

    mcp = FastMCP(
        "liel",
        instructions=(
            "liel AI memory MCP server - connected to "
            f"{os.path.basename(path)}. Official tools: liel_overview, "
            "liel_find, liel_explore, liel_trace, liel_map, "
            "liel_append, liel_merge."
        ),
    )

    @mcp.tool()
    def liel_overview() -> str:
        """Return a high-level overview of the memory graph."""
        return _do_inspect(db, path)

    @mcp.tool()
    def liel_find(
        label: str = "",
        where: str = "",
        limit: int = _QUERY_LIMIT_DEFAULT,
        cursor: int = 0,
    ) -> str:
        """Find nodes by label and/or exact property filter with cursor pagination."""
        return _do_query(db, label=label, where=where, limit=limit, cursor=cursor)

    @mcp.tool()
    def liel_explore(
        node_id: int,
        max_depth: int = _EXPLORE_DEPTH_DEFAULT,
        edge_label: str = "",
    ) -> str:
        """Explore the neighbourhood of a node via BFS."""
        return _do_explore(db, node_id=node_id, max_depth=max_depth, edge_label=edge_label)

    @mcp.tool()
    def liel_trace(
        from_node: int,
        to_node: int,
        edge_label: str = "",
    ) -> str:
        """Find the shortest path between two nodes."""
        return _do_path(db, from_node=from_node, to_node=to_node, edge_label=edge_label)

    @mcp.tool()
    def liel_map(node_ids: str = "", limit: int = _MERMAID_LIMIT_DEFAULT) -> str:
        """Render a portion of the graph as a Mermaid flowchart diagram."""
        return _do_mermaid(db, node_ids=node_ids, limit=limit)

    @mcp.tool()
    def liel_append(
        nodes: str = "[]",
        edges: str = "[]",
        session: str = "",
    ) -> str:
        """Append nodes and edges in one atomic commit.

        Args:
            nodes: JSON array of node objects. Each entry may contain
                ``ref`` (local alias), ``labels`` (string or string array),
                and ``props`` (object).
            edges: JSON array of edge objects. Each entry must contain
                ``from``, ``to``, and ``label``; ``from`` / ``to`` may be
                existing node IDs or refs created earlier in the same call.
            session: Reserved for future metadata support. It is currently
                accepted for compatibility but does not add properties.
        """
        return _do_append(db, nodes=nodes, edges=edges, session=session)

    @mcp.tool()
    def liel_merge(
        nodes: str = "[]",
        edges: str = "[]",
        session: str = "",
    ) -> str:
        """Merge nodes and edges in one atomic commit.

        Node entries may specify ``id`` to update a known node directly, or
        ``match`` to find one existing node by exact property match before
        reusing it. If no node matches, a new node is created from ``labels``
        and ``props``. Merged edges use idempotent creation.
        """
        return _do_merge(db, nodes=nodes, edges=edges, session=session)

    return mcp

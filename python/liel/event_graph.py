"""Event-sourced knowledge graph helpers.

Experimental v0.8 helpers that keep the core model intentionally small:
``Actor`` / ``Event`` / ``Source`` plus ordinary graph nodes and edges.  Tool
specific concepts such as Omnigent sessions or tool calls should be mapped into
``Event.operation`` / ``Event.payload`` by adapters instead of becoming core
storage concepts.
"""

from __future__ import annotations

from datetime import datetime, timezone
from typing import Any

from liel.liel import GraphDB, Node

ACTOR_LABEL = "Actor"
AGENT_LABEL = "Agent"
EVENT_LABEL = "Event"
SOURCE_LABEL = "Source"
AUTHORED_EDGE = "AUTHORED"
CAUSED_BY_EDGE = "CAUSED_BY"
CITES_EDGE = "CITES"


def utc_timestamp() -> str:
    """Return an ISO 8601 UTC timestamp using the repository's ``Z`` convention."""

    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def find_node_by_key(db: GraphDB, label: str, key: str) -> Node | None:
    """Return the first node with *label* and stable ``key``, if one exists."""

    nodes = db.nodes().label(label).where_(lambda node: node.get("key") == key).fetch()
    if not nodes:
        return None
    return nodes[0]


def ensure_actor(
    db: GraphDB,
    key: str,
    *,
    name: str | None = None,
    actor_kind: str = "ai_agent",
    legacy_agent_key: str | None = None,
    created_at: str | None = None,
    **props: Any,
) -> int:
    """Return an ``Actor`` node ID, creating one when missing.

    ``Agent`` remains a backward-compatible alias/subtype.  When
    ``legacy_agent_key`` is provided, the created node receives both ``Actor``
    and ``Agent`` labels and stores the legacy key in ``legacy_agent_key``.
    """

    existing = find_node_by_key(db, ACTOR_LABEL, key)
    if existing is not None:
        return existing.id

    labels = [ACTOR_LABEL]
    if legacy_agent_key is not None:
        labels.append(AGENT_LABEL)

    properties: dict[str, Any] = {
        "key": key,
        "actor_kind": actor_kind,
        "created_at": created_at or utc_timestamp(),
        **props,
    }
    if name is not None:
        properties["name"] = name
    if legacy_agent_key is not None:
        properties["legacy_agent_key"] = legacy_agent_key

    node = db.add_node(labels, **properties)
    return node.id


def append_event(
    db: GraphDB,
    *,
    author: str,
    operation: str,
    target: str,
    payload: dict[str, Any] | None = None,
    event_id: str | None = None,
    parent_event_id: str | None = None,
    timestamp: str | None = None,
    actor_name: str | None = None,
    actor_kind: str = "ai_agent",
    legacy_agent_key: str | None = None,
    caused_by: str | None = None,
    source_keys: list[str] | None = None,
) -> int:
    """Append a minimal ``Event`` node and return its node ID.

    The helper deliberately does not create first-class Revision / Proposal /
    Branch objects.  Those concepts should be derived from the event chain until
    real usage proves that a higher-level primitive is necessary.
    """

    created_at = timestamp or utc_timestamp()
    resolved_event_id = event_id or _next_event_id(db)
    if find_node_by_key(db, EVENT_LABEL, resolved_event_id) is not None:
        raise ValueError(f"event_id already exists: {resolved_event_id}")

    actor_id = ensure_actor(
        db,
        author,
        name=actor_name,
        actor_kind=actor_kind,
        legacy_agent_key=legacy_agent_key,
        created_at=created_at,
    )
    event = db.add_node(
        [EVENT_LABEL],
        key=resolved_event_id,
        event_id=resolved_event_id,
        parent_event_id=parent_event_id,
        timestamp=created_at,
        author=author,
        operation=operation,
        target=target,
        payload=payload or {},
    )
    db.add_edge(actor_id, AUTHORED_EDGE, event)

    if caused_by is not None:
        cause = find_node_by_key(db, EVENT_LABEL, caused_by)
        if cause is not None:
            db.add_edge(event, CAUSED_BY_EDGE, cause)

    for source_key in source_keys or []:
        source = find_node_by_key(db, SOURCE_LABEL, source_key)
        if source is not None:
            db.add_edge(event, CITES_EDGE, source)

    return event.id


def list_events(db: GraphDB) -> list[dict[str, Any]]:
    """Return event records sorted by timestamp then ``event_id``."""

    records = db.nodes().label(EVENT_LABEL).fetch()
    payloads = [dict(node.properties) for node in records]
    return sorted(
        payloads,
        key=lambda item: (str(item.get("timestamp", "")), str(item.get("event_id", ""))),
    )


def _next_event_id(db: GraphDB) -> str:
    event_numbers: list[int] = []
    for event in db.nodes().label(EVENT_LABEL).fetch():
        raw = str(event.get("event_id") or event.get("key") or "")
        prefix, _, suffix = raw.partition(":")
        if prefix == "event" and suffix.isdigit():
            event_numbers.append(int(suffix))
    next_number = max(event_numbers, default=0) + 1
    return f"event:{next_number:06d}"


__all__ = [
    "ACTOR_LABEL",
    "AGENT_LABEL",
    "AUTHORED_EDGE",
    "CAUSED_BY_EDGE",
    "CITES_EDGE",
    "EVENT_LABEL",
    "SOURCE_LABEL",
    "append_event",
    "ensure_actor",
    "find_node_by_key",
    "list_events",
    "utc_timestamp",
]

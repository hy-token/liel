"""CodingMemory helpers — thin conventions on :class:`GraphDB`.

Experimental (Wave D). Helpers apply labels/properties aligned with the
maintainer design ``docs/internal/design/coding-memory.ja.md`` and
``docs/conventions/recommended-labels.md``.

Bug-shaped work uses label ``Task`` with ``task_kind=\"bug\"`` because the starter
vocabulary does not reserve a separate ``Bug`` label.

Typical use::

    import liel
    from liel.coding_memory import link, record_bug, record_decision, record_file

    db = liel.open("memory.liel")
    with db.transaction():
        f = record_file(db, "src/main.rs", role="hotspot")
        d = record_decision(db, "Use bounded WAL entries", status="accepted")
        b = record_bug(db, "Crash when WAL exceeds page", severity="S1")
        link(db, f, "RELATES_TO", d)
        link(db, b, "DEPENDS_ON", f)
    db.commit()
    db.close()

"""

from __future__ import annotations

from typing import Any

from liel.liel import GraphDB

DECISION_LABEL = "Decision"
FILE_LABEL = "File"
TASK_LABEL = "Task"


def find_file_node_id(db: GraphDB, path: str) -> int | None:
    """Return the ID of a ``File`` node whose ``path`` property equals *path*, else ``None``.

    If multiple files share the same path, the first match wins (deterministic
    scan order is not guaranteed — avoid duplicates in application logic).
    """

    nodes = db.nodes().label(FILE_LABEL).where_(lambda n: n["path"] == path).fetch()
    if not nodes:
        return None
    return nodes[0].id


def record_file(db: GraphDB, path: str, **props: Any) -> int:
    """Return a ``File`` node ID for *path*, creating one if missing."""

    existing = find_file_node_id(db, path)
    if existing is not None:
        return existing
    node = db.add_node([FILE_LABEL], path=path, **props)
    return node.id


def record_decision(db: GraphDB, title: str, **props: Any) -> int:
    """Create a ``Decision`` node and return its ID."""

    node = db.add_node([DECISION_LABEL], title=title, **props)
    return node.id


def record_bug(db: GraphDB, title: str, **props: Any) -> int:
    """Create a ``Task`` node representing a bug-tracked item and return its ID."""

    merged = {**props, "task_kind": "bug"}
    node = db.add_node([TASK_LABEL], title=title, **merged)
    return node.id


def link(
    db: GraphDB,
    from_node: int,
    edge_label: str,
    to_node: int,
    **edge_props: Any,
) -> int:
    """Add a directed edge *from_node* → *to_node*; return the new edge ID."""

    edge = db.add_edge(from_node, edge_label, to_node, **edge_props)
    return edge.id


__all__ = [
    "DECISION_LABEL",
    "FILE_LABEL",
    "TASK_LABEL",
    "find_file_node_id",
    "link",
    "record_bug",
    "record_decision",
    "record_file",
]

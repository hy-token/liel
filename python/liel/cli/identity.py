from __future__ import annotations

from typing import Any


def records_by_id(records: list[dict[str, Any]]) -> dict[int, dict[str, Any]]:
    """Index normalized records by local `.liel` record ID."""
    return {record["id"]: record for record in records}


def diff_records_by_id(
    left: dict[int, dict[str, Any]], right: dict[int, dict[str, Any]]
) -> dict[str, list[int]]:
    """Compare records by local ID only.

    This is intentionally mechanical: it is appropriate for the same file
    lineage, not for semantic identity across independently created files.
    """
    left_ids = set(left)
    right_ids = set(right)
    shared = left_ids & right_ids
    return {
        "added": sorted(right_ids - left_ids),
        "removed": sorted(left_ids - right_ids),
        "changed": sorted(record_id for record_id in shared if left[record_id] != right[record_id]),
    }


def normalize_node_key(node_key: list[str] | None) -> list[str] | None:
    """Return the explicit merge identity key, or None for append-oriented merge."""
    return node_key or None

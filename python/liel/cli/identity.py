from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .common import EXIT_USAGE, CliError, require_existing_file

_NODE_META_KEYS = {"id", "labels"}


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


def identity_string(properties: dict[str, Any], keys: list[str]) -> str:
    """Return a stable display string for an explicit property identity."""
    parts = []
    for key in keys:
        parts.append(f"{key}={_stable_value_repr(properties[key])}")
    return ",".join(parts)


def record_properties(record: dict[str, Any]) -> dict[str, Any]:
    """Return node properties from either a raw or normalized node record."""
    nested = record.get("properties")
    if isinstance(nested, dict):
        return nested
    return {key: record[key] for key in sorted(record) if key not in _NODE_META_KEYS}


def identity_from_rules(
    record: dict[str, Any],
    rules: dict[str, list[str]],
    *,
    side: str,
    require_match: bool = True,
) -> str | None:
    """Resolve a node identity from label-specific rules."""
    matched_labels = [label for label in sorted(record.get("labels", [])) if label in rules]
    if not matched_labels:
        if require_match:
            raise CliError(
                f"{side} node {record['id']} does not match any --identity-rules label",
                EXIT_USAGE,
            )
        return None
    if len(matched_labels) > 1:
        labels = ", ".join(matched_labels)
        raise CliError(
            f"{side} node {record['id']} matches multiple --identity-rules labels: {labels}",
            EXIT_USAGE,
        )
    label = matched_labels[0]
    keys = rules[label]
    properties = record_properties(record)
    for key in keys:
        if key not in properties:
            raise CliError(
                f"{side} node {record['id']} is missing identity rule property: {label}.{key}",
                EXIT_USAGE,
            )
    return f"{label}:{identity_string(properties, keys)}"


def load_identity_rules(path: str | Path) -> dict[str, list[str]]:
    """Load label-specific identity rules from a JSON file."""
    rules_path = require_existing_file(path)
    try:
        payload = json.loads(rules_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CliError(f"failed to read identity rules: {exc}", EXIT_USAGE) from exc

    rules = payload.get("identity_rules") if isinstance(payload, dict) else None
    if not isinstance(rules, dict) or not rules:
        raise CliError(
            "identity rules file must contain a non-empty identity_rules object", EXIT_USAGE
        )

    normalized: dict[str, list[str]] = {}
    for label, keys in rules.items():
        if not isinstance(label, str) or not label:
            raise CliError("identity rule labels must be non-empty strings", EXIT_USAGE)
        if (
            not isinstance(keys, list)
            or not keys
            or any(not isinstance(key, str) or not key for key in keys)
        ):
            raise CliError(f"identity rule for {label} must be a non-empty string list", EXIT_USAGE)
        normalized[label] = keys
    return normalized


def _stable_value_repr(value: Any) -> str:
    if isinstance(value, str):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(_stable_value_repr(item) for item in value) + "]"
    if isinstance(value, dict):
        items = sorted(value.items(), key=lambda item: str(item[0]))
        return "{" + ", ".join(f"{k!r}: {_stable_value_repr(v)}" for k, v in items) + "}"
    return repr(value)

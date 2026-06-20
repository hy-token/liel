from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import liel
from liel.event_graph import append_event, list_events

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    CliError,
    emit_json,
    emit_text,
    require_existing_file,
)


def run_append(args: argparse.Namespace) -> int:
    payload = _payload_from_args(args)
    try:
        with liel.open(str(args.source)) as db:
            event_node_id = append_event(
                db,
                author=args.author,
                operation=args.operation,
                target=args.target,
                payload=payload,
                event_id=args.event_id,
                parent_event_id=args.parent_event_id,
                timestamp=args.timestamp,
                actor_name=args.actor_name,
                actor_kind=args.actor_kind,
                legacy_agent_key=args.legacy_agent_key,
                caused_by=args.caused_by,
                source_keys=args.source_key,
            )
            db.commit()
            events = list_events(db)
    except (OSError, ValueError, liel.GraphDBError, json.JSONDecodeError) as exc:
        raise CliError(f"event append failed: {exc}", EXIT_ERROR) from exc

    created = next(
        (event for event in events if event.get("key") == args.event_id),
        events[-1],
    )
    result = {"event_node_id": event_node_id, "event": created}
    if args.format == "json":
        emit_json(result)
    else:
        emit_text(f"Appended {created.get('event_id')} to {args.source}")
    return EXIT_OK


def run_list(args: argparse.Namespace) -> int:
    source = require_existing_file(args.source)
    try:
        with liel.open(str(source)) as db:
            events = list_events(db)
    except (OSError, liel.GraphDBError) as exc:
        raise CliError(f"event list failed: {exc}", EXIT_ERROR) from exc

    if args.format == "json":
        emit_json({"events": events})
    else:
        emit_text(_format_text(events))
    return EXIT_OK


def _payload_from_args(args: argparse.Namespace) -> dict[str, Any]:
    if args.payload_json is None:
        return {}
    value = args.payload_json
    payload_text = Path(value[1:]).read_text() if value.startswith("@") else value
    parsed = json.loads(payload_text)
    if not isinstance(parsed, dict):
        raise CliError("--payload-json must decode to a JSON object", EXIT_ERROR)
    return parsed


def _format_text(events: list[dict[str, Any]]) -> str:
    if not events:
        return "No events."
    lines = []
    for event in events:
        lines.append(
            f"{event.get('event_id')} {event.get('timestamp')} "
            f"{event.get('author')} {event.get('operation')} {event.get('target')}"
        )
    return "\n".join(lines)

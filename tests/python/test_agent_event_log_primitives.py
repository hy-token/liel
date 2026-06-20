from __future__ import annotations

import json

import liel
from liel.cli import __main__ as cli

_ACTOR_KEY = "actor:local-coder"
_LEGACY_AGENT_KEY = "agent:local-coder"
_CREATED_AT = "2026-06-18T00:00:00Z"


def _write_actor_event_memory(path, *, event_payload_title: str) -> None:
    """Write the minimal v0.8-alpha shape using today's raw graph API.

    This intentionally avoids any future first-class Revision / Proposal / Branch
    API.  The guarantee we want today is smaller: a single `.liel` file can hold
    actor provenance plus an append-only event record, and existing diff can see
    changes by stable node keys.
    """

    with liel.open(str(path)) as db:
        actor = db.add_node(
            ["Actor", "Agent"],
            key=_ACTOR_KEY,
            legacy_agent_key=_LEGACY_AGENT_KEY,
            actor_kind="ai_agent",
            name="Local Coder",
            created_at=_CREATED_AT,
        )
        decision = db.add_node(
            ["Decision"],
            key="decision:event-log-first",
            title="Start with append-only Event log",
            created_by=_ACTOR_KEY,
            created_at=_CREATED_AT,
        )
        source = db.add_node(
            ["Source"],
            key="source:sidecar:tool-output:001",
            source_kind="sidecar",
            path_kind="relative",
            base_kind="memory_file_dir",
            path=".liel-sidecars/evidence/tool-output-001.txt",
            created_at=_CREATED_AT,
        )
        event = db.add_node(
            ["Event"],
            key="event:000001",
            event_id="event:000001",
            parent_event_id=None,
            timestamp=_CREATED_AT,
            author=_ACTOR_KEY,
            operation="create_node",
            target="decision:event-log-first",
            payload={
                "labels": ["Decision"],
                "key": "decision:event-log-first",
                "title": event_payload_title,
            },
        )
        db.add_edge(actor, "AUTHORED", event)
        db.add_edge(event, "TARGETS", decision)
        db.add_edge(event, "CITES", source)
        db.commit()


def test_single_file_can_store_actor_provenance_and_append_event(tmp_path):
    path = tmp_path / "memory.liel"
    _write_actor_event_memory(path, event_payload_title="Start with append-only Event log")

    assert path.exists()
    assert not (tmp_path / "memory.liel.lock").exists()

    with liel.open(str(path)) as db:
        actors = db.nodes().label("Actor").where_(lambda n: n["key"] == _ACTOR_KEY).fetch()
        legacy_agents = (
            db.nodes()
            .label("Agent")
            .where_(lambda n: n["legacy_agent_key"] == _LEGACY_AGENT_KEY)
            .fetch()
        )
        events = db.nodes().label("Event").where_(lambda n: n["event_id"] == "event:000001").fetch()
        assert len(actors) == 1
        assert len(legacy_agents) == 1
        assert len(events) == 1

        event = events[0]
        assert event["parent_event_id"] is None
        assert event["author"] == _ACTOR_KEY
        assert event["operation"] == "create_node"
        assert event["payload"]["key"] == "decision:event-log-first"

        authored = db.neighbors(actors[0], edge_label="AUTHORED")
        assert [node["key"] for node in authored] == ["event:000001"]

        cited_sources = db.neighbors(event, edge_label="CITES")
        assert cited_sources[0]["path_kind"] == "relative"
        assert cited_sources[0]["base_kind"] == "memory_file_dir"
        assert cited_sources[0]["path"].startswith(".liel-sidecars/")


def test_key_aware_diff_detects_event_payload_changes(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    _write_actor_event_memory(left, event_payload_title="Start with append-only Event log")
    _write_actor_event_memory(right, event_payload_title="Use Revision first")

    assert cli.main(["diff", str(left), str(right), "--node-key", "key", "--format", "json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["nodes"]["identity"] == {"mode": "node_key", "keys": ["key"]}
    assert payload["nodes"]["changed"] == ["key='event:000001'"]
    assert payload["edges"]["added"] == []
    assert payload["edges"]["removed"] == []

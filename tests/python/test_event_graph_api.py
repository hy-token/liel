from __future__ import annotations

import json

import liel
from liel.cli import __main__ as cli
from liel.event_graph import append_event, ensure_actor, list_events


def test_event_graph_helper_appends_actor_event_and_preserves_agent_alias(tmp_path):
    path = tmp_path / "events.liel"
    with liel.open(str(path)) as db:
        actor_id = ensure_actor(
            db,
            "actor:local-coder",
            name="Local Coder",
            actor_kind="ai_agent",
            legacy_agent_key="agent:local-coder",
            created_at="2026-06-18T00:00:00Z",
        )
        event_id = append_event(
            db,
            author="actor:local-coder",
            operation="create_node",
            target="decision:event-log-first",
            payload={"title": "Event log first"},
            event_id="event:000001",
            timestamp="2026-06-18T00:00:01Z",
        )
        db.commit()

    with liel.open(str(path)) as db:
        actors = db.nodes().label("Actor").where_(lambda n: n["key"] == "actor:local-coder").fetch()
        agents = (
            db.nodes()
            .label("Agent")
            .where_(lambda n: n["legacy_agent_key"] == "agent:local-coder")
            .fetch()
        )
        events = list_events(db)

        assert actors[0].id == actor_id
        assert len(agents) == 1
        assert len(events) == 1
        assert events[0]["event_id"] == "event:000001"
        assert events[0]["author"] == "actor:local-coder"

        authored = db.neighbors(actors[0], edge_label="AUTHORED")
        assert [node.id for node in authored] == [event_id]


def test_cli_events_append_and_list_json(tmp_path, capsys):
    path = tmp_path / "events.liel"
    assert (
        cli.main(
            [
                "events",
                "append",
                str(path),
                "--author",
                "actor:local-coder",
                "--legacy-agent-key",
                "agent:local-coder",
                "--operation",
                "create_node",
                "--target",
                "decision:event-log-first",
                "--event-id",
                "event:000001",
                "--timestamp",
                "2026-06-18T00:00:00Z",
                "--payload-json",
                '{"title":"Event log first"}',
                "--format",
                "json",
            ]
        )
        == 0
    )
    appended = json.loads(capsys.readouterr().out)
    assert appended["event"]["event_id"] == "event:000001"

    assert cli.main(["events", "list", str(path), "--format", "json"]) == 0
    listed = json.loads(capsys.readouterr().out)
    assert listed["events"] == [appended["event"]]

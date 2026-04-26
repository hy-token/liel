"""
liel Agent Memory - external memory for an LLM-style assistant.

This example models how an agent can store durable context outside the prompt:

  Session / UserRequest / AssistantReply / ToolRun / Decision / Observation nodes
  NEXT / RESPONDED_WITH / PRODUCED / LED_TO / DERIVED_FROM / ABOUT edges

At the end, the example builds a compact "context packet" that could be inserted
into the next LLM prompt before answering a related user request.
"""

from __future__ import annotations

import liel


def prop(node, key: str, default: str = ""):
    """Read a node property with a default for missing keys."""
    value = node.get(key)
    return default if value is None else value


def remember_session(db: liel.GraphDB) -> dict[str, object]:
    """Store a tiny conversation plus the facts learned while helping."""
    with db.transaction():
        session = db.add_node(
            ["Session"],
            title="liel documentation polish",
            user_goal="Make the project easier to trust and evaluate",
        )

        request_1 = db.add_node(
            ["UserRequest"],
            text="Clarify what the database guarantees and how it can fail",
            intent="documentation",
        )
        reply_1 = db.add_node(
            ["AssistantReply"],
            summary="Add a reliability and failure model page",
        )
        tool_1 = db.add_node(
            ["ToolRun"],
            tool="mkdocs build --strict",
            result="documentation build passed",
        )
        decision_1 = db.add_node(
            ["Decision"],
            title="Keep reliability details in one reference page",
            reason="Avoid scattering durability, recovery, and operational guidance",
        )
        observation_1 = db.add_node(
            ["Observation"],
            text="Cross-process writers can corrupt the file if not rejected",
            risk="file_corruption",
        )

        request_2 = db.add_node(
            ["UserRequest"],
            text="Do not add convenience APIs yet; add only an agent memory sample",
            intent="scope_control",
        )
        reply_2 = db.add_node(
            ["AssistantReply"],
            summary="Add an example that retrieves context for a future LLM answer",
        )
        decision_2 = db.add_node(
            ["Decision"],
            title="Avoid new core APIs until real usage patterns are clearer",
            reason="Keep the public surface small and easier to stabilize",
        )
        file_1 = db.add_node(
            ["File"],
            path="docs/reference/reliability.md",
            role="public reliability contract",
        )
        file_2 = db.add_node(
            ["File"],
            path="examples/07_agent_memory.py",
            role="LLM external-memory recipe",
        )

        db.add_edge(session, "HAS_REQUEST", request_1)
        db.add_edge(request_1, "RESPONDED_WITH", reply_1)
        db.add_edge(reply_1, "PRODUCED", tool_1)
        db.add_edge(reply_1, "PRODUCED", file_1)
        db.add_edge(request_1, "LED_TO", decision_1)
        db.add_edge(decision_1, "DERIVED_FROM", request_1)
        db.add_edge(decision_1, "SUPPORTED_BY", observation_1)
        db.add_edge(decision_1, "ABOUT", file_1)

        db.add_edge(request_1, "NEXT", request_2)
        db.add_edge(session, "HAS_REQUEST", request_2)
        db.add_edge(request_2, "RESPONDED_WITH", reply_2)
        db.add_edge(reply_2, "PRODUCED", file_2)
        db.add_edge(request_2, "LED_TO", decision_2)
        db.add_edge(decision_2, "DERIVED_FROM", request_2)
        db.add_edge(decision_2, "ABOUT", file_2)

    return {
        "session": session,
        "latest_request": request_2,
        "reliability_decision": decision_1,
        "api_decision": decision_2,
    }


def context_packet(db: liel.GraphDB, request, *, max_depth: int = 2) -> list[str]:
    """Return compact context lines for a future LLM prompt."""
    lines: list[str] = []
    for node, depth in db.bfs(request, max_depth=max_depth):
        labels = "/".join(node.labels)
        text = (
            prop(node, "text")
            or prop(node, "title")
            or prop(node, "summary")
            or prop(node, "path")
            or prop(node, "result")
        )
        if text:
            lines.append(f"{'  ' * depth}- [{labels}] {text}")
    return lines


def find_relevant_memory(db: liel.GraphDB, query: str):
    """A tiny retrieval step: filter remembered requests and decisions by text."""
    query_lower = query.lower()
    return (
        db.nodes()
        .where_(
            lambda n: any(
                query_lower in str(prop(n, key)).lower()
                for key in ("text", "title", "summary", "reason", "role")
            )
        )
        .fetch()
    )


def demo() -> None:
    with liel.open(":memory:") as db:
        nodes = remember_session(db)

        next_user_message = "Before adding APIs, what context should the assistant remember?"
        relevant = find_relevant_memory(db, "api")

        print(f"{db.node_count()} memory nodes, {db.edge_count()} relationships")
        print()
        print("Retrieved memory candidates:")
        for node in relevant:
            label = "/".join(node.labels)
            title = prop(node, "title") or prop(node, "text") or prop(node, "path")
            print(f"  - [{label}] {title}")

        print()
        print("Context packet for the next LLM prompt:")
        print(f"User message: {next_user_message}")
        for line in context_packet(db, nodes["latest_request"]):
            print(line)


if __name__ == "__main__":
    demo()

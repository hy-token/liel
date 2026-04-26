"""
Bundled **agent memory** demo (ships in the PyPI wheel).

**How to run**

- After ``pip install liel``: ``liel-demo`` (console script) or ``python -m liel.demo``
- From a source checkout, the same program is also invoked as
  ``python examples/08_demo.py`` (see that file for maintainer notes).

**What this is for**

- Show that **liel** is about **graph-shaped memory** (labels, explicit edges, simple
  traversals) driving follow-up context — not a cloud LLM product demo.
- **No API keys**; the default path is fully offline. The exploration list in the
  output is **graph-derived** (``SUGGESTS`` neighbors) unless you opt in to Ollama.
- The **suggestion** uses **merged topic names** from ``SUGGESTS`` on both the preference
  node and the place node: topics linked from **both** traversals are listed first, then
  the rest (place and preference, not just one hop). Optional
  **Ollama** (``LIEL_DEMO_LLM=1``) only **re-phrases** the comma exploration list; on failure
  a **canned string** is used so the run always finishes (that fallback is *not* from the graph).

**Longer recipes** (01, 07, notebooks) stay GitHub-only to keep the wheel small.
"""

from __future__ import annotations

import json
import os
import urllib.request
from typing import Any, cast

import liel

_OLLAMA = os.environ.get("OLLAMA_HOST", "http://127.0.0.1:11434").rstrip("/")
_MODEL = os.environ.get("OLLAMA_MODEL", "llama3.2")
# Shown when Ollama is down or returns junk — **not** read from the graph (coffee-themed
# text matches the story but is otherwise arbitrary; avoids implying graph extraction).
# Order matches the merged SUGGESTS list from _build_demo_graph (overlap-first).
_OLLAMA_FALLBACK = "Palo Alto cafes, espresso, single-origin beans"


def _graph_suggestion(preference: str, place: str, related: list[str]) -> str:
    """One-line blurb: preference, place, and merged SUGGESTS topic order (graph-derived)."""
    topics = ", ".join(related) if related else preference
    return f"Since you like {preference} in {place}, you might explore: {topics}."


def _build_demo_graph(
    db: Any,
) -> tuple[Any, Any, Any, list[str], list[str], list[str], list[str]]:
    """Build graph, commit, return nodes plus topic names from two SUGGESTS traversals + merge."""
    you = db.add_node(["Actor"], role="user", display="You")
    coffee = db.add_node(["Topic"], name="coffee")
    place = db.add_node(["Place"], name="Silicon Valley")
    espresso = db.add_node(["Topic"], name="espresso")
    beans = db.add_node(["Topic"], name="single-origin beans")
    palo_cafes = db.add_node(["Topic"], name="Palo Alto cafes")

    # Graph structure:
    #   (You) --LIKES--> (coffee) --SUGGESTS--> (espresso, beans, palo)
    #   (Silicon Valley) --SUGGESTS--> (palo)  # same topic reachable from the place (local scene)
    #     (You) --IN--> (Silicon Valley)
    db.add_edge(you, "LIKES", coffee)
    db.add_edge(you, "IN", place)
    for topic in (espresso, beans, palo_cafes):
        db.add_edge(coffee, "SUGGESTS", topic)
    db.add_edge(place, "SUGGESTS", palo_cafes)
    db.commit()

    from_coffee = [n["name"] for n in db.neighbors(coffee, edge_label="SUGGESTS")]
    from_place = [n["name"] for n in db.neighbors(place, edge_label="SUGGESTS")]
    pset = set(from_place)
    overlap = [n for n in from_coffee if n in pset]
    # Neighbor order is storage-dependent; sort the non-overlap set so the demo is deterministic.
    rest = sorted([n for n in from_coffee if n not in pset])
    # Exploration order: co-supported by both traversals first, then coffee-only SUGGESTS.
    related = overlap + rest
    return you, coffee, place, related, from_coffee, from_place, overlap


def _ollama_exploration_line(utterance: str, memory_lines: list[str]) -> str:
    """Call local Ollama; on any failure return a fixed phrase (not graph-derived)."""
    prompt = (
        "User said:\n"
        f"  {utterance}\n"
        "Known memory:\n"
        + "\n".join(f"  - {x}" for x in memory_lines)
        + "\nReply with ONE short comma-separated list of exactly 3 follow-up topics, no other text."
    )
    payload = json.dumps(
        {"model": _MODEL, "messages": [{"role": "user", "content": prompt}], "stream": False}
    ).encode()
    req = urllib.request.Request(
        f"{_OLLAMA}/api/chat",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=4) as resp:
            parsed: object = json.loads(resp.read().decode())
        if not isinstance(parsed, dict):
            return _OLLAMA_FALLBACK
        data = cast(dict[str, Any], parsed)
        message = data.get("message")
        if not isinstance(message, dict):
            return _OLLAMA_FALLBACK
        msg = cast(dict[str, Any], message)
        content = msg.get("content")
        if not isinstance(content, str):
            return _OLLAMA_FALLBACK
        text = content.strip().split("\n")[0]
        return text if text else _OLLAMA_FALLBACK
    except (OSError, ValueError, TypeError, json.JSONDecodeError, KeyError):
        return _OLLAMA_FALLBACK


def _exploration_list(
    utterance: str,
    memory_lines: list[str],
    related: list[str],
) -> tuple[str, str]:
    """Return (line, provenance note for stdout)."""
    if os.environ.get("LIEL_DEMO_LLM", "") != "1":
        return (
            ", ".join(related),
            "graph-derived from SUGGESTS (coffee and place, overlap first) - no LLM called",
        )

    text = _ollama_exploration_line(utterance, memory_lines)
    if text == _OLLAMA_FALLBACK:
        return text, "Ollama unreachable - canned fallback (not from the graph)"
    return text, "local Ollama /api/chat"


def main() -> None:
    """Print a scripted walkthrough: build a tiny graph, then show stored edges vs chat-log baseline."""
    print("Running liel demo (no API key required)")
    # Fixed user line — the rest of the output is derived from the graph + env (see module docstring).
    utterance = "I like coffee in Silicon Valley."

    with liel.open(":memory:") as db:
        you, coffee, place, related, from_coffee, from_place, overlap = _build_demo_graph(db)
        suggestion = _graph_suggestion(coffee["name"], place["name"], related)
        memory_lines = [
            f"You like {coffee['name']}.",
            f"You mentioned {place['name']}.",
            "Related topics already linked in the graph: " + ", ".join(related) + ".",
        ]
        explore, explore_note = _exploration_list(utterance, memory_lines, related)

        print("\n=== Agent Memory Demo ===\n")
        print("[Input]")
        print(f"User: {utterance} (simple example)")
        print()

        print("[Memory Stored]")
        print(f"  -> Found preference: {coffee['name'].title()}")
        print(f"  -> Context: {place['name']}")
        print(f"  {you['display']} --LIKES--> {coffee['name']}")
        print(f"  {you['display']} --IN----> {place['name']}")
        print(f"  {coffee['name']} --SUGGESTS--> {', '.join(from_coffee)}")
        print(f"  {place['name']} --SUGGESTS--> {', '.join(from_place)}")
        print()

        print("[Graph Inputs]")
        print(f"  preference: {coffee['name']}")
        print(f"  place: {place['name']}")
        print(f"  SUGGESTS overlap (preference & place): {', '.join(overlap)}")
        print()

        print("[Graph Traversal]")
        print("  db.neighbors(coffee, edge_label='SUGGESTS')")
        for name in from_coffee:
            print(f"  -> {name}")
        print("  db.neighbors(place, edge_label='SUGGESTS')")
        for name in from_place:
            print(f"  -> {name}")
        print("  ranked: shared suggestions first, then coffee-only suggestions")
        print()

        print("[Graph-based Suggestion]")
        print(f"  -> {suggestion}")
        print("     why: combines place + preference via SUGGESTS overlap, not a flat chat log")
        print()

        print("[Next Prompt Context]")
        print(
            "  -> User prefers coffee in Silicon Valley - bias future replies toward graph-linked topics:",
            ", ".join(related) + ".",
        )
        print(f"  -> Exploration list: {explore}")
        print(f"      [{explore_note}]")
        print("  -> (Paste into your agent prompt when you want this memory to steer the reply.)")
        print()

    print("Demo completed successfully")


if __name__ == "__main__":
    main()

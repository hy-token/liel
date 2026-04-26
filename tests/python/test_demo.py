"""Smoke test for the PyPI-bundled agent-memory demo."""

from __future__ import annotations

import os
import subprocess
import sys

import liel.demo as demo


def test_demo_main_smoke(capsys, monkeypatch):
    monkeypatch.delenv("LIEL_DEMO_LLM", raising=False)
    demo.main()
    out = capsys.readouterr().out
    assert "Running liel demo (no API key required)" in out
    assert "Agent Memory Demo" in out
    assert "Palo Alto" in out
    assert "SUGGESTS" in out
    assert "Graph Inputs" in out
    assert "Graph Traversal" in out
    assert "db.neighbors(place" in out
    assert "SUGGESTS overlap" in out
    assert "db.neighbors" in out
    assert (
        "Since you like coffee in Silicon Valley, you might explore: "
        "Palo Alto cafes, espresso, single-origin beans" in out
    )
    assert "graph-derived from SUGGESTS (coffee and place, overlap first) - no LLM called" in out
    assert "Demo completed successfully" in out


def test_demo_graph_derived_exploration_list(capsys, monkeypatch):
    monkeypatch.delenv("LIEL_DEMO_LLM", raising=False)
    demo.main()
    out = capsys.readouterr().out
    assert "Exploration list: Palo Alto cafes, espresso, single-origin beans" in out
    assert "no LLM called" in out


def test_demo_module_runs_with_exit_code_zero(monkeypatch):
    monkeypatch.delenv("LIEL_DEMO_LLM", raising=False)
    env = os.environ.copy()
    env.pop("LIEL_DEMO_LLM", None)
    result = subprocess.run(
        [sys.executable, "-m", "liel.demo"],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "Demo completed successfully" in result.stdout
    assert "Running liel demo (no API key required)" in result.stdout

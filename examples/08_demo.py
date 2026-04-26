"""
LLM-optional **agent memory** demo — thin wrapper around the same code that ships
in the wheel as ``liel.demo``.

**What this is for**

- **From a dev checkout** (``maturin develop`` or equivalent): run ``python examples/08_demo.py`` to
  exercise the in-tree Python + Rust build without installing from PyPI. Behaviour should match
  ``liel-demo`` / ``python -m liel.demo`` on an installed package.
- **What it shows**: graph-shaped memory (labels, edges, traversals) drives suggestions; an
  optional local LLM (Ollama) only re-phrases an exploration list, with a **non-LLM fallback**
  so the script always finishes. The point is the **graph + traversal**, not cloud reasoning.

**Elsewhere:** implementation lives in ``python/liel/demo.py``; on PyPI use the console entry or
``-m liel.demo``. For a longer, recipe-style agent-memory example, see ``examples/07_agent_memory.py``.
"""

from __future__ import annotations

from liel.demo import main

if __name__ == "__main__":
    main()

# Coding memory example (Wave D)

This folder shows the optional, **experimental** Python helper
`liel.coding_memory`: thin `add_node` / `add_edge` conventions for a
**coding-agent-shaped** memory file (``File`` / ``Decision`` / ``Task`` with
``task_kind="bug"``).

Treat this as a convention layer on top of the stable low-level graph API, not
as a frozen `1.0` contract. The public guide explains that boundary:

- [`docs/guide/connectors/python.md#coding-memory-helpers`](../../docs/guide/connectors/python.md#coding-memory-helpers)

Run:

```bash
python run_demo.py
```

Requires `liel` installed (`maturin develop` or `pip install liel`).

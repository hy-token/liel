# Example Python scripts

These live in the repository under [`examples/`](https://github.com/hy-token/liel/tree/main/examples) (not inside the built docs tree). Clone the repo, install the package for development (`maturin develop` or `pip install liel`), then run with `python examples/...` from the checkout root.

## At a glance

| File | What it shows |
|------|----------------|
| [`01_quickstart.py`](https://github.com/hy-token/liel/blob/main/examples/01_quickstart.py) | CRUD, transactions, neighbor / edge queries |
| [`02_knowledge_graph.py`](https://github.com/hy-token/liel/blob/main/examples/02_knowledge_graph.py) | Heterogeneous labels, QueryBuilder, BFS, shortest path |
| [`03_bulk_import.py`](https://github.com/hy-token/liel/blob/main/examples/03_bulk_import.py) | Bulk import (JSON + CSV helpers, Les Miserables demo) |
| [`04_pandas_integration.py`](https://github.com/hy-token/liel/blob/main/examples/04_pandas_integration.py) | DataFrame conversion and aggregation (`pandas`) |
| [`05_visualization.py`](https://github.com/hy-token/liel/blob/main/examples/05_visualization.py) | `networkx` + `matplotlib` drawing |
| [`06_export.py`](https://github.com/hy-token/liel/blob/main/examples/06_export.py) | Export graph to JSON / CSV (stdlib) |
| [`07_agent_memory.py`](https://github.com/hy-token/liel/blob/main/examples/07_agent_memory.py) | LLM-style session memory as a graph + “context packet” |
| [`08_demo.py`](https://github.com/hy-token/liel/blob/main/examples/08_demo.py) | Same as `liel-demo` for a local dev build |
| [`09_cli_smoke_files.py`](https://github.com/hy-token/liel/blob/main/examples/09_cli_smoke_files.py) | Generate small `.liel` files for CLI smoke tests |

## Script walkthrough

### [`01_quickstart.py`](https://github.com/hy-token/liel/blob/main/examples/01_quickstart.py)

Uses a file `example_1.liel` (created in the current directory). Walks through **add** three `Person` nodes and `KNOWS` edges, **read** with `get_node` / `get_edge`, **adjacency** with `neighbors`, `out_edges`, and `in_edges`, **update** a property, then **delete** a node and **rollback** so the node comes back. Good first read if you want line-by-line API usage without extras.

### [`02_knowledge_graph.py`](https://github.com/hy-token/liel/blob/main/examples/02_knowledge_graph.py)

Builds a richer **Person / Company / Technology** graph in memory, including **multiple labels** on one node (`Person` + `Manager`). Demonstrates [**QueryBuilder**](../guide/connectors/python.md)-style queries (`nodes().label(...).where_(...).fetch()`), **BFS** from a start node with depth, **shortest_path** between two people, and prints readable paths. Use this when teaching filters and traversals beyond raw CRUD.

### [`03_bulk_import.py`](https://github.com/hy-token/liel/blob/main/examples/03_bulk_import.py)

Shows **bulk loading inside transactions**: reusable helpers `import_nodes_csv` / `import_edges_csv` map external string IDs to liel node IDs. The **`demo()`** path downloads Vega’s **Les Miserables** co-appearance JSON, imports **Character** nodes and **APPEARS_WITH** weighted edges in one transaction, then queries neighbors for a sample character. Illustrates mapping remote datasets into local IDs without extra tooling beyond the stdlib + urllib.

### [`04_pandas_integration.py`](https://github.com/hy-token/liel/blob/main/examples/04_pandas_integration.py)

Depends on **`pandas`**. Defines **`nodes_to_df`** / **`edges_to_df`** on top of `all_nodes_as_records` / `all_edges_as_records`, **`degree_df`** combining nodes with **`degree_stats()`** from Rust, builds a small social **FOLLOWS** graph, then prints tables and a simple **groupby** (average age by role). Bridges liel to spreadsheets and analytics workflows.

### [`05_visualization.py`](https://github.com/hy-token/liel/blob/main/examples/05_visualization.py)

Depends on **`networkx`** and **`matplotlib`**. Converts the graph to **`networkx`** (`to_networkx`), then **`draw_graph`** lays out with spring placement, optional coloring by a node property, edge labels, and a legend. Ends with a demo graph (companies, projects, dependencies). Use when you need a quick static diagram from an open `.liel` file.

### [`06_export.py`](https://github.com/hy-token/liel/blob/main/examples/06_export.py)

**Stdlib only.** Implements **`export_json`** / **`export_nodes_csv`** / **`export_edges_csv`** in the same spirit as hand-built interchange files (not identical field-for-field to CLI **`liel export`**, which follows the product export contract—see [CLI JSON inventory](../reference/cli-json-inventory.md)). Pairs with **`03_bulk_import.py`** for round-trip experiments. Creates temp files under **`tempfile`** in `__main__`.

### [`07_agent_memory.py`](https://github.com/hy-token/liel/blob/main/examples/07_agent_memory.py)

Long-form **agent-memory narrative**: nodes such as **Session**, **UserRequest**, **AssistantReply**, **ToolRun**, **Decision**, **Observation**, **File**, wired with edges like **NEXT**, **RESPONDED_WITH**, **LED_TO**. Ends by assembling a small **context packet** (text you could inject before the next LLM call). Complements the [AI memory playbook](../guide/mcp/agent-memory.md) and MCP; use when you want a self-contained story in one file.

### [`08_demo.py`](https://github.com/hy-token/liel/blob/main/examples/08_demo.py)

A **one-liner** to `liel.demo.main`—the same behavior as **`liel-demo`** or **`python -m liel.demo`** when run from a **dev checkout** so you exercise the in-tree Rust + Python stack. Optional Ollama rephrasing is off unless you set env vars; the script is still meant to highlight **graph + traversal**, not cloud LLMs.

### [`09_cli_smoke_files.py`](https://github.com/hy-token/liel/blob/main/examples/09_cli_smoke_files.py)

**Maintainer-oriented:** reads `examples/cli_smoke_data/*.csv` and writes small **`.liel`** files under `target/cli-smoke/` (default) for trying **`liel diff`**, **`liel merge`**, etc. Use **`--force`** to overwrite, **`--clean-locks`** on Windows if lock folders linger. Not a learning script; it is a file generator for manual CLI checks and docs media.

## Folders

### [`demo_memory/`](https://github.com/hy-token/liel/tree/main/examples/demo_memory)

**[`make_demo_files.py`](https://github.com/hy-token/liel/blob/main/examples/demo_memory/make_demo_files.py)** writes the **SaaS-style** `base.liel` / `agent-a.liel` / `agent-b.liel` and **identity** JSON used in README demos, VHS tapes, and merge/diff/trace stories. **`--extras`** can add more pairs for CI-oriented tapes. Output is usually under `target/demo-memory/`. See [`demo_memory/README.md`](https://github.com/hy-token/liel/blob/main/examples/demo_memory/README.md) in the repo.

### [`coding_memory/`](https://github.com/hy-token/liel/tree/main/examples/coding_memory)

**[`run_demo.py`](https://github.com/hy-token/liel/blob/main/examples/coding_memory/run_demo.py)** is a **minimal** graph using [`liel.coding_memory`](../guide/connectors/python.md#coding-memory-helpers) (`record_file`, `record_decision`, `link`, etc.). Read this after the Python guide section on coding-memory helpers.

### [`github-actions/`](https://github.com/hy-token/liel/tree/main/examples/github-actions)

Ready-to-copy **workflow fragments** for `liel stats` / manifest checks, plus a short [README](https://github.com/hy-token/liel/blob/main/examples/github-actions/README.md). Pairs with [CI / GitHub Actions](../guide/ci.md).

### [`notebooks/`](https://github.com/hy-token/liel/tree/main/examples/notebooks)

Heavier **Jupyter** experiments (e.g. Wikipedia tour, social network notebook) and shared **`_utils.py`**. See the folder [README](https://github.com/hy-token/liel/blob/main/examples/notebooks/README.md). Not required for the core product.

## Related

- [Quickstart](../guide/quickstart.md) — install paths and the in-doc code walkthroughs
- [Sample viewer (read-only)](../guide/sample-viewer.md) — browser UI for `liel export` JSON

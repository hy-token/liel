# Benchmarks and file size notes

`liel` ships a small benchmark script for local regression tracking:

```bash
python scripts/bench/bench_python_api.py --nodes 2000
```

The script creates a synthetic chain graph:

- `insert_nodes`: creates `N` nodes with one `ordinal` property each
- `insert_edges`: creates `N - 1` `NEXT` edges with one `ordinal` property each
- `neighbors_midpoint`: reads the outgoing `NEXT` neighbor from the middle node
- `shortest_path_full`: searches from the first node to the last node
- `all_nodes_records`: exports all nodes as Python records

The output includes the measured item count for each row. Insert and export rows
also show throughput as `ops/s`.

Example:

```text
insert_nodes         count=2000        0.310s      6457.4 ops/s
insert_edges         count=1999        0.275s      7264.5 ops/s
neighbors_midpoint   queries=1         0.000s
shortest_path_full   path_nodes=2000   0.032s
all_nodes_records    count=2000        0.008s    241688.9 ops/s
database_path         target\bench-python-api\bench.liel
file_size             7.02 MiB (7356416 bytes)
```

## Interpreting file size

Small `.liel` files are not perfectly linear. The file starts tiny, then grows
in allocation steps as pages and extents are reserved. That means a 2,000-node
benchmark and a 10,000-node benchmark can occupy the same on-disk size.

Measured on the benchmark shape above, with one property on each node and edge:

| Graph shape | Approximate `.liel` size |
|---:|---:|
| Empty database | 4 KiB |
| 2,000 nodes + 1,999 edges | 7.02 MiB |
| 10,000 nodes + 9,999 edges | 7.02 MiB |
| 50,000 nodes + 49,999 edges | 15.85 MiB |
| 100,000 nodes + 99,999 edges | 26.07 MiB |

The larger rows were measured with batched commits to stay within the 4 MiB WAL
reservation. The default benchmark script commits all inserted nodes once and
all inserted edges once, so very large `--nodes` values can fail with a
transaction-size error unless you split the workload.

Use these numbers as a practical order-of-magnitude guide, not a capacity
guarantee. Real projects vary with property sizes, label counts, edge density,
and how often large text values are stored.

For AI-agent memory, the efficient pattern is to store durable facts,
decisions, tasks, preferences, and relationships. Avoid storing full chat logs
or large documents directly in node properties; keep those as external files and
store summaries or references in `liel`.

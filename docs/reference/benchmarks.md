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
- `export_json`: serializes the graph through the public export path
- `import_roundtrip`: restores that export into a fresh `.liel`
- `diff_roundtrip`: compares the source and imported file with `ordinal` identity
- `merge_preview`: dry-runs an idempotent key-aware merge preview
- `trace_full_path`: builds the full CLI trace payload from the first node to the last

The output includes the measured item count for each row. Insert and record-scan
rows also show throughput as `ops/s`.

Example:

```text
insert_nodes         count=2000        0.310s      6457.4 ops/s
insert_edges         count=1999        0.275s      7264.5 ops/s
neighbors_midpoint   queries=1         0.000s
shortest_path_full   path_nodes=2000   0.032s
all_nodes_records    count=2000        0.008s    241688.9 ops/s
export_json          bytes=561327      0.037s
import_roundtrip     records=3999      0.337s
diff_roundtrip       changes=0         0.072s
merge_preview        reused=3999      12.468s
trace_full_path      path_nodes=2000   0.058s
database_path         target\bench-python-api\bench.liel
export_path           target\bench-python-api\bench.export.json
imported_path         target\bench-python-api\bench.imported.liel
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

In the expanded `1.0` prep baseline above, key-aware `merge_preview` is the
slowest measured row. That is expected for a dry-run workflow that opens,
indexes, simulates merge behavior, and reports reuse counts. Treat it as a
review-path metric, not as the normal hot loop for reads or single-record writes.

For AI-agent memory, the efficient pattern is to store durable facts,
decisions, tasks, preferences, and relationships. Avoid storing full chat logs
or large documents directly in node properties; keep those as external files and
store summaries or references in `liel`.

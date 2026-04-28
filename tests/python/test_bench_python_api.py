from __future__ import annotations

import uuid
from pathlib import Path

from scripts.bench.bench_python_api import run_benchmarks


def test_python_api_benchmark_output_includes_counts_and_file_size():
    workdir = Path("target") / f"test-bench-python-api-{uuid.uuid4().hex}"
    results = run_benchmarks(5, workdir)

    assert any(line.startswith("insert_nodes         count=5") for line in results)
    assert any(line.startswith("insert_edges         count=4") for line in results)
    assert any(line.startswith("neighbors_midpoint   queries=1") for line in results)
    assert any(line.startswith("shortest_path_full   path_nodes=5") for line in results)
    assert any(line.startswith("all_nodes_records    count=5") for line in results)
    assert any(line.startswith("file_size             ") and " bytes)" in line for line in results)

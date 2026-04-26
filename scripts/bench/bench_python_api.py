"""Lightweight reproducible benchmarks for the Python API surface.

These benchmarks are meant for local regression tracking, not for vendor-style
performance claims. They use simple synthetic graphs so contributors can rerun
them easily on different revisions.
"""

from __future__ import annotations

import argparse
import time
from pathlib import Path

import liel


def _format_result(name: str, elapsed: float, operations: int | None = None) -> str:
    """Render a human-friendly benchmark result line."""
    if operations is None or elapsed == 0:
        return f"{name:<20} {elapsed:>8.3f}s"
    throughput = operations / elapsed
    return f"{name:<20} {elapsed:>8.3f}s  {throughput:>10.1f} ops/s"


def run_benchmarks(node_count: int, workdir: Path) -> list[str]:
    """Execute the baseline benchmark set and return formatted result lines."""
    results: list[str] = []
    workdir.mkdir(parents=True, exist_ok=True)
    db_path = workdir / "bench.liel"
    if db_path.exists():
        db_path.unlink()

    with liel.open(str(db_path)) as db:
        start = time.perf_counter()
        nodes = [db.add_node(["Bench"], ordinal=index) for index in range(node_count)]
        db.commit()
        elapsed = time.perf_counter() - start
        results.append(_format_result("insert_nodes", elapsed, node_count))

        start = time.perf_counter()
        for index in range(node_count - 1):
            db.add_edge(nodes[index], "NEXT", nodes[index + 1], ordinal=index)
        db.commit()
        elapsed = time.perf_counter() - start
        results.append(_format_result("insert_edges", elapsed, max(node_count - 1, 0)))

        midpoint = nodes[node_count // 2]
        start = time.perf_counter()
        db.neighbors(midpoint, edge_label="NEXT")
        elapsed = time.perf_counter() - start
        results.append(_format_result("neighbors_midpoint", elapsed))

        start = time.perf_counter()
        db.shortest_path(nodes[0], nodes[-1], edge_label="NEXT")
        elapsed = time.perf_counter() - start
        results.append(_format_result("shortest_path_full", elapsed))

        start = time.perf_counter()
        records = db.all_nodes_as_records()
        elapsed = time.perf_counter() - start
        results.append(_format_result("all_nodes_records", elapsed, len(records)))

    results.append(f"database_path         {db_path}")
    return results


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--nodes",
        type=int,
        default=10_000,
        help="Number of nodes to insert before running traversal/export scenarios.",
    )
    parser.add_argument(
        "--workdir",
        type=Path,
        default=Path("target") / "bench-python-api",
        help="Workspace-local directory used for the temporary benchmark database.",
    )
    return parser.parse_args()


def main() -> int:
    """CLI entry point."""
    args = parse_args()
    if args.nodes < 2:
        raise SystemExit("--nodes must be at least 2")
    for line in run_benchmarks(args.nodes, args.workdir):
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

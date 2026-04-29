from __future__ import annotations

import argparse
import shutil
from pathlib import Path
from typing import Any

import liel

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    EXIT_USAGE,
    CliError,
    emit_json,
    emit_text,
    refuse_overwrite,
    require_existing_file,
)
from .identity import normalize_node_key


def run(args: argparse.Namespace) -> int:
    payload = merge_files(
        args.left,
        args.right,
        args.output,
        force=args.force,
        node_key=args.node_key,
        edge_strategy=args.edge_strategy,
        on_node_conflict=args.on_node_conflict,
    )
    if args.format == "json":
        emit_json(payload)
    else:
        emit_text(format_text(payload))
    return EXIT_OK


def merge_files(
    left_path: str | Path,
    right_path: str | Path,
    output_path: str | Path,
    *,
    force: bool = False,
    node_key: list[str] | None = None,
    edge_strategy: str = "append",
    on_node_conflict: str = "keep_dst",
) -> dict[str, Any]:
    left = require_existing_file(left_path)
    right = require_existing_file(right_path)
    output = refuse_overwrite(output_path, force=force)
    _reject_in_place_output(left, right, output)

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(left, output)
        with liel.open(str(output)) as dst, liel.open(str(right)) as src:
            report = dst.merge_from(
                src,
                node_key=normalize_node_key(node_key),
                edge_strategy=edge_strategy,
                on_node_conflict=on_node_conflict,
            )
            dst.commit()
    except (OSError, ValueError, liel.GraphDBError) as exc:
        _remove_created_output(output)
        raise CliError(f"merge failed: {exc}", EXIT_ERROR) from exc

    return _report_payload(report, output)


def format_text(payload: dict[str, Any]) -> str:
    return "\n".join(
        [
            f"Merged into {payload['output']}",
            f"Nodes: +{payload['nodes_created']} reused {payload['nodes_reused']}",
            f"Edges: +{payload['edges_created']} reused {payload['edges_reused']}",
        ]
    )


def _reject_in_place_output(left: Path, right: Path, output: Path) -> None:
    resolved_output = output.resolve()
    if resolved_output in {left.resolve(), right.resolve()}:
        raise CliError("merge output must be different from both input files", EXIT_USAGE)


def _remove_created_output(path: Path) -> None:
    try:
        path.unlink(missing_ok=True)
    except OSError:
        pass


def _report_payload(report: liel.MergeReport, output: Path) -> dict[str, Any]:
    return {
        "output": str(output),
        "nodes_created": report.nodes_created,
        "nodes_reused": report.nodes_reused,
        "edges_created": report.edges_created,
        "edges_reused": report.edges_reused,
        "node_id_map": report.node_id_map,
        "edge_id_map": report.edge_id_map,
    }

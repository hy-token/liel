from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import liel
from liel.cli import __main__ as cli
from liel.cli import diff as cli_diff
from liel.cli import identity as cli_identity
from liel.cli import merge as cli_merge
from liel.cli.common import CliError, refuse_overwrite


def test_cli_without_args_prints_help(capsys):
    assert cli.main([]) == 0
    out = capsys.readouterr().out
    assert "Local graph memory CLI" in out
    assert "help" in out
    assert "version" in out


def test_cli_help_prints_top_level_help(capsys):
    assert cli.main(["help"]) == 0
    out = capsys.readouterr().out
    assert "Local graph memory CLI" in out
    assert "diff" in out
    assert "merge" in out


def test_cli_help_prints_command_help(capsys):
    assert cli.main(["help", "merge"]) == 0
    out = capsys.readouterr().out
    assert "usage: liel merge" in out
    assert "--node-key" in out


def test_cli_version_text(capsys):
    assert cli.main(["version"]) == 0
    assert capsys.readouterr().out.strip() == liel.__version__


def test_cli_version_json(capsys):
    assert cli.main(["version", "--format", "json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload == {"version": liel.__version__}


def test_cli_module_runs_with_exit_code_zero():
    result = subprocess.run(
        [sys.executable, "-m", "liel.cli", "version"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == liel.__version__


def test_refuse_overwrite_requires_force(monkeypatch):
    existing = Path("out.liel")
    monkeypatch.setattr(Path, "exists", lambda self: self == existing)

    try:
        refuse_overwrite(existing)
    except CliError as exc:
        assert exc.exit_code == 2
        assert "refusing to overwrite" in exc.message
    else:
        raise AssertionError("expected CliError")

    assert refuse_overwrite(existing, force=True) == existing


def test_cli_diff_identical_files_json(capsys, monkeypatch):
    monkeypatch.setattr(cli_diff, "diff_files", lambda left, right: _diff_report(changed=False))

    assert cli.main(["diff", "left.liel", "right.liel", "--format", "json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["changed"] is False
    assert payload["left"]["nodes"] == 2
    assert payload["right"]["edges"] == 1


def test_cli_diff_reports_added_and_changed_records(capsys, monkeypatch):
    monkeypatch.setattr(cli_diff, "diff_files", lambda left, right: _diff_report(changed=True))

    assert cli.main(["diff", "left.liel", "right.liel"]) == 1
    out = capsys.readouterr().out
    assert "Nodes: +1 -0 ~1" in out
    assert "Edges: +0 -0 ~0" in out
    assert "node added: 3" in out
    assert "node changed: 1" in out


def test_cli_diff_rejects_missing_files(capsys):
    assert cli.main(["diff", "missing-left.liel", "missing-right.liel"]) == 2
    err = capsys.readouterr().err
    assert "file does not exist: missing-left.liel" in err


def test_diff_record_comparison_is_mechanical_by_id():
    left = {
        1: {"id": 1, "labels": ["Person"], "properties": {"name": "Alice"}},
        2: {"id": 2, "labels": ["Task"], "properties": {"key": "old"}},
    }
    right = {
        1: {"id": 1, "labels": ["Person"], "properties": {"name": "Alice", "age": 30}},
        3: {"id": 3, "labels": ["Task"], "properties": {"key": "new"}},
    }

    assert cli_identity.diff_records_by_id(left, right) == {
        "added": [3],
        "removed": [2],
        "changed": [1],
    }


def test_identity_helpers_normalize_records_and_node_key():
    records = [
        {"id": 2, "properties": {"name": "B"}},
        {"id": 1, "properties": {"name": "A"}},
    ]

    assert cli_identity.records_by_id(records) == {2: records[0], 1: records[1]}
    assert cli_identity.normalize_node_key(None) is None
    assert cli_identity.normalize_node_key([]) is None
    assert cli_identity.normalize_node_key(["path"]) == ["path"]


def test_cli_merge_prints_text_report(capsys, monkeypatch):
    monkeypatch.setattr(
        cli_merge,
        "merge_files",
        lambda *args, **kwargs: _merge_payload(output="merged.liel"),
    )

    assert cli.main(["merge", "left.liel", "right.liel", "-o", "merged.liel"]) == 0
    out = capsys.readouterr().out
    assert "Merged into merged.liel" in out
    assert "Nodes: +2 reused 1" in out
    assert "Edges: +1 reused 0" in out


def test_cli_merge_prints_json_report(capsys, monkeypatch):
    monkeypatch.setattr(
        cli_merge,
        "merge_files",
        lambda *args, **kwargs: _merge_payload(output="merged.liel"),
    )

    assert (
        cli.main(
            [
                "merge",
                "left.liel",
                "right.liel",
                "-o",
                "merged.liel",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["output"] == "merged.liel"
    assert payload["nodes_created"] == 2


def test_merge_rejects_in_place_output():
    try:
        cli_merge._reject_in_place_output(Path("left.liel"), Path("right.liel"), Path("left.liel"))
    except CliError as exc:
        assert exc.exit_code == 2
        assert "output must be different" in exc.message
    else:
        raise AssertionError("expected CliError")


def test_merge_report_payload_preserves_maps():
    report = SimpleNamespace(
        nodes_created=2,
        nodes_reused=1,
        edges_created=1,
        edges_reused=0,
        node_id_map={1: 10},
        edge_id_map={1: 20},
    )

    assert cli_merge._report_payload(report, Path("out.liel")) == {
        "output": "out.liel",
        "nodes_created": 2,
        "nodes_reused": 1,
        "edges_created": 1,
        "edges_reused": 0,
        "node_id_map": {1: 10},
        "edge_id_map": {1: 20},
    }


def _diff_report(*, changed: bool) -> dict[str, object]:
    node_diff = {"added": [3] if changed else [], "removed": [], "changed": [1] if changed else []}
    edge_diff = {"added": [], "removed": [], "changed": []}
    return {
        "changed": changed,
        "left": {"path": "left.liel", "nodes": 2, "edges": 1},
        "right": {"path": "right.liel", "nodes": 3 if changed else 2, "edges": 1},
        "nodes": node_diff,
        "edges": edge_diff,
    }


def _merge_payload(*, output: str) -> dict[str, object]:
    return {
        "output": output,
        "nodes_created": 2,
        "nodes_reused": 1,
        "edges_created": 1,
        "edges_reused": 0,
        "node_id_map": {1: 10, 2: 11},
        "edge_id_map": {1: 20},
    }

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import liel
from liel.cli import __main__ as cli
from liel.cli import diff as cli_diff
from liel.cli import exchange as cli_exchange
from liel.cli import identity as cli_identity
from liel.cli import manifest as cli_manifest
from liel.cli import merge as cli_merge
from liel.cli import pack as cli_pack
from liel.cli import signature as cli_signature
from liel.cli import stats as cli_stats
from liel.cli.common import CliError, refuse_overwrite


def test_cli_without_args_prints_help(capsys):
    assert cli.main([]) == 0
    out = capsys.readouterr().out
    assert "Local graph memory CLI" in out
    assert "help" in out
    assert "version" in out
    assert "pack" in out


def test_cli_help_prints_top_level_help(capsys):
    assert cli.main(["help"]) == 0
    out = capsys.readouterr().out
    assert "Local graph memory CLI" in out
    assert "diff" in out
    assert "merge" in out
    assert "pack" in out


def test_cli_help_prints_command_help(capsys):
    assert cli.main(["help", "merge"]) == 0
    out = capsys.readouterr().out
    assert "usage: liel merge" in out
    assert "--node-key" in out
    assert "--identity-rules" in out

    assert cli.main(["help", "diff"]) == 0
    diff_out = capsys.readouterr().out
    assert "usage: liel diff" in diff_out
    assert "--node-key" in diff_out
    assert "--identity-rules" in diff_out


def test_cli_help_prints_pack_help(capsys):
    assert cli.main(["help", "pack"]) == 0
    out = capsys.readouterr().out
    assert "usage: liel pack" in out
    assert "--include-labels" in out


def test_cli_help_prints_manifest_help(capsys):
    assert cli.main(["help", "manifest"]) == 0
    out = capsys.readouterr().out
    assert "usage: liel manifest" in out
    assert "--output" in out


def test_cli_help_prints_sign_and_verify_help(capsys):
    assert cli.main(["help", "sign"]) == 0
    sign_out = capsys.readouterr().out
    assert "usage: liel sign" in sign_out
    assert "--key-file" in sign_out

    assert cli.main(["help", "verify"]) == 0
    verify_out = capsys.readouterr().out
    assert "usage: liel verify" in verify_out
    assert "--signature" in verify_out


def test_cli_help_prints_stats_help(capsys):
    assert cli.main(["help", "stats"]) == 0
    out = capsys.readouterr().out
    assert "usage: liel stats" in out
    assert "--format" in out


def test_cli_help_prints_export_and_import_help(capsys):
    assert cli.main(["help", "export"]) == 0
    export_out = capsys.readouterr().out
    assert "usage: liel export" in export_out
    assert "--output" in export_out

    assert cli.main(["help", "import"]) == 0
    import_out = capsys.readouterr().out
    assert "usage: liel import" in import_out
    assert "--format" in import_out


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
    monkeypatch.setattr(
        cli_diff, "diff_files", lambda left, right, **kwargs: _diff_report(changed=False)
    )

    assert cli.main(["diff", "left.liel", "right.liel", "--format", "json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["changed"] is False
    assert payload["left"]["nodes"] == 2
    assert payload["right"]["edges"] == 1


def test_cli_diff_reports_added_and_changed_records(capsys, monkeypatch):
    monkeypatch.setattr(
        cli_diff, "diff_files", lambda left, right, **kwargs: _diff_report(changed=True)
    )

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


def test_cli_diff_node_key_matches_independent_ids(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"

    with liel.open(str(left)) as db:
        a = db.add_node(["File"], path="src/a.py", title="A")
        b = db.add_node(["File"], path="src/b.py", title="B")
        db.add_edge(a, "DEPENDS_ON", b)
        db.commit()

    with liel.open(str(right)) as db:
        b = db.add_node(["File"], path="src/b.py", title="B")
        a = db.add_node(["File"], path="src/a.py", title="A")
        db.add_edge(a, "DEPENDS_ON", b)
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--node-key", "path"]) == 0
    assert capsys.readouterr().out.strip() == "No differences."


def test_cli_diff_node_key_reports_property_changes(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"

    with liel.open(str(left)) as db:
        db.add_node(["File"], path="src/a.py", title="A")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["File"], path="src/a.py", title="A2")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--node-key", "path", "--format", "json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes"]["identity"] == {"mode": "node_key", "keys": ["path"]}
    assert payload["nodes"]["changed"] == ["path='src/a.py'"]


def test_cli_diff_node_key_reports_edge_property_changes(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"

    with liel.open(str(left)) as db:
        a = db.add_node(["File"], path="src/a.py")
        b = db.add_node(["File"], path="src/b.py")
        db.add_edge(a, "DEPENDS_ON", b, reason="old")
        db.commit()
    with liel.open(str(right)) as db:
        b = db.add_node(["File"], path="src/b.py")
        a = db.add_node(["File"], path="src/a.py")
        db.add_edge(a, "DEPENDS_ON", b, reason="new")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--node-key", "path", "--format", "json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    assert payload["edges"]["changed"] == []
    assert payload["edges"]["identity"] == {
        "mode": "node_key_edge_multiset",
        "node_keys": ["path"],
    }
    assert payload["edges"]["removed"] == [
        "path='src/a.py' -[DEPENDS_ON reason='old']-> path='src/b.py'"
    ]
    assert payload["edges"]["added"] == [
        "path='src/a.py' -[DEPENDS_ON reason='new']-> path='src/b.py'"
    ]


def test_cli_diff_node_key_compares_duplicate_edges_as_multiset(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"

    with liel.open(str(left)) as db:
        a = db.add_node(["File"], path="src/a.py")
        b = db.add_node(["File"], path="src/b.py")
        db.add_edge(a, "DEPENDS_ON", b, reason="same")
        db.add_edge(a, "DEPENDS_ON", b, reason="same")
        db.commit()
    with liel.open(str(right)) as db:
        b = db.add_node(["File"], path="src/b.py")
        a = db.add_node(["File"], path="src/a.py")
        db.add_edge(a, "DEPENDS_ON", b, reason="same")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--node-key", "path", "--format", "json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    assert payload["edges"]["removed"] == [
        "path='src/a.py' -[DEPENDS_ON reason='same']-> path='src/b.py'"
    ]
    assert payload["edges"]["added"] == []
    assert payload["edges"]["changed"] == []


def test_cli_diff_node_key_rejects_missing_key(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"

    with liel.open(str(left)) as db:
        db.add_node(["File"], path="src/a.py")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["File"], title="missing")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--node-key", "path"]) == 2
    assert "missing --node-key property: path" in capsys.readouterr().err


def test_cli_diff_identity_rules_match_by_label_specific_keys(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(
        json.dumps({"identity_rules": {"File": ["path"], "Task": ["system", "external_id"]}}),
        encoding="utf-8",
    )

    with liel.open(str(left)) as db:
        file_node = db.add_node(["File"], path="src/a.py", title="A")
        task = db.add_node(["Task"], system="github", external_id="123", title="Fix")
        db.add_edge(task, "DEPENDS_ON", file_node)
        db.commit()
    with liel.open(str(right)) as db:
        task = db.add_node(["Task"], system="github", external_id="123", title="Fix")
        file_node = db.add_node(["File"], path="src/a.py", title="A2")
        db.add_edge(task, "DEPENDS_ON", file_node)
        db.commit()

    assert (
        cli.main(
            ["diff", str(left), str(right), "--identity-rules", str(rules), "--format", "json"]
        )
        == 1
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes"]["identity"]["mode"] == "identity_rules"
    assert payload["nodes"]["changed"] == ["File:path='src/a.py'"]
    assert payload["edges"]["identity"] == {
        "mode": "identity_rules_edge_multiset",
        "rules": {"File": ["path"], "Task": ["system", "external_id"]},
    }
    assert payload["edges"]["added"] == []
    assert payload["edges"]["removed"] == []


def test_cli_diff_identity_rules_rejects_unmatched_node_label(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(json.dumps({"identity_rules": {"File": ["path"]}}), encoding="utf-8")

    with liel.open(str(left)) as db:
        db.add_node(["File"], path="src/a.py")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Task"], external_id="123")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--identity-rules", str(rules)]) == 2
    assert "does not match any --identity-rules label" in capsys.readouterr().err


def test_cli_diff_identity_rules_rejects_multiple_matching_labels(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(
        json.dumps({"identity_rules": {"File": ["path"], "Source": ["url"]}}),
        encoding="utf-8",
    )

    with liel.open(str(left)) as db:
        db.add_node(["File"], path="src/a.py")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["File", "Source"], path="src/a.py", url="https://example.com")
        db.commit()

    assert cli.main(["diff", str(left), str(right), "--identity-rules", str(rules)]) == 2
    assert "matches multiple --identity-rules labels" in capsys.readouterr().err


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
    assert "Can merge: yes" in out
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


def test_cli_merge_requires_output_unless_dry_run(capsys):
    assert cli.main(["merge", "left.liel", "right.liel"]) == 2
    assert "merge output is required unless --dry-run is set" in capsys.readouterr().err


def test_cli_merge_dry_run_prints_preview_without_output(capsys, monkeypatch):
    monkeypatch.setattr(
        cli_merge,
        "merge_files",
        lambda *args, **kwargs: _merge_payload(output=None, dry_run=True),
    )

    assert cli.main(["merge", "left.liel", "right.liel", "--dry-run"]) == 0
    out = capsys.readouterr().out
    assert "Dry-run merge preview for (no output path)" in out
    assert "Nodes: +2 reused 1" in out


def test_cli_merge_dry_run_does_not_create_output(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    output = tmp_path / "merged.liel"
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], tag="B")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "-o",
                str(output),
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["dry_run"] is True
    assert payload["can_merge"] is True
    assert payload["conflicts"] == []
    assert payload["warnings"] == []
    assert payload["output"] == str(output)
    assert payload["nodes_created"] == 1
    assert not output.exists()


def test_cli_merge_dry_run_reports_missing_node_key_conflict(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], name="missing")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--node-key",
                "tag",
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["can_merge"] is False
    assert payload["conflicts"][0]["type"] == "missing_node_key"
    assert payload["conflicts"][0]["side"] == "source"
    assert payload["node_id_map"] == {}


def test_cli_merge_dry_run_reports_duplicate_destination_key_conflict(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A")
        db.add_node(["Item"], tag="A")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], tag="A")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--node-key",
                "tag",
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    conflict_types = {conflict["type"] for conflict in payload["conflicts"]}
    assert "duplicate_node_key" in conflict_types
    assert "ambiguous_destination_node_key" in conflict_types


def test_cli_merge_dry_run_reports_property_conflict_warnings(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A", status="open")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], tag="A", status="closed")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--node-key",
                "tag",
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["can_merge"] is True
    assert payload["conflicts"] == []
    assert payload["warnings"] == [
        {
            "type": "node_property_conflict",
            "identity": "tag='A'",
            "property": "status",
            "destination": "open",
            "source": "closed",
            "policy": "keep_dst",
            "resolution": "source_ignored",
            "message": "tag='A' property 'status' differs; source_ignored by keep_dst",
        }
    ]


def test_cli_merge_dry_run_prints_warning_summary(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A", status="open")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], tag="A", status="closed")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--node-key",
                "tag",
                "--dry-run",
            ]
        )
        == 0
    )
    out = capsys.readouterr().out
    assert "Warnings: 1" in out
    assert "node_property_conflict" in out


def test_cli_merge_node_key_edge_strategy_and_merge_props_options(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    output = tmp_path / "merged.liel"
    with liel.open(str(left)) as db:
        a = db.add_node(["Item"], tag="A", name="dst")
        b = db.add_node(["Item"], tag="B")
        db.add_edge(a, "LINKS", b, kind="same")
        db.commit()
    with liel.open(str(right)) as db:
        a = db.add_node(["Item"], tag="A", name="src", age=7)
        b = db.add_node(["Item"], tag="B")
        db.add_edge(a, "LINKS", b, kind="same")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "-o",
                str(output),
                "--node-key",
                "tag",
                "--edge-strategy",
                "idempotent",
                "--on-node-conflict",
                "merge_props",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes_created"] == 0
    assert payload["nodes_reused"] == 2
    assert payload["edges_created"] == 0
    assert payload["edges_reused"] == 1

    with liel.open(str(output)) as db:
        records = {record["tag"]: record for record in db.all_nodes_as_records()}
        assert db.edge_count() == 1
    assert records["A"]["name"] == "dst"
    assert records["A"]["age"] == 7


def test_cli_merge_identity_rules_reuses_label_specific_nodes(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    output = tmp_path / "merged.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(
        json.dumps({"identity_rules": {"File": ["path"], "Task": ["system", "external_id"]}}),
        encoding="utf-8",
    )

    with liel.open(str(left)) as db:
        file_node = db.add_node(["File"], path="src/a.py", title="dst")
        task = db.add_node(["Task"], system="github", external_id="123", status="open")
        db.add_edge(task, "DEPENDS_ON", file_node, reason="same")
        db.commit()
    with liel.open(str(right)) as db:
        file_node = db.add_node(["File"], path="src/a.py", title="src", language="python")
        task = db.add_node(["Task"], system="github", external_id="123", status="done")
        db.add_edge(task, "DEPENDS_ON", file_node, reason="same")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "-o",
                str(output),
                "--identity-rules",
                str(rules),
                "--edge-strategy",
                "idempotent",
                "--on-node-conflict",
                "merge_props",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes_created"] == 0
    assert payload["nodes_reused"] == 2
    assert payload["edges_created"] == 0
    assert payload["edges_reused"] == 1

    with liel.open(str(output)) as db:
        nodes = db.all_nodes_as_records()
        edges = db.all_edges_as_records()
    file_record = next(record for record in nodes if "File" in record["labels"])
    task_record = next(record for record in nodes if "Task" in record["labels"])
    assert file_record["title"] == "dst"
    assert file_record["language"] == "python"
    assert task_record["status"] == "open"
    assert len(edges) == 1


def test_cli_merge_identity_rules_dry_run_reports_unmatched_source(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(json.dumps({"identity_rules": {"File": ["path"]}}), encoding="utf-8")
    with liel.open(str(left)) as db:
        db.add_node(["File"], path="src/a.py")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Note"], text="no identity rule")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--identity-rules",
                str(rules),
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["can_merge"] is False
    assert payload["conflicts"][0]["type"] == "unmatched_identity_rule"
    assert payload["node_id_map"] == {}


def test_cli_merge_identity_rules_dry_run_reports_property_and_label_warnings(tmp_path, capsys):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    rules = tmp_path / "rules.json"
    rules.write_text(json.dumps({"identity_rules": {"Task": ["external_id"]}}), encoding="utf-8")
    with liel.open(str(left)) as db:
        db.add_node(["Task"], external_id="123", status="open")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Task", "Imported"], external_id="123", status="closed")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "--identity-rules",
                str(rules),
                "--dry-run",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    warning_types = {warning["type"] for warning in payload["warnings"]}
    assert warning_types == {"node_label_difference", "node_property_conflict"}


def test_cli_merge_overwrite_conflict_option_updates_matching_node(tmp_path):
    left = tmp_path / "left.liel"
    right = tmp_path / "right.liel"
    output = tmp_path / "merged.liel"
    with liel.open(str(left)) as db:
        db.add_node(["User"], email="alice@example.com", name="old")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["User"], email="alice@example.com", name="new")
        db.commit()

    assert (
        cli.main(
            [
                "merge",
                str(left),
                str(right),
                "-o",
                str(output),
                "--node-key",
                "email",
                "--on-node-conflict",
                "overwrite_from_src",
            ]
        )
        == 0
    )

    with liel.open(str(output)) as db:
        records = db.all_nodes_as_records()
    assert len(records) == 1
    assert records[0]["name"] == "new"


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
        "dry_run": False,
        "can_merge": True,
        "conflicts": [],
        "warnings": [],
        "output": "out.liel",
        "nodes_created": 2,
        "nodes_reused": 1,
        "edges_created": 1,
        "edges_reused": 0,
        "node_id_map": {1: 10},
        "edge_id_map": {1: 20},
    }


def test_cli_pack_extracts_selected_labels(tmp_path):
    source = tmp_path / "source.liel"
    output = tmp_path / "packed.liel"
    with liel.open(str(source)) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Person"], name="Bob")
        acme = db.add_node(["Company"], name="Acme")
        db.add_edge(alice, "KNOWS", bob, since=2024)
        db.add_edge(alice, "WORKS_AT", acme)
        db.commit()

    assert (
        cli.main(
            [
                "pack",
                str(source),
                str(output),
                "--include-labels",
                "Person",
            ]
        )
        == 0
    )

    with liel.open(str(output)) as db:
        nodes = db.all_nodes_as_records()
        edges = db.all_edges_as_records()

    assert [node["name"] for node in nodes] == ["Alice", "Bob"]
    assert [edge["label"] for edge in edges] == ["KNOWS"]
    assert edges[0]["from_node"] == 1
    assert edges[0]["to_node"] == 2


def test_cli_pack_prints_json_report(capsys, monkeypatch):
    monkeypatch.setattr(
        cli_pack,
        "pack_file",
        lambda *args, **kwargs: {
            "source": "source.liel",
            "output": "packed.liel",
            "include_labels": ["Person"],
            "source_nodes": 3,
            "source_edges": 2,
            "nodes_packed": 2,
            "edges_packed": 1,
            "node_id_map": {1: 1, 2: 2},
        },
    )

    assert (
        cli.main(
            [
                "pack",
                "source.liel",
                "packed.liel",
                "--include-labels",
                "Person",
                "--format",
                "json",
            ]
        )
        == 0
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes_packed"] == 2
    assert payload["edges_packed"] == 1


def test_pack_rejects_in_place_output():
    try:
        cli_pack._reject_in_place_output(Path("source.liel"), Path("source.liel"))
    except CliError as exc:
        assert exc.exit_code == 2
        assert "output must be different" in exc.message
    else:
        raise AssertionError("expected CliError")


def test_pack_normalizes_include_labels_for_deterministic_reports(tmp_path):
    source = tmp_path / "source.liel"
    output = tmp_path / "packed.liel"
    with liel.open(str(source)) as db:
        db.add_node(["Task"], key="T-1")
        db.add_node(["Person"], name="Alice")
        db.commit()

    payload = cli_pack.pack_file(
        source,
        output,
        include_labels=[" Task , Person ", "Person"],
    )

    # Canonical report shape: labels are unique, trimmed, and sorted.
    assert payload["include_labels"] == ["Person", "Task"]


def test_pack_report_is_stable_across_label_argument_order(tmp_path):
    source = tmp_path / "source.liel"
    out_a = tmp_path / "packed-a.liel"
    out_b = tmp_path / "packed-b.liel"
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.add_node(["Task"], key="T-1")
        db.commit()

    payload_a = cli_pack.pack_file(source, out_a, include_labels=["Task", "Person"])
    payload_b = cli_pack.pack_file(source, out_b, include_labels=["Person", "Task"])

    assert payload_a["include_labels"] == payload_b["include_labels"] == ["Person", "Task"]
    assert (
        cli_pack.format_text(payload_a).splitlines()[1]
        == cli_pack.format_text(payload_b).splitlines()[1]
    )


def test_pack_node_id_map_order_follows_source_id_order(tmp_path):
    source = tmp_path / "source.liel"
    output = tmp_path / "packed.liel"
    with liel.open(str(source)) as db:
        first = db.add_node(["Person"], name="Alice")
        db.add_node(["Company"], name="Acme")
        third = db.add_node(["Person"], name="Bob")
        db.commit()

    payload = cli_pack.pack_file(source, output, include_labels=["Person"])

    assert list(payload["node_id_map"].keys()) == [first.id, third.id]


def test_manifest_bytes_match_expected_json(tmp_path):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Person", "Task"], key="T-1")
        db.add_edge(alice, "KNOWS", bob, since=2024)
        db.commit()

    assert cli_manifest.build_manifest_bytes(source).decode("utf-8") == (
        "{\n"
        '  "edge_count": 1,\n'
        '  "edges": [\n'
        "    {\n"
        '      "from_node": 1,\n'
        '      "id": 1,\n'
        '      "label": "KNOWS",\n'
        '      "properties": {\n'
        '        "since": 2024\n'
        "      },\n"
        '      "to_node": 2\n'
        "    }\n"
        "  ],\n"
        '  "liel_format": "1.0",\n'
        '  "manifest_version": 1,\n'
        '  "node_count": 2,\n'
        '  "nodes": [\n'
        "    {\n"
        '      "id": 1,\n'
        '      "labels": [\n'
        '        "Person"\n'
        "      ],\n"
        '      "properties": {\n'
        '        "name": "Alice"\n'
        "      }\n"
        "    },\n"
        "    {\n"
        '      "id": 2,\n'
        '      "labels": [\n'
        '        "Person",\n'
        '        "Task"\n'
        "      ],\n"
        '      "properties": {\n'
        '        "key": "T-1"\n'
        "      }\n"
        "    }\n"
        "  ]\n"
        "}\n"
    )


def test_manifest_generation_is_byte_stable(tmp_path):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        db.add_node(["Task", "Person"], z=2, a=1)
        db.commit()

    assert cli_manifest.build_manifest_bytes(source) == cli_manifest.build_manifest_bytes(source)


def test_manifest_does_not_depend_on_file_name(tmp_path):
    source = tmp_path / "source.liel"
    renamed = tmp_path / "renamed.liel"
    with liel.open(str(source)) as db:
        db.add_node(["File"], path="docs/guide/cli.md")
        db.commit()
    shutil.copyfile(source, renamed)

    assert cli_manifest.build_manifest_bytes(source) == cli_manifest.build_manifest_bytes(renamed)


def test_cli_manifest_writes_output_file(tmp_path):
    source = tmp_path / "source.liel"
    output = tmp_path / "manifest.json"
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()

    assert cli.main(["manifest", str(source), "-o", str(output)]) == 0
    assert output.read_text(encoding="utf-8").startswith('{\n  "edge_count": 0,')


def test_sign_writes_deterministic_external_signature(tmp_path):
    source = tmp_path / "source.liel"
    key = tmp_path / "secret.key"
    key.write_bytes(b"test-secret")
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()

    payload = cli_signature.sign_file(source, key)
    assert payload["algorithm"] == "hmac-sha256"
    assert payload["manifest_version"] == 1
    assert len(payload["manifest_sha256"]) == 64
    assert len(payload["signature"]) == 64
    assert cli_signature.signature_payload_bytes(payload) == cli_signature.signature_payload_bytes(
        cli_signature.sign_file(source, key)
    )


def test_cli_sign_and_verify_round_trip(tmp_path, capsys):
    source = tmp_path / "source.liel"
    key = tmp_path / "secret.key"
    signature = tmp_path / "source.liel.sig"
    key.write_bytes(b"test-secret")
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()

    assert cli.main(["sign", str(source), "--key-file", str(key), "-o", str(signature)]) == 0
    assert (
        cli.main(["verify", str(source), "--key-file", str(key), "--signature", str(signature)])
        == 0
    )
    assert capsys.readouterr().out.strip() == "Signature OK."


def test_cli_verify_rejects_changed_graph(tmp_path, capsys):
    source = tmp_path / "source.liel"
    key = tmp_path / "secret.key"
    signature = tmp_path / "source.liel.sig"
    key.write_bytes(b"test-secret")
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()

    assert cli.main(["sign", str(source), "--key-file", str(key), "-o", str(signature)]) == 0
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Bob")
        db.commit()

    assert (
        cli.main(["verify", str(source), "--key-file", str(key), "--signature", str(signature)])
        == 1
    )
    assert capsys.readouterr().out.strip() == "Signature verification failed."


def test_cli_verify_json_reports_failure(tmp_path, capsys):
    source = tmp_path / "source.liel"
    key = tmp_path / "secret.key"
    wrong_key = tmp_path / "wrong.key"
    signature = tmp_path / "source.liel.sig"
    key.write_bytes(b"test-secret")
    wrong_key.write_bytes(b"wrong-secret")
    with liel.open(str(source)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()

    assert cli.main(["sign", str(source), "--key-file", str(key), "-o", str(signature)]) == 0
    assert (
        cli.main(
            [
                "verify",
                str(source),
                "--key-file",
                str(wrong_key),
                "--signature",
                str(signature),
                "--format",
                "json",
            ]
        )
        == 1
    )
    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is False
    assert payload["algorithm"] == "hmac-sha256"


def test_cli_stats_prints_text_summary(tmp_path, capsys):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Person", "Task"], key="T-1")
        db.add_edge(alice, "KNOWS", bob)
        db.commit()

    assert cli.main(["stats", str(source)]) == 0
    out = capsys.readouterr().out
    assert f"File: {source}" in out
    assert "Nodes: 2" in out
    assert "Edges: 1" in out
    assert "  Person: 2" in out
    assert "  Task: 1" in out
    assert "  KNOWS: 1" in out


def test_cli_stats_prints_json_summary(tmp_path, capsys):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Task"], key="T-1")
        db.add_edge(alice, "KNOWS", bob)
        db.commit()

    assert cli.main(["stats", str(source), "--format", "json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["node_count"] == 2
    assert payload["edge_count"] == 1
    assert payload["node_labels"] == {"Person": 1, "Task": 1}
    assert payload["edge_labels"] == {"KNOWS": 1}


def test_stats_file_sorts_label_counts(tmp_path):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        db.add_node(["Zed", "Alpha"])
        db.add_node(["Alpha"])
        db.commit()

    payload = cli_stats.stats_file(source)
    assert list(payload["node_labels"]) == ["Alpha", "Zed"]


def test_export_json_matches_expected_deterministic_shape(tmp_path):
    source = tmp_path / "source.liel"
    with liel.open(str(source)) as db:
        alice = db.add_node(["Person"], name="Alice")
        bob = db.add_node(["Task", "Person"], key="T-1")
        db.add_edge(alice, "KNOWS", bob, since=2024)
        db.commit()

    assert cli_exchange.build_export_bytes(source).decode("utf-8") == (
        "{\n"
        '  "edge_count": 1,\n'
        '  "edges": [\n'
        "    {\n"
        '      "from_node": 1,\n'
        '      "id": 1,\n'
        '      "label": "KNOWS",\n'
        '      "properties": {\n'
        '        "since": 2024\n'
        "      },\n"
        '      "to_node": 2\n'
        "    }\n"
        "  ],\n"
        '  "export_version": 1,\n'
        '  "liel_format": "1.0",\n'
        '  "node_count": 2,\n'
        '  "nodes": [\n'
        "    {\n"
        '      "id": 1,\n'
        '      "labels": [\n'
        '        "Person"\n'
        "      ],\n"
        '      "properties": {\n'
        '        "name": "Alice"\n'
        "      }\n"
        "    },\n"
        "    {\n"
        '      "id": 2,\n'
        '      "labels": [\n'
        '        "Person",\n'
        '        "Task"\n'
        "      ],\n"
        '      "properties": {\n'
        '        "key": "T-1"\n'
        "      }\n"
        "    }\n"
        "  ]\n"
        "}\n"
    )


def test_cli_export_writes_json_file(tmp_path):
    source = tmp_path / "source.liel"
    output = tmp_path / "graph.json"
    with liel.open(str(source)) as db:
        db.add_node(["File"], path="docs/guide/cli.md")
        db.commit()

    assert cli.main(["export", str(source), "-o", str(output)]) == 0
    payload = json.loads(output.read_text(encoding="utf-8"))
    assert payload["export_version"] == 1
    assert payload["nodes"][0]["properties"] == {"path": "docs/guide/cli.md"}


def test_cli_import_restores_unordered_export_json(tmp_path):
    source = tmp_path / "graph.json"
    output = tmp_path / "restored.liel"
    source.write_text(
        json.dumps(
            {
                "export_version": 1,
                "liel_format": "1.0",
                "node_count": 2,
                "edge_count": 1,
                "nodes": [
                    {"id": 20, "labels": ["Task"], "properties": {"key": "B"}},
                    {"id": 10, "labels": ["Task"], "properties": {"key": "A"}},
                ],
                "edges": [
                    {
                        "id": 99,
                        "from_node": 20,
                        "to_node": 10,
                        "label": "DEPENDS_ON",
                        "properties": {"kind": "test"},
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    assert cli.main(["import", str(source), "-o", str(output)]) == 0
    with liel.open(str(output)) as db:
        nodes = db.all_nodes_as_records()
        edges = db.all_edges_as_records()

    assert [node["key"] for node in nodes] == ["A", "B"]
    assert edges[0]["label"] == "DEPENDS_ON"
    assert edges[0]["from_node"] == 2
    assert edges[0]["to_node"] == 1
    assert edges[0]["kind"] == "test"


def test_cli_import_json_report_preserves_id_maps(tmp_path, capsys):
    source = tmp_path / "graph.json"
    output = tmp_path / "restored.liel"
    source.write_text(
        json.dumps(
            {
                "export_version": 1,
                "liel_format": "1.0",
                "node_count": 1,
                "edge_count": 0,
                "nodes": [{"id": 7, "labels": ["Person"], "properties": {"name": "Alice"}}],
                "edges": [],
            }
        ),
        encoding="utf-8",
    )

    assert cli.main(["import", str(source), "-o", str(output), "--format", "json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["nodes_imported"] == 1
    assert payload["edges_imported"] == 0
    assert payload["node_id_map"] == {"7": 1}


def test_cli_output_commands_refuse_existing_output_without_force(tmp_path, capsys):
    for name, output, args in _output_command_cases(tmp_path, force=False):
        output.write_text("existing", encoding="utf-8")

        assert cli.main(args) == 2, name
        assert "refusing to overwrite existing file" in capsys.readouterr().err


def test_cli_output_commands_accept_force_for_existing_output(tmp_path):
    for name, output, args in _output_command_cases(tmp_path, force=True):
        output.write_text("existing", encoding="utf-8")

        assert cli.main(args) == 0, name
        assert output.exists()


def test_import_rejects_missing_edge_endpoint(tmp_path):
    source = tmp_path / "graph.json"
    output = tmp_path / "restored.liel"
    source.write_text(
        json.dumps(
            {
                "export_version": 1,
                "liel_format": "1.0",
                "node_count": 1,
                "edge_count": 1,
                "nodes": [{"id": 1, "labels": [], "properties": {}}],
                "edges": [
                    {"id": 1, "from_node": 1, "to_node": 2, "label": "BROKEN", "properties": {}}
                ],
            }
        ),
        encoding="utf-8",
    )

    try:
        cli_exchange.import_file(source, output)
    except CliError as exc:
        assert exc.exit_code == 2
        assert "references a missing node" in exc.message
    else:
        raise AssertionError("expected CliError")


def test_cli_sign_rejects_empty_key_file(tmp_path, capsys):
    source = _create_sample_liel(tmp_path / "source.liel")
    key = tmp_path / "empty.key"
    key.write_bytes(b"")

    assert cli.main(["sign", str(source), "--key-file", str(key)]) == 2
    assert "key file must not be empty" in capsys.readouterr().err


def test_cli_verify_rejects_invalid_signature_shape(tmp_path, capsys):
    source = _create_sample_liel(tmp_path / "source.liel")
    key = tmp_path / "secret.key"
    signature = tmp_path / "bad.sig"
    key.write_bytes(b"test-secret")
    signature.write_text("{}", encoding="utf-8")

    assert (
        cli.main(["verify", str(source), "--key-file", str(key), "--signature", str(signature)])
        == 2
    )
    assert "invalid signature file" in capsys.readouterr().err


def test_cli_import_rejects_non_object_export_json(tmp_path, capsys):
    source = tmp_path / "graph.json"
    output = tmp_path / "restored.liel"
    source.write_text("[]", encoding="utf-8")

    assert cli.main(["import", str(source), "-o", str(output)]) == 2
    assert "export JSON must contain an object" in capsys.readouterr().err


def test_cli_import_rejects_invalid_node_record_shape(tmp_path, capsys):
    source = tmp_path / "graph.json"
    output = tmp_path / "restored.liel"
    source.write_text(
        json.dumps(
            {
                "export_version": 1,
                "liel_format": "1.0",
                "node_count": 1,
                "edge_count": 0,
                "nodes": [{"id": "1", "labels": [], "properties": {}}],
                "edges": [],
            }
        ),
        encoding="utf-8",
    )

    assert cli.main(["import", str(source), "-o", str(output)]) == 2
    assert "node id must be an integer" in capsys.readouterr().err


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


def _merge_payload(*, output: str | None, dry_run: bool = False) -> dict[str, object]:
    return {
        "dry_run": dry_run,
        "can_merge": True,
        "conflicts": [],
        "warnings": [],
        "output": output,
        "nodes_created": 2,
        "nodes_reused": 1,
        "edges_created": 1,
        "edges_reused": 0,
        "node_id_map": {1: 10, 2: 11},
        "edge_id_map": {1: 20},
    }


def _create_sample_liel(path: Path) -> Path:
    with liel.open(str(path)) as db:
        db.add_node(["Person"], name="Alice")
        db.commit()
    return path


def _write_minimal_export(path: Path) -> None:
    path.write_text(
        json.dumps(
            {
                "export_version": 1,
                "liel_format": "1.0",
                "node_count": 1,
                "edge_count": 0,
                "nodes": [{"id": 1, "labels": ["Person"], "properties": {"name": "Alice"}}],
                "edges": [],
            }
        ),
        encoding="utf-8",
    )


def _output_command_cases(tmp_path: Path, *, force: bool) -> list[tuple[str, Path, list[str]]]:
    source = _create_sample_liel(tmp_path / "source.liel")
    right = _create_sample_liel(tmp_path / "right.liel")
    export_json = tmp_path / "graph.json"
    _write_minimal_export(export_json)
    key = tmp_path / "secret.key"
    key.write_bytes(b"test-secret")

    maybe_force = ["--force"] if force else []
    merge_output = tmp_path / "merge.liel"
    pack_output = tmp_path / "pack.liel"
    manifest_output = tmp_path / "manifest.json"
    sign_output = tmp_path / "sign.sig"
    export_output = tmp_path / "export.json"
    import_output = tmp_path / "import.liel"
    return [
        (
            "merge",
            merge_output,
            ["merge", str(source), str(right), "-o", str(merge_output), *maybe_force],
        ),
        (
            "pack",
            pack_output,
            ["pack", str(source), str(pack_output), "--include-labels", "Person", *maybe_force],
        ),
        (
            "manifest",
            manifest_output,
            ["manifest", str(source), "-o", str(manifest_output), *maybe_force],
        ),
        (
            "sign",
            sign_output,
            ["sign", str(source), "--key-file", str(key), "-o", str(sign_output), *maybe_force],
        ),
        (
            "export",
            export_output,
            ["export", str(source), "-o", str(export_output), *maybe_force],
        ),
        (
            "import",
            import_output,
            ["import", str(export_json), "-o", str(import_output), *maybe_force],
        ),
    ]

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS_FIXTURE = ROOT / "docs/guide/sample-viewer/app/fixtures/trace-why-postgres.export.json"
EXAMPLE_FIXTURE = ROOT / "examples/sample_viewer/fixtures/trace-why-postgres.export.json"
DOCS_VIEWER_JS = ROOT / "docs/guide/sample-viewer/app/viewer.js"
EXAMPLE_VIEWER_JS = ROOT / "examples/sample_viewer/viewer.js"
SAMPLE_VIEWER_DOC = ROOT / "docs/guide/sample-viewer.md"
VIEWER_JSON_DOC = ROOT / "docs/reference/viewer-json.md"


def load_fixture(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def embedded_fixture_text(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    marker = "const DEFAULT_EXPORT = "
    start = text.index(marker) + len(marker)
    end = text.index(";\n\n  const statusEl", start)
    object_literal = text[start:end]
    quoted_keys = re.sub(
        r"([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:",
        r'\1"\2":',
        object_literal,
    )
    return re.sub(r",(\s*[}\]])", r"\1", quoted_keys)


def test_sample_viewer_fixtures_stay_in_sync() -> None:
    assert load_fixture(DOCS_FIXTURE) == load_fixture(EXAMPLE_FIXTURE)


def test_sample_viewer_fixture_matches_export_contract() -> None:
    payload = load_fixture(DOCS_FIXTURE)
    assert set(payload) == {
        "edge_count",
        "edges",
        "export_version",
        "liel_format",
        "node_count",
        "nodes",
    }
    assert payload["export_version"] == 1
    assert payload["liel_format"] == "1.0"

    nodes = payload["nodes"]
    edges = payload["edges"]
    assert isinstance(nodes, list)
    assert isinstance(edges, list)
    assert payload["node_count"] == len(nodes)
    assert payload["edge_count"] == len(edges)

    node_ids = set()
    for node in nodes:
        assert set(node) == {"id", "labels", "properties"}
        assert isinstance(node["id"], int)
        assert isinstance(node["labels"], list)
        assert isinstance(node["properties"], dict)
        node_ids.add(node["id"])

    edge_ids = set()
    for edge in edges:
        assert set(edge) == {"from_node", "id", "label", "properties", "to_node"}
        assert isinstance(edge["id"], int)
        assert isinstance(edge["from_node"], int)
        assert isinstance(edge["to_node"], int)
        assert isinstance(edge["label"], str)
        assert isinstance(edge["properties"], dict)
        assert edge["from_node"] in node_ids
        assert edge["to_node"] in node_ids
        edge_ids.add(edge["id"])

    assert len(node_ids) == len(nodes)
    assert len(edge_ids) == len(edges)


def test_sample_viewer_and_docs_reference_the_shared_fixture() -> None:
    fixture_name = "trace-why-postgres.export.json"
    for path in [DOCS_VIEWER_JS, EXAMPLE_VIEWER_JS, SAMPLE_VIEWER_DOC, VIEWER_JSON_DOC]:
        assert fixture_name in path.read_text(encoding="utf-8")


def test_sample_viewer_embedded_fallback_matches_checked_in_fixture() -> None:
    expected = load_fixture(DOCS_FIXTURE)
    assert json.loads(embedded_fixture_text(DOCS_VIEWER_JS)) == expected
    assert json.loads(embedded_fixture_text(EXAMPLE_VIEWER_JS)) == expected


def test_sample_viewer_preserves_fallback_status_until_render() -> None:
    for path in [DOCS_VIEWER_JS, EXAMPLE_VIEWER_JS]:
        text = path.read_text(encoding="utf-8")
        assert "statusMessage: null" in text
        assert "state.statusMessage ||" in text
        assert "state.statusMessage =" in text

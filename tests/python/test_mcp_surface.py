"""Integration-style checks for the public MCP surface."""

from __future__ import annotations

import sys
import types
import uuid
from pathlib import Path

import pytest

from liel.mcp import __main__ as mcp_main
from liel.mcp.server import LielFileDiscoveryError, _discover_liel_file, create_server


class _FakeFastMCP:
    """Tiny stand-in for FastMCP used to inspect registered tools."""

    def __init__(self, name: str, instructions: str):
        self.name = name
        self.instructions = instructions
        self.tools: dict[str, object] = {}
        self.runs: list[str] = []

    def tool(self):
        def _decorator(func):
            self.tools[func.__name__] = func
            return func

        return _decorator

    def run(self, transport: str = "stdio") -> None:
        self.runs.append(transport)


def _install_fake_fastmcp(monkeypatch: pytest.MonkeyPatch) -> None:
    mcp_pkg = types.ModuleType("mcp")
    server_pkg = types.ModuleType("mcp.server")
    fastmcp_pkg = types.ModuleType("mcp.server.fastmcp")
    fastmcp_pkg.FastMCP = _FakeFastMCP

    monkeypatch.setitem(sys.modules, "mcp", mcp_pkg)
    monkeypatch.setitem(sys.modules, "mcp.server", server_pkg)
    monkeypatch.setitem(sys.modules, "mcp.server.fastmcp", fastmcp_pkg)


def test_create_server_registers_official_tools(monkeypatch):
    _install_fake_fastmcp(monkeypatch)
    cleanup_callbacks: list[object] = []

    def _capture_cleanup(callback):
        cleanup_callbacks.append(callback)
        return callback

    monkeypatch.setattr("liel.mcp.server.atexit.register", _capture_cleanup)

    try:
        mcp = create_server(":memory:")

        assert mcp.name == "liel"
        assert set(mcp.tools) == {
            "liel_overview",
            "liel_find",
            "liel_explore",
            "liel_trace",
            "liel_map",
            "liel_append",
            "liel_merge",
        }
    finally:
        for callback in cleanup_callbacks:
            callback()


def test_cli_help_mentions_ai_memory(capsys, monkeypatch):
    monkeypatch.setattr(sys, "argv", ["liel-mcp", "--help"])

    with pytest.raises(SystemExit) as exc_info:
        mcp_main.main()

    output = capsys.readouterr().out
    assert exc_info.value.code == 0
    assert "AI memory MCP server" in output
    assert "liel_append" in output
    assert "liel_merge" in output


def test_cli_main_delegates_to_create_server(monkeypatch):
    calls: dict[str, object] = {}

    class _StubServer:
        def run(self, transport: str = "stdio") -> None:
            calls["transport"] = transport

    def _fake_create_server(path=None):
        calls["path"] = path
        return _StubServer()

    import liel.mcp.server as server_module

    monkeypatch.setattr(server_module, "create_server", _fake_create_server)
    monkeypatch.setattr(sys, "argv", ["liel-mcp", "--path", "demo.liel", "--transport", "sse"])

    mcp_main.main()

    assert calls == {"path": "demo.liel", "transport": "sse"}


def _discovery_test_dir(name: str) -> Path:
    path = Path("target") / f"test-mcp-discovery-{name}-{uuid.uuid4().hex}"
    path.mkdir(parents=True)
    return path


def test_discover_liel_file_uses_single_candidate():
    test_dir = _discovery_test_dir("single")
    memory = test_dir / "memory.liel"
    memory.touch()

    assert _discover_liel_file(test_dir) == str(memory)


def test_discover_liel_file_defaults_to_memory_file_when_empty():
    test_dir = _discovery_test_dir("empty")

    assert _discover_liel_file(test_dir) == str(test_dir / "memory.liel")


def test_discover_liel_file_rejects_multiple_candidates():
    test_dir = _discovery_test_dir("multiple")
    memory = test_dir / "memory.liel"
    other = test_dir / "other.liel"
    memory.touch()
    other.touch()

    with pytest.raises(LielFileDiscoveryError) as exc_info:
        _discover_liel_file(test_dir)

    message = str(exc_info.value)
    assert "Multiple .liel files found in the current directory" in message
    assert "register it with --path" in message
    assert memory.resolve().as_posix() in message
    assert other.resolve().as_posix() in message


def test_discover_liel_file_ignores_nested_candidates_when_empty():
    test_dir = _discovery_test_dir("nested")
    nested = test_dir / "nested" / "other.liel"
    nested.parent.mkdir()
    nested.touch()

    assert _discover_liel_file(test_dir) == str(test_dir / "memory.liel")

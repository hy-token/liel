"""Integration-style checks for the public MCP surface."""

from __future__ import annotations

import sys
import types

import pytest

from liel.mcp import __main__ as mcp_main
from liel.mcp.server import create_server


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

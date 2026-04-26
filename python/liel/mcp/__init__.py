"""liel MCP plugin - optional AI memory server for ``.liel`` files.

Usage (programmatic)::

    from liel.mcp import create_server
    mcp = create_server("my.liel")
    mcp.run()

Usage (CLI)::

    liel-mcp --path my.liel
    python -m liel.mcp --path my.liel
"""

from liel.mcp.server import create_server

__all__ = ["create_server"]

"""CLI entry point for the liel MCP server.

Usage::

    python -m liel.mcp --path my.liel
    liel-mcp --path my.liel          # after pip install "liel[mcp]"
    liel-mcp                          # auto-discovers *.liel in cwd
"""

from __future__ import annotations

import argparse
import sys


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="liel-mcp",
        description="Start the liel AI memory MCP server for a .liel file.",
        epilog=(
            "Official tools: liel_overview, liel_find, liel_explore, "
            "liel_trace, liel_map, liel_append, liel_merge."
        ),
    )
    parser.add_argument(
        "--path",
        metavar="FILE",
        default=None,
        help=(
            "Path to the .liel database file used as durable AI memory. "
            "If omitted, the first .liel file found under the current "
            "directory is used."
        ),
    )
    parser.add_argument(
        "--transport",
        choices=["stdio", "sse"],
        default="stdio",
        help="MCP transport to use for the AI memory server (default: stdio).",
    )
    args = parser.parse_args()

    try:
        from liel.mcp.server import create_server
    except ImportError as exc:
        print(
            f'Error: {exc}\nInstall AI memory MCP support with:  pip install "liel[mcp]"',
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        mcp = create_server(path=args.path)
    except FileNotFoundError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)

    mcp.run(transport=args.transport)


if __name__ == "__main__":
    main()

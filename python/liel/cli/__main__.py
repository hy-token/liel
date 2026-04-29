from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence

import liel

from .common import EXIT_ERROR, EXIT_OK, CliError, add_format_argument, emit_json, emit_text
from .diff import run as run_diff
from .merge import run as run_merge


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="liel",
        description="Local graph memory CLI.",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"%(prog)s {liel.__version__}",
    )

    subparsers = parser.add_subparsers(dest="command", metavar="<command>")
    command_parsers: dict[str, argparse.ArgumentParser] = {}

    help_parser = subparsers.add_parser("help", help="Show help for liel or a command.")
    help_parser.add_argument(
        "topic",
        nargs="?",
        choices=("version", "diff", "merge"),
        help="Command to show help for.",
    )
    help_parser.set_defaults(func=_help, root_parser=parser, command_parsers=command_parsers)

    version_parser = subparsers.add_parser("version", help="Print the installed liel version.")
    add_format_argument(version_parser)
    version_parser.set_defaults(func=_version)
    command_parsers["version"] = version_parser

    diff_parser = subparsers.add_parser("diff", help="Compare two .liel files.")
    diff_parser.add_argument("left", help="Left .liel file.")
    diff_parser.add_argument("right", help="Right .liel file.")
    add_format_argument(diff_parser)
    diff_parser.set_defaults(func=run_diff)
    command_parsers["diff"] = diff_parser

    merge_parser = subparsers.add_parser("merge", help="Merge two .liel files into a new file.")
    merge_parser.add_argument("left", help="Base .liel file copied to the output first.")
    merge_parser.add_argument("right", help=".liel file merged into the output.")
    merge_parser.add_argument("-o", "--output", required=True, help="Output .liel file.")
    merge_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    merge_parser.add_argument(
        "--node-key",
        action="append",
        help="Property name used for node identity. Repeat for a compound key.",
    )
    merge_parser.add_argument(
        "--edge-strategy",
        choices=("append", "idempotent"),
        default="append",
        help="How to handle merged edges.",
    )
    merge_parser.add_argument(
        "--on-node-conflict",
        choices=("keep_dst", "overwrite_from_src", "merge_props"),
        default="keep_dst",
        help="How to combine properties when --node-key reuses a node.",
    )
    add_format_argument(merge_parser)
    merge_parser.set_defaults(func=run_merge)
    command_parsers["merge"] = merge_parser

    return parser


def _version(args: argparse.Namespace) -> int:
    if args.format == "json":
        emit_json({"version": liel.__version__})
    else:
        emit_text(liel.__version__)
    return EXIT_OK


def _help(args: argparse.Namespace) -> int:
    if args.topic is None:
        args.root_parser.print_help()
    else:
        args.command_parsers[args.topic].print_help()
    return EXIT_OK


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    command = getattr(args, "func", None)
    if command is None:
        parser.print_help()
        return EXIT_OK

    try:
        return int(command(args))
    except CliError as exc:
        print(f"liel: {exc.message}", file=sys.stderr)
        return exc.exit_code
    except BrokenPipeError:
        return EXIT_ERROR


if __name__ == "__main__":
    raise SystemExit(main())

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence

import liel

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    CliError,
    add_format_argument,
    emit_json,
    emit_text,
)
from .diff import run as run_diff
from .events import run_append as run_event_append
from .events import run_list as run_event_list
from .exchange import run_export, run_import
from .manifest import run as run_manifest
from .merge import run as run_merge
from .pack import run as run_pack
from .signature import run_sign, run_verify
from .stats import run as run_stats
from .trace import run as run_trace


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
        choices=(
            "version",
            "diff",
            "merge",
            "pack",
            "manifest",
            "sign",
            "verify",
            "stats",
            "trace",
            "events",
            "export",
            "import",
        ),
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
    diff_parser.add_argument(
        "--node-key",
        action="append",
        help="Property name used for node identity. Repeat for a compound key.",
    )
    diff_parser.add_argument(
        "--identity-rules",
        help="JSON file containing label-specific identity_rules for key-aware diff.",
    )
    add_format_argument(diff_parser)
    diff_parser.set_defaults(func=run_diff)
    command_parsers["diff"] = diff_parser

    merge_parser = subparsers.add_parser("merge", help="Merge two .liel files into a new file.")
    merge_parser.add_argument("left", help="Base .liel file copied to the output first.")
    merge_parser.add_argument("right", help=".liel file merged into the output.")
    merge_parser.add_argument("-o", "--output", help="Output .liel file.")
    merge_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    merge_parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview the merge report without writing an output file.",
    )
    merge_parser.add_argument(
        "--fail-on-conflict",
        action="store_true",
        help="With --dry-run, exit with status 1 when can_merge is false or conflicts is non-empty.",
    )
    merge_parser.add_argument(
        "--node-key",
        action="append",
        help="Property name used for node identity. Repeat for a compound key.",
    )
    merge_parser.add_argument(
        "--identity-rules",
        help="JSON file containing label-specific identity_rules for key-aware merge.",
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

    pack_parser = subparsers.add_parser(
        "pack", help="Extract selected labels into a new .liel file."
    )
    pack_parser.add_argument("source", help="Source .liel file.")
    pack_parser.add_argument("output", help="Output .liel file.")
    pack_parser.add_argument(
        "--include-labels",
        action="append",
        required=True,
        help="Comma-separated node labels to include. Repeat to add more labels.",
    )
    pack_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    add_format_argument(pack_parser)
    pack_parser.set_defaults(func=run_pack)
    command_parsers["pack"] = pack_parser

    manifest_parser = subparsers.add_parser(
        "manifest", help="Emit a deterministic JSON manifest for a .liel file."
    )
    manifest_parser.add_argument("source", help="Source .liel file.")
    manifest_parser.add_argument("-o", "--output", help="Output manifest JSON file.")
    manifest_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    manifest_parser.set_defaults(func=run_manifest)
    command_parsers["manifest"] = manifest_parser

    sign_parser = subparsers.add_parser(
        "sign", help="Sign a .liel manifest with an external HMAC key."
    )
    sign_parser.add_argument("source", help="Source .liel file.")
    sign_parser.add_argument(
        "--key-file", required=True, help="File containing the HMAC key bytes."
    )
    sign_parser.add_argument("-o", "--output", help="Output signature JSON file.")
    sign_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    sign_parser.set_defaults(func=run_sign)
    command_parsers["sign"] = sign_parser

    verify_parser = subparsers.add_parser(
        "verify", help="Verify a .liel file against an external signature."
    )
    verify_parser.add_argument("source", help="Source .liel file.")
    verify_parser.add_argument(
        "--signature", required=True, help="Signature JSON file produced by liel sign."
    )
    verify_parser.add_argument(
        "--key-file", required=True, help="File containing the HMAC key bytes."
    )
    add_format_argument(verify_parser)
    verify_parser.set_defaults(func=run_verify)
    command_parsers["verify"] = verify_parser

    stats_parser = subparsers.add_parser("stats", help="Summarize a .liel file.")
    stats_parser.add_argument("source", help="Source .liel file.")
    add_format_argument(stats_parser)
    stats_parser.set_defaults(func=run_stats)
    command_parsers["stats"] = stats_parser

    trace_parser = subparsers.add_parser(
        "trace",
        help="Shortest path between two node IDs (unweighted directed hops).",
    )
    trace_parser.add_argument("source", help="Source .liel file.")
    trace_parser.add_argument(
        "--from",
        dest="from_node",
        type=int,
        required=True,
        metavar="ID",
        help="Starting node ID.",
    )
    trace_parser.add_argument(
        "--to",
        dest="to_node",
        type=int,
        required=True,
        metavar="ID",
        help="Ending node ID.",
    )
    trace_parser.add_argument(
        "--edge-label",
        default="",
        help="If set, only follow edges with this label (empty = any label).",
    )
    trace_parser.add_argument(
        "--no-mermaid",
        action="store_true",
        help="Omit the Mermaid diagram from text output (path summary only).",
    )
    add_format_argument(trace_parser)
    trace_parser.set_defaults(func=run_trace)
    command_parsers["trace"] = trace_parser

    events_parser = subparsers.add_parser("events", help="Append or list Event log records.")
    events_subparsers = events_parser.add_subparsers(
        dest="events_command", metavar="<events-command>"
    )

    events_append_parser = events_subparsers.add_parser(
        "append", help="Append an Actor-authored Event record."
    )
    events_append_parser.add_argument("source", help="Target .liel file; created if missing.")
    events_append_parser.add_argument(
        "--author", required=True, help="Actor stable key, e.g. actor:local-coder."
    )
    events_append_parser.add_argument(
        "--operation", required=True, help="Event operation, e.g. create_node."
    )
    events_append_parser.add_argument(
        "--target", required=True, help="Target node / edge / property key."
    )
    events_append_parser.add_argument(
        "--payload-json", help="JSON object payload or @path to a JSON file."
    )
    events_append_parser.add_argument(
        "--event-id", help="Stable event id; auto-generated when omitted."
    )
    events_append_parser.add_argument("--parent-event-id", help="Previous or branching event id.")
    events_append_parser.add_argument(
        "--timestamp", help="ISO 8601 UTC timestamp; defaults to now."
    )
    events_append_parser.add_argument("--actor-name", help="Human-readable Actor name.")
    events_append_parser.add_argument("--actor-kind", default="ai_agent", help="Actor kind.")
    events_append_parser.add_argument(
        "--legacy-agent-key", help="Optional legacy agent:* compatibility key."
    )
    events_append_parser.add_argument(
        "--caused-by", help="Event id that semantically caused this event."
    )
    events_append_parser.add_argument(
        "--source-key", action="append", help="Existing Source key cited by this event."
    )
    add_format_argument(events_append_parser)
    events_append_parser.set_defaults(func=run_event_append)

    events_list_parser = events_subparsers.add_parser("list", help="List Event log records.")
    events_list_parser.add_argument("source", help="Source .liel file.")
    add_format_argument(events_list_parser)
    events_list_parser.set_defaults(func=run_event_list)
    command_parsers["events"] = events_parser

    export_parser = subparsers.add_parser("export", help="Export a .liel file as JSON.")
    export_parser.add_argument("source", help="Source .liel file.")
    export_parser.add_argument("-o", "--output", help="Output JSON file.")
    export_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    export_parser.set_defaults(func=run_export)
    command_parsers["export"] = export_parser

    import_parser = subparsers.add_parser("import", help="Import a JSON export into a .liel file.")
    import_parser.add_argument("source", help="Source export JSON file.")
    import_parser.add_argument("-o", "--output", required=True, help="Output .liel file.")
    import_parser.add_argument("--force", action="store_true", help="Overwrite the output file.")
    add_format_argument(import_parser)
    import_parser.set_defaults(func=run_import)
    command_parsers["import"] = import_parser

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

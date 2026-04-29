from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NoReturn, TextIO

EXIT_OK = 0
EXIT_ERROR = 1
EXIT_USAGE = 2


@dataclass
class CliError(Exception):
    """User-facing CLI error with a stable process exit code."""

    message: str
    exit_code: int = EXIT_ERROR


def add_format_argument(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Output format.",
    )


def emit_text(text: str, *, stream: TextIO | None = None) -> None:
    target = stream if stream is not None else sys.stdout
    print(text, file=target)


def emit_json(payload: Any, *, stream: TextIO | None = None) -> None:
    target = stream if stream is not None else sys.stdout
    print(json.dumps(payload, ensure_ascii=False, sort_keys=True), file=target)


def refuse_overwrite(path: str | Path, *, force: bool = False) -> Path:
    output = Path(path)
    if output.exists() and not force:
        raise CliError(f"refusing to overwrite existing file: {output}", EXIT_USAGE)
    return output


def require_existing_file(path: str | Path) -> Path:
    input_path = Path(path)
    if not input_path.is_file():
        raise CliError(f"file does not exist: {input_path}", EXIT_USAGE)
    return input_path


def fail(message: str, *, exit_code: int = EXIT_ERROR) -> NoReturn:
    raise CliError(message, exit_code)

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
from pathlib import Path
from typing import Any

from .common import (
    EXIT_ERROR,
    EXIT_OK,
    EXIT_USAGE,
    CliError,
    emit_json,
    emit_text,
    refuse_overwrite,
    require_existing_file,
)
from .manifest import MANIFEST_VERSION, build_manifest_bytes

SIGNATURE_VERSION = 1
SIGNATURE_ALGORITHM = "hmac-sha256"


def run_sign(args: argparse.Namespace) -> int:
    payload = sign_file(args.source, args.key_file)
    signature_bytes = signature_payload_bytes(payload)
    if args.output is None:
        emit_text(signature_bytes.decode().rstrip("\n"))
    else:
        output = refuse_overwrite(args.output, force=args.force)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(signature_bytes)
    return EXIT_OK


def run_verify(args: argparse.Namespace) -> int:
    payload = verify_file(args.source, args.signature, args.key_file)
    if args.format == "json":
        emit_json(payload)
    elif payload["ok"]:
        emit_text("Signature OK.")
    else:
        emit_text("Signature verification failed.")
    return EXIT_OK if payload["ok"] else EXIT_ERROR


def sign_file(source_path: str | Path, key_file: str | Path) -> dict[str, Any]:
    manifest_bytes = build_manifest_bytes(source_path)
    key = _read_key_file(key_file)
    return _signature_payload(manifest_bytes, key)


def verify_file(
    source_path: str | Path,
    signature_path: str | Path,
    key_file: str | Path,
) -> dict[str, Any]:
    manifest_bytes = build_manifest_bytes(source_path)
    expected = _signature_payload(manifest_bytes, _read_key_file(key_file))
    actual = _load_signature(signature_path)

    compatible = (
        actual["signature_version"] == SIGNATURE_VERSION
        and actual["algorithm"] == SIGNATURE_ALGORITHM
        and actual["manifest_version"] == MANIFEST_VERSION
    )
    digest_ok = hmac.compare_digest(actual["manifest_sha256"], expected["manifest_sha256"])
    signature_ok = hmac.compare_digest(actual["signature"], expected["signature"])
    return {
        "algorithm": SIGNATURE_ALGORITHM,
        "manifest_sha256": expected["manifest_sha256"],
        "ok": compatible and digest_ok and signature_ok,
        "signature_version": SIGNATURE_VERSION,
    }


def signature_payload_bytes(payload: dict[str, Any]) -> bytes:
    text = json.dumps(
        payload,
        ensure_ascii=False,
        sort_keys=True,
        indent=2,
        separators=(",", ": "),
        allow_nan=False,
    )
    return f"{text}\n".encode()


def _signature_payload(manifest_bytes: bytes, key: bytes) -> dict[str, Any]:
    return {
        "algorithm": SIGNATURE_ALGORITHM,
        "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
        "manifest_version": MANIFEST_VERSION,
        "signature": hmac.new(key, manifest_bytes, hashlib.sha256).hexdigest(),
        "signature_version": SIGNATURE_VERSION,
    }


def _read_key_file(path: str | Path) -> bytes:
    key_path = require_existing_file(path)
    try:
        key = key_path.read_bytes()
    except OSError as exc:
        raise CliError(f"failed to read key file {key_path}: {exc}", EXIT_ERROR) from exc
    if not key:
        raise CliError("key file must not be empty", EXIT_USAGE)
    return key


def _load_signature(path: str | Path) -> dict[str, Any]:
    signature_path = require_existing_file(path)
    try:
        payload = json.loads(signature_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise CliError(f"failed to read signature {signature_path}: {exc}", EXIT_ERROR) from exc

    required = {
        "algorithm",
        "manifest_sha256",
        "manifest_version",
        "signature",
        "signature_version",
    }
    if not isinstance(payload, dict) or not required.issubset(payload):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    if not isinstance(payload["algorithm"], str):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    if not isinstance(payload["manifest_sha256"], str):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    if not isinstance(payload["manifest_version"], int):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    if not isinstance(payload["signature"], str):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    if not isinstance(payload["signature_version"], int):
        raise CliError(f"invalid signature file: {signature_path}", EXIT_USAGE)
    return payload

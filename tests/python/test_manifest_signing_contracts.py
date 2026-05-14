"""Tests for manifest + signing stability contracts (1.0 risk mitigation).

``liel manifest`` / ``liel sign`` / ``liel verify`` depend on JSON that uses
``allow_nan=False``. Non-finite floats in stored properties therefore block
manifest generation rather than emitting non-standard JSON.
"""

from __future__ import annotations

import math
from pathlib import Path

import pytest

import liel
from liel.cli import manifest as cli_manifest
from liel.cli import signature as cli_signature
from liel.cli.common import CliError


def test_manifest_and_signature_modules_share_manifest_version() -> None:
    assert cli_signature.MANIFEST_VERSION == cli_manifest.MANIFEST_VERSION


def test_build_manifest_bytes_rejects_non_finite_float_properties(tmp_path: Path) -> None:
    path = tmp_path / "nan.liel"
    with liel.open(str(path)) as db:
        db.add_node(["Probe"], value=float("nan"))
        db.commit()

    with pytest.raises(CliError, match="manifest serialization failed"):
        cli_manifest.build_manifest_bytes(path)


def test_build_manifest_bytes_rejects_infinity_float_properties(tmp_path: Path) -> None:
    path = tmp_path / "inf.liel"
    with liel.open(str(path)) as db:
        db.add_node(["Probe"], value=math.inf)
        db.commit()

    with pytest.raises(CliError, match="manifest serialization failed"):
        cli_manifest.build_manifest_bytes(path)

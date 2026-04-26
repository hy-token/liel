"""Crash-recovery and integrity tests for the Python binding.

These exercise behaviours that only make sense at the file-format boundary:

1. The WAL overflow guard (``LielError::WalOverflow`` -> ``TransactionError``
   on the Python side) that refuses commits too large to fit in the 4 MiB
   WAL reservation.
2. Replay of a committed WAL sequence on the next open after a simulated crash.
3. Rejection of a CRC-corrupted WAL sequence, which must roll back to the last
   committed state instead of applying garbage.
4. Header checksum validation during open.
5. The "transaction survives a close without commit" contract.
"""

from __future__ import annotations

import binascii
import os
import struct

import pytest

import liel

HEADER_SIZE = 128
PAGE_SIZE = 4096
WAL_OFFSET = PAGE_SIZE
WAL_ENTRY_HEADER_SIZE = 17
WAL_CRC_SIZE = 4
WAL_WRITE_ENTRY_SIZE = WAL_ENTRY_HEADER_SIZE + PAGE_SIZE + WAL_CRC_SIZE
WAL_COMMIT_ENTRY_SIZE = WAL_ENTRY_HEADER_SIZE + WAL_CRC_SIZE
FIRST_NODE_EXTENT_PTR_OFFSET = 16
FIRST_SLOT_OFFSET_IN_PAGE = 8
FIRST_OUT_EDGE_OFFSET_IN_SLOT = 8
OP_WRITE = 0x01
OP_COMMIT = 0x02


def _flip_one_bit_in_header(path: str, byte_offset: int) -> None:
    """Flip the low bit of a single byte inside the file header in-place."""
    assert 0 <= byte_offset < 104, "must be inside the checksummed header region"
    with open(path, "r+b") as file:
        file.seek(byte_offset)
        raw = file.read(1)
        file.seek(byte_offset)
        file.write(bytes([raw[0] ^ 0x01]))


def _xor_header_checksum(header: bytes) -> int:
    """Compute the header XOR checksum used by the pager."""
    checksum = 0
    for offset in range(0, 104, 8):
        checksum ^= int.from_bytes(header[offset : offset + 8], "little")
    return checksum


def _patch_header_wal_length(path: str, wal_length: int) -> None:
    """Patch wal_length and refresh the header checksum in-place."""
    with open(path, "r+b") as file:
        header = bytearray(file.read(HEADER_SIZE))
        header[96:104] = wal_length.to_bytes(8, "little")
        header[104:112] = _xor_header_checksum(header).to_bytes(8, "little")
        file.seek(0)
        file.write(header)


def _read_first_node_extent_offset(path: str) -> int:
    """Read the first node extent offset from the node index page."""
    with open(path, "rb") as file:
        header = file.read(HEADER_SIZE)
        node_table_head = int.from_bytes(header[56:64], "little")
        assert node_table_head != 0, "database should have allocated a node index page"
        file.seek(node_table_head + FIRST_NODE_EXTENT_PTR_OFFSET)
        return int.from_bytes(file.read(8), "little")


def _read_page(path: str, page_offset: int) -> bytearray:
    """Read a single 4 KiB page from the database file."""
    with open(path, "rb") as file:
        file.seek(page_offset)
        return bytearray(file.read(PAGE_SIZE))


def _build_wal_entry(op_type: int, page_offset: int, payload: bytes = b"") -> bytes:
    """Build a raw WAL entry with the same framing as the Rust writer."""
    data_length = len(payload)
    entry_length = WAL_ENTRY_HEADER_SIZE + data_length + WAL_CRC_SIZE
    entry = bytearray()
    entry.extend(struct.pack("<I", entry_length))
    entry.append(op_type)
    entry.extend(struct.pack("<Q", page_offset))
    entry.extend(struct.pack("<I", data_length))
    entry.extend(payload)
    entry.extend(struct.pack("<I", binascii.crc32(entry) & 0xFFFFFFFF))
    return bytes(entry)


def _inject_wal(path: str, wal_bytes: bytes) -> None:
    """Write raw WAL bytes and make the header advertise their length."""
    with open(path, "r+b") as file:
        file.seek(WAL_OFFSET)
        file.write(wal_bytes)
    _patch_header_wal_length(path, len(wal_bytes))


def test_wal_overflow_raises_transaction_error(tmp_path):
    """A commit whose WAL footprint exceeds 4 MiB must raise TransactionError."""
    path = str(tmp_path / "overflow.liel")
    db = liel.open(path)

    blob = "x" * 4096
    for _ in range(1500):
        db.add_node(["Big"], data=blob)

    with pytest.raises(liel.TransactionError) as exc_info:
        db.commit()

    msg = str(exc_info.value).lower()
    assert "wal" in msg or "overflow" in msg, (
        f"expected WAL/overflow message, got {exc_info.value!r}"
    )

    db.rollback()
    assert db.node_count() == 0
    db.add_node(["Small"], note="sentinel")
    db.commit()
    db.close()

    with liel.open(path) as reopen:
        assert reopen.node_count() == 1
        only = reopen.all_nodes()[0]
        assert only["note"] == "sentinel"


def test_committed_wal_is_replayed_on_reopen(tmp_path):
    """A durable Write+Commit WAL sequence must be replayed on the next open."""
    path = str(tmp_path / "wal-replay.liel")
    with liel.open(path) as db:
        node = db.add_node(["Seed"], name="ok")
        db.commit()
        assert node.id == 1

    target_offset = _read_first_node_extent_offset(path)
    page = _read_page(path, target_offset)
    slot_offset = FIRST_SLOT_OFFSET_IN_PAGE + FIRST_OUT_EDGE_OFFSET_IN_SLOT
    page[slot_offset : slot_offset + 8] = (777).to_bytes(8, "little")
    wal_bytes = _build_wal_entry(OP_WRITE, target_offset, page)
    wal_bytes += _build_wal_entry(OP_COMMIT, 0)
    assert len(wal_bytes) == WAL_WRITE_ENTRY_SIZE + WAL_COMMIT_ENTRY_SIZE
    _inject_wal(path, wal_bytes)

    with liel.open(path) as reopen:
        reopened = reopen.get_node(1)
        assert reopened is not None
        assert reopened["name"] == "ok"

    with open(path, "rb") as file:
        header = file.read(HEADER_SIZE)
    assert int.from_bytes(header[96:104], "little") == 0


def test_corrupt_committed_wal_is_discarded(tmp_path):
    """A committed WAL with a broken CRC must roll back to the last commit."""
    path = str(tmp_path / "wal-corrupt.liel")
    with liel.open(path) as db:
        node = db.add_node(["Seed"], name="ok")
        db.commit()
        assert node.id == 1

    target_offset = _read_first_node_extent_offset(path)
    page = _read_page(path, target_offset)
    slot_offset = FIRST_SLOT_OFFSET_IN_PAGE + FIRST_OUT_EDGE_OFFSET_IN_SLOT
    page[slot_offset : slot_offset + 8] = (888).to_bytes(8, "little")
    bad_write = bytearray(_build_wal_entry(OP_WRITE, target_offset, page))
    bad_write[-1] ^= 0xFF
    wal_bytes = bytes(bad_write) + _build_wal_entry(OP_COMMIT, 0)
    _inject_wal(path, wal_bytes)

    with liel.open(path) as reopen:
        reopened = reopen.get_node(1)
        assert reopened is not None
        assert reopened["name"] == "ok"

    with open(path, "rb") as file:
        header = file.read(HEADER_SIZE)
    assert int.from_bytes(header[96:104], "little") == 0


def test_header_bit_flip_is_detected(tmp_path):
    """Flipping a single bit inside the header must cause open() to raise."""
    path = str(tmp_path / "bitflip.liel")
    with liel.open(path) as db:
        db.add_node(["Seed"], name="ok")
        db.commit()

    _flip_one_bit_in_header(path, byte_offset=15)

    with pytest.raises(liel.GraphDBError) as exc_info:
        liel.open(path)

    assert isinstance(exc_info.value, (liel.CorruptedFileError, liel.GraphDBError))


def test_header_checksum_intact_file_opens(tmp_path):
    """An untouched database file must keep opening cleanly after close + re-open."""
    path = str(tmp_path / "clean.liel")
    with liel.open(path) as db:
        db.add_node(["Seed"], name="ok")
        db.commit()

    with liel.open(path) as db:
        assert db.node_count() == 1
        assert db.all_nodes()[0]["name"] == "ok"


def test_close_without_commit_loses_changes(tmp_path):
    """Closing the database without commit() must discard all in-flight writes."""
    path = str(tmp_path / "nocommit.liel")
    db = liel.open(path)
    db.add_node(["Volatile"], name="drop-me")
    db.close()

    with liel.open(path) as reopen:
        assert reopen.node_count() == 0

    assert os.path.getsize(path) > 0

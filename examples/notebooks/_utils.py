"""Shared utilities for liel notebook samples and benchmarks.

Notebooks under ``examples/notebooks/`` use this module to:

* Download and cache SNAP datasets (Wikispeedia, wiki-topcats).
* Time code blocks with nanosecond precision.
* Estimate runtime / disk usage from past runs (or from baselines).
* Append measurements to ``examples/notebooks/data/benchrun_history.jsonl`` so future
  runs get better estimates.

This file is intentionally dependency-light: only the Python stdlib plus
``tqdm`` (optional). ``liel`` itself is never imported here.
"""

from __future__ import annotations

import gzip
import json
import os
import shutil
import statistics
import tarfile
import time
import urllib.request
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# ──────────────────────────────────────────────────────────────────────────────
# Paths
# ──────────────────────────────────────────────────────────────────────────────

DEFAULT_DATA_DIR = Path(__file__).resolve().parent / "data"
HISTORY_FILE = DEFAULT_DATA_DIR / "benchrun_history.jsonl"


# ──────────────────────────────────────────────────────────────────────────────
# Dataset URLs (SNAP)
# ──────────────────────────────────────────────────────────────────────────────

WIKISPEEDIA_URL = (
    "https://snap.stanford.edu/data/wikispeedia/wikispeedia_paths-and-graph.tar.gz"
)
TOPCATS_EDGES_URL = "https://snap.stanford.edu/data/wiki-topcats.txt.gz"
TOPCATS_NAMES_URL = "https://snap.stanford.edu/data/wiki-topcats-page-names.txt.gz"
TOPCATS_CATS_URL = "https://snap.stanford.edu/data/wiki-topcats-categories.txt.gz"


# ──────────────────────────────────────────────────────────────────────────────
# Presets
# ──────────────────────────────────────────────────────────────────────────────
#
# Baselines are used until `benchrun_history.jsonl` has at least one real
# measurement for (notebook, preset, op). After that the estimate is the
# median of the last ``keep_last`` runs.

BASELINES: dict[tuple[str, str, str], dict[str, float]] = {
    # 01_wikipedia_graph_tour
    ("01_tour", "S", "load"): {"seconds": 5.0, "disk_mb": 8, "peak_mem_mb": 150},
    ("01_tour", "L", "load"): {"seconds": 25.0, "disk_mb": 20, "peak_mem_mb": 300},
    # 02_bench_bulk_load
    ("02_bulk", "S", "load"): {"seconds": 15.0, "disk_mb": 10, "peak_mem_mb": 250},
    ("02_bulk", "L", "load"): {"seconds": 900.0, "disk_mb": 2500, "peak_mem_mb": 3500},
    # 03_bench_queries
    ("03_query", "S", "load"): {"seconds": 5.0, "disk_mb": 10, "peak_mem_mb": 250},
    ("03_query", "L", "load"): {"seconds": 60.0, "disk_mb": 2500, "peak_mem_mb": 3500},
}


PRESET_DESCRIPTION: dict[tuple[str, str], str] = {
    ("01_tour", "S"): "Wikispeedia: Article + LINKS_TO のみ",
    ("01_tour", "L"): "Wikispeedia: + category / + NAVIGATED エッジ（全部入り）",
    ("02_bulk", "S"): "Wikispeedia 全件（~120k edges）",
    ("02_bulk", "L"): "wiki-topcats 全件（~28M edges）",
    ("03_query", "S"): "Wikispeedia 全件（~120k edges）",
    ("03_query", "L"): "wiki-topcats 全件（~28M edges）",
}


def presets_table(nb: str) -> str:
    """Markdown-friendly one-line preset summary for the given notebook."""
    lines = [
        "| Preset | 内容 | 推定時間 | 推定 disk | 推定 peak RAM |",
        "|---|---|---|---|---|",
    ]
    for preset in ("S", "L"):
        key = (nb, preset, "load")
        if key not in BASELINES:
            continue
        b = BASELINES[key]
        desc = PRESET_DESCRIPTION.get((nb, preset), "")
        lines.append(
            f"| **{preset}** | {desc} "
            f"| ~{b['seconds']:.0f} s "
            f"| ~{b['disk_mb']:.0f} MB "
            f"| ~{b['peak_mem_mb']:.0f} MB |"
        )
    return "\n".join(lines)


# ──────────────────────────────────────────────────────────────────────────────
# Download helpers
# ──────────────────────────────────────────────────────────────────────────────


def _download_with_progress(url: str, dest: Path, chunk: int = 65536) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "liel-notebook/1.0"})
    with urllib.request.urlopen(request, timeout=60) as resp, tmp.open("wb") as fh:
        total = int(resp.headers.get("Content-Length") or 0)
        got = 0
        last_print = 0.0
        name = url.rsplit("/", 1)[-1]
        while True:
            buf = resp.read(chunk)
            if not buf:
                break
            fh.write(buf)
            got += len(buf)
            now = time.monotonic()
            if now - last_print > 0.5:
                pct = (got / total * 100) if total else 0
                print(f"  {name}: {got / 1e6:6.1f} MB ({pct:5.1f}%)", end="\r")
                last_print = now
    tmp.rename(dest)
    print()  # newline after the progress line


def fetch_wikispeedia(data_dir: Path = DEFAULT_DATA_DIR) -> Path:
    """Download and extract Wikispeedia under ``data_dir/wikispeedia``.

    Returns the directory that contains ``articles.tsv`` / ``links.tsv`` etc.
    Idempotent: skips download/extract if files already exist.
    """
    root = data_dir / "wikispeedia"
    marker = root / ".extracted"
    if marker.exists():
        return root
    root.mkdir(parents=True, exist_ok=True)
    archive = root / "wikispeedia_paths-and-graph.tar.gz"
    if not archive.exists():
        print(f"Downloading Wikispeedia (~10 MB) from {WIKISPEEDIA_URL} ...")
        _download_with_progress(WIKISPEEDIA_URL, archive)
    print("Extracting ...")
    with tarfile.open(archive) as tf:
        _safe_extract_tar(tf, root)
    marker.write_text("ok\n", encoding="utf-8")
    return root


def _safe_extract_tar(tf: tarfile.TarFile, dest: Path) -> None:
    """Extract a tar archive without allowing paths to escape ``dest``."""
    dest_resolved = dest.resolve()
    for member in tf.getmembers():
        target = (dest / member.name).resolve()
        if os.path.commonpath([str(dest_resolved), str(target)]) != str(dest_resolved):
            raise RuntimeError(f"Refusing to extract unsafe tar member: {member.name}")
        if member.issym() or member.islnk():
            link_target = (target.parent / member.linkname).resolve()
            if os.path.commonpath([str(dest_resolved), str(link_target)]) != str(dest_resolved):
                raise RuntimeError(f"Refusing to extract unsafe tar link: {member.name}")
    tf.extractall(dest)


def wikispeedia_paths(root: Path) -> dict[str, Path]:
    """Map logical names to concrete .tsv paths inside Wikispeedia."""
    base = root / "wikispeedia_paths-and-graph"
    return {
        "articles": base / "articles.tsv",
        "links": base / "links.tsv",
        "categories": base / "categories.tsv",
        "paths_finished": base / "paths_finished.tsv",
        "paths_unfinished": base / "paths_unfinished.tsv",
    }


def fetch_topcats(data_dir: Path = DEFAULT_DATA_DIR) -> dict[str, Path]:
    """Download and decompress wiki-topcats files.

    Returns a mapping ``{"edges": ..., "names": ..., "categories": ...}``
    of decompressed ``.txt`` paths.
    """
    root = data_dir / "topcats"
    root.mkdir(parents=True, exist_ok=True)
    out: dict[str, Path] = {}
    for name, url in (
        ("edges", TOPCATS_EDGES_URL),
        ("names", TOPCATS_NAMES_URL),
        ("categories", TOPCATS_CATS_URL),
    ):
        gz = root / Path(url).name
        txt = gz.with_suffix("")  # drop .gz suffix
        if not txt.exists():
            if not gz.exists():
                print(f"Downloading {gz.name} ...")
                _download_with_progress(url, gz)
            print(f"Decompressing {gz.name} ...")
            with gzip.open(gz, "rb") as src, txt.open("wb") as dst:
                shutil.copyfileobj(src, dst)
        out[name] = txt
    return out


# ──────────────────────────────────────────────────────────────────────────────
# Timing
# ──────────────────────────────────────────────────────────────────────────────


@dataclass
class Timer:
    """Context manager recording wall-clock time with nanosecond precision.

    Example::

        with Timer("load") as t:
            ...
        # prints "[load] 1.234 s" on exit and stores seconds in ``t.elapsed``.
    """

    label: str = ""
    elapsed: float = 0.0  # seconds

    def __enter__(self) -> Timer:
        self._t0 = time.perf_counter_ns()
        return self

    def __exit__(self, *exc: Any) -> None:
        self.elapsed = (time.perf_counter_ns() - self._t0) / 1e9
        if self.label:
            print(f"[{self.label}] {self.elapsed:.3f} s")


@contextmanager
def timed(label: str) -> Iterator[Timer]:
    """Alias for ``Timer`` usable as ``with timed("foo") as t: ...``."""
    t = Timer(label)
    with t:
        yield t


# ──────────────────────────────────────────────────────────────────────────────
# History / estimate
# ──────────────────────────────────────────────────────────────────────────────


def save_run(record: dict[str, Any], path: Path = HISTORY_FILE) -> None:
    """Append a single measurement record to the run history (JSONL)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    record = {**record, "timestamp": time.time()}
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(record, ensure_ascii=False) + "\n")


def load_runs(path: Path = HISTORY_FILE) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    runs: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            runs.append(json.loads(line))
        except json.JSONDecodeError:
            # ignore corrupted lines rather than failing the whole notebook
            continue
    return runs


def estimate(
    nb: str,
    preset: str,
    op: str = "load",
    history_path: Path = HISTORY_FILE,
    keep_last: int = 5,
) -> dict[str, Any]:
    """Return an estimate for ``(nb, preset, op)``.

    Uses the median of the last ``keep_last`` matching runs from history.
    Falls back to the hard-coded baseline when no history is available.
    """
    runs = [
        r
        for r in load_runs(history_path)
        if r.get("nb") == nb and r.get("preset") == preset and r.get("op") == op
    ]
    runs = runs[-keep_last:]
    if runs:
        return {
            "seconds": statistics.median(r.get("seconds", 0.0) for r in runs),
            "disk_mb": statistics.median(r.get("disk_mb", 0.0) for r in runs),
            "peak_mem_mb": statistics.median(r.get("peak_mem_mb", 0.0) for r in runs),
            "source": f"history (n={len(runs)})",
        }
    base = BASELINES.get((nb, preset, op))
    if base is None:
        return {
            "seconds": float("nan"),
            "disk_mb": float("nan"),
            "peak_mem_mb": float("nan"),
            "source": "unknown",
        }
    return {**base, "source": "baseline"}


def confirm_run(est: dict[str, Any], threshold_seconds: float = 60.0) -> None:
    """Print an estimate; warn (non-blocking) when it exceeds ``threshold_seconds``.

    Notebooks don't block on ``input()`` well, so this is advisory only.
    """
    msg = (
        f"[estimate / {est.get('source', '?')}] "
        f"~{est['seconds']:.1f} s  "
        f"disk ~{est['disk_mb']:.0f} MB  "
        f"peak RAM ~{est['peak_mem_mb']:.0f} MB"
    )
    if est["seconds"] > threshold_seconds:
        print(f"[WARN] {msg}  — large preset, 実行に時間がかかる可能性があります")
    else:
        print(msg)


# ──────────────────────────────────────────────────────────────────────────────
# Memory sampling (best-effort, uses psutil if installed; otherwise no-op)
# ──────────────────────────────────────────────────────────────────────────────


def current_rss_mb() -> float:
    """Return the current process RSS in MiB, or 0 if psutil is unavailable."""
    try:
        import psutil  # type: ignore

        return psutil.Process().memory_info().rss / (1024 * 1024)
    except Exception:
        return 0.0


# ──────────────────────────────────────────────────────────────────────────────
# Small helpers
# ──────────────────────────────────────────────────────────────────────────────


def format_bytes(n: float) -> str:
    """Human-readable byte count, e.g. ``1.5 MiB``."""
    units = ("B", "KiB", "MiB", "GiB", "TiB")
    for unit in units:
        if n < 1024 or unit == units[-1]:
            return f"{n:.1f} {unit}"
        n /= 1024
    return f"{n:.1f} PiB"

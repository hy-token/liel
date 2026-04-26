from liel.liel import (
    AlreadyOpenError,
    CapacityExceededError,
    CorruptedFileError,
    Edge,
    EdgeNotFoundError,
    EdgeQuery,
    GraphDB,
    GraphDBError,
    MergeError,
    MergeReport,
    Node,
    NodeNotFoundError,
    NodeQuery,
    Transaction,
    TransactionError,
    open,
)

# Resolve ``__version__`` from the installed distribution metadata first so it
# tracks the published wheel version (which may carry a PEP 440 pre-release
# suffix the Rust crate cannot express). Fall back to the version baked into
# the native extension when the package is being imported from a source tree
# that has not been ``pip install``-ed (e.g. ``maturin develop`` followed by a
# direct PYTHONPATH import on a clean environment).
try:
    from importlib.metadata import PackageNotFoundError as _PackageNotFoundError
    from importlib.metadata import version as _dist_version

    try:
        __version__ = _dist_version("liel")
    except _PackageNotFoundError:
        from liel.liel import __version__
except ImportError:
    from liel.liel import __version__

__all__ = [
    "__version__",
    "open",
    "GraphDB",
    "Node",
    "Edge",
    "Transaction",
    "NodeQuery",
    "EdgeQuery",
    "MergeReport",
    "GraphDBError",
    "NodeNotFoundError",
    "EdgeNotFoundError",
    "CorruptedFileError",
    "TransactionError",
    "MergeError",
    "CapacityExceededError",
    "AlreadyOpenError",
]

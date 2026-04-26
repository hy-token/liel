/// Top-level crate modules.
///
/// The crate is organised as a stack of layers, each building on the one below:
///
/// | Module    | Responsibility |
/// |-----------|----------------|
/// | `storage` | Raw byte I/O: pager, WAL, serialiser, property codec, LRU cache |
/// | `graph`   | Node/edge CRUD, adjacency-list management, traversal algorithms |
/// | `query`   | QueryBuilder — fluent API for filtered node/edge iteration |
/// | `db`      | High-level `GraphDB` facade — wires storage + graph + query into the public Rust API used by the Python bindings |
/// | `python`  | PyO3 bindings that expose `GraphDB` to Python callers |
/// | `error`   | The `LielError` enum and the `Result<T>` alias used throughout |
pub mod db;
pub mod error;
pub mod graph;
pub mod python;
pub mod query;
pub mod storage;

use pyo3::prelude::*;
use python::types::{
    open, AlreadyOpenError, CapacityExceededError, CorruptedFileError, EdgeNotFoundError,
    GraphDBError, MergeError, NodeNotFoundError, PyEdge, PyEdgeQuery, PyGraphDB, PyMergeReport,
    PyNode, PyNodeQuery, PyTransaction, TransactionError,
};

/// Entry point for the `liel` Python extension module.
///
/// This function is called automatically by the Python interpreter when
/// `import liel` is executed.  It registers every public symbol — classes,
/// free functions, and exception types — into the module's namespace.
///
/// # Registration order matters for exceptions
/// PyO3 requires that a base exception class is registered *before* any
/// subclass that derives from it.  `GraphDBError` is therefore added before
/// `NodeNotFoundError`, `EdgeNotFoundError`, `CorruptedFileError`, and
/// `TransactionError`, all of which inherit from it on the Python side.
///
/// # Registered symbols
/// | Python name          | Rust type        | Kind |
/// |----------------------|------------------|------|
/// | `liel.open`          | `open`           | function |
/// | `liel.GraphDB`       | `PyGraphDB`      | class |
/// | `liel.Node`          | `PyNode`         | class |
/// | `liel.Edge`          | `PyEdge`         | class |
/// | `liel.NodeQuery`     | `PyNodeQuery`    | class |
/// | `liel.EdgeQuery`     | `PyEdgeQuery`    | class |
/// | `liel.Transaction`   | `PyTransaction`  | class |
/// | `liel.GraphDBError`  | `GraphDBError`   | exception |
/// | `liel.NodeNotFoundError` | `NodeNotFoundError` | exception |
/// | `liel.EdgeNotFoundError` | `EdgeNotFoundError` | exception |
/// | `liel.CorruptedFileError`| `CorruptedFileError`| exception |
/// | `liel.TransactionError`  | `TransactionError`  | exception |
/// | `liel.CapacityExceededError` | `CapacityExceededError` | exception |
/// | `liel.MergeError`        | `MergeError`        | exception |
/// | `liel.AlreadyOpenError`  | `AlreadyOpenError`  | exception |
#[pymodule]
fn liel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Expose the crate version as Python-level package metadata.
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // Compile-time flag exported to Python so the crash-safety harness can
    // skip itself when the wheel was built without `test-fault-injection`.
    // Reading the constant at import time is enough; we never want to
    // dispatch on this from production code paths.
    m.add(
        "_BUILT_WITH_FAULT_INJECTION",
        cfg!(feature = "test-fault-injection"),
    )?;
    m.add_function(wrap_pyfunction!(open, m)?)?;
    m.add_class::<PyGraphDB>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyEdge>()?;
    m.add_class::<PyNodeQuery>()?;
    m.add_class::<PyEdgeQuery>()?;
    m.add_class::<PyTransaction>()?;
    m.add_class::<PyMergeReport>()?;
    // Register the base exception class first, then all subclasses.
    // PyO3 will raise a TypeError at import time if a subclass is registered
    // before its base class is present in the module namespace.
    m.add("GraphDBError", m.py().get_type::<GraphDBError>())?;
    m.add("NodeNotFoundError", m.py().get_type::<NodeNotFoundError>())?;
    m.add("EdgeNotFoundError", m.py().get_type::<EdgeNotFoundError>())?;
    m.add(
        "CorruptedFileError",
        m.py().get_type::<CorruptedFileError>(),
    )?;
    m.add("TransactionError", m.py().get_type::<TransactionError>())?;
    m.add(
        "CapacityExceededError",
        m.py().get_type::<CapacityExceededError>(),
    )?;
    m.add("MergeError", m.py().get_type::<MergeError>())?;
    m.add("AlreadyOpenError", m.py().get_type::<AlreadyOpenError>())?;
    Ok(())
}

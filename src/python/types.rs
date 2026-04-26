#![allow(clippy::useless_conversion)] // PyO3 + Rust: occasional false positives on `PyResult` in pymethods.

//! Python bindings for the liel graph database (PyO3 layer).
//!
//! This module wraps the pure-Rust `crate::db::GraphDB` behind PyO3
//! `#[pyclass]` types so it can be used from Python.  All public surface is
//! re-exported via `crate::python` and registered in `lib.rs::liel`.
//!
//! Design notes:
//! - User-facing API documentation lives in `python/liel/liel.pyi`
//!   (PEP 561 stub), not in `///` comments here.  The stub is the single
//!   source of truth for argument names, return types, and docstrings shown
//!   by IDEs / `help()`; this file only documents Rust-side structure.
//! - `crate::db::GraphDB` is wrapped in `Arc<Mutex<_>>` to satisfy PyO3's
//!   `Send + Sync` requirement for `#[pyclass]`.  Every `#[pymethods]` entry
//!   point goes through `acquire_graph_lock` for the minimum scope needed.
//! - Errors flow through `liel_error_to_pyerr`, which maps
//!   `crate::error::LielError` variants onto the Python exception
//!   hierarchy declared just below (`GraphDBError` and its subclasses).
//! - `PropValue` ↔ Python conversion lives in `py_to_prop` / `prop_to_py`
//!   and is used by every method that touches node / edge properties.

use pyo3::exceptions::{PyKeyError, PyOSError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::db::GraphDB;
use crate::error::{format_liel_error, LielError};
use crate::graph::edge::{Direction, Edge};
use crate::graph::merge::{
    self, ConflictMode, EdgeStrategy, MergePolicy, MergeReport, NodeIdentity,
};
use crate::graph::node::Node;
use crate::storage::prop_codec::PropValue;

type PyObject = Py<PyAny>;

/// Smart guard returned by [`acquire_graph_lock`] that hides the
/// `Option<GraphDB>` indirection used to support explicit `close()`/`__exit__`.
///
/// The wrapper derefs to `GraphDB`, so the existing `pymethods` callsites can
/// keep their idiomatic `acquire_graph_lock(&self.inner)?.add_node(...)`
/// shape.  When `close()` (or the `with` block's `__exit__`) has already
/// dropped the inner database, [`acquire_graph_lock`] short-circuits with a
/// typed Python error before the guard is constructed.
pub(crate) struct GraphGuard<'a> {
    inner: MutexGuard<'a, Option<GraphDB>>,
}

impl<'a> Deref for GraphGuard<'a> {
    type Target = GraphDB;
    fn deref(&self) -> &GraphDB {
        self.inner
            .as_ref()
            .expect("acquire_graph_lock guarantees the inner Option is Some")
    }
}

impl<'a> DerefMut for GraphGuard<'a> {
    fn deref_mut(&mut self) -> &mut GraphDB {
        self.inner
            .as_mut()
            .expect("acquire_graph_lock guarantees the inner Option is Some")
    }
}

fn acquire_graph_lock(db: &Arc<Mutex<Option<GraphDB>>>) -> PyResult<GraphGuard<'_>> {
    let guard = db.lock().map_err(|_| {
        PyRuntimeError::new_err(
            "liel: database lock was poisoned (another thread panicked while holding the lock). Open a new GraphDB connection.",
        )
    })?;
    if guard.is_none() {
        return Err(PyValueError::new_err(
            "liel: database handle is closed; reopen with liel.open(path) before issuing further operations.",
        ));
    }
    Ok(GraphGuard { inner: guard })
}

// ─── Error types ────────────────────────────────────────────────────────────

pyo3::create_exception!(liel, GraphDBError, pyo3::exceptions::PyException);
pyo3::create_exception!(liel, NodeNotFoundError, GraphDBError);
pyo3::create_exception!(liel, EdgeNotFoundError, GraphDBError);
pyo3::create_exception!(liel, CorruptedFileError, GraphDBError);
pyo3::create_exception!(liel, TransactionError, GraphDBError);
pyo3::create_exception!(liel, CapacityExceededError, GraphDBError);
pyo3::create_exception!(liel, MergeError, GraphDBError);
pyo3::create_exception!(liel, AlreadyOpenError, GraphDBError);

pub fn liel_error_to_pyerr(e: LielError) -> PyErr {
    let text = format_liel_error(&e);
    match e {
        LielError::NodeNotFound(_) => NodeNotFoundError::new_err(text),
        LielError::EdgeNotFound(_) => EdgeNotFoundError::new_err(text),
        LielError::CorruptedFile(_) => CorruptedFileError::new_err(text),
        LielError::Io(_) => PyOSError::new_err(text),
        LielError::InvalidArgument(_) => PyValueError::new_err(text),
        LielError::TransactionError(_) => TransactionError::new_err(text),
        LielError::CapacityExceeded { .. } => CapacityExceededError::new_err(text),
        LielError::MergeKeyNotFound { .. } => MergeError::new_err(text),
        // WalOverflow is surfaced as TransactionError on the Python side so
        // users only see one "transaction failed" exception class; the
        // format_liel_error message already includes the split-and-retry
        // guidance, so text is the right payload here too.
        LielError::WalOverflow(_) => TransactionError::new_err(text),
        LielError::AlreadyOpen(_) => AlreadyOpenError::new_err(text),
    }
}

// ─── PropValue ↔ Python conversion ──────────────────────────────────────────

const MAX_PROP_DEPTH: usize = 32;

pub fn py_to_prop(py: Python, obj: &Bound<PyAny>) -> PyResult<PropValue> {
    py_to_prop_depth(py, obj, 0)
}

fn py_to_prop_depth(_py: Python, obj: &Bound<PyAny>, depth: usize) -> PyResult<PropValue> {
    if depth > MAX_PROP_DEPTH {
        return Err(PyValueError::new_err(format!(
            "Property nesting depth exceeds the maximum of {MAX_PROP_DEPTH}. \
             Flatten nested structures before storing them as properties."
        )));
    }
    if obj.is_none() {
        return Ok(PropValue::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(PropValue::Bool(b));
    }
    if let Ok(n) = obj.extract::<i64>() {
        return Ok(PropValue::Int(n));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(PropValue::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(PropValue::String(s));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_prop_depth(_py, &item, depth + 1)?);
        }
        return Ok(PropValue::List(items));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = HashMap::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, py_to_prop_depth(_py, &v, depth + 1)?);
        }
        return Ok(PropValue::Map(map));
    }
    Err(PyValueError::new_err(format!(
        "Property values must be None, bool, int, float, str, list, or dict (nested values follow the same rules). Got type `{}`.",
        obj.get_type().name()?
    )))
}

pub fn prop_to_py(py: Python, value: &PropValue) -> PyResult<PyObject> {
    match value {
        PropValue::Null => Ok(py.None()),
        PropValue::Bool(b) => bool_to_pyobject(py, *b),
        PropValue::Int(n) => Ok(n.into_pyobject(py)?.into_any().unbind()),
        PropValue::Float(f) => Ok(f.into_pyobject(py)?.into_any().unbind()),
        PropValue::String(s) => Ok(s.clone().into_pyobject(py)?.into_any().unbind()),
        PropValue::List(items) => {
            let py_items: Vec<PyObject> = items
                .iter()
                .map(|item| prop_to_py(py, item))
                .collect::<PyResult<_>>()?;
            Ok(PyList::new(py, py_items)?.into_any().unbind())
        }
        PropValue::Map(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k.clone(), prop_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

/// Convert a Rust `bool` into an owned Python `bool` object.
///
/// Python booleans are singletons, so `IntoPyObject for bool` produces a
/// `Borrowed<'_, '_, PyBool>` rather than a `Bound<'_, PyBool>`.  The
/// difference matters because `Borrowed::into_any()` cannot be called
/// directly (moving out of a deref); `.to_owned()` lifts the value back to
/// `Bound` and then the usual `.into_any().unbind()` pipeline produces a
/// `PyObject`.  Centralising this here keeps the ceremony out of every call
/// site and gives future pyo3 upgrades a single place to absorb API shifts.
fn bool_to_pyobject(py: Python, b: bool) -> PyResult<PyObject> {
    Ok(b.into_pyobject(py)?.to_owned().into_any().unbind())
}

// ─── Internal helpers ────────────────────────────────────────────────────────

fn node_id_from_py(py_node: &Bound<PyAny>) -> PyResult<u64> {
    if let Ok(node) = py_node.downcast::<PyNode>() {
        return Ok(node.borrow().id());
    }
    if let Ok(id) = py_node.extract::<u64>() {
        return Ok(id);
    }
    let got = py_node.get_type().name()?.to_string();
    Err(PyValueError::new_err(format!(
        "Expected a liel.Node instance or a non-negative integer node id; got type `{got}`. Use the value returned by add_node() or node.id."
    )))
}

fn kwargs_to_props(kwargs: Option<&Bound<PyDict>>) -> PyResult<HashMap<String, PropValue>> {
    let mut props = HashMap::new();
    if let Some(kw) = kwargs {
        let py = kw.py();
        for (k, v) in kw.iter() {
            let key: String = k.extract()?;
            let value = py_to_prop(py, &v)?;
            props.insert(key, value);
        }
    }
    Ok(props)
}

fn build_merge_policy(
    node_key: Option<Vec<String>>,
    edge_strategy: &str,
    on_node_conflict: &str,
) -> PyResult<MergePolicy> {
    let node_identity = match node_key {
        None => NodeIdentity::AlwaysNew,
        Some(keys) if keys.is_empty() => {
            return Err(PyValueError::new_err(
                "merge_from: node_key must contain at least one property name (or be None)",
            ));
        }
        Some(keys) => NodeIdentity::ByProperty(keys),
    };
    let edge_strategy = match edge_strategy {
        "append" => EdgeStrategy::Append,
        "idempotent" => EdgeStrategy::Idempotent,
        other => {
            return Err(PyValueError::new_err(format!(
                "merge_from: invalid edge_strategy '{other}': expected 'append' or 'idempotent'"
            )));
        }
    };
    let on_node_conflict = match on_node_conflict {
        "keep_dst" => ConflictMode::KeepDst,
        "overwrite_from_src" => ConflictMode::OverwriteFromSrc,
        "merge_props" => ConflictMode::MergeProps,
        other => {
            return Err(PyValueError::new_err(format!(
                "merge_from: invalid on_node_conflict '{other}': expected 'keep_dst', 'overwrite_from_src', or 'merge_props'"
            )));
        }
    };
    Ok(MergePolicy {
        node_identity,
        edge_strategy,
        on_node_conflict,
    })
}

fn parse_direction(direction: &str) -> PyResult<Direction> {
    match direction {
        "out" => Ok(Direction::Out),
        "in" => Ok(Direction::In),
        "both" => Ok(Direction::Both),
        other => Err(PyValueError::new_err(format!(
            "invalid direction: got '{other}' (use 'out', 'in', or 'both').",
        ))),
    }
}

fn node_to_py(py: Python, n: Node) -> PyResult<PyObject> {
    Ok(Py::new(py, PyNode { inner: n })?.into_any())
}

fn edge_to_py(py: Python, e: Edge) -> PyResult<PyObject> {
    Ok(Py::new(py, PyEdge { inner: e })?.into_any())
}

// ─── PyNode ──────────────────────────────────────────────────────────────────

/// PyO3 wrapper around `crate::graph::node::Node`, exposed to Python as
/// `liel.Node`.
///
/// Holds an immutable snapshot of the node's id / labels / properties at
/// fetch time; mutations go through `PyGraphDB` and require a re-fetch
/// to be observed.  See `python/liel/liel.pyi::Node` for the user-facing
/// docstring and method contracts.
#[pyclass(name = "Node")]
pub struct PyNode {
    pub inner: Node,
}

#[pymethods]
impl PyNode {
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    #[getter]
    fn labels(&self) -> Vec<String> {
        self.inner.labels.clone()
    }

    fn __getitem__(&self, py: Python, key: &str) -> PyResult<PyObject> {
        match self.inner.properties.get(key) {
            Some(v) => prop_to_py(py, v),
            None => Err(PyKeyError::new_err(format!(
                "No property named '{key}' on this node. Use .get('{key}') if the key may be absent, or inspect .keys()."
            ))),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.properties.contains_key(key)
    }

    fn get(&self, py: Python, key: &str) -> PyResult<PyObject> {
        match self.inner.properties.get(key) {
            Some(v) => prop_to_py(py, v),
            None => Ok(py.None()),
        }
    }

    fn keys(&self) -> Vec<String> {
        self.inner.properties.keys().cloned().collect()
    }

    #[getter]
    fn properties(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.properties {
            dict.set_item(k, prop_to_py(py, v)?)?;
        }
        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!("Node(id={}, labels={:?})", self.inner.id, self.inner.labels)
    }
}

// ─── PyEdge ──────────────────────────────────────────────────────────────────

/// PyO3 wrapper around `crate::graph::edge::Edge`, exposed to Python as
/// `liel.Edge`.
///
/// Like `PyNode`, this is an immutable snapshot of the edge (id, label,
/// endpoints, properties); mutations happen on `PyGraphDB`.  See
/// `python/liel/liel.pyi::Edge` for the user-facing docstring.
#[pyclass(name = "Edge")]
pub struct PyEdge {
    pub inner: Edge,
}

#[pymethods]
impl PyEdge {
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    #[getter]
    fn label(&self) -> String {
        self.inner.label.clone()
    }

    #[getter]
    #[allow(clippy::wrong_self_convention)] // Python property name `from_node`
    fn from_node(&self) -> u64 {
        self.inner.from
    }

    #[getter]
    fn to_node(&self) -> u64 {
        self.inner.to
    }

    fn __getitem__(&self, py: Python, key: &str) -> PyResult<PyObject> {
        match self.inner.properties.get(key) {
            Some(v) => prop_to_py(py, v),
            None => Err(PyKeyError::new_err(format!(
                "No property named '{key}' on this edge. Use .get('{key}') if the key may be absent, or inspect .keys()."
            ))),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.properties.contains_key(key)
    }

    fn get(&self, py: Python, key: &str) -> PyResult<PyObject> {
        match self.inner.properties.get(key) {
            Some(v) => prop_to_py(py, v),
            None => Ok(py.None()),
        }
    }

    fn keys(&self) -> Vec<String> {
        self.inner.properties.keys().cloned().collect()
    }

    #[getter]
    fn properties(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.inner.properties {
            dict.set_item(k, prop_to_py(py, v)?)?;
        }
        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "Edge(id={}, label='{}', from={}, to={})",
            self.inner.id, self.inner.label, self.inner.from, self.inner.to
        )
    }
}

// ─── PyMergeReport ───────────────────────────────────────────────────────────

/// Summary of a `GraphDB.merge_from(...)` call, exposed to Python as
/// `liel.MergeReport`.  All fields are plain Python `dict`s / `int`s so the
/// object can be passed freely through user code without needing further
/// conversion helpers.
#[pyclass(name = "MergeReport")]
pub struct PyMergeReport {
    pub inner: MergeReport,
}

#[pymethods]
impl PyMergeReport {
    #[getter]
    fn node_id_map(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        for (src_id, dst_id) in &self.inner.node_id_map {
            d.set_item(*src_id, *dst_id)?;
        }
        Ok(d.into_any().unbind())
    }

    #[getter]
    fn edge_id_map(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        for (src_id, dst_id) in &self.inner.edge_id_map {
            d.set_item(*src_id, *dst_id)?;
        }
        Ok(d.into_any().unbind())
    }

    #[getter]
    fn nodes_created(&self) -> u64 {
        self.inner.nodes_created
    }
    #[getter]
    fn nodes_reused(&self) -> u64 {
        self.inner.nodes_reused
    }
    #[getter]
    fn edges_created(&self) -> u64 {
        self.inner.edges_created
    }
    #[getter]
    fn edges_reused(&self) -> u64 {
        self.inner.edges_reused
    }

    fn __repr__(&self) -> String {
        format!(
            "MergeReport(nodes_created={}, nodes_reused={}, edges_created={}, edges_reused={})",
            self.inner.nodes_created,
            self.inner.nodes_reused,
            self.inner.edges_created,
            self.inner.edges_reused,
        )
    }
}

// ─── PyGraphDB ───────────────────────────────────────────────────────────────

/// PyO3 wrapper around `crate::db::GraphDB`, exposed to Python as
/// `liel.GraphDB` (constructed via `liel.open(path)`).
///
/// The inner `crate::db::GraphDB` is held in `Arc<Mutex<_>>` because
/// PyO3 requires `Send + Sync` for `#[pyclass]` types.  Every method
/// below acquires the lock through `acquire_graph_lock` for the minimum
/// scope needed and converts results into Python objects via the
/// `*_to_py` helpers above.  User-facing method docstrings live in
/// `python/liel/liel.pyi::GraphDB`.
#[pyclass(name = "GraphDB")]
pub struct PyGraphDB {
    inner: Arc<Mutex<Option<GraphDB>>>,
}

impl PyGraphDB {
    pub fn new(db: GraphDB) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(db))),
        }
    }
}

#[pymethods]
impl PyGraphDB {
    /// Release the underlying `GraphDB` and free its writer-guard slot.
    ///
    /// After `close()` returns, every method on this handle raises a
    /// :class:`ValueError` so the Python-side reference can outlive the
    /// resource without holding the file open.  Calling `close()` more than
    /// once is allowed and is a no-op on second and subsequent calls.
    ///
    /// `with liel.open(path) as db: ...` calls this implicitly via
    /// `__exit__`, which is what releases the in-process single-writer slot
    /// (`AlreadyOpenError`) at the end of the block – relying on Python GC
    /// for that release would be unreliable on alternative implementations.
    fn close(&self) -> PyResult<()> {
        let mut guard = self.inner.lock().map_err(|_| {
            PyRuntimeError::new_err(
                "liel: database lock was poisoned during close(); the handle may still be open. Open a new GraphDB connection.",
            )
        })?;
        // Take ownership and drop *after* releasing the lock so the
        // GraphDB::Drop impl (which touches the open-file registry)
        // never races with another thread that is still holding the
        // mutex for a metadata read.
        let taken = guard.take();
        drop(guard);
        drop(taken);
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Exit the `with` block.  Drops the inner `GraphDB` so the
    /// in-process single-writer guard releases its slot for this file
    /// and a subsequent `liel.open(same_path)` is allowed without
    /// requiring the caller to also `del` the binding.  Returning
    /// `false` propagates any in-flight exception.
    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        _exc_type: Option<Bound<PyAny>>,
        _exc_val: Option<Bound<PyAny>>,
        _exc_tb: Option<Bound<PyAny>>,
    ) -> PyResult<bool> {
        self.close()?;
        Ok(false)
    }

    // ── Node operations ──────────────────────────────────────────────────────

    #[pyo3(signature = (labels, **kwargs))]
    fn add_node(
        &self,
        py: Python,
        labels: Vec<String>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<PyObject> {
        let props = kwargs_to_props(kwargs)?;
        let n = acquire_graph_lock(&self.inner)?
            .add_node(labels, props)
            .map_err(liel_error_to_pyerr)?;
        node_to_py(py, n)
    }

    fn get_node(&self, py: Python, node_id: u64) -> PyResult<PyObject> {
        match acquire_graph_lock(&self.inner)?
            .get_node(node_id)
            .map_err(liel_error_to_pyerr)?
        {
            Some(n) => node_to_py(py, n),
            None => Ok(py.None()),
        }
    }

    #[pyo3(signature = (node, **kwargs))]
    fn update_node(&self, node: &Bound<PyAny>, kwargs: Option<&Bound<PyDict>>) -> PyResult<()> {
        let node_id = node_id_from_py(node)?;
        let props = kwargs_to_props(kwargs)?;
        acquire_graph_lock(&self.inner)?
            .update_node(node_id, props)
            .map_err(liel_error_to_pyerr)
    }

    fn delete_node(&self, node_arg: &Bound<PyAny>) -> PyResult<()> {
        let id = node_id_from_py(node_arg)?;
        acquire_graph_lock(&self.inner)?
            .delete_node(id)
            .map_err(liel_error_to_pyerr)
    }

    // ── Edge operations ──────────────────────────────────────────────────────

    #[pyo3(signature = (from_node, label, to_node, **kwargs))]
    fn add_edge(
        &self,
        py: Python,
        from_node: &Bound<PyAny>,
        label: String,
        to_node: &Bound<PyAny>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<PyObject> {
        let from_id = node_id_from_py(from_node)?;
        let to_id = node_id_from_py(to_node)?;
        let props = kwargs_to_props(kwargs)?;
        let e = acquire_graph_lock(&self.inner)?
            .add_edge(from_id, label, to_id, props)
            .map_err(liel_error_to_pyerr)?;
        edge_to_py(py, e)
    }

    fn get_edge(&self, py: Python, edge_id: u64) -> PyResult<PyObject> {
        match acquire_graph_lock(&self.inner)?
            .get_edge(edge_id)
            .map_err(liel_error_to_pyerr)?
        {
            Some(e) => edge_to_py(py, e),
            None => Ok(py.None()),
        }
    }

    #[pyo3(signature = (edge_id, **kwargs))]
    fn update_edge(&self, edge_id: u64, kwargs: Option<&Bound<PyDict>>) -> PyResult<()> {
        let props = kwargs_to_props(kwargs)?;
        acquire_graph_lock(&self.inner)?
            .update_edge(edge_id, props)
            .map_err(liel_error_to_pyerr)
    }

    fn delete_edge(&self, edge_arg: &Bound<PyAny>) -> PyResult<()> {
        let id = if let Ok(e) = edge_arg.downcast::<PyEdge>() {
            e.borrow().id()
        } else {
            edge_arg.extract::<u64>()?
        };
        acquire_graph_lock(&self.inner)?
            .delete_edge(id)
            .map_err(liel_error_to_pyerr)
    }

    #[pyo3(signature = (from_node, label, to_node, **kwargs))]
    fn merge_edge(
        &self,
        py: Python,
        from_node: &Bound<PyAny>,
        label: String,
        to_node: &Bound<PyAny>,
        kwargs: Option<&Bound<PyDict>>,
    ) -> PyResult<PyObject> {
        let from_id = node_id_from_py(from_node)?;
        let to_id = node_id_from_py(to_node)?;
        let props = kwargs_to_props(kwargs)?;
        let e = acquire_graph_lock(&self.inner)?
            .merge_edge(from_id, label, to_id, props)
            .map_err(liel_error_to_pyerr)?;
        edge_to_py(py, e)
    }

    // ── Adjacency ────────────────────────────────────────────────────────────

    #[pyo3(signature = (node_arg, label=None))]
    fn out_edges(
        &self,
        py: Python,
        node_arg: &Bound<PyAny>,
        label: Option<&str>,
    ) -> PyResult<Vec<PyObject>> {
        let id = node_id_from_py(node_arg)?;
        let edges = acquire_graph_lock(&self.inner)?
            .out_edges(id, label)
            .map_err(liel_error_to_pyerr)?;
        edges.into_iter().map(|e| edge_to_py(py, e)).collect()
    }

    #[pyo3(signature = (node_arg, label=None))]
    fn in_edges(
        &self,
        py: Python,
        node_arg: &Bound<PyAny>,
        label: Option<&str>,
    ) -> PyResult<Vec<PyObject>> {
        let id = node_id_from_py(node_arg)?;
        let edges = acquire_graph_lock(&self.inner)?
            .in_edges(id, label)
            .map_err(liel_error_to_pyerr)?;
        edges.into_iter().map(|e| edge_to_py(py, e)).collect()
    }

    #[pyo3(signature = (node_arg, edge_label=None, direction="out"))]
    fn neighbors(
        &self,
        py: Python,
        node_arg: &Bound<PyAny>,
        edge_label: Option<&str>,
        direction: &str,
    ) -> PyResult<Vec<PyObject>> {
        let id = node_id_from_py(node_arg)?;
        let dir = parse_direction(direction)?;
        let nodes = acquire_graph_lock(&self.inner)?
            .neighbors(id, edge_label, dir)
            .map_err(liel_error_to_pyerr)?;
        nodes.into_iter().map(|n| node_to_py(py, n)).collect()
    }

    // ── Traversal ────────────────────────────────────────────────────────────

    fn bfs(&self, py: Python, start: &Bound<PyAny>, max_depth: usize) -> PyResult<Vec<PyObject>> {
        let id = node_id_from_py(start)?;
        let result = acquire_graph_lock(&self.inner)?
            .bfs(id, max_depth)
            .map_err(liel_error_to_pyerr)?;
        result
            .into_iter()
            .map(|(n, depth)| {
                use pyo3::types::PyTuple;
                let node_py = Py::new(py, PyNode { inner: n })?.into_any();
                let depth_py = (depth as u64).into_pyobject(py)?.into_any().unbind();
                Ok(PyTuple::new(py, [node_py, depth_py])?.into_any().unbind())
            })
            .collect()
    }

    fn dfs(&self, py: Python, start: &Bound<PyAny>, max_depth: usize) -> PyResult<Vec<PyObject>> {
        let id = node_id_from_py(start)?;
        let result = acquire_graph_lock(&self.inner)?
            .dfs(id, max_depth)
            .map_err(liel_error_to_pyerr)?;
        result
            .into_iter()
            .map(|(n, depth)| {
                use pyo3::types::PyTuple;
                let node_py = Py::new(py, PyNode { inner: n })?.into_any();
                let depth_py = (depth as u64).into_pyobject(py)?.into_any().unbind();
                Ok(PyTuple::new(py, [node_py, depth_py])?.into_any().unbind())
            })
            .collect()
    }

    #[pyo3(signature = (start, goal, edge_label=None))]
    fn shortest_path(
        &self,
        py: Python,
        start: &Bound<PyAny>,
        goal: &Bound<PyAny>,
        edge_label: Option<&str>,
    ) -> PyResult<PyObject> {
        let start_id = node_id_from_py(start)?;
        let goal_id = node_id_from_py(goal)?;
        match acquire_graph_lock(&self.inner)?
            .shortest_path(start_id, goal_id, edge_label)
            .map_err(liel_error_to_pyerr)?
        {
            Some(path) => {
                let pylist: Vec<PyObject> = path
                    .into_iter()
                    .map(|n| Py::new(py, PyNode { inner: n }).map(|p| p.into_any()))
                    .collect::<PyResult<_>>()?;
                Ok(PyList::new(py, pylist)?.into_any().unbind())
            }
            None => Ok(py.None()),
        }
    }

    // ── Full scan ────────────────────────────────────────────────────────────

    fn all_nodes(&self, py: Python) -> PyResult<Vec<PyObject>> {
        let nodes = acquire_graph_lock(&self.inner)?
            .all_nodes()
            .map_err(liel_error_to_pyerr)?;
        nodes.into_iter().map(|n| node_to_py(py, n)).collect()
    }

    fn all_edges(&self, py: Python) -> PyResult<Vec<PyObject>> {
        let edges = acquire_graph_lock(&self.inner)?
            .all_edges()
            .map_err(liel_error_to_pyerr)?;
        edges.into_iter().map(|e| edge_to_py(py, e)).collect()
    }

    // ── Bulk record accessors (single boundary crossing per call) ────────────

    fn all_nodes_as_records(&self, py: Python) -> PyResult<Vec<PyObject>> {
        let nodes = acquire_graph_lock(&self.inner)?
            .all_nodes()
            .map_err(liel_error_to_pyerr)?;
        nodes
            .into_iter()
            .map(|n| {
                let dict = PyDict::new(py);
                dict.set_item("id", n.id)?;
                dict.set_item("labels", n.labels.clone())?;
                for (k, v) in &n.properties {
                    dict.set_item(k, prop_to_py(py, v)?)?;
                }
                Ok(dict.into_any().unbind())
            })
            .collect()
    }

    fn all_edges_as_records(&self, py: Python) -> PyResult<Vec<PyObject>> {
        let edges = acquire_graph_lock(&self.inner)?
            .all_edges()
            .map_err(liel_error_to_pyerr)?;
        edges
            .into_iter()
            .map(|e| {
                let dict = PyDict::new(py);
                dict.set_item("id", e.id)?;
                dict.set_item("label", e.label.clone())?;
                dict.set_item("from_node", e.from)?;
                dict.set_item("to_node", e.to)?;
                for (k, v) in &e.properties {
                    dict.set_item(k, prop_to_py(py, v)?)?;
                }
                Ok(dict.into_any().unbind())
            })
            .collect()
    }

    // Returns {node_id: (out_degree, in_degree)} computed entirely in Rust.
    fn degree_stats(&self, py: Python) -> PyResult<PyObject> {
        let edges = acquire_graph_lock(&self.inner)?
            .all_edges()
            .map_err(liel_error_to_pyerr)?;
        let mut stats: HashMap<u64, (u64, u64)> = HashMap::new();
        for e in &edges {
            stats
                .entry(e.from)
                .and_modify(|(o, _)| *o += 1)
                .or_insert((1, 0));
            stats
                .entry(e.to)
                .and_modify(|(_, i)| *i += 1)
                .or_insert((0, 1));
        }
        let dict = PyDict::new(py);
        for (node_id, (out_deg, in_deg)) in stats {
            dict.set_item(node_id, (out_deg, in_deg).into_pyobject(py)?)?;
        }
        Ok(dict.into_any().unbind())
    }

    // Returns edge records where both endpoints are in node_ids.
    // Accepts any Python iterable of int node IDs (set, list, etc.).
    fn edges_between(&self, py: Python, node_ids: &Bound<PyAny>) -> PyResult<Vec<PyObject>> {
        let ids: std::collections::HashSet<u64> = node_ids
            .try_iter()?
            .map(|x| x?.extract::<u64>())
            .collect::<PyResult<_>>()?;
        let edges = acquire_graph_lock(&self.inner)?
            .all_edges()
            .map_err(liel_error_to_pyerr)?;
        edges
            .into_iter()
            .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
            .map(|e| {
                let dict = PyDict::new(py);
                dict.set_item("id", e.id)?;
                dict.set_item("label", e.label.clone())?;
                dict.set_item("from_node", e.from)?;
                dict.set_item("to_node", e.to)?;
                for (k, v) in &e.properties {
                    dict.set_item(k, prop_to_py(py, v)?)?;
                }
                Ok(dict.into_any().unbind())
            })
            .collect()
    }

    fn node_count(&self) -> PyResult<u64> {
        Ok(acquire_graph_lock(&self.inner)?.node_count())
    }

    fn edge_count(&self) -> PyResult<u64> {
        Ok(acquire_graph_lock(&self.inner)?.edge_count())
    }

    // ── Transaction ──────────────────────────────────────────────────────────

    fn begin(&self) -> PyResult<()> {
        Ok(())
    }

    fn commit(&self) -> PyResult<()> {
        acquire_graph_lock(&self.inner)?
            .commit()
            .map_err(liel_error_to_pyerr)
    }

    fn rollback(&self) -> PyResult<()> {
        acquire_graph_lock(&self.inner)?
            .rollback()
            .map_err(liel_error_to_pyerr)
    }

    fn transaction(&self, py: Python) -> PyResult<Py<PyTransaction>> {
        // The explicit-transaction flag is toggled in `__enter__` /
        // `__exit__`, not here, so a stray `db.transaction()` that is
        // never entered (e.g. discarded by Python before the `with`
        // block runs) does not leave the flag stuck on `true`.  This
        // mirrors the Rust-level `TransactionGuard` whose lifecycle is
        // tied to the actual scope, not to construction.
        Py::new(
            py,
            PyTransaction {
                db: self.inner.clone(),
            },
        )
    }

    // ── Cross-database merge ─────────────────────────────────────────────────

    /// Merge every live node and edge from `other` into `self`.
    ///
    /// Two-phase locking: snapshot the source under `other`'s lock, then
    /// release it and acquire `self`'s lock to write.  The locks are never
    /// held simultaneously, so merging two databases from two Python threads
    /// cannot deadlock regardless of which side is locked first.
    #[pyo3(signature = (
        other,
        *,
        node_key = None,
        edge_strategy = "append",
        on_node_conflict = "keep_dst",
    ))]
    fn merge_from(
        &self,
        py: Python,
        other: &Bound<PyGraphDB>,
        node_key: Option<Vec<String>>,
        edge_strategy: &str,
        on_node_conflict: &str,
    ) -> PyResult<Py<PyMergeReport>> {
        let policy = build_merge_policy(node_key, edge_strategy, on_node_conflict)?;

        let other_inner = other.borrow().inner.clone();
        if Arc::ptr_eq(&self.inner, &other_inner) {
            return Err(PyValueError::new_err(
                "merge_from: source and destination must be different GraphDB instances",
            ));
        }

        // Snapshot the source under its own lock only, so we never hold both
        // Mutexes at once.  After this block `src_nodes` / `src_edges` are
        // owned Vecs, independent of src's pager cache state.
        // Use `acquire_graph_lock` (not raw `.lock().unwrap()`) so a poisoned
        // mutex surfaces as `PyRuntimeError`, matching the rest of this module.
        let (src_nodes, src_edges) = {
            let mut src_db = acquire_graph_lock(&other_inner)?;
            let nodes = src_db.all_nodes().map_err(liel_error_to_pyerr)?;
            let edges = src_db.all_edges().map_err(liel_error_to_pyerr)?;
            (nodes, edges)
        };

        let report = {
            let mut dst_db = acquire_graph_lock(&self.inner)?;
            merge::merge_from_snapshot(&mut dst_db, &src_nodes, &src_edges, &policy)
                .map_err(liel_error_to_pyerr)?
        };

        Py::new(py, PyMergeReport { inner: report })
    }

    // ── Maintenance ──────────────────────────────────────────────────────────

    fn vacuum(&self) -> PyResult<()> {
        acquire_graph_lock(&self.inner)?
            .vacuum()
            .map_err(liel_error_to_pyerr)
    }

    fn clear(&self) -> PyResult<()> {
        acquire_graph_lock(&self.inner)?
            .clear()
            .map_err(liel_error_to_pyerr)
    }

    fn repair_adjacency(&self, py: Python) -> PyResult<PyObject> {
        let report = acquire_graph_lock(&self.inner)?
            .repair_adjacency()
            .map_err(liel_error_to_pyerr)?;
        let dict = PyDict::new(py);
        dict.set_item("nodes_rewritten", report.nodes_rewritten)?;
        dict.set_item("edges_relinked", report.edges_relinked)?;
        Ok(dict.into_any().unbind())
    }

    fn info(&self, py: Python) -> PyResult<PyObject> {
        let info = acquire_graph_lock(&self.inner)?.info();
        let dict = PyDict::new(py);
        dict.set_item("version", info.version)?;
        dict.set_item("node_count", info.node_count)?;
        dict.set_item("edge_count", info.edge_count)?;
        dict.set_item("file_size", info.file_size)?;
        Ok(dict.into_any().unbind())
    }

    // ── Query builder entry points ───────────────────────────────────────────

    fn nodes(&self, py: Python) -> PyResult<Py<PyNodeQuery>> {
        Py::new(
            py,
            PyNodeQuery {
                db: self.inner.clone(),
                label_filters: Vec::new(),
                predicate: None,
                skip_n: 0,
                limit_n: None,
            },
        )
    }

    fn edges(&self, py: Python) -> PyResult<Py<PyEdgeQuery>> {
        Py::new(
            py,
            PyEdgeQuery {
                db: self.inner.clone(),
                label_filters: Vec::new(),
                predicate: None,
                skip_n: 0,
                limit_n: None,
            },
        )
    }
}

// ─── PyTransaction ───────────────────────────────────────────────────────────

/// PyO3 wrapper exposing a transaction context manager as `liel.Transaction`.
///
/// Returned by `PyGraphDB::transaction` and used through Python's `with`
/// statement.  Holds a clone of the same `Arc<Mutex<GraphDB>>` as
/// `PyGraphDB` so commit / rollback target the same underlying database.
/// `__enter__` / `__exit__` map to commit-on-success / rollback-on-exception
/// semantics; see `python/liel/liel.pyi::Transaction` for the user-facing
/// contract.
#[pyclass(name = "Transaction")]
pub struct PyTransaction {
    db: Arc<Mutex<Option<GraphDB>>>,
}

#[pymethods]
impl PyTransaction {
    /// Enter the `with` block: set the explicit-transaction flag so a
    /// nested `with db.transaction(): with db.transaction(): ...`
    /// raises `TransactionError` here at the inner `__enter__`.  See
    /// product-tradeoffs §5.5.
    fn __enter__(slf: PyRef<'_, Self>) -> PyResult<PyRef<'_, Self>> {
        {
            let mut db = acquire_graph_lock(&slf.db)?;
            db.begin_explicit_transaction()
                .map_err(liel_error_to_pyerr)?;
        }
        Ok(slf)
    }

    #[pyo3(signature = (exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &self,
        exc_type: Option<Bound<PyAny>>,
        _exc_val: Option<Bound<PyAny>>,
        _exc_tb: Option<Bound<PyAny>>,
    ) -> PyResult<bool> {
        let mut db = acquire_graph_lock(&self.db)?;
        // `commit()` and `rollback()` both clear the explicit-transaction
        // flag, so successive `with db.transaction()` blocks remain
        // independent.
        if exc_type.is_some() {
            db.rollback().map_err(liel_error_to_pyerr)?;
        } else {
            db.commit().map_err(liel_error_to_pyerr)?;
        }
        Ok(false)
    }
}

// ─── PyNodeQuery ─────────────────────────────────────────────────────────────

#[pyclass(name = "NodeQuery")]
pub struct PyNodeQuery {
    db: Arc<Mutex<Option<GraphDB>>>,
    label_filters: Vec<String>,
    predicate: Option<Py<PyAny>>,
    skip_n: usize,
    limit_n: Option<usize>,
}

#[pymethods]
impl PyNodeQuery {
    fn label(&self, py: Python, label: String) -> PyResult<Py<PyNodeQuery>> {
        let mut filters = self.label_filters.clone();
        filters.push(label);
        Py::new(
            py,
            PyNodeQuery {
                db: self.db.clone(),
                label_filters: filters,
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: self.skip_n,
                limit_n: self.limit_n,
            },
        )
    }

    fn where_(&self, py: Python, predicate: Py<PyAny>) -> PyResult<Py<PyNodeQuery>> {
        Py::new(
            py,
            PyNodeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: Some(predicate),
                skip_n: self.skip_n,
                limit_n: self.limit_n,
            },
        )
    }

    fn skip(&self, py: Python, n: usize) -> PyResult<Py<PyNodeQuery>> {
        Py::new(
            py,
            PyNodeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: n,
                limit_n: self.limit_n,
            },
        )
    }

    fn limit(&self, py: Python, n: usize) -> PyResult<Py<PyNodeQuery>> {
        Py::new(
            py,
            PyNodeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: self.skip_n,
                limit_n: Some(n),
            },
        )
    }

    fn fetch(&self, py: Python) -> PyResult<Vec<PyObject>> {
        self.collect_nodes(py)?
            .into_iter()
            .map(|n| node_to_py(py, n))
            .collect()
    }

    fn count(&self, py: Python) -> PyResult<usize> {
        Ok(self.collect_nodes(py)?.len())
    }

    fn exists(&self, py: Python) -> PyResult<bool> {
        // Override any caller-supplied limit: short-circuiting to 1 cannot
        // change the answer (if one match exists we return true either way)
        // and avoids evaluating the Python predicate more than necessary.
        Ok(!self.collect_nodes_with_limit(py, Some(1))?.is_empty())
    }
}

impl PyNodeQuery {
    // Phase 1 (under lock): label filtering via GraphDB::scan_nodes.
    // Phase 2 (lock released): Python predicate + skip/limit.
    // The split is required so the Python callback is never invoked while
    // the Mutex is held, which would deadlock if the callback calls back into db.
    fn collect_nodes(&self, py: Python) -> PyResult<Vec<Node>> {
        self.collect_nodes_with_limit(py, self.limit_n)
    }

    fn collect_nodes_with_limit(
        &self,
        py: Python,
        limit_override: Option<usize>,
    ) -> PyResult<Vec<Node>> {
        let candidates = {
            let mut db = acquire_graph_lock(&self.db)?;
            db.scan_nodes(&self.label_filters)
                .map_err(liel_error_to_pyerr)?
        };

        let mut results = Vec::new();
        let mut skipped = 0usize;
        for n in candidates {
            if let Some(ref pred) = self.predicate {
                let py_node = Py::new(py, PyNode { inner: n.clone() })?;
                let ok: bool = pred.call1(py, (py_node,))?.extract(py)?;
                if !ok {
                    continue;
                }
            }
            if skipped < self.skip_n {
                skipped += 1;
                continue;
            }
            results.push(n);
            if limit_override.is_some_and(|l| results.len() >= l) {
                break;
            }
        }
        Ok(results)
    }
}

// ─── PyEdgeQuery ─────────────────────────────────────────────────────────────

#[pyclass(name = "EdgeQuery")]
pub struct PyEdgeQuery {
    db: Arc<Mutex<Option<GraphDB>>>,
    label_filters: Vec<String>,
    predicate: Option<Py<PyAny>>,
    skip_n: usize,
    limit_n: Option<usize>,
}

#[pymethods]
impl PyEdgeQuery {
    fn label(&self, py: Python, label: String) -> PyResult<Py<PyEdgeQuery>> {
        let mut filters = self.label_filters.clone();
        filters.push(label);
        Py::new(
            py,
            PyEdgeQuery {
                db: self.db.clone(),
                label_filters: filters,
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: self.skip_n,
                limit_n: self.limit_n,
            },
        )
    }

    fn where_(&self, py: Python, predicate: Py<PyAny>) -> PyResult<Py<PyEdgeQuery>> {
        Py::new(
            py,
            PyEdgeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: Some(predicate),
                skip_n: self.skip_n,
                limit_n: self.limit_n,
            },
        )
    }

    fn skip(&self, py: Python, n: usize) -> PyResult<Py<PyEdgeQuery>> {
        Py::new(
            py,
            PyEdgeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: n,
                limit_n: self.limit_n,
            },
        )
    }

    fn limit(&self, py: Python, n: usize) -> PyResult<Py<PyEdgeQuery>> {
        Py::new(
            py,
            PyEdgeQuery {
                db: self.db.clone(),
                label_filters: self.label_filters.clone(),
                predicate: self.predicate.as_ref().map(|p| p.clone_ref(py)),
                skip_n: self.skip_n,
                limit_n: Some(n),
            },
        )
    }

    fn fetch(&self, py: Python) -> PyResult<Vec<PyObject>> {
        self.collect_edges(py)?
            .into_iter()
            .map(|e| edge_to_py(py, e))
            .collect()
    }

    fn count(&self, py: Python) -> PyResult<usize> {
        Ok(self.collect_edges(py)?.len())
    }

    fn exists(&self, py: Python) -> PyResult<bool> {
        Ok(!self.collect_edges_with_limit(py, Some(1))?.is_empty())
    }
}

impl PyEdgeQuery {
    fn collect_edges(&self, py: Python) -> PyResult<Vec<Edge>> {
        self.collect_edges_with_limit(py, self.limit_n)
    }

    fn collect_edges_with_limit(
        &self,
        py: Python,
        limit_override: Option<usize>,
    ) -> PyResult<Vec<Edge>> {
        let candidates = {
            let mut db = acquire_graph_lock(&self.db)?;
            db.scan_edges(&self.label_filters)
                .map_err(liel_error_to_pyerr)?
        };

        let mut results = Vec::new();
        let mut skipped = 0usize;
        for e in candidates {
            if let Some(ref pred) = self.predicate {
                let py_edge = Py::new(py, PyEdge { inner: e.clone() })?;
                let ok: bool = pred.call1(py, (py_edge,))?.extract(py)?;
                if !ok {
                    continue;
                }
            }
            if skipped < self.skip_n {
                skipped += 1;
                continue;
            }
            results.push(e);
            if limit_override.is_some_and(|l| results.len() >= l) {
                break;
            }
        }
        Ok(results)
    }
}

// ─── Module-level open() ─────────────────────────────────────────────────────

#[pyfunction]
pub fn open(py: Python, path: &str) -> PyResult<PyObject> {
    let db = GraphDB::open(path).map_err(liel_error_to_pyerr)?;
    Ok(Py::new(py, PyGraphDB::new(db))?.into_any())
}

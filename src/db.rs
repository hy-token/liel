use std::collections::{HashMap, HashSet};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::error::{LielError, Result};
use crate::graph::{
    edge::{self, Direction, Edge},
    index::LabelIndex,
    merge::{self, MergePolicy, MergeReport},
    node::{self, Node},
    repair::{self, RepairReport},
    traverse, vacuum,
};
use crate::query::builder;
use crate::storage::lock::WriterLock;
use crate::storage::pager::Pager;
use crate::storage::prop_codec::PropValue;

/// In-process registry of `.liel` files that currently have a live
/// `GraphDB` handle.
///
/// liel guarantees crash-safety only with **one writer per file**.  This
/// registry is the cheap, fast-path enforcement of that contract: if a
/// caller tries to open the same canonicalised path twice from the same
/// process the second `open()` is rejected with [`LielError::AlreadyOpen`]
/// before any I/O races can occur.
///
/// The set holds canonicalised `PathBuf`s so that paths differing only in
/// case (Windows), trailing separators, or `./` segments still collide.
/// In-memory (`":memory:"`) databases are not tracked because each one
/// owns an independent in-process buffer.
fn open_files() -> &'static Mutex<HashSet<PathBuf>> {
    static REGISTRY: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Test-only: clear the in-process open-file registry.
///
/// Intended for unit tests that intentionally open and close the same path
/// in tight succession; production code never calls this.
#[cfg(test)]
pub fn _reset_open_files_for_test() {
    if let Ok(mut guard) = open_files().lock() {
        guard.clear();
    }
}

/// Metadata returned by [`GraphDB::info`].
///
/// The `version` field is a **fixed API string** (`"1.0"`) for human-facing
/// output. It is **not** parsed from the on-disk file header at runtime; it
/// matches the documented on-disk format major/minor (1.0) by convention.
pub struct DbInfo {
    /// Human-readable version label (currently always `"1.0"`).
    pub version: String,
    pub node_count: u64,
    pub edge_count: u64,
    pub file_size: u64,
}

/// The core graph database handle.
///
/// Owns the `Pager` and encapsulates all business logic: CRUD, cascade delete,
/// traversal, transaction control, and queries.  Language bindings (PyO3, C FFI,
/// etc.) wrap this type and are responsible only for type conversion.
pub struct GraphDB {
    pager: Pager,
    label_index: LabelIndex,
    /// Canonicalised path registered in [`open_files`] so the entry can be
    /// removed in [`Drop`].  `None` for `":memory:"` and any future
    /// non-file backends that bypass the registry.
    registry_key: Option<PathBuf>,
    writer_lock: Option<WriterLock>,
    /// Whether an **explicit** transaction (`GraphDB::transaction()` /
    /// `with db.transaction()`) is currently in flight.  Used to enforce
    /// the no-nesting rule confirmed in product-tradeoffs §5.5: a second
    /// `transaction()` call while one is active fails with
    /// [`LielError::TransactionError`] instead of silently swallowing the
    /// inner scope's commit.
    ///
    /// `commit()` and `rollback()` outside an explicit transaction
    /// continue to work; they target the **implicit** transaction begun
    /// at `open()` time.  This flag tracks only the explicit overlay.
    transaction_active: bool,
}

impl GraphDB {
    pub fn open(path: &str) -> Result<Self> {
        // In-memory databases are isolated per handle and don't share state
        // through a filesystem path, so the writer-per-file guard does not
        // apply to them.
        if path == ":memory:" {
            let mut pager = Pager::open(path)?;
            let label_index = LabelIndex::build(&mut pager)?;
            return Ok(Self {
                pager,
                label_index,
                registry_key: None,
                writer_lock: None,
                transaction_active: false,
            });
        }

        // Hold the registry lock across pager creation so two threads racing
        // to open the same path cannot both succeed.  The lock is uncontended
        // in the common case (one open per handle) and the work inside is
        // bounded — we are not doing any sustained I/O while holding it.
        //
        // Poison policy (intentional split, see product-tradeoffs §5.5 +
        // PR ".../prioritize-report-improvements" decision log):
        //
        //   - This `open_files` registry recovers via `into_inner` because it
        //     only tracks "is this path already open in-process".  A panic in
        //     a previous critical section cannot leave the registry in a
        //     state worse than "this entry might be stale", and a stale
        //     entry just produces an `AlreadyOpen` until the dead handle
        //     drops; data integrity is unaffected.
        //
        //   - The `GraphDB` mutex inside `PyGraphDB` (src/python/types.rs)
        //     does NOT recover.  A poisoned graph lock means a panic
        //     happened mid-CRUD with potentially half-updated dirty pages,
        //     so the only safe answer is to surface a runtime error and
        //     force the caller to reopen.
        let registry = open_files();
        let mut guard = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let canonical = canonical_db_path(path)?;

        if guard.contains(&canonical) {
            return Err(LielError::AlreadyOpen(canonical.display().to_string()));
        }

        let writer_lock = WriterLock::acquire(&canonical)?;

        // Sweep any stale `<basename>.tmp` left by a vacuum that crashed
        // mid-rewrite (see product-tradeoffs §5.6).  Under §5.1's
        // single-writer guarantee the only owner of `.tmp` is a previous
        // vacuum from this same path, so unconditional removal is safe and
        // does not need to inspect the file's contents.  An ENOENT here is
        // the expected "no leftover" path; surface anything else.
        remove_stale_vacuum_tmp(&canonical)?;

        let mut pager = Pager::open(path)?;
        let label_index = LabelIndex::build(&mut pager)?;
        guard.insert(canonical.clone());
        drop(guard);

        Ok(Self {
            pager,
            label_index,
            registry_key: Some(canonical),
            writer_lock: Some(writer_lock),
            transaction_active: false,
        })
    }

    // ── Node CRUD ────────────────────────────────────────────────────────────

    pub fn add_node(
        &mut self,
        labels: Vec<String>,
        props: HashMap<String, PropValue>,
    ) -> Result<Node> {
        let n = node::add_node(&mut self.pager, labels, props)?;
        self.label_index.insert(n.id, &n.labels);
        Ok(n)
    }

    pub fn get_node(&mut self, id: u64) -> Result<Option<Node>> {
        node::get_node(&mut self.pager, id)
    }

    pub fn update_node(&mut self, id: u64, props: HashMap<String, PropValue>) -> Result<()> {
        node::update_node(&mut self.pager, id, props)
    }

    /// Delete a node and cascade-delete all its incident edges.
    ///
    /// Fail-fast: if any individual edge deletion fails the method returns
    /// immediately without touching the remaining edges or the node itself,
    /// leaving the in-memory dirty pages in a partially-updated state.  The
    /// caller is expected to recover by **not** calling `commit()` and instead
    /// closing and reopening the database, which discards the uncommitted WAL
    /// and restores the last committed state (see `docs/reference/features.ja.md`).
    pub fn delete_node(&mut self, id: u64) -> Result<()> {
        // Capture labels before deletion so the index can be updated.
        let labels: Vec<String> = node::get_node(&mut self.pager, id)?
            .map(|n| n.labels)
            .unwrap_or_default();
        let mut deleted_edges = HashSet::new();

        for e in edge::out_edges(&mut self.pager, id, None)? {
            edge::delete_edge(&mut self.pager, e.id)?;
            deleted_edges.insert(e.id);
        }
        for e in edge::in_edges(&mut self.pager, id, None)? {
            if deleted_edges.insert(e.id) {
                edge::delete_edge(&mut self.pager, e.id)?;
            }
        }
        node::delete_node(&mut self.pager, id)?;
        self.label_index.remove(id, &labels);
        Ok(())
    }

    // ── Edge CRUD ────────────────────────────────────────────────────────────

    pub fn add_edge(
        &mut self,
        from: u64,
        label: String,
        to: u64,
        props: HashMap<String, PropValue>,
    ) -> Result<Edge> {
        edge::add_edge(&mut self.pager, from, label, to, props)
    }

    pub fn get_edge(&mut self, id: u64) -> Result<Option<Edge>> {
        edge::get_edge(&mut self.pager, id)
    }

    pub fn update_edge(&mut self, id: u64, props: HashMap<String, PropValue>) -> Result<()> {
        edge::update_edge(&mut self.pager, id, props)
    }

    pub fn delete_edge(&mut self, id: u64) -> Result<()> {
        edge::delete_edge(&mut self.pager, id)
    }

    /// Return an existing edge matching (from, label, to, props) or create one.
    pub fn merge_edge(
        &mut self,
        from: u64,
        label: String,
        to: u64,
        props: HashMap<String, PropValue>,
    ) -> Result<Edge> {
        edge::merge_edge(&mut self.pager, from, label, to, props)
    }

    // ── Adjacency ────────────────────────────────────────────────────────────

    pub fn out_edges(&mut self, node_id: u64, label: Option<&str>) -> Result<Vec<Edge>> {
        edge::out_edges(&mut self.pager, node_id, label)
    }

    pub fn in_edges(&mut self, node_id: u64, label: Option<&str>) -> Result<Vec<Edge>> {
        edge::in_edges(&mut self.pager, node_id, label)
    }

    /// Return neighbor nodes reachable via the given direction and optional label filter.
    pub fn neighbors(
        &mut self,
        node_id: u64,
        edge_label: Option<&str>,
        direction: Direction,
    ) -> Result<Vec<Node>> {
        let neighbor_ids = edge::neighbors(&mut self.pager, node_id, edge_label, direction)?;
        let mut result = Vec::new();
        for nid in neighbor_ids {
            if let Some(n) = node::get_node(&mut self.pager, nid)? {
                result.push(n);
            }
        }
        Ok(result)
    }

    // ── Traversal ────────────────────────────────────────────────────────────

    pub fn bfs(&mut self, start: u64, max_depth: usize) -> Result<Vec<(Node, usize)>> {
        traverse::bfs(&mut self.pager, start, max_depth)
    }

    pub fn dfs(&mut self, start: u64, max_depth: usize) -> Result<Vec<(Node, usize)>> {
        traverse::dfs(&mut self.pager, start, max_depth)
    }

    pub fn shortest_path(
        &mut self,
        start: u64,
        goal: u64,
        edge_label: Option<&str>,
    ) -> Result<Option<Vec<Node>>> {
        traverse::shortest_path(&mut self.pager, start, goal, edge_label)
    }

    // ── Full scan ────────────────────────────────────────────────────────────

    pub fn all_nodes(&mut self) -> Result<Vec<Node>> {
        node::all_nodes(&mut self.pager)
    }

    pub fn all_edges(&mut self) -> Result<Vec<Edge>> {
        edge::all_edges(&mut self.pager)
    }

    pub fn node_count(&self) -> u64 {
        self.pager.node_count()
    }

    pub fn edge_count(&self) -> u64 {
        self.pager.edge_count()
    }

    // ── Query builder entry points (label-only filtering; predicate stays in callers) ──

    /// Scan nodes filtered by label. Used by language-binding query builders
    /// that apply predicates (e.g. Python lambdas) after this call.
    ///
    /// When label filters are present the label index is consulted to avoid a
    /// full slot scan: only the matching node IDs are fetched from the pager.
    pub fn scan_nodes(&mut self, label_filters: &[String]) -> Result<Vec<Node>> {
        if label_filters.is_empty() {
            return builder::nodes(&mut self.pager).fetch();
        }

        let ids = self.label_index.ids_for_labels(label_filters);
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(n) = node::get_node(&mut self.pager, id)? {
                result.push(n);
            }
        }
        Ok(result)
    }

    /// Scan edges filtered by label.
    pub fn scan_edges(&mut self, label_filters: &[String]) -> Result<Vec<Edge>> {
        let mut q = builder::edges(&mut self.pager);
        for lbl in label_filters {
            q = q.label(lbl);
        }
        q.fetch()
    }

    // ── Transaction ──────────────────────────────────────────────────────────

    pub fn commit(&mut self) -> Result<()> {
        // `commit()` may be called both inside an explicit transaction
        // (the guard's `commit()` delegates here) and on its own to land
        // the implicit transaction.  Either way the explicit-transaction
        // flag — if set — ends here, because the underlying pager state
        // is now durable.
        let result = self.pager.commit();
        self.transaction_active = false;
        result
    }

    pub fn rollback(&mut self) -> Result<()> {
        let result = self.pager.rollback().and_then(|()| {
            // Rebuild the index from the now-committed on-disk state so it stays
            // consistent with the pager after dirty pages are discarded.
            self.label_index = LabelIndex::build(&mut self.pager)?;
            Ok(())
        });
        // The explicit transaction is over regardless of whether the
        // pager rollback itself succeeded — leaving the flag set would
        // permanently lock out future `transaction()` calls.
        self.transaction_active = false;
        result
    }

    /// Begin an explicit transaction and return a RAII guard.
    ///
    /// The returned [`TransactionGuard`] borrows `self` mutably; calling
    /// any other `&mut self` method on this `GraphDB` is impossible until
    /// the guard is dropped, which makes nesting structurally unrepresentable
    /// in safe Rust code.  When the guard is dropped without a prior
    /// [`commit()`](TransactionGuard::commit) the `Drop` impl performs a
    /// best-effort rollback so the database does not silently retain
    /// unreviewed dirty pages.
    ///
    /// # Errors
    ///
    /// Returns [`LielError::TransactionError`] if an explicit transaction
    /// is already active for this handle.  See product-tradeoffs §5.5.
    pub fn transaction(&mut self) -> Result<TransactionGuard<'_>> {
        if self.transaction_active {
            return Err(LielError::TransactionError(
                "transaction already active".into(),
            ));
        }
        self.transaction_active = true;
        Ok(TransactionGuard {
            db: self,
            settled: false,
        })
    }

    /// Whether an explicit `transaction()` is currently in flight.
    ///
    /// Used by language bindings (Python's `PyTransaction`) to enforce
    /// the same nesting rule across the FFI boundary, where a Rust-style
    /// borrow check is not available.
    pub fn is_transaction_active(&self) -> bool {
        self.transaction_active
    }

    /// Mark the explicit-transaction flag without going through the RAII
    /// guard.  Returns [`LielError::TransactionError`] if a transaction is
    /// already active so the caller can reject re-entry the same way the
    /// Rust-level [`transaction`](Self::transaction) does.
    ///
    /// Intended exclusively for FFI wrappers that cannot hold a
    /// `&'a mut GraphDB` for the lifetime of an exposed context manager
    /// (e.g. PyO3, where the lock must be released between `__enter__`
    /// and `__exit__`).  The wrapper is then responsible for calling
    /// `commit()` or `rollback()` to clear the flag, exactly as the
    /// guard's `Drop` would.
    pub fn begin_explicit_transaction(&mut self) -> Result<()> {
        if self.transaction_active {
            return Err(LielError::TransactionError(
                "transaction already active".into(),
            ));
        }
        self.transaction_active = true;
        Ok(())
    }
}

/// RAII handle returned by [`GraphDB::transaction`].
///
/// On `Drop`, if neither [`commit`](Self::commit) nor [`rollback`](Self::rollback)
/// was called explicitly, the underlying `GraphDB::rollback()` runs as a
/// best-effort safety net.  Errors from the implicit rollback are
/// swallowed because `Drop` cannot return a `Result`; callers that need
/// to know whether their work survived should call `commit()` or
/// `rollback()` themselves.
///
/// The guard implements [`Deref`] / [`DerefMut`] to the borrowed
/// `GraphDB`, so every method on `GraphDB` is callable directly on the
/// guard for the duration of the transaction:
///
/// ```rust,ignore
/// let mut tx = db.transaction()?;
/// tx.add_node(vec!["X".into()], HashMap::new())?;
/// tx.commit()?;
/// ```
///
/// Without these impls the guard would hold an exclusive `&mut GraphDB`
/// borrow that prevented every other method call inside the scope,
/// making the API unusable from external Rust code.
pub struct TransactionGuard<'a> {
    db: &'a mut GraphDB,
    /// True once `commit()` or `rollback()` has been invoked on this
    /// guard, suppressing the `Drop` rollback.  The name avoids
    /// `committed` because a successful explicit `rollback()` also flips
    /// it to `true`.
    settled: bool,
}

impl<'a> Deref for TransactionGuard<'a> {
    type Target = GraphDB;

    fn deref(&self) -> &GraphDB {
        self.db
    }
}

impl<'a> DerefMut for TransactionGuard<'a> {
    fn deref_mut(&mut self) -> &mut GraphDB {
        self.db
    }
}

impl<'a> TransactionGuard<'a> {
    /// Commit every change accumulated since the guard was created
    /// (and any earlier work in the implicit transaction the guard
    /// inherited from `open()`), making them durable on disk.
    pub fn commit(mut self) -> Result<()> {
        let result = self.db.commit();
        self.settled = true;
        result
    }

    /// Discard every change accumulated since the guard was created and
    /// rebuild the in-memory label index from the previous committed
    /// state.  The `transaction_active` flag is cleared.
    pub fn rollback(mut self) -> Result<()> {
        let result = self.db.rollback();
        self.settled = true;
        result
    }
}

impl<'a> Drop for TransactionGuard<'a> {
    fn drop(&mut self) {
        if !self.settled {
            // Best-effort rollback — explicitly ignore the error because
            // panicking from `Drop` would mask the original cause that
            // caused the user to leave the scope without committing.
            let _ = self.db.rollback();
        }
    }
}

impl GraphDB {
    // ── Cross-database merge ─────────────────────────────────────────────────

    /// Merge every live node and edge from `other` into `self`.
    ///
    /// Delegates to [`graph::merge::merge_from`].  The source database is
    /// read-only for the duration of the call; all writes land in the current
    /// destination transaction and are visible only after `commit()` is
    /// called by the caller.  See the `graph::merge` module for the full
    /// contract (ID remap, file-format invariants, transaction model).
    pub fn merge_from(&mut self, other: &mut GraphDB, policy: &MergePolicy) -> Result<MergeReport> {
        merge::merge_from(self, other, policy)
    }

    // ── Maintenance ──────────────────────────────────────────────────────────

    /// Compact property storage and reclaim space from deleted records.
    ///
    /// For on-disk databases this is a **crash-safe** copy-on-write rewrite
    /// (product-tradeoffs §5.6): a sibling `<basename>.liel.tmp` is built
    /// from scratch and atomically renamed over the live file.  After the
    /// rename succeeds the original pager's file handle points at an
    /// unlinked inode, so we drop it and reopen from the same path.  The
    /// label index is rebuilt because rebuilding from the fresh on-disk
    /// state is cheaper than tracking the moves through vacuum.
    ///
    /// For `:memory:` databases there is nothing to rename, so vacuum
    /// falls back to the in-place algorithm with no pager swap.
    ///
    /// # Errors
    ///
    /// Returns [`LielError::TransactionError`] when called inside an
    /// explicit `transaction()` scope.  Vacuum forces an internal
    /// `commit()` at entry, which would silently flush whatever the
    /// caller has staged in the surrounding transaction — usually the
    /// opposite of what `with db.transaction()` was meant to provide.
    /// Commit (or rollback) the transaction first, then call `vacuum()`.
    pub fn vacuum(&mut self) -> Result<()> {
        if self.transaction_active {
            return Err(LielError::TransactionError(
                "vacuum() cannot run inside an explicit transaction; \
                 commit or rollback the transaction first"
                    .into(),
            ));
        }
        if let Some(path) = self.registry_key.clone() {
            let tmp_path = vacuum::build_file_vacuum_tmp(&mut self.pager, &path)?;
            let path_str = path.to_str().ok_or_else(|| {
                LielError::InvalidArgument("vacuum: database path is not valid UTF-8".into())
            })?;

            // Windows cannot atomically replace a file while our own old
            // file handle is still live. Build the tmp file first, swap in
            // an in-memory placeholder to drop the file-backed pager, then
            // rename and reopen the real path.
            let stale_pager = std::mem::replace(&mut self.pager, Pager::open(":memory:")?);
            drop(stale_pager);

            if let Err(err) = vacuum::install_file_vacuum_tmp(&tmp_path, &path) {
                let _ = std::fs::remove_file(&tmp_path);
                self.pager = Pager::open(path_str)?;
                self.label_index = LabelIndex::build(&mut self.pager)?;
                return Err(err);
            }

            self.pager = Pager::open(path_str)?;
            self.label_index = LabelIndex::build(&mut self.pager)?;
        } else {
            vacuum::vacuum(&mut self.pager, None)?;
        }
        Ok(())
    }

    /// Rebuild node adjacency heads, degree counters, and edge next-pointers
    /// from the live edge set, then commit the repaired structure.
    pub fn repair_adjacency(&mut self) -> Result<RepairReport> {
        repair::repair_adjacency(&mut self.pager)
    }

    /// Reset the database to the same logical state as a brand-new file.
    ///
    /// This discards any uncommitted dirty pages, clears all extent metadata,
    /// truncates the backing store to the fixed header + WAL reservation, and
    /// resets the in-memory label index. After `clear()` the next allocated
    /// node or edge ID is 1 and a later `commit()` cannot resurrect pre-clear
    /// slot pages from the old dirty buffer.
    pub fn clear(&mut self) -> Result<()> {
        self.pager.clear_to_empty()?;
        self.label_index = LabelIndex::default();
        Ok(())
    }

    pub fn info(&self) -> DbInfo {
        DbInfo {
            version: "1.0".to_string(),
            node_count: self.pager.node_count(),
            edge_count: self.pager.edge_count(),
            file_size: self.pager.file_size(),
        }
    }
}

fn canonical_db_path(path: &str) -> Result<PathBuf> {
    let raw = Path::new(path);
    match std::fs::canonicalize(raw) {
        Ok(path) => Ok(path),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let parent = raw
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let name = raw.file_name().ok_or_else(|| {
                LielError::InvalidArgument(format!("database path has no file name: {path}"))
            })?;
            Ok(std::fs::canonicalize(parent)
                .map_err(LielError::Io)?
                .join(name))
        }
        Err(err) => Err(LielError::Io(err)),
    }
}

/// Compute the sibling path used by vacuum's copy-on-write rewrite.
///
/// For a database at `…/foo.liel` the tmp path is `…/foo.liel.tmp`; the
/// `.tmp` suffix is appended to the *full* canonical path so we never
/// strip a user-supplied extension.  Used both for `open()`'s stale-tmp
/// sweep and (later) by vacuum itself to construct the destination.
pub(crate) fn vacuum_tmp_path(canonical: &Path) -> PathBuf {
    let mut s = canonical.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

/// Unconditionally delete any sibling `<canonical>.tmp` left over from a
/// previous vacuum run that did not finish.  ENOENT is the success case
/// for a clean tree and is silently absorbed; every other I/O error
/// surfaces so the caller does not silently inherit a corrupted state.
fn remove_stale_vacuum_tmp(canonical: &Path) -> Result<()> {
    let tmp = vacuum_tmp_path(canonical);
    match std::fs::remove_file(&tmp) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(LielError::Io(err)),
    }
}

/// Release the in-process writer-guard slot when the handle is dropped.
///
/// Without this, a Python `with liel.open(p) as db:` block would re-open
/// the same path on the next iteration of a loop and trigger
/// [`LielError::AlreadyOpen`] even though the previous handle is gone.
impl Drop for GraphDB {
    fn drop(&mut self) {
        if let Some(key) = self.registry_key.take() {
            // `into_inner` on a poisoned registry is safe here for the same
            // reason as in `open()` above: the registry tracks only "is this
            // path open" and a stale entry can only over-report an
            // `AlreadyOpen` error, never under-report it.
            let mut guard = open_files()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.remove(&key);
        }
        let _ = self.writer_lock.take();
    }
}

// ── Integration tests: GraphDB orchestrates storage + graph + query layers ──
//
// Lower modules have their own #[cfg(test)] blocks; these tests assert that the
// public handle wires everything correctly end-to-end (including commit and,
// where applicable, persistence to a real file).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::Direction;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn empty_props() -> HashMap<String, PropValue> {
        HashMap::new()
    }

    #[test]
    fn graphdb_memory_crud_commit_and_counts() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec!["P".into()], empty_props()).unwrap();
        let b = db.add_node(vec!["P".into()], empty_props()).unwrap();
        db.add_edge(a.id, "E".into(), b.id, empty_props()).unwrap();
        assert_eq!(db.node_count(), 2);
        assert_eq!(db.edge_count(), 1);
        db.commit().unwrap();

        let got = db.get_node(a.id).unwrap().unwrap();
        assert_eq!(got.id, a.id);
        assert!(db.get_edge(1).unwrap().is_some());
    }

    #[test]
    fn graphdb_scan_nodes_and_edges() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let _ = db
            .add_node(vec!["Person".into(), "X".into()], empty_props())
            .unwrap();
        let _ = db.add_node(vec!["Company".into()], empty_props()).unwrap();
        db.add_edge(1, "KNOWS".into(), 2, empty_props()).unwrap();

        let persons = db.scan_nodes(&["Person".into()]).unwrap();
        assert_eq!(persons.len(), 1);
        let knows = db.scan_edges(&["KNOWS".into()]).unwrap();
        assert_eq!(knows.len(), 1);
        assert_eq!(knows[0].from, 1);
        assert_eq!(knows[0].to, 2);
    }

    #[test]
    fn graphdb_delete_node_cascades_incident_edges() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec![], empty_props()).unwrap();
        let b = db.add_node(vec![], empty_props()).unwrap();
        db.add_edge(a.id, "L".into(), b.id, empty_props()).unwrap();
        assert_eq!(db.edge_count(), 1);
        db.delete_node(a.id).unwrap();
        assert_eq!(db.edge_count(), 0);
        assert!(db.get_edge(1).unwrap().is_none());
        assert!(db.get_node(b.id).unwrap().is_some());
    }

    #[test]
    fn graphdb_delete_node_with_self_loop_deletes_edge_once() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec![], empty_props()).unwrap();
        let self_loop = db
            .add_edge(a.id, "SELF".into(), a.id, empty_props())
            .unwrap();

        db.delete_node(a.id).unwrap();

        assert!(db.get_node(a.id).unwrap().is_none());
        assert!(db.get_edge(self_loop.id).unwrap().is_none());
        assert_eq!(db.edge_count(), 0);
    }

    #[test]
    fn graphdb_merge_edge_same_props_is_idempotent() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec![], empty_props()).unwrap();
        let b = db.add_node(vec![], empty_props()).unwrap();
        let mut props = HashMap::new();
        props.insert("k".into(), PropValue::Int(1));
        let e1 = db
            .merge_edge(a.id, "M".into(), b.id, props.clone())
            .unwrap();
        let e2 = db.merge_edge(a.id, "M".into(), b.id, props).unwrap();
        assert_eq!(e1.id, e2.id);
        assert_eq!(db.edge_count(), 1);
    }

    #[test]
    fn graphdb_neighbors_and_shortest_path() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec![], empty_props()).unwrap();
        let b = db.add_node(vec![], empty_props()).unwrap();
        let c = db.add_node(vec![], empty_props()).unwrap();
        db.add_edge(a.id, "R".into(), b.id, empty_props()).unwrap();
        db.add_edge(b.id, "R".into(), c.id, empty_props()).unwrap();

        let nbrs = db.neighbors(a.id, Some("R"), Direction::Out).unwrap();
        assert_eq!(nbrs.len(), 1);
        assert_eq!(nbrs[0].id, b.id);

        let path = db.shortest_path(a.id, c.id, Some("R")).unwrap().unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].id, a.id);
        assert_eq!(path[2].id, c.id);
    }

    #[test]
    fn graphdb_clear_resets_counts() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let _ = db.add_node(vec![], empty_props()).unwrap();
        db.commit().unwrap();
        db.clear().unwrap();
        assert_eq!(db.node_count(), 0);
        assert_eq!(db.edge_count(), 0);
    }

    #[test]
    fn graphdb_clear_discards_old_dirty_pages_before_new_commit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("clear-resets-dirty-state.liel");
        let path_str = path.to_str().unwrap();

        {
            let mut db = GraphDB::open(path_str).unwrap();
            db.add_node(vec!["Old".into()], empty_props()).unwrap();
            db.clear().unwrap();
            let fresh = db.add_node(vec!["Fresh".into()], empty_props()).unwrap();
            assert_eq!(fresh.id, 1);
            db.commit().unwrap();
        }

        {
            let mut reopened = GraphDB::open(path_str).unwrap();
            assert_eq!(reopened.node_count(), 1);
            let fresh = reopened.get_node(1).unwrap().unwrap();
            assert_eq!(fresh.labels, vec!["Fresh"]);
            assert!(reopened.get_node(2).unwrap().is_none());
        }
    }

    #[test]
    fn graphdb_repair_adjacency_round_trips_after_manual_damage() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let a = db.add_node(vec!["A".into()], empty_props()).unwrap();
        let b = db.add_node(vec!["B".into()], empty_props()).unwrap();
        let e = db
            .add_edge(a.id, "REL".into(), b.id, empty_props())
            .unwrap();
        db.commit().unwrap();

        let mut a_slot = db.pager.read_node_slot(a.id).unwrap();
        a_slot.first_out_edge = 0;
        a_slot.out_degree = 0;
        db.pager.write_node_slot(&a_slot).unwrap();

        let mut edge_slot = db.pager.read_edge_slot(e.id).unwrap();
        edge_slot.next_out_edge = 42;
        db.pager.write_edge_slot(&edge_slot).unwrap();
        db.commit().unwrap();

        let report = db.repair_adjacency().unwrap();
        assert_eq!(report.nodes_rewritten, 2);
        assert_eq!(report.edges_relinked, 1);

        let repaired = db.out_edges(a.id, Some("REL")).unwrap();
        assert_eq!(repaired.len(), 1);
        assert_eq!(repaired[0].id, e.id);
    }

    #[test]
    fn graphdb_tempfile_roundtrip_survives_reopen() {
        // Use a path that does not exist yet so Pager treats the DB as new (writes header).
        // `NamedTempFile` pre-creates an empty file and would be mis-classified as existing.
        let dir = tempdir().unwrap();
        let path = dir.path().join("roundtrip.liel");
        let path_str = path.to_str().unwrap();

        {
            let mut db = GraphDB::open(path_str).unwrap();
            let mut props = HashMap::new();
            props.insert("name".into(), PropValue::String("Zed".into()));
            let n = db.add_node(vec!["P".into()], props).unwrap();
            assert_eq!(n.id, 1);
            db.commit().unwrap();
        }

        let mut db2 = GraphDB::open(path_str).unwrap();
        let n = db2.get_node(1).unwrap().unwrap();
        assert_eq!(
            n.properties.get("name"),
            Some(&PropValue::String("Zed".into()))
        );
    }

    #[test]
    fn graphdb_open_rejects_second_handle_to_same_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("guarded.liel");
        let path_str = path.to_str().unwrap();

        let _first = GraphDB::open(path_str).expect("first open should succeed");

        match GraphDB::open(path_str) {
            Err(LielError::AlreadyOpen(reported)) => {
                // The reported path should be the canonicalised form of the
                // input, which on Windows includes the `\\?\` UNC prefix.
                let canonical = std::fs::canonicalize(path_str).unwrap();
                assert_eq!(reported, canonical.display().to_string());
            }
            Err(other) => panic!("expected AlreadyOpen, got {other:?}"),
            Ok(_) => {
                panic!("expected AlreadyOpen, got Ok(GraphDB) — second open should be rejected")
            }
        }
    }

    #[test]
    fn graphdb_open_after_drop_is_allowed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("reopen.liel");
        let path_str = path.to_str().unwrap();

        {
            let _first = GraphDB::open(path_str).expect("first open should succeed");
        } // Dropped here; the in-process slot must be released.

        let _second = GraphDB::open(path_str).expect("second open after drop must succeed");
    }

    #[test]
    fn graphdb_open_memory_does_not_share_slot() {
        // ":memory:" handles are independent — opening twice must succeed
        // because each call yields its own in-process buffer.
        let _a = GraphDB::open(":memory:").unwrap();
        let _b = GraphDB::open(":memory:").unwrap();
    }

    /// `open()` must unconditionally remove any sibling `<path>.tmp` left
    /// behind by a vacuum that crashed mid-rewrite.  See
    /// product-tradeoffs §5.6.
    #[test]
    fn graphdb_open_sweeps_stale_vacuum_tmp() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("withtmp.liel");
        let tmp_path = {
            let mut s = db_path.as_os_str().to_owned();
            s.push(".tmp");
            std::path::PathBuf::from(s)
        };

        // Pre-create a fake "abandoned vacuum" tmp file with arbitrary
        // contents.  The DB itself does not exist yet, which mirrors the
        // worst case (vacuum died before its rename ever ran).
        std::fs::write(&tmp_path, b"leftover from a crashed vacuum").unwrap();
        assert!(tmp_path.exists());

        let _db = GraphDB::open(db_path.to_str().unwrap()).expect("open must succeed");
        assert!(
            !tmp_path.exists(),
            "stale .tmp must be removed at open() time"
        );
    }

    /// File-backed vacuum must (a) preserve every live record, (b) keep
    /// node and edge IDs stable across the rewrite (format-spec §7.2),
    /// and (c) leave no `.tmp` sibling on success.
    #[test]
    fn graphdb_vacuum_preserves_ids_and_data_on_disk() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vacuumed.liel");
        let path_str = db_path.to_str().unwrap();
        let tmp_path = {
            let mut s = db_path.as_os_str().to_owned();
            s.push(".tmp");
            std::path::PathBuf::from(s)
        };

        let (alice_id, bob_id, carol_id, edge_id);
        {
            let mut db = GraphDB::open(path_str).unwrap();
            let mut alice_props = HashMap::new();
            alice_props.insert("name".into(), PropValue::String("Alice".into()));
            let alice = db.add_node(vec!["Person".into()], alice_props).unwrap();
            let bob = db.add_node(vec!["Person".into()], HashMap::new()).unwrap();
            let mut carol_props = HashMap::new();
            carol_props.insert("name".into(), PropValue::String("Carol".into()));
            let carol = db.add_node(vec!["Person".into()], carol_props).unwrap();

            let mut eprops = HashMap::new();
            eprops.insert("since".into(), PropValue::Int(2020));
            let edge = db
                .add_edge(alice.id, "KNOWS".into(), bob.id, eprops)
                .unwrap();

            db.commit().unwrap();
            // Delete carol so vacuum has dead bytes to reclaim.
            db.delete_node(carol.id).unwrap();
            db.commit().unwrap();

            db.vacuum().unwrap();
            assert!(
                !tmp_path.exists(),
                "successful vacuum must not leave a .tmp behind"
            );

            alice_id = alice.id;
            bob_id = bob.id;
            carol_id = carol.id;
            edge_id = edge.id;
        }

        // Reopen from disk: IDs survive, deleted node is gone, alive
        // records still report their properties.
        let mut db2 = GraphDB::open(path_str).unwrap();
        let alice = db2
            .get_node(alice_id)
            .unwrap()
            .expect("alice must survive vacuum");
        assert_eq!(
            alice.properties.get("name"),
            Some(&PropValue::String("Alice".into()))
        );
        assert!(db2.get_node(bob_id).unwrap().is_some());
        assert!(
            db2.get_node(carol_id).unwrap().is_none(),
            "carol was deleted before vacuum"
        );

        let edge = db2
            .get_edge(edge_id)
            .unwrap()
            .expect("edge survives vacuum");
        assert_eq!(edge.from, alice_id);
        assert_eq!(edge.to, bob_id);
        assert_eq!(edge.properties.get("since"), Some(&PropValue::Int(2020)));
    }

    /// A successful vacuum must not interfere with continued use of the
    /// `GraphDB` handle: post-vacuum, new add_node calls allocate IDs
    /// continuing from the pre-vacuum counter (proves `next_node_id`
    /// was preserved on the fresh file).
    #[test]
    fn graphdb_vacuum_keeps_id_counter_monotonic() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("monotonic.liel");
        let path_str = db_path.to_str().unwrap();

        let mut db = GraphDB::open(path_str).unwrap();
        let _a = db.add_node(vec![], HashMap::new()).unwrap(); // id=1
        let b = db.add_node(vec![], HashMap::new()).unwrap(); // id=2
        let _c = db.add_node(vec![], HashMap::new()).unwrap(); // id=3
        db.commit().unwrap();
        db.delete_node(b.id).unwrap();
        db.commit().unwrap();

        db.vacuum().unwrap();

        // The next allocation must be id=4, not id=2 — IDs never roll back.
        let next = db.add_node(vec![], HashMap::new()).unwrap();
        assert_eq!(next.id, 4);
    }

    /// `transaction()` returns a guard that commits on `commit()`.
    /// Nodes added inside the guard's scope must be visible after the
    /// commit lands.  This test also exercises the `Deref`/`DerefMut`
    /// ergonomic path: `tx.add_node(...)` reaches `GraphDB::add_node`
    /// directly without the caller needing to know about the inner
    /// `db` field.
    #[test]
    fn graphdb_transaction_commit_persists_changes() {
        let mut db = GraphDB::open(":memory:").unwrap();
        {
            let mut tx = db.transaction().unwrap();
            tx.add_node(vec!["X".into()], HashMap::new()).unwrap();
            tx.commit().unwrap();
        }
        // The flag is cleared, so a second transaction starts fresh.
        assert!(!db.is_transaction_active());
        assert_eq!(db.node_count(), 1);
    }

    /// Dropping the guard without calling `commit()` rolls back.  The
    /// node added inside the dropped block must be invisible after the
    /// scope exits.
    #[test]
    fn graphdb_transaction_drop_without_commit_rolls_back() {
        let mut db = GraphDB::open(":memory:").unwrap();
        // Pre-commit a baseline so rollback has somewhere to land.
        let _ = db.add_node(vec!["Base".into()], HashMap::new()).unwrap();
        db.commit().unwrap();
        assert_eq!(db.node_count(), 1);

        {
            let mut tx = db.transaction().unwrap();
            tx.add_node(vec!["Inner".into()], HashMap::new()).unwrap();
            // Intentionally do NOT call tx.commit(); guard's Drop rolls back.
        }
        assert!(!db.is_transaction_active(), "Drop must clear the flag");
        assert_eq!(
            db.node_count(),
            1,
            "uncommitted work must be rolled back when the guard is dropped"
        );
    }

    /// Deref/DerefMut on the guard let callers issue normal `GraphDB`
    /// operations through the guard without reaching into private fields,
    /// which is the whole point of returning a guard from `transaction()`.
    /// Confirms the API is actually usable from external Rust code, not
    /// just from inside this crate's tests.
    #[test]
    fn graphdb_transaction_guard_supports_deref_to_graph_db() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let edge_id;
        {
            let mut tx = db.transaction().unwrap();
            // Reach the public `GraphDB` API straight through the guard.
            let a = tx.add_node(vec!["A".into()], HashMap::new()).unwrap();
            let b = tx.add_node(vec!["B".into()], HashMap::new()).unwrap();
            let e = tx.add_edge(a.id, "L".into(), b.id, HashMap::new()).unwrap();
            edge_id = e.id;
            // Read APIs work too — Deref (immutable) is exercised here.
            assert_eq!(tx.node_count(), 2);
            tx.commit().unwrap();
        }
        // After commit the data is visible from the underlying handle.
        assert!(db.get_edge(edge_id).unwrap().is_some());
    }

    /// `vacuum()` must refuse to run inside an explicit transaction.
    /// The internal forced commit would otherwise silently flush
    /// whatever the caller staged — the opposite of what an explicit
    /// transaction was meant to provide.
    #[test]
    fn graphdb_vacuum_in_explicit_transaction_returns_transaction_error() {
        let mut db = GraphDB::open(":memory:").unwrap();
        // Use the FFI-style flag setter so we can keep `&mut db` available
        // for the vacuum call; the borrow-checked `transaction()` would
        // also work but reads less directly.
        db.begin_explicit_transaction().unwrap();
        match db.vacuum() {
            Err(LielError::TransactionError(msg)) => {
                assert!(
                    msg.contains("explicit transaction"),
                    "expected message about explicit transaction, got {msg:?}"
                );
            }
            other => panic!("expected TransactionError, got {other:?}"),
        }
        // The flag is unaffected by the rejected vacuum so the caller
        // can still rollback or commit explicitly afterwards.
        assert!(db.is_transaction_active());
    }

    /// Calling `transaction()` while another is active must error with
    /// `TransactionError("transaction already active")`.  Confirms the
    /// no-nesting rule from product-tradeoffs §5.5.
    #[test]
    fn graphdb_transaction_nested_call_returns_transaction_error() {
        let mut db = GraphDB::open(":memory:").unwrap();
        let _outer = db.transaction().unwrap();
        // While `_outer` is alive, `db` is mutably borrowed and a second
        // `db.transaction()` call would not even compile in safe Rust.
        // Drop the outer guard explicitly so we can take the borrow back
        // and verify the FFI-style entry point (`begin_explicit_transaction`)
        // also enforces the rule.
        drop(_outer);

        // Now exercise the FFI-style flag without the borrow checker:
        db.begin_explicit_transaction().unwrap();
        match db.begin_explicit_transaction() {
            Err(LielError::TransactionError(msg)) => {
                assert!(
                    msg.contains("already active"),
                    "expected nesting message, got {msg:?}"
                );
            }
            other => panic!("expected TransactionError, got {other:?}"),
        }
    }
}

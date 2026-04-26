use std::fmt;

/// The primary error type for all liel operations.
///
/// `LielError` is returned by every fallible operation in the liel crate through
/// the [`Result`] type alias.  The variants cover the full range of failure modes:
/// OS-level I/O problems, logical "not found" conditions, file-format corruption,
/// bad caller arguments, and transaction-layer faults.
///
/// # Design rationale
/// A single enum rather than multiple separate error types keeps the public API
/// simple.  Callers that only care about "did it work?" propagate with `?`.
/// Callers that need to distinguish "node missing" from "disk full" can match on
/// individual variants.
///
/// This type intentionally has no external dependencies — `Display`, `Error`, and
/// `From<io::Error>` are all implemented by hand so that the `liel-core` library
/// can remain dependency-free.
#[derive(Debug)]
pub enum LielError {
    /// Wraps any [`std::io::Error`] surfaced during file reads, writes, seeks,
    /// flushes, or truncations performed by the storage layer.
    ///
    /// The `From<io::Error>` implementation means that `io_err?` inside a function
    /// returning `Result<_, LielError>` automatically converts — no manual
    /// `map_err` required.
    Io(std::io::Error),

    /// The requested node ID does not exist, has never been allocated, or has
    /// already been deleted (its `FLAG_DELETED` bit is set in the slot).
    ///
    /// The `u64` payload is the node ID that was looked up, which makes error
    /// messages self-describing without additional context.
    ///
    /// Note: `NodeId(0)` is the NULL sentinel and is never a valid user-facing ID,
    /// so lookups of 0 also produce this variant.
    NodeNotFound(u64),

    /// The requested edge ID does not exist, has never been allocated, or has
    /// already been deleted.
    ///
    /// Analogous to [`NodeNotFound`] — the `u64` payload is the edge ID that
    /// was looked up.  `EdgeId(0)` is likewise the NULL sentinel.
    EdgeNotFound(u64),

    /// The on-disk file contains data that violates the expected format invariants.
    ///
    /// Examples of situations that produce this variant:
    /// - The magic bytes at the start of the file do not match `"LIEL\0..."`.
    /// - A version field contains an unrecognised major version number.
    /// - An edge label slot contains bytes that decode to a non-String `PropValue`.
    /// - A property type-tag byte is not one of the seven defined codes.
    ///
    /// The `String` payload should contain a human-readable description of exactly
    /// which invariant was violated, to aid debugging.
    CorruptedFile(String),

    /// The caller passed logically invalid arguments to an API function.
    ///
    /// This is distinct from `NodeNotFound` / `EdgeNotFound`: those say "that ID
    /// does not exist in the database"; `InvalidArgument` says "the arguments make
    /// no sense regardless of database state" (e.g., a negative depth, an empty
    /// label string where one is required, or a mismatched from/to pair).
    ///
    /// The `String` payload describes what was wrong with the argument.
    InvalidArgument(String),

    /// An error arising from the transaction / WAL layer.
    ///
    /// Produced when the WAL is in an inconsistent state, when a commit or rollback
    /// is attempted outside of an active transaction, or when WAL replay during
    /// `open()` detects a partial write that cannot be recovered.
    ///
    /// The `String` payload gives details about the transaction failure.
    TransactionError(String),

    /// An allocation request would exceed the addressable capacity of the
    /// current on-disk layout.
    ///
    /// Emitted by [`crate::storage::pager::Pager::alloc_node_id`] and
    /// [`crate::storage::pager::Pager::alloc_edge_id`] when the monotonically
    /// increasing ID counter would grow past the number of slots the file
    /// layout can physically address.
    ///
    /// This variant exists purely as a safety net: it guarantees that a write
    /// past the end of an allocated region fails loudly rather than silently
    /// corrupting adjacent regions.  The caller sees a clean, typed error and
    /// nothing is written to disk for the failed call.
    ///
    /// - `kind` is a short descriptor, currently `"node"` or `"edge"`, so
    ///   callers can distinguish the two without parsing a message.
    /// - `limit` is the maximum addressable capacity for that resource kind.
    /// - `unit` describes what `limit` is measured in (for example `nodes`,
    ///   `edges`, or `bytes of prop storage`).
    CapacityExceeded {
        kind: &'static str,
        limit: u64,
        unit: &'static str,
    },

    /// A source node encountered during `merge_from` with
    /// `NodeIdentity::ByProperty` does not carry one of the keys requested for
    /// the identity check.
    ///
    /// Merging cannot decide whether the node matches an existing destination
    /// node without every requested key, so the operation fails fast rather
    /// than silently falling back to a "create new" branch.  The `node_id` is
    /// the ID in the *source* database; `key` is the missing property name.
    MergeKeyNotFound { node_id: u64, key: String },

    /// The current transaction would produce more WAL bytes than the reserved
    /// WAL section can hold.
    ///
    /// The WAL section is a fixed 4 MiB region (see `WAL_RESERVED` in
    /// `storage::pager`).  A single commit can cover at most `WAL_RESERVED /
    /// WAL_WRITE_ENTRY_SIZE` dirty pages — roughly 1000 pages, i.e. tens of
    /// thousands of node or edge updates per transaction.  Exceeding that
    /// would overwrite the node/edge data region, so liel refuses the commit
    /// and leaves the in-memory dirty pages untouched.  The caller can split
    /// the work across several commits and retry.
    ///
    /// The `String` payload gives the observed size vs the limit in bytes.
    WalOverflow(String),

    /// Another live writer is already attached to this database file.
    ///
    /// liel guarantees crash-safety only with **a single writer per file**.
    /// Opening the same `.liel` file twice would let two independent `Pager`
    /// instances issue WAL commits against the same header, silently
    /// clobbering one another. liel rejects the second `open()` outright,
    /// whether the conflict is detected by the in-process registry or by the
    /// cross-process lock directory.
    ///
    /// The `String` payload is the canonicalised filesystem path, optionally
    /// with lock-directory details.
    AlreadyOpen(String),
}

/// Human-readable explanation shared by Rust `Display` and the Python bindings.
pub fn format_liel_error(err: &LielError) -> String {
    match err {
        LielError::Io(e) => format!(
            "Could not read or write the database file (check the path, permissions, and free disk space): {e}"
        ),
        LielError::NodeNotFound(id) => format!(
            "No node with id {id}. Use a positive integer id returned from add_node() or an existing Node.id; id 0 is never valid."
        ),
        LielError::EdgeNotFound(id) => format!(
            "No edge with id {id}. Use a positive integer id returned from add_edge()/merge_edge() or an existing Edge.id; id 0 is never valid."
        ),
        LielError::CorruptedFile(msg) => format!(
            "The database file failed an internal integrity check and may be damaged: {msg}"
        ),
        LielError::InvalidArgument(msg) => format!("Invalid request: {msg}"),
        LielError::TransactionError(msg) => format!("Transaction could not be completed: {msg}"),
        LielError::CapacityExceeded { kind, limit, unit } => format!(
            "{kind} capacity exceeded: the current file layout can address at most {limit} {unit}"
        ),
        LielError::MergeKeyNotFound { node_id, key } => format!(
            "Merge key not found: source node {node_id} has no property '{key}' required by NodeIdentity::ByProperty"
        ),
        LielError::WalOverflow(msg) => format!(
            "Transaction is too large to fit in the 4 MiB WAL reservation; split it across multiple commits: {msg}"
        ),
        LielError::AlreadyOpen(path) => format!(
            "liel: database file is already open for writing: {path}. \
             liel allows only a single writer per file; close the existing \
             GraphDB handle, wait for the other writer process to exit, or retry \
             after stale lock recovery has completed."
        ),
    }
}

impl fmt::Display for LielError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_liel_error(self))
    }
}

impl std::error::Error for LielError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // Expose the underlying io::Error so callers can inspect OS error codes.
            LielError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Automatic conversion from `std::io::Error` into `LielError::Io`.
///
/// This lets the `?` operator work transparently inside any function that
/// returns `Result<_, LielError>` when calling standard library I/O functions.
impl From<std::io::Error> for LielError {
    fn from(e: std::io::Error) -> Self {
        LielError::Io(e)
    }
}

/// Convenience alias so every module can write `Result<T>` instead of
/// `std::result::Result<T, LielError>`.
pub type Result<T> = std::result::Result<T, LielError>;

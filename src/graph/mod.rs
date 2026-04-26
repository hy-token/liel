/// Graph layer — node/edge CRUD, adjacency lists, traversal, and maintenance.
///
/// This module hierarchy implements the logical graph model that sits on top of
/// the raw storage layer (`crate::storage`).  Each sub-module has a single,
/// well-defined responsibility:
///
/// | Sub-module  | Responsibility |
/// |-------------|----------------|
/// | `node`      | `Node` type and create/read/update/delete operations |
/// | `edge`      | `Edge` type, create/read/update/delete, adjacency-list management |
/// | `merge`     | Cross-database merge: combine two `GraphDB` instances into one |
/// | `repair`    | Adjacency repair and future graph-integrity maintenance helpers |
/// | `traverse`  | BFS, DFS, and shortest-path algorithms |
/// | `vacuum`    | Property-area compaction (reclaims space from deleted records) |
///
/// All public items in these sub-modules operate directly on a [`Pager`]
/// (`crate::storage::pager::Pager`), which provides page-level I/O and the WAL.
/// The Python bindings (`crate::python`) wrap these functions behind an
/// `Arc<Mutex<GraphDBInner>>` to satisfy PyO3's `Send + Sync` requirements.
pub mod edge;
pub(crate) mod fault_inject;
pub mod index;
pub mod merge;
pub mod node;
pub mod repair;
pub mod traverse;
pub mod vacuum;

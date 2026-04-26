/// Query module for liel.
///
/// This module exposes the Rust-side query builder types and the convenience
/// free functions [`nodes`] and [`edges`] that create new builder instances
/// bound to a given [`crate::storage::pager::Pager`].  The Python-facing query
/// types (`PyNodeQuery`, `PyEdgeQuery`) live in `crate::python::types` and
/// wrap these builders in an `Arc<Mutex<Pager>>` so they can be used safely
/// from PyO3 callbacks.
pub mod builder;

pub use builder::{edges, nodes, EdgeQueryBuilder, QueryBuilder};

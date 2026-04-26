use crate::error::Result;
use crate::graph::edge::Edge;
use crate::graph::node::Node;
use crate::storage::pager::Pager;

type NodePredicate = Box<dyn Fn(&Node) -> bool + 'static>;
type EdgePredicate = Box<dyn Fn(&Edge) -> bool + 'static>;

/// Builder for node queries against the graph database.
///
/// `QueryBuilder` implements a lazy, composable pipeline for scanning nodes
/// stored in the pager.  Filters are accumulated through a series of chainable
/// method calls and are not evaluated until the terminal methods [`fetch`],
/// [`count`], or [`exists`] are called — at that point the full scan runs once
/// and all accumulated predicates are applied in a single pass.
///
/// # Evaluation order
///
/// 1. **Label filter** — if any labels have been supplied via [`label`], only
///    nodes that carry at least one of those labels pass through.
/// 2. **Predicate** — if a closure has been registered via [`where_fn`], it is
///    called for every label-passing node and nodes that return `false` are
///    dropped.
/// 3. **Skip** — the first `skip` surviving nodes are discarded.
/// 4. **Limit** — iteration stops as soon as `limit` nodes have been collected.
///
/// # Example (Rust)
///
/// ```rust,ignore
/// let results = nodes(&mut pager)
///     .label("Person")
///     .where_fn(|n| n.properties.get("age").map_or(false, |v| v > &PropValue::Int(18)))
///     .skip(0)
///     .limit(10)
///     .fetch()?;
/// ```
///
/// [`fetch`]: QueryBuilder::fetch
/// [`count`]: QueryBuilder::count
/// [`exists`]: QueryBuilder::exists
/// [`label`]: QueryBuilder::label
/// [`where_fn`]: QueryBuilder::where_fn
pub struct QueryBuilder<'a> {
    pager: &'a mut Pager,
    /// Labels that a node must possess (any-match, not all-match).
    label_filters: Vec<String>,
    /// Optional Rust closure evaluated for each label-passing node.
    predicate: Option<NodePredicate>,
    /// Number of matching nodes to skip before collecting results.
    skip: usize,
    /// Maximum number of nodes to return.  `None` means no limit.
    limit: Option<usize>,
}

/// Builder for edge queries against the graph database.
///
/// `EdgeQueryBuilder` mirrors [`QueryBuilder`] but operates over edges instead
/// of nodes.  Filters are accumulated through chainable method calls and are
/// not applied until a terminal method is invoked.
///
/// # Evaluation order
///
/// 1. **Label filter** — if any labels have been supplied via [`label`], only
///    edges whose `label` field equals one of the supplied strings pass through.
/// 2. **Predicate** — if a closure has been registered via [`where_fn`], it is
///    called for every label-passing edge; edges that return `false` are dropped.
/// 3. **Skip** — the first `skip` surviving edges are discarded.
/// 4. **Limit** — iteration stops as soon as `limit` edges have been collected.
///
/// # Example (Rust)
///
/// ```rust,ignore
/// let results = edges(&mut pager)
///     .label("KNOWS")
///     .where_fn(|e| e.properties.get("since").map_or(false, |v| v >= &PropValue::Int(2020)))
///     .fetch()?;
/// ```
///
/// [`label`]: EdgeQueryBuilder::label
/// [`where_fn`]: EdgeQueryBuilder::where_fn
pub struct EdgeQueryBuilder<'a> {
    pager: &'a mut Pager,
    /// Labels that an edge must match (any-match).
    label_filters: Vec<String>,
    /// Optional Rust closure evaluated for each label-passing edge.
    predicate: Option<EdgePredicate>,
    /// Number of matching edges to skip before collecting results.
    skip: usize,
    /// Maximum number of edges to return.  `None` means no limit.
    limit: Option<usize>,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new `QueryBuilder` backed by `pager` with no filters applied.
    ///
    /// The builder takes a mutable reference to the pager so that it can call
    /// [`get_node`](crate::graph::node::get_node) during the scan phase.
    pub fn new(pager: &'a mut Pager) -> Self {
        Self {
            pager,
            label_filters: Vec::new(),
            predicate: None,
            skip: 0,
            limit: None,
        }
    }

    /// Restrict results to nodes that carry the given label.
    ///
    /// Multiple calls are combined with **OR** semantics: a node passes if it
    /// has *any* of the specified labels.  If this method is never called, the
    /// label filter is disabled and all nodes are considered.
    pub fn label(mut self, label: &str) -> Self {
        self.label_filters.push(label.to_string());
        self
    }

    /// Register a Rust predicate closure that is evaluated per node.
    ///
    /// The closure receives a reference to a [`Node`] and must return `true` for
    /// nodes that should be included in the results.  Any node for which the
    /// closure returns `false` is excluded.
    ///
    /// Only one predicate can be active at a time; calling this method a second
    /// time replaces the previous predicate.
    pub fn where_fn(mut self, f: impl Fn(&Node) -> bool + 'static) -> Self {
        self.predicate = Some(Box::new(f));
        self
    }

    /// Skip the first `n` nodes that pass all other filters.
    ///
    /// Combined with [`limit`](QueryBuilder::limit) this enables page-based
    /// result sets.  A skip of `0` (the default) means no nodes are skipped.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = n;
        self
    }

    /// Cap the number of nodes returned at `n`.
    ///
    /// Iteration over the node space terminates early once `n` nodes have been
    /// collected, so this is an efficient operation on large graphs.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the query and return all matching nodes.
    ///
    /// Internally delegates to the private [`collect`](QueryBuilder::collect)
    /// method which performs the full scan.
    ///
    /// # Errors
    ///
    /// Returns a [`LielError`](crate::error::LielError) if reading a node slot
    /// from the pager fails (e.g. I/O error or corrupted page data).
    pub fn fetch(self) -> Result<Vec<Node>> {
        self.collect()
    }

    /// Execute the query and return the number of matching nodes.
    ///
    /// Equivalent to `fetch()?.len()` but communicates intent more clearly
    /// when the caller only needs the count and not the actual nodes.
    ///
    /// # Errors
    ///
    /// Propagates any pager I/O error from the underlying scan.
    pub fn count(self) -> Result<usize> {
        Ok(self.collect()?.len())
    }

    /// Execute the query and return `true` if at least one node matches.
    ///
    /// Short-circuits after the first surviving node: the scan stops as soon
    /// as one node passes all filters (label → predicate → skip), so the cost
    /// is bounded by the position of the first match, not the size of the
    /// graph.  Any previously set [`limit`](QueryBuilder::limit) is overridden
    /// because a smaller cap cannot make existence more conservative.
    ///
    /// # Errors
    ///
    /// Propagates any pager I/O error from the underlying scan.
    pub fn exists(mut self) -> Result<bool> {
        self.limit = Some(1);
        Ok(!self.collect()?.is_empty())
    }

    /// Internal: perform the full node scan and apply all accumulated filters.
    ///
    /// Iterates node IDs from `1` to `max_node_id` (inclusive), reading each
    /// live node from the pager.  Deleted slots return `None` from `get_node`
    /// and are silently skipped.
    ///
    /// Filter application order: label → predicate → skip → limit.
    fn collect(self) -> Result<Vec<Node>> {
        let max_id = self.pager.max_node_id();
        let mut results = Vec::new();
        let mut skipped = 0usize;

        for node_id in 1..=max_id {
            if let Some(node) = crate::graph::node::get_node(self.pager, node_id)? {
                // Apply label filter: the node must have at least one label
                // that appears in the label_filters list.  If no labels were
                // registered the filter is bypassed entirely.
                if !self.label_filters.is_empty()
                    && !self
                        .label_filters
                        .iter()
                        .any(|label| node.labels.contains(label))
                {
                    continue;
                }

                // Apply the optional Rust predicate closure.
                if let Some(ref pred) = self.predicate {
                    if !pred(&node) {
                        continue;
                    }
                }

                // Honor the skip offset: discard the first `self.skip` survivors.
                if skipped < self.skip {
                    skipped += 1;
                    continue;
                }

                results.push(node);

                // Honor the limit: stop early once enough results are collected.
                if self.limit.is_some_and(|limit| results.len() >= limit) {
                    break;
                }
            }
        }

        Ok(results)
    }
}

impl<'a> EdgeQueryBuilder<'a> {
    /// Create a new `EdgeQueryBuilder` backed by `pager` with no filters applied.
    ///
    /// The builder takes a mutable reference to the pager so that it can call
    /// [`get_edge`](crate::graph::edge::get_edge) during the scan phase.
    pub fn new(pager: &'a mut Pager) -> Self {
        Self {
            pager,
            label_filters: Vec::new(),
            predicate: None,
            skip: 0,
            limit: None,
        }
    }

    /// Restrict results to edges whose `label` field matches the given string.
    ///
    /// Multiple calls are combined with **OR** semantics: an edge passes if its
    /// label equals *any* of the supplied strings.  If this method is never
    /// called, the label filter is disabled and all edges are considered.
    pub fn label(mut self, label: &str) -> Self {
        self.label_filters.push(label.to_string());
        self
    }

    /// Register a Rust predicate closure that is evaluated per edge.
    ///
    /// The closure receives a reference to an [`Edge`] and must return `true`
    /// for edges that should be included in the results.
    ///
    /// Only one predicate can be active at a time; a second call replaces the
    /// previous predicate.
    pub fn where_fn(mut self, f: impl Fn(&Edge) -> bool + 'static) -> Self {
        self.predicate = Some(Box::new(f));
        self
    }

    /// Skip the first `n` edges that pass all other filters.
    ///
    /// A skip of `0` (the default) means no edges are skipped.
    pub fn skip(mut self, n: usize) -> Self {
        self.skip = n;
        self
    }

    /// Cap the number of edges returned at `n`.
    ///
    /// Iteration terminates early once `n` edges have been collected.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Execute the query and return all matching edges.
    ///
    /// Internally delegates to the private [`collect`](EdgeQueryBuilder::collect)
    /// method which performs the full scan.
    ///
    /// # Errors
    ///
    /// Returns a [`LielError`](crate::error::LielError) if reading an edge slot
    /// from the pager fails.
    pub fn fetch(self) -> Result<Vec<Edge>> {
        self.collect()
    }

    /// Execute the query and return the number of matching edges.
    ///
    /// # Errors
    ///
    /// Propagates any pager I/O error from the underlying scan.
    pub fn count(self) -> Result<usize> {
        Ok(self.collect()?.len())
    }

    /// Execute the query and return `true` if at least one edge matches.
    ///
    /// Short-circuits after the first surviving edge (same semantics as
    /// [`QueryBuilder::exists`]): any previously set
    /// [`limit`](EdgeQueryBuilder::limit) is overridden to 1 because a smaller
    /// cap cannot make existence more conservative.
    ///
    /// # Errors
    ///
    /// Propagates any pager I/O error from the underlying scan.
    pub fn exists(mut self) -> Result<bool> {
        self.limit = Some(1);
        Ok(!self.collect()?.is_empty())
    }

    /// Internal: perform the full edge scan and apply all accumulated filters.
    ///
    /// Iterates edge IDs from `1` to `max_edge_id` (inclusive), reading each
    /// live edge from the pager.  Deleted slots return `None` from `get_edge`
    /// and are silently skipped.
    ///
    /// Filter application order: label → predicate → skip → limit.
    fn collect(self) -> Result<Vec<Edge>> {
        let max_id = self.pager.max_edge_id();
        let mut results = Vec::new();
        let mut skipped = 0usize;

        for edge_id in 1..=max_id {
            if let Some(edge) = crate::graph::edge::get_edge(self.pager, edge_id)? {
                // Apply label filter: the edge label must equal at least one of
                // the registered filter strings.
                if !self.label_filters.is_empty() && !self.label_filters.contains(&edge.label) {
                    continue;
                }

                // Apply the optional Rust predicate closure.
                if let Some(ref pred) = self.predicate {
                    if !pred(&edge) {
                        continue;
                    }
                }

                // Honor the skip offset.
                if skipped < self.skip {
                    skipped += 1;
                    continue;
                }

                results.push(edge);

                // Honor the limit: stop early once enough results are collected.
                if self.limit.is_some_and(|limit| results.len() >= limit) {
                    break;
                }
            }
        }

        Ok(results)
    }
}

/// Create a [`QueryBuilder`] that scans all nodes in `pager`.
///
/// This is the canonical entry point for building node queries in Rust code.
/// The Python API exposes this indirectly via [`PyGraphDB::nodes`].
///
/// # Example
///
/// ```rust,ignore
/// let adults = nodes(&mut pager).label("Person").fetch()?;
/// ```
pub fn nodes(pager: &mut Pager) -> QueryBuilder<'_> {
    QueryBuilder::new(pager)
}

/// Create an [`EdgeQueryBuilder`] that scans all edges in `pager`.
///
/// This is the canonical entry point for building edge queries in Rust code.
/// The Python API exposes this indirectly via [`PyGraphDB::edges`].
///
/// # Example
///
/// ```rust,ignore
/// let knows_edges = edges(&mut pager).label("KNOWS").fetch()?;
/// ```
pub fn edges(pager: &mut Pager) -> EdgeQueryBuilder<'_> {
    EdgeQueryBuilder::new(pager)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{edges, nodes};
    use crate::graph::edge::add_edge;
    use crate::graph::node::add_node;
    use crate::storage::pager::Pager;
    use crate::storage::prop_codec::PropValue;

    #[test]
    fn node_query_supports_skip_limit_count_and_exists() {
        let mut pager = Pager::open(":memory:").unwrap();

        let mut alice_props = HashMap::new();
        alice_props.insert("name".into(), PropValue::String("Alice".into()));
        alice_props.insert("age".into(), PropValue::Int(30));
        add_node(&mut pager, vec!["Person".into()], alice_props).unwrap();

        let mut bob_props = HashMap::new();
        bob_props.insert("name".into(), PropValue::String("Bob".into()));
        bob_props.insert("age".into(), PropValue::Int(19));
        add_node(&mut pager, vec!["Person".into()], bob_props).unwrap();

        let mut carol_props = HashMap::new();
        carol_props.insert("name".into(), PropValue::String("Carol".into()));
        carol_props.insert("age".into(), PropValue::Int(42));
        add_node(&mut pager, vec!["Person".into()], carol_props).unwrap();

        let results = nodes(&mut pager)
            .label("Person")
            .where_fn(|node| matches!(node.properties.get("age"), Some(PropValue::Int(age)) if *age >= 20))
            .skip(1)
            .limit(1)
            .fetch()
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].properties.get("name"),
            Some(&PropValue::String("Carol".into()))
        );

        let count = nodes(&mut pager)
            .label("Person")
            .where_fn(|node| matches!(node.properties.get("age"), Some(PropValue::Int(age)) if *age >= 20))
            .count()
            .unwrap();
        assert_eq!(count, 2);

        let exists = nodes(&mut pager)
            .label("Person")
            .where_fn(|node| matches!(node.properties.get("name"), Some(PropValue::String(name)) if name == "Alice"))
            .exists()
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn edge_query_filters_labels_and_properties() {
        let mut pager = Pager::open(":memory:").unwrap();
        let a = add_node(&mut pager, vec!["Person".into()], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec!["Person".into()], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec!["Person".into()], HashMap::new()).unwrap();

        let mut knows_props = HashMap::new();
        knows_props.insert("since".into(), PropValue::Int(2022));
        add_edge(&mut pager, a.id, "KNOWS".into(), b.id, knows_props).unwrap();

        let mut likes_props = HashMap::new();
        likes_props.insert("since".into(), PropValue::Int(2018));
        add_edge(&mut pager, a.id, "LIKES".into(), c.id, likes_props).unwrap();

        let mut recent_props = HashMap::new();
        recent_props.insert("since".into(), PropValue::Int(2024));
        add_edge(&mut pager, b.id, "KNOWS".into(), c.id, recent_props).unwrap();

        let results = edges(&mut pager)
            .label("KNOWS")
            .where_fn(|edge| matches!(edge.properties.get("since"), Some(PropValue::Int(year)) if *year >= 2020))
            .fetch()
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|edge| edge.label == "KNOWS"));

        let count = edges(&mut pager).label("LIKES").count().unwrap();
        assert_eq!(count, 1);

        let exists = edges(&mut pager)
            .label("KNOWS")
            .skip(1)
            .limit(1)
            .exists()
            .unwrap();
        assert!(exists);
    }

    #[test]
    fn exists_short_circuits_after_first_match() {
        use std::cell::Cell;
        use std::rc::Rc;

        let mut pager = Pager::open(":memory:").unwrap();
        for _ in 0..10 {
            add_node(&mut pager, vec!["Person".into()], HashMap::new()).unwrap();
        }

        // Counter captured by the predicate closure: when `exists()` short-
        // circuits after the first surviving node the closure must be invoked
        // at most once.  Without the short-circuit the old implementation
        // invoked it for every one of the 10 label matches.
        let calls = Rc::new(Cell::new(0usize));
        let calls_for_closure = Rc::clone(&calls);
        let found = nodes(&mut pager)
            .label("Person")
            .where_fn(move |_| {
                calls_for_closure.set(calls_for_closure.get() + 1);
                true
            })
            .exists()
            .unwrap();

        assert!(found);
        assert_eq!(
            calls.get(),
            1,
            "exists() must stop scanning after the first surviving node"
        );
    }

    #[test]
    fn edge_exists_short_circuits_after_first_match() {
        use std::cell::Cell;
        use std::rc::Rc;

        let mut pager = Pager::open(":memory:").unwrap();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        for _ in 0..5 {
            add_edge(&mut pager, a.id, "R".into(), b.id, HashMap::new()).unwrap();
        }

        let calls = Rc::new(Cell::new(0usize));
        let calls_for_closure = Rc::clone(&calls);
        let found = edges(&mut pager)
            .label("R")
            .where_fn(move |_| {
                calls_for_closure.set(calls_for_closure.get() + 1);
                true
            })
            .exists()
            .unwrap();

        assert!(found);
        assert_eq!(
            calls.get(),
            1,
            "EdgeQueryBuilder::exists() must stop scanning after the first match"
        );
    }
}

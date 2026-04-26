//! Cross-database merge: combine two `GraphDB` instances into one.
//!
//! `merge_from` walks every live node and edge in a source database, remaps
//! their IDs, and writes them into a destination database using the existing
//! high-level [`add_node`](crate::graph::node::add_node),
//! [`add_edge`](crate::graph::edge::add_edge), and
//! [`merge_edge`](crate::graph::edge::merge_edge) paths.  The on-disk file
//! format is **unchanged** — merging is a pure in-API operation built on top
//! of the existing storage primitives.
//!
//! [`LabelIndex`](crate::graph::index::LabelIndex) in `index.rs` **is
//! implemented** for label-filtered scans (`scan_nodes`, `QueryBuilder`).
//! The slow path is [`NodeIdentity::ByProperty`], which linearly scans **every
//! live node in the destination** for each source node — not a “placeholder”
//! index. See **Scope (Phase 1)** below.
//!
//! # Why IDs must be remapped
//! liel issues `NodeId` and `EdgeId` values from a per-file u64 counter
//! starting at 1, and never reuses a slot.  Two independent `.liel` files can
//! both contain `NodeId(5)` referring to completely different entities, so a
//! raw byte-level merge is impossible.  `merge_from` therefore assigns each
//! source node a brand new destination ID (or reuses an existing one under
//! `NodeIdentity::ByProperty`) and rewrites every edge endpoint through the
//! resulting [`MergeReport::node_id_map`].
//!
//! # Transaction model
//! `merge_from` does not call `commit()` itself.  Every write goes through the
//! pager's implicit transaction; callers are expected to wrap a `merge_from`
//! call in `with db.transaction():` (Python) or to call `db.commit()`
//! explicitly, exactly as they would around a batch of `add_node` / `add_edge`
//! calls.  On error, no `commit()` has happened, so a subsequent `rollback()`
//! will discard the partial merge.
//!
//! # Scope (Phase 1)
//! - Both source and destination must already be opened as `GraphDB` handles.
//!   A streaming `merge_file(path)` variant is tracked as a Phase 2 item in
//!   `docs/internal/process/future-roadmap.ja.md`.
//! - [`NodeIdentity::ByProperty`] matches destination nodes by scanning
//!   **every live node in `dst`** (`dst.all_nodes()`) for each source node — O(|N|)
//!   work per source node, with **no on-disk secondary index for arbitrary
//!   property keys**. This is unrelated to [`crate::graph::index::LabelIndex`],
//!   which only speeds up **label-filtered** listing (`scan_nodes` / QueryBuilder)
//!   and is built at `GraphDB::open`. A future on-disk property index (format
//!   evolution / migration — see project design discussions on secondary indexes)
//!   could shrink the `ByProperty` path without changing the public API.

use std::collections::HashMap;

use crate::db::GraphDB;
use crate::error::{LielError, Result};
use crate::graph::edge::Edge;
use crate::graph::node::Node;
use crate::storage::prop_codec::PropValue;

/// Strategy for deciding whether a source node should reuse an existing
/// destination node or always create a fresh one.
#[derive(Debug, Clone)]
pub enum NodeIdentity {
    /// Every source node becomes a new destination node.  IDs are fresh;
    /// `MergeReport::nodes_reused` will always be 0.  This is the default and
    /// matches the behaviour of calling `add_node` for each source node.
    AlwaysNew,

    /// Two nodes are considered the same entity when **every** listed property
    /// key is present on both sides and the values compare equal under
    /// `PropValue::PartialEq`.
    ///
    /// If the source node is missing any of the requested keys, the merge
    /// aborts with [`LielError::MergeKeyNotFound`].  Destination nodes that
    /// lack any of the keys simply do not match (never produce an error); they
    /// just will not be selected as the merge target.
    ByProperty(Vec<String>),
}

/// Strategy for writing each source edge into the destination.
#[derive(Debug, Clone, Copy)]
pub enum EdgeStrategy {
    /// Always call `add_edge`.  Preserves duplicates exactly as they exist in
    /// the source (multigraph semantics), matching Phase 1 `add_edge`
    /// behaviour.
    Append,

    /// Call `merge_edge`: an edge with the same `(from, label, to, props)` is
    /// reused; otherwise a new edge is created.  Makes the whole merge
    /// idempotent when combined with `NodeIdentity::ByProperty`.
    Idempotent,
}

/// What to do with the destination node's properties when a source node
/// matches an existing destination node (via `NodeIdentity::ByProperty`).
///
/// Labels are never modified by `merge_from` regardless of the conflict mode;
/// the destination node's labels are always preserved as-is.  This keeps the
/// implementation straightforward and avoids silent label explosions on
/// repeated merges.
#[derive(Debug, Clone, Copy)]
pub enum ConflictMode {
    /// Leave the destination node completely untouched.  The source node's
    /// properties are discarded.  Safe default.
    KeepDst,

    /// Overlay the source node's properties on top of the destination's:
    /// keys present in the source replace the destination's value; keys
    /// only present on the destination are preserved.  Equivalent to calling
    /// `update_node(dst_id, src_props)`.
    OverwriteFromSrc,

    /// Fill in gaps only: keys present on the destination win, keys only
    /// present on the source are added.  Never overwrites an existing value.
    MergeProps,
}

/// Full configuration for a single `merge_from` invocation.
#[derive(Debug, Clone)]
pub struct MergePolicy {
    pub node_identity: NodeIdentity,
    pub edge_strategy: EdgeStrategy,
    pub on_node_conflict: ConflictMode,
}

impl Default for MergePolicy {
    /// The default policy mirrors the behaviour of looping over
    /// `src.all_nodes()` / `src.all_edges()` and calling `dst.add_node` /
    /// `dst.add_edge` by hand: every source node becomes a fresh destination
    /// node, every source edge is appended, and there is never a conflict to
    /// resolve because nothing is ever reused.
    fn default() -> Self {
        Self {
            node_identity: NodeIdentity::AlwaysNew,
            edge_strategy: EdgeStrategy::Append,
            on_node_conflict: ConflictMode::KeepDst,
        }
    }
}

/// Outcome of a `merge_from` call.
///
/// `node_id_map` is always complete: every source node ID is mapped to a
/// destination node ID, whether that destination node was freshly created or
/// reused.  `edge_id_map` is similar and contains either the newly created
/// edge ID (`EdgeStrategy::Append`) or the surviving/merged edge ID
/// (`EdgeStrategy::Idempotent`).
#[derive(Debug, Clone)]
pub struct MergeReport {
    pub node_id_map: HashMap<u64, u64>,
    pub edge_id_map: HashMap<u64, u64>,
    pub nodes_created: u64,
    pub nodes_reused: u64,
    pub edges_created: u64,
    pub edges_reused: u64,
}

impl MergeReport {
    fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            node_id_map: HashMap::with_capacity(nodes),
            edge_id_map: HashMap::with_capacity(edges),
            nodes_created: 0,
            nodes_reused: 0,
            edges_created: 0,
            edges_reused: 0,
        }
    }
}

/// Merge every live node and edge from `src` into `dst`.
///
/// The source database is read-only for the duration of the call (no writes
/// are performed on it).  The destination is mutated in the pager's implicit
/// transaction; the caller is responsible for committing or rolling back.
///
/// See the module-level documentation for the invariants that every merge
/// upholds (ID remap, file format non-change, transaction model).
pub fn merge_from(
    dst: &mut GraphDB,
    src: &mut GraphDB,
    policy: &MergePolicy,
) -> Result<MergeReport> {
    // Snapshot source contents up-front.  This means we never hold a read
    // reference to src while mutating dst, which matters because at the PyO3
    // layer src and dst live behind two independent Mutex locks that we prefer
    // to hold one-at-a-time to avoid deadlock risk.
    let src_nodes = src.all_nodes()?;
    let src_edges = src.all_edges()?;
    merge_from_snapshot(dst, &src_nodes, &src_edges, policy)
}

/// Merge from an already-materialised snapshot of source nodes and edges.
///
/// This is the lock-free core that both [`merge_from`] and the PyO3 binding
/// call into once they have obtained the source's node/edge lists.
pub fn merge_from_snapshot(
    dst: &mut GraphDB,
    src_nodes: &[Node],
    src_edges: &[Edge],
    policy: &MergePolicy,
) -> Result<MergeReport> {
    let mut report = MergeReport::with_capacity(src_nodes.len(), src_edges.len());

    for src_node in src_nodes {
        let dst_id = resolve_node(dst, src_node, policy, &mut report)?;
        report.node_id_map.insert(src_node.id, dst_id);
    }

    for src_edge in src_edges {
        // Both endpoints must have been processed in the node phase above;
        // if not, the source was internally inconsistent (an edge pointing at
        // a node that is not returned by all_nodes()), which means the source
        // file is corrupted by our invariants.
        let from_id = *report.node_id_map.get(&src_edge.from).ok_or_else(|| {
            LielError::CorruptedFile(format!(
                "merge_from: source edge {} references node {} which was not present in src.all_nodes()",
                src_edge.id, src_edge.from,
            ))
        })?;
        let to_id = *report.node_id_map.get(&src_edge.to).ok_or_else(|| {
            LielError::CorruptedFile(format!(
                "merge_from: source edge {} references node {} which was not present in src.all_nodes()",
                src_edge.id, src_edge.to,
            ))
        })?;

        let new_id = match policy.edge_strategy {
            EdgeStrategy::Append => {
                let e = dst.add_edge(
                    from_id,
                    src_edge.label.clone(),
                    to_id,
                    src_edge.properties.clone(),
                )?;
                report.edges_created += 1;
                e.id
            }
            EdgeStrategy::Idempotent => {
                // merge_edge does not tell us whether it created or reused, so
                // we compare edge_count before and after.  This works because
                // every dst mutation in the loop happens under the same pager
                // and edge_count reflects the live total.
                let before = dst.edge_count();
                let e = dst.merge_edge(
                    from_id,
                    src_edge.label.clone(),
                    to_id,
                    src_edge.properties.clone(),
                )?;
                if dst.edge_count() > before {
                    report.edges_created += 1;
                } else {
                    report.edges_reused += 1;
                }
                e.id
            }
        };
        report.edge_id_map.insert(src_edge.id, new_id);
    }

    Ok(report)
}

/// Decide the destination ID for `src_node` and apply any conflict-mode side
/// effects (property overwrite, property fill-in) in-place on `dst`.
fn resolve_node(
    dst: &mut GraphDB,
    src_node: &Node,
    policy: &MergePolicy,
    report: &mut MergeReport,
) -> Result<u64> {
    match &policy.node_identity {
        NodeIdentity::AlwaysNew => {
            let new_node = dst.add_node(src_node.labels.clone(), src_node.properties.clone())?;
            report.nodes_created += 1;
            Ok(new_node.id)
        }
        NodeIdentity::ByProperty(keys) => {
            // Extract the values that define identity.  If any key is missing
            // from the source node we abort — merging is ambiguous in that
            // case and we prefer to surface the problem to the caller rather
            // than silently creating a duplicate node.
            let mut key_values: Vec<(&str, &PropValue)> = Vec::with_capacity(keys.len());
            for key in keys {
                match src_node.properties.get(key) {
                    Some(v) => key_values.push((key.as_str(), v)),
                    None => {
                        return Err(LielError::MergeKeyNotFound {
                            node_id: src_node.id,
                            key: key.clone(),
                        });
                    }
                }
            }

            // Linear scan of the current dst.  For small to mid-sized dbs this
            // is fine; the Phase 2 secondary-index work will replace this with
            // an O(log n) lookup without changing the API surface.
            let candidates = dst.all_nodes()?;
            let matched = candidates.into_iter().find(|n| {
                key_values
                    .iter()
                    .all(|(k, v)| n.properties.get(*k) == Some(*v))
            });

            match matched {
                Some(existing) => {
                    apply_conflict_mode(dst, &existing, src_node, policy.on_node_conflict)?;
                    report.nodes_reused += 1;
                    Ok(existing.id)
                }
                None => {
                    let new_node =
                        dst.add_node(src_node.labels.clone(), src_node.properties.clone())?;
                    report.nodes_created += 1;
                    Ok(new_node.id)
                }
            }
        }
    }
}

fn apply_conflict_mode(
    dst: &mut GraphDB,
    existing: &Node,
    src_node: &Node,
    mode: ConflictMode,
) -> Result<()> {
    match mode {
        ConflictMode::KeepDst => Ok(()),
        ConflictMode::OverwriteFromSrc => {
            if src_node.properties.is_empty() {
                return Ok(());
            }
            // update_node performs a src-priority key-by-key merge, which is
            // exactly "overlay src on top of dst" semantics.
            dst.update_node(existing.id, src_node.properties.clone())
        }
        ConflictMode::MergeProps => {
            // Only add keys that dst does not already have — dst values win
            // on any collision.
            let mut to_add: HashMap<String, PropValue> = HashMap::new();
            for (k, v) in &src_node.properties {
                if !existing.properties.contains_key(k) {
                    to_add.insert(k.clone(), v.clone());
                }
            }
            if to_add.is_empty() {
                return Ok(());
            }
            dst.update_node(existing.id, to_add)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::GraphDB;
    use std::collections::HashMap;

    fn empty_props() -> HashMap<String, PropValue> {
        HashMap::new()
    }

    fn str_prop(key: &str, value: &str) -> HashMap<String, PropValue> {
        let mut p = HashMap::new();
        p.insert(key.to_string(), PropValue::String(value.to_string()));
        p
    }

    #[test]
    fn merge_append_two_memory_dbs_sums_counts() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let a = dst.add_node(vec!["P".into()], empty_props()).unwrap();
        let b = dst.add_node(vec!["P".into()], empty_props()).unwrap();
        dst.add_edge(a.id, "E".into(), b.id, empty_props()).unwrap();

        let mut src = GraphDB::open(":memory:").unwrap();
        let x = src.add_node(vec!["P".into()], empty_props()).unwrap();
        let y = src.add_node(vec!["P".into()], empty_props()).unwrap();
        src.add_edge(x.id, "E".into(), y.id, empty_props()).unwrap();

        let report = merge_from(&mut dst, &mut src, &MergePolicy::default()).unwrap();

        assert_eq!(dst.node_count(), 4);
        assert_eq!(dst.edge_count(), 2);
        assert_eq!(report.nodes_created, 2);
        assert_eq!(report.nodes_reused, 0);
        assert_eq!(report.edges_created, 1);
        assert_eq!(report.edges_reused, 0);
        assert_eq!(report.node_id_map.len(), 2);
        assert_eq!(report.edge_id_map.len(), 1);
    }

    #[test]
    fn merge_remaps_edge_endpoints() {
        // Both dbs start with NodeId 1. After merging src into dst the src
        // edge must connect the remapped pair, not the original NodeId(1).
        let mut dst = GraphDB::open(":memory:").unwrap();
        let d1 = dst.add_node(vec![], empty_props()).unwrap();
        assert_eq!(d1.id, 1);

        let mut src = GraphDB::open(":memory:").unwrap();
        let s1 = src.add_node(vec![], str_prop("name", "S1")).unwrap();
        let s2 = src.add_node(vec![], str_prop("name", "S2")).unwrap();
        assert_eq!(s1.id, 1);
        assert_eq!(s2.id, 2);
        let se = src
            .add_edge(s1.id, "L".into(), s2.id, empty_props())
            .unwrap();

        let report = merge_from(&mut dst, &mut src, &MergePolicy::default()).unwrap();

        let new_edge_id = report.edge_id_map[&se.id];
        let e = dst.get_edge(new_edge_id).unwrap().unwrap();
        assert_eq!(e.from, report.node_id_map[&s1.id]);
        assert_eq!(e.to, report.node_id_map[&s2.id]);
        // And critically, it is NOT the dst NodeId(1) that existed before merge.
        assert_ne!(e.from, d1.id);
    }

    #[test]
    fn merge_idempotent_edges_deduplicate_on_second_run() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut src = GraphDB::open(":memory:").unwrap();

        let a = src.add_node(vec![], str_prop("tag", "A")).unwrap();
        let b = src.add_node(vec![], str_prop("tag", "B")).unwrap();
        src.add_edge(a.id, "R".into(), b.id, empty_props()).unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["tag".into()]),
            edge_strategy: EdgeStrategy::Idempotent,
            on_node_conflict: ConflictMode::KeepDst,
        };

        let first = merge_from(&mut dst, &mut src, &policy).unwrap();
        assert_eq!(first.nodes_created, 2);
        assert_eq!(first.edges_created, 1);
        assert_eq!(first.edges_reused, 0);
        assert_eq!(dst.node_count(), 2);
        assert_eq!(dst.edge_count(), 1);

        let second = merge_from(&mut dst, &mut src, &policy).unwrap();
        assert_eq!(second.nodes_created, 0);
        assert_eq!(second.nodes_reused, 2);
        assert_eq!(second.edges_created, 0);
        assert_eq!(second.edges_reused, 1);
        assert_eq!(dst.node_count(), 2);
        assert_eq!(dst.edge_count(), 1);
    }

    #[test]
    fn merge_by_property_reuses_matching_node() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let existing = dst
            .add_node(vec!["User".into()], str_prop("email", "a@example.com"))
            .unwrap();

        let mut src = GraphDB::open(":memory:").unwrap();
        src.add_node(vec!["User".into()], str_prop("email", "a@example.com"))
            .unwrap();
        src.add_node(vec!["User".into()], str_prop("email", "b@example.com"))
            .unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["email".into()]),
            ..MergePolicy::default()
        };
        let report = merge_from(&mut dst, &mut src, &policy).unwrap();

        assert_eq!(report.nodes_reused, 1);
        assert_eq!(report.nodes_created, 1);
        assert_eq!(dst.node_count(), 2);
        // The matched node is the pre-existing one (ID preserved).
        let n = dst.get_node(existing.id).unwrap().unwrap();
        assert_eq!(
            n.properties.get("email"),
            Some(&PropValue::String("a@example.com".into()))
        );
    }

    #[test]
    fn merge_overwrite_from_src_overlays_props() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut dst_props = HashMap::new();
        dst_props.insert("email".into(), PropValue::String("x@example.com".into()));
        dst_props.insert("name".into(), PropValue::String("OLD".into()));
        let existing = dst.add_node(vec![], dst_props).unwrap();

        let mut src = GraphDB::open(":memory:").unwrap();
        let mut src_props = HashMap::new();
        src_props.insert("email".into(), PropValue::String("x@example.com".into()));
        src_props.insert("name".into(), PropValue::String("NEW".into()));
        src_props.insert("age".into(), PropValue::Int(42));
        src.add_node(vec![], src_props).unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["email".into()]),
            edge_strategy: EdgeStrategy::Append,
            on_node_conflict: ConflictMode::OverwriteFromSrc,
        };
        merge_from(&mut dst, &mut src, &policy).unwrap();

        let n = dst.get_node(existing.id).unwrap().unwrap();
        assert_eq!(
            n.properties.get("name"),
            Some(&PropValue::String("NEW".into()))
        );
        assert_eq!(n.properties.get("age"), Some(&PropValue::Int(42)));
    }

    #[test]
    fn merge_keep_dst_retains_props() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut dst_props = HashMap::new();
        dst_props.insert("email".into(), PropValue::String("x@example.com".into()));
        dst_props.insert("name".into(), PropValue::String("ORIGINAL".into()));
        let existing = dst.add_node(vec![], dst_props).unwrap();

        let mut src = GraphDB::open(":memory:").unwrap();
        let mut src_props = HashMap::new();
        src_props.insert("email".into(), PropValue::String("x@example.com".into()));
        src_props.insert("name".into(), PropValue::String("CHANGED".into()));
        src.add_node(vec![], src_props).unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["email".into()]),
            edge_strategy: EdgeStrategy::Append,
            on_node_conflict: ConflictMode::KeepDst,
        };
        merge_from(&mut dst, &mut src, &policy).unwrap();

        let n = dst.get_node(existing.id).unwrap().unwrap();
        assert_eq!(
            n.properties.get("name"),
            Some(&PropValue::String("ORIGINAL".into()))
        );
    }

    #[test]
    fn merge_merge_props_fills_gaps_only() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut dst_props = HashMap::new();
        dst_props.insert("email".into(), PropValue::String("x@example.com".into()));
        dst_props.insert("name".into(), PropValue::String("DST".into()));
        let existing = dst.add_node(vec![], dst_props).unwrap();

        let mut src = GraphDB::open(":memory:").unwrap();
        let mut src_props = HashMap::new();
        src_props.insert("email".into(), PropValue::String("x@example.com".into()));
        src_props.insert("name".into(), PropValue::String("SRC".into())); // must NOT win
        src_props.insert("age".into(), PropValue::Int(7)); // should be added
        src.add_node(vec![], src_props).unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["email".into()]),
            edge_strategy: EdgeStrategy::Append,
            on_node_conflict: ConflictMode::MergeProps,
        };
        merge_from(&mut dst, &mut src, &policy).unwrap();

        let n = dst.get_node(existing.id).unwrap().unwrap();
        assert_eq!(
            n.properties.get("name"),
            Some(&PropValue::String("DST".into()))
        );
        assert_eq!(n.properties.get("age"), Some(&PropValue::Int(7)));
    }

    #[test]
    fn merge_fails_when_key_missing() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut src = GraphDB::open(":memory:").unwrap();
        // This source node has "name" but not "email".
        src.add_node(vec![], str_prop("name", "no-key")).unwrap();

        let policy = MergePolicy {
            node_identity: NodeIdentity::ByProperty(vec!["email".into()]),
            ..MergePolicy::default()
        };
        let err = merge_from(&mut dst, &mut src, &policy).unwrap_err();
        match err {
            LielError::MergeKeyNotFound { node_id: _, key } => {
                assert_eq!(key, "email");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn merge_report_covers_every_src_id() {
        let mut dst = GraphDB::open(":memory:").unwrap();
        let mut src = GraphDB::open(":memory:").unwrap();
        let n1 = src.add_node(vec![], empty_props()).unwrap();
        let n2 = src.add_node(vec![], empty_props()).unwrap();
        let n3 = src.add_node(vec![], empty_props()).unwrap();
        let e1 = src
            .add_edge(n1.id, "A".into(), n2.id, empty_props())
            .unwrap();
        let e2 = src
            .add_edge(n2.id, "B".into(), n3.id, empty_props())
            .unwrap();

        let report = merge_from(&mut dst, &mut src, &MergePolicy::default()).unwrap();

        for id in [n1.id, n2.id, n3.id] {
            assert!(report.node_id_map.contains_key(&id));
        }
        for id in [e1.id, e2.id] {
            assert!(report.edge_id_map.contains_key(&id));
        }
    }
}

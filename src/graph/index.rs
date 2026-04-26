use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::graph::node;
use crate::storage::pager::Pager;

/// In-memory inverted index: label → set of live node IDs carrying that label.
///
/// Built once at `GraphDB::open()` by a single full scan (the same cost as one
/// label-filtered query today), then maintained in O(1) per `add_node` /
/// `delete_node`.  No on-disk representation — rebuilt from the pager on any
/// rollback or reopen.
pub struct LabelIndex {
    map: HashMap<String, HashSet<u64>>,
}

impl Default for LabelIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl LabelIndex {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Build the index from the current committed state of the pager.
    ///
    /// Iterates every allocated node ID, skipping deleted slots.  Cost equals
    /// one full slot scan (same as a single label-filtered query before this
    /// index existed).
    pub fn build(pager: &mut Pager) -> Result<Self> {
        let mut idx = Self::new();
        let max_id = pager.max_node_id();
        for node_id in 1..=max_id {
            if let Some(n) = node::get_node(pager, node_id)? {
                for label in &n.labels {
                    idx.map.entry(label.clone()).or_default().insert(node_id);
                }
            }
        }
        Ok(idx)
    }

    /// Record that `node_id` carries the given labels.  Called after a node is
    /// successfully written to the pager.
    pub fn insert(&mut self, node_id: u64, labels: &[String]) {
        for label in labels {
            self.map.entry(label.clone()).or_default().insert(node_id);
        }
    }

    /// Remove `node_id` from all its label entries.  Called before a node slot
    /// is marked deleted.
    pub fn remove(&mut self, node_id: u64, labels: &[String]) {
        for label in labels {
            if let Some(set) = self.map.get_mut(label) {
                set.remove(&node_id);
                // Leave empty sets in place; they are cheap and avoid
                // reallocating the Vec on the next insert for the same label.
            }
        }
    }

    /// Return the sorted list of node IDs for a single label, or `None` if the
    /// label has never been seen.
    pub fn ids_for_label(&self, label: &str) -> Option<Vec<u64>> {
        self.map.get(label).map(|set| {
            let mut v: Vec<u64> = set.iter().copied().collect();
            v.sort_unstable();
            v
        })
    }

    /// Return the sorted, deduplicated union of node IDs matching any of the
    /// given labels.  Returns an empty `Vec` if none match.
    pub fn ids_for_labels(&self, labels: &[String]) -> Vec<u64> {
        let mut ids: HashSet<u64> = HashSet::new();
        for label in labels {
            if let Some(set) = self.map.get(label) {
                ids.extend(set.iter().copied());
            }
        }
        let mut v: Vec<u64> = ids.into_iter().collect();
        v.sort_unstable();
        v
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::prop_codec::PropValue;
    use std::collections::HashMap;

    fn open_mem() -> Pager {
        Pager::open(":memory:").unwrap()
    }

    fn empty_props() -> HashMap<String, PropValue> {
        HashMap::new()
    }

    #[test]
    fn build_empty_pager() {
        let mut pager = open_mem();
        let idx = LabelIndex::build(&mut pager).unwrap();
        assert!(idx.ids_for_label("Person").is_none());
    }

    #[test]
    fn insert_and_lookup() {
        let mut idx = LabelIndex::new();
        idx.insert(1, &["Person".into(), "Admin".into()]);
        idx.insert(2, &["Person".into()]);
        idx.insert(3, &["Company".into()]);

        let persons = idx.ids_for_label("Person").unwrap();
        assert_eq!(persons, vec![1, 2]);
        let companies = idx.ids_for_label("Company").unwrap();
        assert_eq!(companies, vec![3]);
        assert!(idx.ids_for_label("Unknown").is_none());
    }

    #[test]
    fn remove_updates_set() {
        let mut idx = LabelIndex::new();
        idx.insert(1, &["X".into()]);
        idx.insert(2, &["X".into()]);
        idx.remove(1, &["X".into()]);
        let xs = idx.ids_for_label("X").unwrap();
        assert_eq!(xs, vec![2]);
    }

    #[test]
    fn ids_for_labels_union() {
        let mut idx = LabelIndex::new();
        idx.insert(1, &["A".into()]);
        idx.insert(2, &["B".into()]);
        idx.insert(3, &["A".into(), "B".into()]);
        let ids = idx.ids_for_labels(&["A".into(), "B".into()]);
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn build_from_populated_pager() {
        let mut pager = open_mem();
        node::add_node(&mut pager, vec!["Person".into()], empty_props()).unwrap();
        node::add_node(&mut pager, vec!["Person".into()], empty_props()).unwrap();
        node::add_node(&mut pager, vec!["Company".into()], empty_props()).unwrap();

        let idx = LabelIndex::build(&mut pager).unwrap();
        let persons = idx.ids_for_label("Person").unwrap();
        assert_eq!(persons.len(), 2);
        let companies = idx.ids_for_label("Company").unwrap();
        assert_eq!(companies.len(), 1);
    }

    #[test]
    fn build_skips_deleted_nodes() {
        let mut pager = open_mem();
        let n1 = node::add_node(&mut pager, vec!["X".into()], empty_props()).unwrap();
        node::add_node(&mut pager, vec!["X".into()], empty_props()).unwrap();
        node::delete_node(&mut pager, n1.id).unwrap();

        let idx = LabelIndex::build(&mut pager).unwrap();
        let xs = idx.ids_for_label("X").unwrap();
        assert_eq!(xs.len(), 1);
        assert!(!xs.contains(&n1.id));
    }
}

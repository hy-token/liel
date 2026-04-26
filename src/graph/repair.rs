use crate::error::{LielError, Result};
use crate::storage::pager::Pager;
use crate::storage::serializer::{EdgeSlot, NodeSlot};

/// Summary of a completed adjacency repair pass.
#[derive(Debug, Clone, Default)]
pub struct RepairReport {
    pub nodes_rewritten: u64,
    pub edges_relinked: u64,
}

/// Rebuild every node's adjacency heads and degree counters from the live edge set.
///
/// This repair pass treats the live edge slots as the source of truth and
/// reconstructs:
/// - `NodeSlot.first_out_edge`
/// - `NodeSlot.first_in_edge`
/// - `NodeSlot.out_degree`
/// - `NodeSlot.in_degree`
/// - `EdgeSlot.next_out_edge`
/// - `EdgeSlot.next_in_edge`
///
/// The algorithm is intentionally conservative: if a live edge points to a
/// missing or deleted endpoint node, the function fails with
/// [`LielError::CorruptedFile`] instead of guessing how to heal the graph.
pub fn repair_adjacency(pager: &mut Pager) -> Result<RepairReport> {
    let max_node_id = pager.max_node_id();
    let max_edge_id = pager.max_edge_id();

    let mut node_slots: Vec<Option<NodeSlot>> =
        vec![None; (max_node_id as usize).saturating_add(1)];
    for node_id in 1..=max_node_id {
        let mut slot = pager.read_node_slot(node_id)?;
        if slot.is_active() {
            slot.first_out_edge = 0;
            slot.first_in_edge = 0;
            slot.out_degree = 0;
            slot.in_degree = 0;
            node_slots[node_id as usize] = Some(slot);
        }
    }

    let mut edge_slots: Vec<Option<EdgeSlot>> =
        vec![None; (max_edge_id as usize).saturating_add(1)];
    let mut report = RepairReport::default();

    for edge_id in 1..=max_edge_id {
        let mut slot = pager.read_edge_slot(edge_id)?;
        if !slot.is_active() {
            continue;
        }

        let from_idx = slot.from_node_id as usize;
        let to_idx = slot.to_node_id as usize;
        let Some(from_node) = node_slots.get_mut(from_idx).and_then(Option::as_mut) else {
            return Err(LielError::CorruptedFile(format!(
                "Database integrity failure: live edge {edge_id} points to missing or deleted source node {}. \
Adjacency repair stopped before making changes. Treat this database as damaged, take a backup, and restore from a known-good copy or salvage readable records into a new database.",
                slot.from_node_id
            )));
        };
        let prev_out = from_node.first_out_edge;
        from_node.first_out_edge = edge_id;
        from_node.out_degree = from_node.out_degree.checked_add(1).ok_or_else(|| {
            LielError::CorruptedFile(format!(
                "Database integrity failure: outgoing degree overflow while repairing node {} from edge {edge_id}. \
Treat this database as damaged and restore from backup before retrying.",
                slot.from_node_id
            ))
        })?;

        let Some(to_node) = node_slots.get_mut(to_idx).and_then(Option::as_mut) else {
            return Err(LielError::CorruptedFile(format!(
                "Database integrity failure: live edge {edge_id} points to missing or deleted target node {}. \
Adjacency repair stopped before making changes. Treat this database as damaged, take a backup, and restore from a known-good copy or salvage readable records into a new database.",
                slot.to_node_id
            )));
        };
        let prev_in = to_node.first_in_edge;
        to_node.first_in_edge = edge_id;
        to_node.in_degree = to_node.in_degree.checked_add(1).ok_or_else(|| {
            LielError::CorruptedFile(format!(
                "Database integrity failure: incoming degree overflow while repairing node {} from edge {edge_id}. \
Treat this database as damaged and restore from backup before retrying.",
                slot.to_node_id
            ))
        })?;

        slot.next_out_edge = prev_out;
        slot.next_in_edge = prev_in;
        edge_slots[edge_id as usize] = Some(slot);
        report.edges_relinked += 1;
    }

    for slot in node_slots.into_iter().flatten() {
        pager.write_node_slot(&slot)?;
        report.nodes_rewritten += 1;
    }
    for slot in edge_slots.into_iter().flatten() {
        pager.write_edge_slot(&slot)?;
    }

    pager.commit()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::{add_edge, out_edges};
    use crate::graph::node::{add_node, get_node};
    use crate::storage::prop_codec::PropValue;
    use std::collections::HashMap;

    fn empty_props() -> HashMap<String, PropValue> {
        HashMap::new()
    }

    #[test]
    fn repair_adjacency_rebuilds_heads_and_degrees() {
        let mut pager = Pager::open(":memory:").unwrap();
        let a = add_node(&mut pager, vec!["A".into()], empty_props()).unwrap();
        let b = add_node(&mut pager, vec!["B".into()], empty_props()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "R".into(), b.id, empty_props()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "R".into(), b.id, empty_props()).unwrap();
        pager.commit().unwrap();

        let mut a_slot = pager.read_node_slot(a.id).unwrap();
        a_slot.first_out_edge = 0;
        a_slot.out_degree = 0;
        pager.write_node_slot(&a_slot).unwrap();

        let mut b_slot = pager.read_node_slot(b.id).unwrap();
        b_slot.first_in_edge = 0;
        b_slot.in_degree = 0;
        pager.write_node_slot(&b_slot).unwrap();

        let mut e1_slot = pager.read_edge_slot(e1.id).unwrap();
        e1_slot.next_out_edge = 999;
        e1_slot.next_in_edge = 999;
        pager.write_edge_slot(&e1_slot).unwrap();
        pager.commit().unwrap();

        let report = repair_adjacency(&mut pager).unwrap();
        assert_eq!(report.nodes_rewritten, 2);
        assert_eq!(report.edges_relinked, 2);

        let a_fixed = pager.read_node_slot(a.id).unwrap();
        let b_fixed = pager.read_node_slot(b.id).unwrap();
        assert_eq!(a_fixed.out_degree, 2);
        assert_eq!(b_fixed.in_degree, 2);

        let edges = out_edges(&mut pager, a.id, Some("R")).unwrap();
        assert_eq!(edges.len(), 2);
        let node = get_node(&mut pager, a.id).unwrap().unwrap();
        assert_eq!(node.labels, vec!["A"]);

        let repaired_e1 = pager.read_edge_slot(e1.id).unwrap();
        let repaired_e2 = pager.read_edge_slot(e2.id).unwrap();
        assert!(repaired_e1.next_out_edge == 0 || repaired_e2.next_out_edge == 0);
    }
}

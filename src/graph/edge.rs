use crate::error::{LielError, Result};
use crate::storage::pager::Pager;
use crate::storage::prop_codec::{decode_props, encode_props, PropValue};
use crate::storage::serializer::{EdgeSlot, FLAG_DELETED};
use std::collections::HashMap;

/// A directed edge in the liel graph database.
///
/// An `Edge` represents a directed relationship between two nodes.  It carries:
///
/// - A unique 64-bit `id` (1-based; 0 is the NULL sentinel).
/// - `from`: The ID of the source (origin) node.
/// - `to`:   The ID of the destination (target) node.
/// - A `label` string that names the relationship type (e.g. `"KNOWS"`,
///   `"OWNS"`, `"DEPENDS_ON"`).
/// - A map of key→value `properties`, identical in structure to node properties.
///
/// liel is a *multigraph*: multiple edges with the same label may exist between
/// the same pair of nodes, and self-loops are also permitted.  No uniqueness
/// constraint is enforced at the storage layer.
///
/// `Edge` is a purely in-memory view and must be re-persisted via [`update_edge`]
/// for property changes, or removed with [`delete_edge`].
#[derive(Debug, Clone)]
pub struct Edge {
    /// The unique identifier for this edge.  Starts at 1; 0 is NULL.
    pub id: u64,
    /// The ID of the source node that this edge originates from.
    pub from: u64,
    /// The ID of the destination node that this edge points to.
    pub to: u64,
    /// The relationship type label (e.g. `"KNOWS"`, `"FOLLOWS"`).
    pub label: String,
    /// Arbitrary key-value properties attached to this edge.
    pub properties: HashMap<String, PropValue>,
}

/// Create a new directed edge and update both adjacency lists.
///
/// This function implements *head-insert* on two independent singly-linked
/// lists maintained inside the node slots:
///
/// - The **out-edge list** of the `from` node:
///   `from.first_out_edge → new_edge.next_out_edge → old_head → … → 0`
/// - The **in-edge list** of the `to` node:
///   `to.first_in_edge → new_edge.next_in_edge → old_head → … → 0`
///
/// The full write sequence is:
///
/// 1. Validate that both `from` and `to` exist and are not deleted.
/// 2. Allocate a new edge ID.
/// 3. Serialise and append the `label` string to the property storage.
/// 4. Serialise and append `props` to the property storage (if non-empty).
/// 5. Read the current head pointers (`first_out_edge`, `first_in_edge`) from
///    the respective node slots.
/// 6. Write the new [`EdgeSlot`] with `next_out_edge = old_from_head` and
///    `next_in_edge = old_to_head`, effectively prepending it to both lists.
/// 7. Update the `from` node slot: set `first_out_edge = new_edge_id` and
///    increment `out_degree`.
/// 8. Update the `to` node slot: set `first_in_edge = new_edge_id` and
///    increment `in_degree`.
///
/// Head-insert is O(1) and avoids traversing the full list, which is critical
/// for nodes with high degree.  The trade-off is that iteration order is
/// reverse-insertion order (most recently added edge appears first).
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
/// - `from`:  Source node ID.
/// - `label`: Relationship type string.
/// - `to`:    Destination node ID.
/// - `props`: Initial properties for this edge.  May be empty.
///
/// # Returns
/// An [`Edge`] value reflecting the persisted state.
///
/// # Errors
/// - [`LielError::NodeNotFound`] if either `from` or `to` is 0, out of range,
///   or deleted.
/// - [`LielError::Io`] from any pager write.
pub fn add_edge(
    pager: &mut Pager,
    from: u64,
    label: String,
    to: u64,
    props: HashMap<String, PropValue>,
) -> Result<Edge> {
    // Validate source node: must be in [1, next_node_id) and not deleted.
    if from == 0 || from >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(from));
    }
    if to == 0 || to >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(to));
    }

    let edge_id = pager.alloc_edge_id()?;

    // Serialise the label as a PropValue::String so it is stored using the same
    // binary codec as all other property data.  The label is always present, so
    // we never record (0, 0) for it.
    let label_bytes = {
        use crate::storage::prop_codec::encode;
        encode(&PropValue::String(label.clone()))
    };
    let label_offset = pager.append_prop(&label_bytes)?;
    let label_length = label_bytes.len() as u32;

    // Serialise and append the property map.  An empty map writes nothing.
    let (prop_offset, prop_length) = if props.is_empty() {
        (0, 0)
    } else {
        let prop_bytes = encode_props(&props);
        let offset = pager.append_prop(&prop_bytes)?;
        (offset, prop_bytes.len() as u32)
    };

    // Read the current head of the from-node's out-edge linked list.
    // We will prepend the new edge to this list (head-insert).
    let from_slot = pager.read_node_slot(from)?;
    if from_slot.node_id == 0 || from_slot.is_deleted() {
        return Err(LielError::NodeNotFound(from));
    }
    let prev_first_out = from_slot.first_out_edge;

    // Read the current head of the to-node's in-edge linked list.
    // We will prepend the new edge to this list as well.
    let to_slot = pager.read_node_slot(to)?;
    if to_slot.node_id == 0 || to_slot.is_deleted() {
        return Err(LielError::NodeNotFound(to));
    }
    let prev_first_in = to_slot.first_in_edge;

    // Write the new EdgeSlot.  The two "next" pointers are set to the old list
    // heads so this new edge becomes the new head of both lists simultaneously.
    let edge_slot = EdgeSlot {
        edge_id,
        from_node_id: from,
        to_node_id: to,
        next_out_edge: prev_first_out, // links into the from-node's former out-list
        next_in_edge: prev_first_in,   // links into the to-node's former in-list
        prop_offset,
        prop_length,
        label_offset,
        label_length,
        flags: 0,
    };
    pager.write_edge_slot(&edge_slot)?;
    pager.increment_edge_count();

    if from == to {
        // Self-loop: both adjacency-list head updates must be applied to the
        // same node slot image before we write it back.
        let mut slot = from_slot;
        slot.first_out_edge = edge_id;
        slot.out_degree += 1;
        slot.first_in_edge = edge_id;
        slot.in_degree += 1;
        pager.write_node_slot(&slot)?;
    } else {
        // Update the from-node slot to point its out-list head at the new edge.
        let mut from_slot_mut = from_slot;
        from_slot_mut.first_out_edge = edge_id;
        from_slot_mut.out_degree += 1;
        pager.write_node_slot(&from_slot_mut)?;

        // Update the to-node slot to point its in-list head at the new edge.
        let mut to_slot_mut = to_slot;
        to_slot_mut.first_in_edge = edge_id;
        to_slot_mut.in_degree += 1;
        pager.write_node_slot(&to_slot_mut)?;
    }

    Ok(Edge {
        id: edge_id,
        from,
        to,
        label,
        properties: props,
    })
}

/// Retrieve an edge by ID, returning `None` if it does not exist or is deleted.
///
/// Mirrors the logic of [`get_node`](crate::graph::node::get_node):
///
/// 1. Fast range check against `next_edge_id`.
/// 2. Read the [`EdgeSlot`].
/// 3. Return `Ok(None)` for deleted or logically empty slots.
/// 4. Deserialise the label by decoding the stored `PropValue::String`.
/// 5. Deserialise the properties if present.
///
/// # Parameters
/// - `pager`:   Mutable reference to the open [`Pager`].
/// - `edge_id`: The ID to look up.
///
/// # Returns
/// - `Ok(Some(Edge))` if the edge exists and is active.
/// - `Ok(None)` if the ID is out of range or the edge was deleted.
/// - `Err(LielError::Io)` on a storage read failure.
/// - `Err(LielError::CorruptedFile)` if the label slot does not decode to a
///   `PropValue::String` (which would indicate on-disk corruption).
pub fn get_edge(pager: &mut Pager, edge_id: u64) -> Result<Option<Edge>> {
    if edge_id == 0 || edge_id >= pager.header.next_edge_id {
        return Ok(None);
    }
    let slot = pager.read_edge_slot(edge_id)?;
    if slot.edge_id == 0 || slot.is_deleted() {
        return Ok(None);
    }

    // The label is stored as a serialised PropValue::String in the property storage.
    // Any other decoded type indicates file corruption.
    let label = if slot.label_length > 0 {
        let bytes = pager.read_prop(slot.label_offset, slot.label_length)?;
        use crate::storage::prop_codec::decode;
        match decode(&bytes)? {
            PropValue::String(s) => s,
            _ => {
                return Err(LielError::CorruptedFile(
                    "edge label is not a string".into(),
                ))
            }
        }
    } else {
        String::new()
    };

    // Deserialise the property map; empty if prop_length == 0.
    let properties = if slot.prop_length > 0 {
        let bytes = pager.read_prop(slot.prop_offset, slot.prop_length)?;
        decode_props(&bytes)?
    } else {
        HashMap::new()
    };

    Ok(Some(Edge {
        id: edge_id,
        from: slot.from_node_id,
        to: slot.to_node_id,
        label,
        properties,
    }))
}

/// Delete an edge and remove it from both adjacency lists.
///
/// This is the most structurally complex write in the storage layer because it
/// must surgically remove a node from two independent singly-linked lists
/// (the out-list of `from` and the in-list of `to`).
///
/// The full sequence is:
///
/// 1. Validate the edge ID and check the slot is not already deleted.
/// 2. Call [`remove_from_out_list`] to unlink the edge from the `from` node's
///    outgoing edge list.
/// 3. Call [`remove_from_in_list`] to unlink the edge from the `to` node's
///    incoming edge list.
/// 4. Set `FLAG_DELETED` on the edge slot.
/// 5. Decrement the live-edge counter in the file header.
///
/// After deletion, the edge's property-area bytes become dead data until
/// [`vacuum`](crate::graph::vacuum::vacuum) is called.
///
/// # Parameters
/// - `pager`:   Mutable reference to the open [`Pager`].
/// - `edge_id`: The ID of the edge to delete.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// - [`LielError::EdgeNotFound`] if `edge_id` is 0, out of range, or already
///   deleted.
/// - [`LielError::Io`] from any underlying slot read or write.
pub fn delete_edge(pager: &mut Pager, edge_id: u64) -> Result<()> {
    if edge_id == 0 || edge_id >= pager.header.next_edge_id {
        return Err(LielError::EdgeNotFound(edge_id));
    }
    let slot = pager.read_edge_slot(edge_id)?;
    if slot.edge_id == 0 || slot.is_deleted() {
        return Err(LielError::EdgeNotFound(edge_id));
    }

    let from = slot.from_node_id;
    let to = slot.to_node_id;

    // Remove this edge from the from-node's outgoing adjacency list and
    // decrement the from-node's out_degree counter.
    remove_from_out_list(pager, from, edge_id)?;

    // Remove this edge from the to-node's incoming adjacency list and
    // decrement the to-node's in_degree counter.
    remove_from_in_list(pager, to, edge_id)?;

    // Mark the edge slot as deleted.  The slot index will never be reused.
    let mut slot_mut = slot;
    slot_mut.flags |= FLAG_DELETED;
    pager.write_edge_slot(&slot_mut)?;
    pager.decrement_edge_count();

    Ok(())
}

/// Unlink `edge_id` from the out-edge singly-linked list of `node_id`.
///
/// The out-edge list is a singly-linked list threaded through `EdgeSlot.next_out_edge`
/// with the head stored in `NodeSlot.first_out_edge`.  Removal requires finding the
/// predecessor of `edge_id` in the list and updating its `next_out_edge` to skip
/// over the target edge.
///
/// Two cases:
///
/// 1. **Head removal**: `node_slot.first_out_edge == edge_id`.  Simply advance
///    the head pointer to `edge_slot.next_out_edge` (which may be 0 for an
///    empty list after removal).
///
/// 2. **Mid/tail removal**: Walk the list from the head until the predecessor
///    (the edge whose `next_out_edge == edge_id`) is found, then bridge its
///    `next_out_edge` over the target to `target.next_out_edge`.
///
/// In both cases, `node_slot.out_degree` is decremented using saturating
/// subtraction to guard against underflow on corrupted files.
///
/// If the edge is not found in the list, the database's adjacency metadata is
/// inconsistent with the edge slot and the function fails with
/// [`LielError::CorruptedFile`]. This is treated as a hard integrity error so
/// callers stop writing rather than silently widening the damage.
///
/// # Parameters
/// - `pager`:   Mutable reference to the open [`Pager`].
/// - `node_id`: The node whose out-edge list is being modified.
/// - `edge_id`: The edge to remove from that list.
fn remove_from_out_list(pager: &mut Pager, node_id: u64, edge_id: u64) -> Result<()> {
    let mut node_slot = pager.read_node_slot(node_id)?;
    if node_slot.first_out_edge == edge_id {
        // Case 1: the target edge is the head of the list.
        // Read the target's own next pointer to find the new head.
        let edge_slot = pager.read_edge_slot(edge_id)?;
        node_slot.first_out_edge = edge_slot.next_out_edge;
        node_slot.out_degree = node_slot.out_degree.saturating_sub(1);
        pager.write_node_slot(&node_slot)?;
    } else {
        // Case 2: walk the list to find the predecessor node (the edge whose
        // next_out_edge points to edge_id), then patch its pointer.
        let mut current_id = node_slot.first_out_edge;
        loop {
            if current_id == 0 {
                return Err(adjacency_corruption_error("out-edge", node_id, edge_id));
            }
            let mut current_slot = pager.read_edge_slot(current_id)?;
            if current_slot.next_out_edge == edge_id {
                // Found the predecessor.  Read the target's next pointer so we
                // can splice it out of the list.
                let target = pager.read_edge_slot(edge_id)?;
                current_slot.next_out_edge = target.next_out_edge;
                pager.write_edge_slot(&current_slot)?;
                node_slot.out_degree = node_slot.out_degree.saturating_sub(1);
                pager.write_node_slot(&node_slot)?;
                break;
            }
            // Advance to the next edge in the list.
            current_id = current_slot.next_out_edge;
        }
    }
    Ok(())
}

/// Unlink `edge_id` from the in-edge singly-linked list of `node_id`.
///
/// Mirrors [`remove_from_out_list`] exactly, but operates on the in-edge
/// linked list threaded through `EdgeSlot.next_in_edge` with the head stored
/// in `NodeSlot.first_in_edge`.
///
/// See [`remove_from_out_list`] for a detailed explanation of the two removal
/// cases (head removal vs. mid/tail removal) and the saturating-subtraction
/// guard on `in_degree`.
///
/// # Parameters
/// - `pager`:   Mutable reference to the open [`Pager`].
/// - `node_id`: The node whose in-edge list is being modified.
/// - `edge_id`: The edge to remove from that list.
fn remove_from_in_list(pager: &mut Pager, node_id: u64, edge_id: u64) -> Result<()> {
    let mut node_slot = pager.read_node_slot(node_id)?;
    if node_slot.first_in_edge == edge_id {
        // Case 1: head removal — advance head to the target's next_in_edge.
        let edge_slot = pager.read_edge_slot(edge_id)?;
        node_slot.first_in_edge = edge_slot.next_in_edge;
        node_slot.in_degree = node_slot.in_degree.saturating_sub(1);
        pager.write_node_slot(&node_slot)?;
    } else {
        // Case 2: walk the in-list to find the predecessor, then splice out.
        let mut current_id = node_slot.first_in_edge;
        loop {
            if current_id == 0 {
                return Err(adjacency_corruption_error("in-edge", node_id, edge_id));
            }
            let mut current_slot = pager.read_edge_slot(current_id)?;
            if current_slot.next_in_edge == edge_id {
                // Predecessor found; bridge it over the target edge.
                let target = pager.read_edge_slot(edge_id)?;
                current_slot.next_in_edge = target.next_in_edge;
                pager.write_edge_slot(&current_slot)?;
                node_slot.in_degree = node_slot.in_degree.saturating_sub(1);
                pager.write_node_slot(&node_slot)?;
                break;
            }
            // Advance to the next edge in the in-list.
            current_id = current_slot.next_in_edge;
        }
    }
    Ok(())
}

fn adjacency_corruption_error(list_kind: &str, node_id: u64, edge_id: u64) -> LielError {
    LielError::CorruptedFile(format!(
        "Database integrity failure: node {node_id}'s {list_kind} adjacency list does not contain edge {edge_id} during unlink. \
This means the database metadata is inconsistent and the file should be treated as damaged. \
Stop writing to this database, take a backup, and if your application exposes a repair function run `repair_adjacency()` before retrying. \
If no repair function is available, restore from a known-good backup."
    ))
}

fn adjacency_cycle_error(list_kind: &str, node_id: u64, edge_id: u64) -> LielError {
    LielError::CorruptedFile(format!(
        "Database integrity failure: detected a cycle while traversing node {node_id}'s {list_kind} adjacency list (revisited edge {edge_id}). \
The adjacency metadata is damaged and reads cannot continue safely. Stop writing to this database, take a backup, and run `repair_adjacency()` if your application exposes it. \
If repair is unavailable or fails, restore from a known-good backup."
    ))
}

/// Return all outgoing edges of a node, with an optional label filter.
///
/// Walks the out-edge singly-linked list starting at `node_slot.first_out_edge`
/// and following `EdgeSlot.next_out_edge` until reaching 0 (the NULL sentinel).
/// Deleted edge slots encountered during traversal are silently skipped.
///
/// The iteration reads the slot twice per edge:
/// - Once inside [`get_edge`] to deserialise the full `Edge` value.
/// - Once more to advance the `current_id` pointer to `next_out_edge`.
///
/// This double-read is a known minor inefficiency that avoids introducing a
/// mutable borrow conflict between `get_edge` and the pointer advance.
///
/// # Parameters
/// - `pager`:        Mutable reference to the open [`Pager`].
/// - `node_id`:      The node whose outgoing edges are requested.
/// - `label_filter`: If `Some(label)`, only edges with `edge.label == label` are included.
///   If `None`, all outgoing edges are returned.
///
/// # Returns
/// A `Vec<Edge>` in reverse-insertion order (most recently added edge first,
/// because the list uses head-insert).
///
/// # Errors
/// - [`LielError::NodeNotFound`] if `node_id` is 0, out of range, or deleted.
/// - [`LielError::Io`] / [`LielError::CorruptedFile`] from the pager.
pub fn out_edges(pager: &mut Pager, node_id: u64, label_filter: Option<&str>) -> Result<Vec<Edge>> {
    if node_id == 0 || node_id >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(node_id));
    }
    let node_slot = pager.read_node_slot(node_id)?;
    if node_slot.node_id == 0 || node_slot.is_deleted() {
        return Err(LielError::NodeNotFound(node_id));
    }

    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Start at the head of the out-edge list.  0 (NULL) terminates the walk.
    let mut current_id = node_slot.first_out_edge;
    while current_id != 0 {
        if !seen.insert(current_id) {
            return Err(adjacency_cycle_error("out-edge", node_id, current_id));
        }
        let edge_slot = pager.read_edge_slot(current_id)?;
        if !edge_slot.is_deleted() {
            // Deserialise the full edge (label + properties).
            if let Some(edge) = get_edge(pager, current_id)? {
                // Apply the optional label filter.
                if let Some(filter) = label_filter {
                    if edge.label == filter {
                        result.push(edge);
                    }
                } else {
                    result.push(edge);
                }
            }
        }
        // Re-read the slot to get next_out_edge.  get_edge already read it
        // internally, but we need the pointer value here to advance the cursor.
        let edge_slot2 = pager.read_edge_slot(current_id)?;
        current_id = edge_slot2.next_out_edge;
    }
    Ok(result)
}

/// Return all incoming edges of a node, with an optional label filter.
///
/// Mirrors [`out_edges`] but walks the in-edge linked list:
/// `node_slot.first_in_edge → EdgeSlot.next_in_edge → … → 0`.
///
/// Deleted edges in the list are silently skipped.
///
/// # Parameters
/// - `pager`:        Mutable reference to the open [`Pager`].
/// - `node_id`:      The node whose incoming edges are requested.
/// - `label_filter`: Optional label constraint; `None` returns all in-edges.
///
/// # Returns
/// A `Vec<Edge>` in reverse-insertion order.
///
/// # Errors
/// - [`LielError::NodeNotFound`] if `node_id` is invalid or deleted.
/// - [`LielError::Io`] / [`LielError::CorruptedFile`] from the pager.
pub fn in_edges(pager: &mut Pager, node_id: u64, label_filter: Option<&str>) -> Result<Vec<Edge>> {
    if node_id == 0 || node_id >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(node_id));
    }
    let node_slot = pager.read_node_slot(node_id)?;
    if node_slot.node_id == 0 || node_slot.is_deleted() {
        return Err(LielError::NodeNotFound(node_id));
    }

    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Start at the head of the in-edge list.  0 (NULL) terminates the walk.
    let mut current_id = node_slot.first_in_edge;
    while current_id != 0 {
        if !seen.insert(current_id) {
            return Err(adjacency_cycle_error("in-edge", node_id, current_id));
        }
        let edge_slot = pager.read_edge_slot(current_id)?;
        if !edge_slot.is_deleted() {
            if let Some(edge) = get_edge(pager, current_id)? {
                if let Some(filter) = label_filter {
                    if edge.label == filter {
                        result.push(edge);
                    }
                } else {
                    result.push(edge);
                }
            }
        }
        // Advance to the next edge in the in-list.
        let edge_slot2 = pager.read_edge_slot(current_id)?;
        current_id = edge_slot2.next_in_edge;
    }
    Ok(result)
}

/// The direction along which to traverse edges when looking up neighbours.
///
/// Used by [`neighbors`] to select which adjacency list(s) to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Follow only outgoing edges: returns the `to` endpoints of out-edges.
    Out,
    /// Follow only incoming edges: returns the `from` endpoints of in-edges.
    In,
    /// Follow both directions: returns the union of `Out` and `In` endpoints,
    /// deduplicated by node ID.
    Both,
}

/// Return the IDs of all nodes adjacent to `node_id` along the given direction.
///
/// This is a convenience wrapper over [`out_edges`] and [`in_edges`] that
/// extracts only the neighbour IDs rather than the full `Edge` structs.
///
/// For `Direction::Both`, the two neighbour sets are unioned with deduplication
/// using a `HashSet`; the output order is: `Out` neighbours first, then any
/// additional `In` neighbours not already present.
///
/// # Parameters
/// - `pager`:     Mutable reference to the open [`Pager`].
/// - `node_id`:   The node to query.
/// - `label`:     Optional edge-label filter; `None` matches all labels.
/// - `direction`: Which edge direction(s) to follow.
///
/// # Returns
/// A `Vec<u64>` of neighbour node IDs (deduplicated for `Both`).
///
/// # Errors
/// Propagates errors from [`out_edges`] / [`in_edges`].
pub fn neighbors(
    pager: &mut Pager,
    node_id: u64,
    label: Option<&str>,
    direction: Direction,
) -> Result<Vec<u64>> {
    match direction {
        Direction::Out => {
            // Collect the `to` endpoint of every matching outgoing edge.
            let edges = out_edges(pager, node_id, label)?;
            Ok(edges.into_iter().map(|e| e.to).collect())
        }
        Direction::In => {
            // Collect the `from` endpoint of every matching incoming edge.
            let edges = in_edges(pager, node_id, label)?;
            Ok(edges.into_iter().map(|e| e.from).collect())
        }
        Direction::Both => {
            // Union of out-neighbours and in-neighbours, deduplicated.
            // We use a HashSet to track which IDs have already been emitted so
            // that nodes reachable from both directions appear only once.
            let mut seen = std::collections::HashSet::new();
            let out = out_edges(pager, node_id, label)?;
            let inc = in_edges(pager, node_id, label)?;
            let mut ids = Vec::new();
            for e in out {
                if seen.insert(e.to) {
                    ids.push(e.to);
                }
            }
            for e in inc {
                if seen.insert(e.from) {
                    ids.push(e.from);
                }
            }
            Ok(ids)
        }
    }
}

/// Return every active (non-deleted) edge in the database.
///
/// Scans all allocated edge IDs from 1 to `max_edge_id()` and calls
/// [`get_edge`] for each.  Deleted or unallocated slots are silently skipped.
///
/// This is an O(N) full scan over ever-allocated edge slots, which may be
/// slow on databases with many deletions.  Use [`out_edges`] / [`in_edges`]
/// for targeted adjacency lookups instead.
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
///
/// # Returns
/// A `Vec<Edge>` containing all active edges, in ascending ID order.
///
/// # Errors
/// Propagates [`LielError::Io`] / [`LielError::CorruptedFile`] from the pager.
pub fn all_edges(pager: &mut Pager) -> Result<Vec<Edge>> {
    let max_id = pager.max_edge_id();
    let mut edges = Vec::new();
    for edge_id in 1..=max_id {
        if let Some(edge) = get_edge(pager, edge_id)? {
            edges.push(edge);
        }
    }
    Ok(edges)
}

/// Merge new properties into an existing edge's property map and persist the result.
///
/// Follows the same append-on-write strategy as
/// [`update_node`](crate::graph::node::update_node):
///
/// 1. Read and deserialise the current property map from the property storage.
/// 2. Insert/overwrite keys from `new_props` (shallow merge).
/// 3. Serialise the merged map and append it to the property storage.
/// 4. Update `slot.prop_offset` and `slot.prop_length` to reference the new bytes.
///
/// The old bytes become dead data reclaimed by a future
/// [`vacuum`](crate::graph::vacuum::vacuum) call.
///
/// Note: the edge's `label` is **not** changed by this function.  Labels are
/// immutable after creation because they identify the relationship type.
///
/// # Parameters
/// - `pager`:     Mutable reference to the open [`Pager`].
/// - `edge_id`:   The ID of the edge to update.
/// - `new_props`: Properties to merge into the existing map.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// - [`LielError::EdgeNotFound`] if `edge_id` is 0, out of range, or deleted.
/// - [`LielError::Io`] / [`LielError::CorruptedFile`] from the pager.
pub fn update_edge(
    pager: &mut Pager,
    edge_id: u64,
    new_props: HashMap<String, PropValue>,
) -> Result<()> {
    if edge_id == 0 || edge_id >= pager.header.next_edge_id {
        return Err(LielError::EdgeNotFound(edge_id));
    }
    let mut slot = pager.read_edge_slot(edge_id)?;
    if slot.edge_id == 0 || slot.is_deleted() {
        return Err(LielError::EdgeNotFound(edge_id));
    }

    // Read the existing property map so we can merge into it.
    let mut existing = if slot.prop_length > 0 {
        let bytes = pager.read_prop(slot.prop_offset, slot.prop_length)?;
        decode_props(&bytes)?
    } else {
        HashMap::new()
    };
    for (k, v) in new_props {
        existing.insert(k, v);
    }

    // Append the merged bytes to the property storage and update the slot pointer.
    let (prop_offset, prop_length) = if existing.is_empty() {
        (0, 0)
    } else {
        let prop_bytes = encode_props(&existing);
        let offset = pager.append_prop(&prop_bytes)?;
        (offset, prop_bytes.len() as u32)
    };
    slot.prop_offset = prop_offset;
    slot.prop_length = prop_length;
    pager.write_edge_slot(&slot)?;
    Ok(())
}

/// Find an existing edge matching (from, label, to, props) or create it if absent.
///
/// This is a "get or create" / upsert operation useful for graph construction
/// patterns where you want to ensure at most one edge exists with a given
/// combination of endpoints, label, and property values.
///
/// The search iterates over the out-edge list of `from` filtered by `label`,
/// looking for a match where both `edge.to == to` *and* `edge.properties == props`.
/// If multiple edges satisfy that condition (legal in a multigraph), the first
/// one found (i.e. the most recently inserted, due to head-insert order) is
/// returned.
///
/// If no match is found, [`add_edge`] is called and the newly created edge is
/// returned.
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
/// - `from`:  Source node ID.
/// - `label`: Relationship type label to match or create.
/// - `to`:    Destination node ID.
/// - `props`: Property map that must match exactly, or will be set on creation.
///
/// # Returns
/// The existing matching [`Edge`] (if found) or a newly created [`Edge`].
///
/// # Errors
/// Propagates errors from [`out_edges`] or [`add_edge`].
pub fn merge_edge(
    pager: &mut Pager,
    from: u64,
    label: String,
    to: u64,
    props: HashMap<String, PropValue>,
) -> Result<Edge> {
    // Search the from-node's out-list for an edge that matches label + to + props.
    let edges = out_edges(pager, from, Some(&label))?;
    for edge in edges {
        if edge.to == to && edge.properties == props {
            return Ok(edge);
        }
    }
    // No match found — create a new edge.
    add_edge(pager, from, label, to, props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::node::add_node;
    use crate::storage::pager::Pager;

    fn make_db() -> Pager {
        Pager::open(":memory:").unwrap()
    }

    #[test]
    fn test_add_and_get_edge() {
        let mut pager = make_db();
        let alice = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let bob = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let mut props = HashMap::new();
        props.insert("since".into(), PropValue::Int(2024));
        let edge = add_edge(&mut pager, alice.id, "KNOWS".into(), bob.id, props).unwrap();
        let fetched = get_edge(&mut pager, edge.id).unwrap().unwrap();
        assert_eq!(fetched.label, "KNOWS");
        assert_eq!(fetched.from, alice.id);
        assert_eq!(fetched.to, bob.id);
        match &fetched.properties["since"] {
            PropValue::Int(n) => assert_eq!(*n, 2024),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn test_out_edge_linked_list_single() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let edge = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let node_slot = pager.read_node_slot(a.id).unwrap();
        assert_eq!(node_slot.first_out_edge, edge.id);
        let edge_slot = pager.read_edge_slot(edge.id).unwrap();
        assert_eq!(edge_slot.next_out_edge, 0);
    }

    #[test]
    fn test_out_edge_linked_list_multiple() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), c.id, HashMap::new()).unwrap();
        let e3 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        // Head-insert means the list order is reverse-insertion: e3 → e2 → e1
        let node_slot = pager.read_node_slot(a.id).unwrap();
        assert_eq!(node_slot.first_out_edge, e3.id);
        let slot3 = pager.read_edge_slot(e3.id).unwrap();
        assert_eq!(slot3.next_out_edge, e2.id);
        let slot2 = pager.read_edge_slot(e2.id).unwrap();
        assert_eq!(slot2.next_out_edge, e1.id);
        let slot1 = pager.read_edge_slot(e1.id).unwrap();
        assert_eq!(slot1.next_out_edge, 0);
    }

    #[test]
    fn test_delete_edge_head() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), c.id, HashMap::new()).unwrap();
        // e2 is at the head of the list (head-insert); deleting it should make e1 the new head.
        delete_edge(&mut pager, e2.id).unwrap();
        let node_slot = pager.read_node_slot(a.id).unwrap();
        assert_eq!(node_slot.first_out_edge, e1.id);
    }

    #[test]
    fn test_delete_edge_middle() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e3 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        // List is: e3 → e2 → e1.  Deleting the middle element e2 should give e3 → e1.
        delete_edge(&mut pager, e2.id).unwrap();
        let slot3 = pager.read_edge_slot(e3.id).unwrap();
        assert_eq!(slot3.next_out_edge, e1.id);
    }

    #[test]
    fn test_delete_edge_tail() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        // List is: e2 → e1.  Deleting the tail e1 should give e2 → 0 (NULL).
        delete_edge(&mut pager, e1.id).unwrap();
        let slot2 = pager.read_edge_slot(e2.id).unwrap();
        assert_eq!(slot2.next_out_edge, 0);
    }

    #[test]
    fn test_out_degree_updates() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let _e2 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let slot = pager.read_node_slot(a.id).unwrap();
        assert_eq!(slot.out_degree, 2);
        delete_edge(&mut pager, e1.id).unwrap();
        let slot = pager.read_node_slot(a.id).unwrap();
        assert_eq!(slot.out_degree, 1);
    }

    #[test]
    fn test_multigraph() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "KNOWS".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "KNOWS".into(), b.id, HashMap::new()).unwrap();
        assert_ne!(e1.id, e2.id);
        let edges = out_edges(&mut pager, a.id, Some("KNOWS")).unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_self_loop_is_visible_in_both_adjacency_lists() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();

        let edge = add_edge(&mut pager, a.id, "SELF".into(), a.id, HashMap::new()).unwrap();

        let out = out_edges(&mut pager, a.id, None).unwrap();
        let inc = in_edges(&mut pager, a.id, None).unwrap();
        let slot = pager.read_node_slot(a.id).unwrap();

        assert_eq!(out.len(), 1);
        assert_eq!(inc.len(), 1);
        assert_eq!(out[0].id, edge.id);
        assert_eq!(inc[0].id, edge.id);
        assert_eq!(slot.first_out_edge, edge.id);
        assert_eq!(slot.first_in_edge, edge.id);
        assert_eq!(slot.out_degree, 1);
        assert_eq!(slot.in_degree, 1);
    }

    #[test]
    fn test_delete_self_loop_updates_both_lists() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();

        let edge = add_edge(&mut pager, a.id, "SELF".into(), a.id, HashMap::new()).unwrap();
        delete_edge(&mut pager, edge.id).unwrap();

        let out = out_edges(&mut pager, a.id, None).unwrap();
        let inc = in_edges(&mut pager, a.id, None).unwrap();
        let slot = pager.read_node_slot(a.id).unwrap();

        assert!(out.is_empty());
        assert!(inc.is_empty());
        assert_eq!(slot.first_out_edge, 0);
        assert_eq!(slot.first_in_edge, 0);
        assert_eq!(slot.out_degree, 0);
        assert_eq!(slot.in_degree, 0);
    }

    #[test]
    fn test_out_edges_detects_adjacency_cycle() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();

        let mut slot1 = pager.read_edge_slot(e1.id).unwrap();
        slot1.next_out_edge = e2.id;
        pager.write_edge_slot(&slot1).unwrap();

        let err = out_edges(&mut pager, a.id, None).unwrap_err();
        match err {
            LielError::CorruptedFile(msg) => {
                assert!(msg.contains("cycle"));
                assert!(msg.contains("repair_adjacency()"));
            }
            other => panic!("expected CorruptedFile, got {other:?}"),
        }
    }

    #[test]
    fn test_in_edges_detects_adjacency_cycle() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let e1 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let e2 = add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();

        let mut slot1 = pager.read_edge_slot(e1.id).unwrap();
        slot1.next_in_edge = e2.id;
        pager.write_edge_slot(&slot1).unwrap();

        let err = in_edges(&mut pager, b.id, None).unwrap_err();
        match err {
            LielError::CorruptedFile(msg) => {
                assert!(msg.contains("cycle"));
                assert!(msg.contains("repair_adjacency()"));
            }
            other => panic!("expected CorruptedFile, got {other:?}"),
        }
    }
}

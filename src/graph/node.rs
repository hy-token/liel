use crate::error::{LielError, Result};
use crate::storage::pager::Pager;
use crate::storage::prop_codec::{
    decode_labels, decode_props, encode_labels, encode_props, PropValue,
};
use crate::storage::serializer::{NodeSlot, FLAG_DELETED};
use std::collections::HashMap;

/// A node in the liel graph database.
///
/// A `Node` is the fundamental entity in the graph model.  It carries:
///
/// - A unique 64-bit `id` assigned by the database (1-based; 0 is the NULL
///   sentinel and is never issued to the caller).
/// - Zero or more string `labels` that categorise the node (e.g. `"Person"`,
///   `"Company"`).  Labels are stored in the property storage as a serialised
///   `Vec<String>` using the custom [`prop_codec`] encoding.
/// - A map of key→value `properties` that store arbitrary typed data on the
///   node.  Supported value types are defined by [`PropValue`].
///
/// `Node` is a purely in-memory view; it is created by reading from storage and
/// does not hold any file handle or reference to the pager.  Mutations must be
/// persisted back via [`update_node`] or [`delete_node`].
#[derive(Debug, Clone)]
pub struct Node {
    /// The unique identifier for this node.
    ///
    /// IDs are allocated sequentially starting at 1.  ID 0 is reserved as the
    /// NULL sentinel value inside the on-disk linked-list pointers; it is never
    /// returned to callers.
    pub id: u64,

    /// The set of type labels attached to this node.
    ///
    /// Labels are free-form strings; the database imposes no uniqueness or
    /// vocabulary constraints.  A node may have zero labels, one label, or
    /// multiple labels simultaneously.
    pub labels: Vec<String>,

    /// Arbitrary key-value properties stored on this node.
    ///
    /// Keys are UTF-8 strings.  Values may be `Null`, `Bool`, `Int`, `Float`,
    /// `String`, `List`, or `Map` as defined by [`PropValue`].  An empty map
    /// means no properties are stored; the on-disk representation uses (0, 0)
    /// for (offset, length) in that case.
    pub properties: HashMap<String, PropValue>,
}

/// Create a new node and persist it to the pager.
///
/// This is the primary write path for nodes.  The function:
///
/// 1. Allocates a new node ID by atomically incrementing `pager.header.next_node_id`.
/// 2. Serialises `labels` using [`encode_labels`] and appends the bytes to the
///    property storage via [`Pager::append_prop`].  If there are no labels, the
///    label offset/length in the slot are both set to 0.
/// 3. Serialises `props` using [`encode_props`] and appends them the same way.
///    If `props` is empty, the property offset/length are both 0.
/// 4. Writes a [`NodeSlot`] to the node page region, with:
///    - `first_out_edge = 0` (no outgoing edges yet)
///    - `first_in_edge  = 0` (no incoming edges yet)
///    - `out_degree = 0`, `in_degree = 0`
///    - `flags = 0` (active, not deleted)
/// 5. Increments the live-node counter in the file header.
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
/// - `labels`: Classification strings to attach to this node.  May be empty.
/// - `props`:  Initial property key-value map.  May be empty.
///
/// # Returns
/// A [`Node`] value reflecting the persisted state, including the newly
/// assigned `id`.
///
/// # Errors
/// Propagates any [`LielError::Io`] produced by the pager during slot writes
/// or property-area appends.
pub fn add_node(
    pager: &mut Pager,
    labels: Vec<String>,
    props: HashMap<String, PropValue>,
) -> Result<Node> {
    let node_id = pager.alloc_node_id()?;

    // Serialise the label list and append it to the property storage.
    // Empty labels are represented in the slot as (0, 0) and write nothing.
    let (label_offset, label_length) = if labels.is_empty() {
        (0, 0)
    } else {
        let label_bytes = encode_labels(&labels);
        let offset = pager.append_prop(&label_bytes)?;
        (offset, label_bytes.len() as u32)
    };

    // Serialise the property map and append it after the labels.
    // Again, an empty map produces no bytes on disk — the slot records (0, 0).
    let (prop_offset, prop_length) = if props.is_empty() {
        (0, 0)
    } else {
        let prop_bytes = encode_props(&props);
        let offset = pager.append_prop(&prop_bytes)?;
        (offset, prop_bytes.len() as u32)
    };

    // Build the fixed-size NodeSlot (64 bytes) and write it to the node-page
    // region.  Both adjacency-list head pointers start at 0 (the NULL sentinel),
    // meaning this node has no edges yet.
    let slot = NodeSlot {
        node_id,
        first_out_edge: 0,
        first_in_edge: 0,
        prop_offset,
        prop_length,
        out_degree: 0,
        in_degree: 0,
        label_offset,
        label_length,
        flags: 0,
    };
    pager.write_node_slot(&slot)?;
    pager.increment_node_count();

    Ok(Node {
        id: node_id,
        labels,
        properties: props,
    })
}

/// Retrieve a node by ID, returning `None` if it does not exist or is deleted.
///
/// This is the primary read path for nodes.  The function:
///
/// 1. Performs a fast range check: IDs ≤ 0 or ≥ `next_node_id` are outside the
///    allocated space and immediately return `Ok(None)`.
/// 2. Reads the [`NodeSlot`] from the node-page region at the slot index
///    corresponding to `node_id`.
/// 3. Returns `Ok(None)` if the slot is logically empty (`node_id == 0`) or
///    has the `FLAG_DELETED` bit set.
/// 4. Reads the label bytes from the property storage (if `label_length > 0`) and
///    deserialises them with [`decode_labels`].
/// 5. Reads the property bytes from the property storage (if `prop_length > 0`)
///    and deserialises them with [`decode_props`].
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`]; mutable because reads
///   update the pager's internal LRU cache.
/// - `node_id`: The ID to look up.
///
/// # Returns
/// - `Ok(Some(Node))` if the node exists and is active.
/// - `Ok(None)` if the ID is out of range or the node was deleted.
/// - `Err(LielError::Io)` if a storage read fails.
/// - `Err(LielError::CorruptedFile)` if the property bytes cannot be decoded.
pub fn get_node(pager: &mut Pager, node_id: u64) -> Result<Option<Node>> {
    // IDs must be in [1, next_node_id).  0 is the NULL sentinel; anything at or
    // above next_node_id has never been allocated.
    if node_id == 0 || node_id >= pager.header.next_node_id {
        return Ok(None);
    }
    let slot = pager.read_node_slot(node_id)?;
    if slot.node_id == 0 || slot.is_deleted() {
        return Ok(None);
    }

    // Deserialise labels from the property storage.  A length of 0 means no labels
    // were stored for this node; skip the read and return an empty Vec.
    let labels = if slot.label_length > 0 {
        let bytes = pager.read_prop(slot.label_offset, slot.label_length)?;
        decode_labels(&bytes)?
    } else {
        Vec::new()
    };

    // Deserialise the property map from the property storage.  A length of 0 means
    // no properties; skip the read and return an empty HashMap.
    let properties = if slot.prop_length > 0 {
        let bytes = pager.read_prop(slot.prop_offset, slot.prop_length)?;
        decode_props(&bytes)?
    } else {
        HashMap::new()
    };

    Ok(Some(Node {
        id: node_id,
        labels,
        properties,
    }))
}

/// Mark a node as deleted in the storage layer.
///
/// Deletion is *logical*: the slot is not zeroed out; instead, the
/// `FLAG_DELETED` bit is set in `slot.flags`.  Subsequent calls to
/// [`get_node`] will see the flag and return `Ok(None)` as if the node never
/// existed.  The slot index is never reused (by design, matching the
/// "no slot reuse" policy documented in `docs/design/product-tradeoffs.ja.md` §5.3).
///
/// **Caller responsibility**: All edges incident to this node (both outgoing
/// and incoming) must be deleted *before* calling this function.  Failing to
/// do so leaves dangling edge-slot pointers that reference a deleted node,
/// which will cause incorrect results during adjacency-list traversal.
///
/// # Parameters
/// - `pager`:   Mutable reference to the open [`Pager`].
/// - `node_id`: The ID of the node to delete.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// - [`LielError::NodeNotFound`] if `node_id` is 0, out of range, or the slot
///   is already deleted.
/// - [`LielError::Io`] if the slot write fails.
pub fn delete_node(pager: &mut Pager, node_id: u64) -> Result<()> {
    if node_id == 0 || node_id >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(node_id));
    }
    let mut slot = pager.read_node_slot(node_id)?;
    if slot.node_id == 0 || slot.is_deleted() {
        return Err(LielError::NodeNotFound(node_id));
    }
    // Set the deleted flag.  The slot remains on disk; it is simply invisible
    // to all subsequent reads.  vacuum() can reclaim the property-area space
    // later.
    slot.flags |= FLAG_DELETED;
    pager.write_node_slot(&slot)?;
    pager.decrement_node_count();
    Ok(())
}

/// Return every active (non-deleted) node in the database.
///
/// Scans all allocated node IDs in order from 1 to `max_node_id()` and
/// calls [`get_node`] for each.  Deleted slots are silently skipped.
///
/// This is an O(N) full scan where N is the number of *ever-allocated* node
/// slots (not just live nodes), because slot IDs are never reused.  For large
/// databases with many deletions, consider using index-based traversal or
/// calling [`vacuum`] to compact the file first.
///
/// # Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
///
/// # Returns
/// A `Vec<Node>` containing all active nodes, in ascending ID order.
///
/// # Errors
/// Propagates any [`LielError::Io`] or [`LielError::CorruptedFile`] produced
/// by the underlying slot/property reads.
pub fn all_nodes(pager: &mut Pager) -> Result<Vec<Node>> {
    let max_id = pager.max_node_id();
    let mut nodes = Vec::new();
    for node_id in 1..=max_id {
        // get_node returns Ok(None) for deleted or unallocated slots, so the
        // if-let silently skips those without error.
        if let Some(node) = get_node(pager, node_id)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}

/// Merge new properties into an existing node's property map and persist the result.
///
/// The update strategy is an *append-on-write* / *copy-on-write* pattern:
///
/// 1. Read the current serialised property bytes from the property storage.
/// 2. Deserialise them into a `HashMap`.
/// 3. Insert every key-value pair from `new_props`, overwriting any existing
///    key with the same name (a shallow merge, not a deep merge).
/// 4. Re-serialise the merged map and *append* the new bytes to the property
///    area — the old bytes are **not** overwritten or freed.
/// 5. Update the slot's `prop_offset` and `prop_length` to point to the newly
///    appended data.
///
/// The old property bytes remain on disk as unreachable dead data until
/// [`vacuum`] is called.  This matches the "no slot reuse" policy: we never
/// overwrite existing data in-place; we always append and update the pointer.
///
/// # Parameters
/// - `pager`:     Mutable reference to the open [`Pager`].
/// - `node_id`:   The ID of the node to update.
/// - `new_props`: Key-value pairs to merge into the existing property map.
///   Any key already present on the node is overwritten.
///
/// # Returns
/// `Ok(())` on success.
///
/// # Errors
/// - [`LielError::NodeNotFound`] if `node_id` is 0, out of range, or deleted.
/// - [`LielError::Io`] / [`LielError::CorruptedFile`] from the pager.
pub fn update_node(
    pager: &mut Pager,
    node_id: u64,
    new_props: HashMap<String, PropValue>,
) -> Result<()> {
    if node_id == 0 || node_id >= pager.header.next_node_id {
        return Err(LielError::NodeNotFound(node_id));
    }
    let mut slot = pager.read_node_slot(node_id)?;
    if slot.node_id == 0 || slot.is_deleted() {
        return Err(LielError::NodeNotFound(node_id));
    }

    // Read the current property map from storage so we can merge into it.
    // If prop_length is 0 the node has no current properties; start from an
    // empty map.
    let mut existing = if slot.prop_length > 0 {
        let bytes = pager.read_prop(slot.prop_offset, slot.prop_length)?;
        decode_props(&bytes)?
    } else {
        HashMap::new()
    };
    // Overwrite matching keys; add new keys.  Keys in `existing` that are NOT
    // present in `new_props` are preserved unchanged.
    for (k, v) in new_props {
        existing.insert(k, v);
    }

    // Append the merged property bytes to the property storage.  The old bytes at
    // (slot.prop_offset, slot.prop_length) become dead data and will be
    // reclaimed by vacuum().  This is consistent with the deleted-slot
    // no-reuse policy: we never overwrite existing live data.
    let (prop_offset, prop_length) = if existing.is_empty() {
        (0, 0)
    } else {
        let prop_bytes = encode_props(&existing);
        let offset = pager.append_prop(&prop_bytes)?;
        (offset, prop_bytes.len() as u32)
    };
    slot.prop_offset = prop_offset;
    slot.prop_length = prop_length;
    pager.write_node_slot(&slot)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::pager::Pager;

    #[test]
    fn test_add_and_get_node() {
        let mut pager = Pager::open(":memory:").unwrap();
        let mut props = HashMap::new();
        props.insert("name".into(), PropValue::String("Alice".into()));
        let node = add_node(&mut pager, vec!["Person".into()], props).unwrap();
        assert_eq!(node.id, 1);

        let fetched = get_node(&mut pager, 1).unwrap().unwrap();
        assert_eq!(fetched.id, 1);
        assert!(fetched.labels.contains(&"Person".to_string()));
        match &fetched.properties["name"] {
            PropValue::String(s) => assert_eq!(s, "Alice"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn test_node_id_starts_at_one() {
        let mut pager = Pager::open(":memory:").unwrap();
        let node = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        assert_eq!(node.id, 1);
    }

    #[test]
    fn test_node_id_increments() {
        let mut pager = Pager::open(":memory:").unwrap();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 2);
        assert_eq!(c.id, 3);
    }

    #[test]
    fn test_get_nonexistent_node() {
        let mut pager = Pager::open(":memory:").unwrap();
        assert!(get_node(&mut pager, 9999).unwrap().is_none());
    }

    #[test]
    fn test_delete_node_returns_none() {
        let mut pager = Pager::open(":memory:").unwrap();
        let node = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        delete_node(&mut pager, node.id).unwrap();
        assert!(get_node(&mut pager, node.id).unwrap().is_none());
    }

    #[test]
    fn test_multiple_labels() {
        let mut pager = Pager::open(":memory:").unwrap();
        let node = add_node(
            &mut pager,
            vec!["Person".into(), "Employee".into()],
            HashMap::new(),
        )
        .unwrap();
        let fetched = get_node(&mut pager, node.id).unwrap().unwrap();
        assert!(fetched.labels.contains(&"Person".to_string()));
        assert!(fetched.labels.contains(&"Employee".to_string()));
    }

    #[test]
    fn test_empty_labels_and_props_use_zero_offsets() {
        let mut pager = Pager::open(":memory:").unwrap();
        let node = add_node(&mut pager, vec![], HashMap::new()).unwrap();

        let slot = pager.read_node_slot(node.id).unwrap();
        assert_eq!(slot.label_offset, 0);
        assert_eq!(slot.label_length, 0);
        assert_eq!(slot.prop_offset, 0);
        assert_eq!(slot.prop_length, 0);
    }
}

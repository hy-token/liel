use crate::error::{LielError, Result};

/// Fixed binary layout of a `NodeSlot` on disk — exactly 64 bytes.
///
/// Every node ever created occupies one slot in one of the node extents
/// managed by [`crate::storage::pager::Pager`].  Slots are written at 1-based
/// node IDs; the mapping from `node_id` to a file offset is:
///
/// ```text
/// extent_idx   = (node_id - 1) / NODES_PER_EXTENT   (= 16 128 nodes per extent)
/// in_extent    = (node_id - 1) % NODES_PER_EXTENT
/// page_index   = in_extent / NODES_PER_PAGE         (= 63 nodes per page)
/// slot_index   = in_extent % NODES_PER_PAGE
/// file_offset  = node_extents[extent_idx]
///              + page_index * 4096
///              + 8                                   (per-page header, reserved)
///              + slot_index * 64
/// ```
///
/// Extent offsets are stored in a linked list of index pages whose head is
/// recorded in `FileHeader::node_table_head`; see the pager documentation for
/// the persistence format.
///
/// # Byte layout
/// ```text
/// Offset   Field           Type   Size   Notes
/// ──────────────────────────────────────────────
///  0- 7   node_id         u64    8 B    1-based; 0 is the NULL sentinel
///  8-15   first_out_edge  u64    8 B    head of the outgoing adjacency list
/// 16-23   first_in_edge   u64    8 B    head of the incoming adjacency list
/// 24-31   prop_offset     u64    8 B    absolute file offset of the property blob (prop extents)
/// 32-35   prop_length     u32    4 B    byte length of the serialised props
/// 36-39   (padding)              4 B    always zero
/// 40-43   out_degree      u32    4 B    number of outgoing edges
/// 44-47   in_degree       u32    4 B    number of incoming edges
/// 48-55   label_offset    u64    8 B    absolute file offset of the serialised label list
/// 56-59   label_length    u32    4 B    byte length of the serialised label list
/// 60      flags           u8     1 B    FLAG_DELETED (0x01) or FLAG_ACTIVE (0x00)
/// 61-63   (reserved)             3 B    always zero; reserved for future use
/// ```
pub const NODE_SLOT_SIZE: usize = 64;

/// Fixed binary layout of an `EdgeSlot` on disk — exactly 80 bytes.
///
/// Every edge ever created occupies one slot in the edge-page region.  The
/// mapping from `edge_id` to a file offset follows the same pattern as nodes
/// but uses `EDGES_PER_PAGE` (= 51) and `EDGE_SLOT_SIZE` (= 80).
///
/// Edges form two independent singly-linked lists through `next_out_edge` and
/// `next_in_edge`.  The head pointers (`first_out_edge`, `first_in_edge`) are
/// stored in the source and target `NodeSlot` respectively.  `0` is the NULL
/// sentinel that terminates a list.
///
/// # Byte layout
/// ```text
/// Offset   Field           Type   Size   Notes
/// ──────────────────────────────────────────────
///  0- 7   edge_id         u64    8 B    1-based; 0 is the NULL sentinel
///  8-15   from_node_id    u64    8 B    source node
/// 16-23   to_node_id      u64    8 B    target node
/// 24-31   next_out_edge   u64    8 B    next edge in from_node's out-list
/// 32-39   next_in_edge    u64    8 B    next edge in to_node's in-list
/// 40-47   prop_offset     u64    8 B    absolute file offset of the property blob (prop extents)
/// 48-51   prop_length     u32    4 B    byte length of the serialised props
/// 52-55   (padding)              4 B    always zero
/// 56-63   label_offset    u64    8 B    absolute file offset of the serialised edge label
/// 64-67   label_length    u32    4 B    byte length of the serialised edge label
/// 68      flags           u8     1 B    FLAG_DELETED (0x01) or FLAG_ACTIVE (0x00)
/// 69-79   (reserved)             11 B   always zero; reserved for future use
/// ```
pub const EDGE_SLOT_SIZE: usize = 80;

/// Bit flag set in the `flags` byte to mark a slot as logically deleted.
///
/// liel never reuses or compacts slots (by design — see `docs/design/product-tradeoffs.ja.md` §5.3).
/// A deleted slot keeps its old field values but has this bit set.  Any code
/// that iterates over all slots must filter out entries where
/// `flags & FLAG_DELETED != 0`.
pub const FLAG_DELETED: u8 = 0x01;

/// Bit flag value indicating a live, non-deleted slot.
///
/// A freshly-written slot always starts with `flags = FLAG_ACTIVE`.
pub const FLAG_ACTIVE: u8 = 0x00;

/// In-memory representation of a single node slot.
///
/// This struct is a direct mirror of the 64-byte on-disk layout described in
/// `NODE_SLOT_SIZE`.  The `Pager` serialises it into a `[u8; 64]` buffer via
/// [`write_to`](NodeSlot::write_to) and deserialises it via
/// [`read_from`](NodeSlot::read_from).
///
/// The adjacency lists (`first_out_edge`, `first_in_edge`) use head-insertion
/// ordering: the most recently added edge is always at the front of the list.
/// Traversal follows the `next_out_edge` / `next_in_edge` chains in the
/// corresponding `EdgeSlot`s until a `0` (NULL sentinel) is reached.
#[derive(Debug, Clone, Default)]
pub struct NodeSlot {
    /// The unique node identifier (1-based).  `0` is the NULL sentinel and
    /// must never appear in a live slot.
    pub node_id: u64,
    /// Edge ID of the first edge in this node's outgoing adjacency list.
    /// `0` means the list is empty.  The list is singly-linked through
    /// `EdgeSlot::next_out_edge`.
    pub first_out_edge: u64,
    /// Edge ID of the first edge in this node's incoming adjacency list.
    /// `0` means the list is empty.  The list is singly-linked through
    /// `EdgeSlot::next_in_edge`.
    pub first_in_edge: u64,
    /// Absolute file offset where this node's serialised property map begins
    /// (a blob inside prop extents).  `0` with `prop_length == 0` means no properties.
    pub prop_offset: u64,
    /// Byte length of this node's serialised property map.
    /// `0` means no properties are stored.
    pub prop_length: u32,
    /// Number of outgoing edges attached to this node.
    /// Kept in sync with insertions and deletions; used by traversal code to
    /// pre-allocate result vectors without scanning the list first.
    pub out_degree: u32,
    /// Number of incoming edges attached to this node.
    pub in_degree: u32,
    /// Absolute file offset where this node's serialised label list begins.
    /// `0` with `label_length == 0` means no labels.
    pub label_offset: u64,
    /// Byte length of this node's serialised label list.
    pub label_length: u32,
    /// Status flags.  Currently only `FLAG_DELETED` (0x01) is defined.
    pub flags: u8,
}

impl NodeSlot {
    /// Return `true` if the `FLAG_DELETED` bit is set in `flags`.
    ///
    /// Deleted nodes are retained in the file forever (no slot reuse) so that
    /// IDs already handed out to the application remain stable.  Callers that
    /// iterate over all node IDs must call this method and skip deleted slots.
    pub fn is_deleted(&self) -> bool {
        self.flags & FLAG_DELETED != 0
    }

    /// Return `true` if this slot represents a live, accessible node.
    ///
    /// A slot is considered active when it is not deleted *and* its `node_id`
    /// is non-zero.  A freshly-zeroed page will have `node_id == 0`, which
    /// should never be returned to the user; this check guards against that.
    pub fn is_active(&self) -> bool {
        !self.is_deleted() && self.node_id != 0
    }

    /// Serialise this `NodeSlot` into the given 64-byte buffer.
    ///
    /// All multi-byte integers are written in little-endian byte order, which
    /// matches the file format specification.  Padding and reserved bytes are
    /// written as zeroes so that future readers see a clean state.
    ///
    /// # Parameters
    /// - `buf`: Exactly 64 bytes of mutable storage.  The caller is
    ///   responsible for positioning this buffer within the correct page.
    pub fn write_to(&self, buf: &mut [u8; NODE_SLOT_SIZE]) {
        buf[0..8].copy_from_slice(&self.node_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.first_out_edge.to_le_bytes());
        buf[16..24].copy_from_slice(&self.first_in_edge.to_le_bytes());
        buf[24..32].copy_from_slice(&self.prop_offset.to_le_bytes());
        buf[32..36].copy_from_slice(&self.prop_length.to_le_bytes());
        buf[36..40].copy_from_slice(&0u32.to_le_bytes()); // 4 bytes of explicit padding — always zero
        buf[40..44].copy_from_slice(&self.out_degree.to_le_bytes());
        buf[44..48].copy_from_slice(&self.in_degree.to_le_bytes());
        buf[48..56].copy_from_slice(&self.label_offset.to_le_bytes());
        buf[56..60].copy_from_slice(&self.label_length.to_le_bytes());
        buf[60] = self.flags;
        buf[61..64].copy_from_slice(&[0u8; 3]); // 3 bytes reserved for future fields — always zero
    }

    /// Deserialise a `NodeSlot` from the given 64-byte buffer.
    ///
    /// All multi-byte integers are read as little-endian.  Padding and
    /// reserved bytes are silently ignored so that future file format
    /// extensions are backward-compatible.
    ///
    /// # Parameters
    /// - `buf`: Exactly 64 bytes previously written by `write_to`.
    ///
    /// # Returns
    /// A fully populated `NodeSlot`.  No validation is performed here; the
    /// caller must check `is_deleted()` and `is_active()` as appropriate.
    pub fn read_from(buf: &[u8; NODE_SLOT_SIZE]) -> Self {
        Self {
            node_id: u64::from_le_bytes(
                buf[0..8]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            first_out_edge: u64::from_le_bytes(
                buf[8..16]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            first_in_edge: u64::from_le_bytes(
                buf[16..24]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            prop_offset: u64::from_le_bytes(
                buf[24..32]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            prop_length: u32::from_le_bytes(
                buf[32..36]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            // bytes 36-39 are padding; skip them
            out_degree: u32::from_le_bytes(
                buf[40..44]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            in_degree: u32::from_le_bytes(
                buf[44..48]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            label_offset: u64::from_le_bytes(
                buf[48..56]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            label_length: u32::from_le_bytes(
                buf[56..60]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            flags: buf[60],
            // bytes 61-63 are reserved; not stored in the struct
        }
    }
}

/// In-memory representation of a single edge slot.
///
/// This struct mirrors the 80-byte on-disk layout described in
/// `EDGE_SLOT_SIZE`.  Like `NodeSlot`, it is serialised via
/// [`write_to`](EdgeSlot::write_to) and deserialised via
/// [`read_from`](EdgeSlot::read_from).
///
/// An edge participates in *two* singly-linked adjacency lists simultaneously:
/// - The **outgoing** list rooted at `from_node.first_out_edge`, linked
///   through `next_out_edge`.
/// - The **incoming** list rooted at `to_node.first_in_edge`, linked through
///   `next_in_edge`.
///
/// Both lists use head-insertion, so the most recently added edge appears
/// first.  When an edge is deleted, both link chains must be repaired so that
/// subsequent traversals skip the deleted slot.
#[derive(Debug, Clone, Default)]
pub struct EdgeSlot {
    /// The unique edge identifier (1-based).  `0` is the NULL sentinel.
    pub edge_id: u64,
    /// ID of the node at the tail (source) of this directed edge.
    pub from_node_id: u64,
    /// ID of the node at the head (target) of this directed edge.
    pub to_node_id: u64,
    /// Edge ID of the next edge in `from_node`'s outgoing adjacency list.
    /// `0` terminates the list.
    pub next_out_edge: u64,
    /// Edge ID of the next edge in `to_node`'s incoming adjacency list.
    /// `0` terminates the list.
    pub next_in_edge: u64,
    /// Absolute file offset where this edge's serialised property map begins.
    pub prop_offset: u64,
    /// Byte length of this edge's serialised property map.
    pub prop_length: u32,
    /// Absolute file offset where this edge's serialised label
    /// (a single string encoded as a `PropValue::List`) begins.
    pub label_offset: u64,
    /// Byte length of this edge's serialised label.
    pub label_length: u32,
    /// Status flags.  Currently only `FLAG_DELETED` (0x01) is defined.
    pub flags: u8,
}

impl EdgeSlot {
    /// Return `true` if the `FLAG_DELETED` bit is set in `flags`.
    ///
    /// Deleted edges must be skipped during adjacency-list traversal.  The
    /// `Pager` does not reuse edge-slot space after deletion.  Orphaned
    /// label/property bytes can be compacted with [`crate::graph::vacuum::vacuum`].
    pub fn is_deleted(&self) -> bool {
        self.flags & FLAG_DELETED != 0
    }

    /// Return `true` if this slot represents a live, accessible edge.
    ///
    /// Guards against reading a freshly-zeroed page region where `edge_id`
    /// would be `0`.
    pub fn is_active(&self) -> bool {
        !self.is_deleted() && self.edge_id != 0
    }

    /// Serialise this `EdgeSlot` into the given 80-byte buffer.
    ///
    /// All multi-byte integers are written in little-endian byte order.
    /// Padding (bytes 52-55) and reserved bytes (69-79) are zeroed explicitly.
    ///
    /// # Parameters
    /// - `buf`: Exactly 80 bytes of mutable storage positioned at the correct
    ///   slot offset within a page.
    pub fn write_to(&self, buf: &mut [u8; EDGE_SLOT_SIZE]) {
        buf[0..8].copy_from_slice(&self.edge_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.from_node_id.to_le_bytes());
        buf[16..24].copy_from_slice(&self.to_node_id.to_le_bytes());
        buf[24..32].copy_from_slice(&self.next_out_edge.to_le_bytes());
        buf[32..40].copy_from_slice(&self.next_in_edge.to_le_bytes());
        buf[40..48].copy_from_slice(&self.prop_offset.to_le_bytes());
        buf[48..52].copy_from_slice(&self.prop_length.to_le_bytes());
        buf[52..56].copy_from_slice(&0u32.to_le_bytes()); // 4 bytes of explicit padding — always zero
        buf[56..64].copy_from_slice(&self.label_offset.to_le_bytes());
        buf[64..68].copy_from_slice(&self.label_length.to_le_bytes());
        buf[68] = self.flags;
        buf[69..80].copy_from_slice(&[0u8; 11]); // 11 bytes reserved for future fields — always zero
    }

    /// Deserialise an `EdgeSlot` from the given 80-byte buffer.
    ///
    /// All multi-byte integers are read as little-endian.  Padding and
    /// reserved bytes are ignored.
    ///
    /// # Parameters
    /// - `buf`: Exactly 80 bytes previously written by `write_to`.
    pub fn read_from(buf: &[u8; EDGE_SLOT_SIZE]) -> Self {
        Self {
            edge_id: u64::from_le_bytes(
                buf[0..8]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            from_node_id: u64::from_le_bytes(
                buf[8..16]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            to_node_id: u64::from_le_bytes(
                buf[16..24]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            next_out_edge: u64::from_le_bytes(
                buf[24..32]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            next_in_edge: u64::from_le_bytes(
                buf[32..40]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            prop_offset: u64::from_le_bytes(
                buf[40..48]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            prop_length: u32::from_le_bytes(
                buf[48..52]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            // bytes 52-55 are padding; skip them
            label_offset: u64::from_le_bytes(
                buf[56..64]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            label_length: u32::from_le_bytes(
                buf[64..68]
                    .try_into()
                    .expect("BUG: serializer slice indices statically within fixed-size buf"),
            ),
            flags: buf[68],
            // bytes 69-79 are reserved; not stored in the struct
        }
    }
}

/// Read a `NodeSlot` from an arbitrary byte slice at the given `offset`.
///
/// This is a convenience wrapper around `NodeSlot::read_from` for cases where
/// the caller already has a larger byte buffer (e.g. a raw page) and only wants
/// to extract one slot from it without constructing an intermediate array.
///
/// # Parameters
/// - `data`: The byte slice containing the serialised slot data.
/// - `offset`: The byte index within `data` where the 64-byte slot begins.
///
/// # Errors
/// Returns `CorruptedFile` if `offset + NODE_SLOT_SIZE > data.len()`, i.e. the
/// slice is too short to contain a complete slot at that position.
pub fn read_node_slot_from_slice(data: &[u8], offset: usize) -> Result<NodeSlot> {
    if offset + NODE_SLOT_SIZE > data.len() {
        return Err(LielError::CorruptedFile(format!(
            "node slot offset {} out of range",
            offset
        )));
    }
    let arr: &[u8; NODE_SLOT_SIZE] = data[offset..offset + NODE_SLOT_SIZE]
        .try_into()
        .expect("BUG: serializer slice indices statically within fixed-size buf");
    Ok(NodeSlot::read_from(arr))
}

/// Read an `EdgeSlot` from an arbitrary byte slice at the given `offset`.
///
/// Convenience wrapper analogous to `read_node_slot_from_slice`, but for the
/// 80-byte `EdgeSlot` layout.
///
/// # Parameters
/// - `data`: The byte slice containing the serialised slot data.
/// - `offset`: The byte index within `data` where the 80-byte slot begins.
///
/// # Errors
/// Returns `CorruptedFile` if `offset + EDGE_SLOT_SIZE > data.len()`.
pub fn read_edge_slot_from_slice(data: &[u8], offset: usize) -> Result<EdgeSlot> {
    if offset + EDGE_SLOT_SIZE > data.len() {
        return Err(LielError::CorruptedFile(format!(
            "edge slot offset {} out of range",
            offset
        )));
    }
    let arr: &[u8; EDGE_SLOT_SIZE] = data[offset..offset + EDGE_SLOT_SIZE]
        .try_into()
        .expect("BUG: serializer slice indices statically within fixed-size buf");
    Ok(EdgeSlot::read_from(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_slot_roundtrip() {
        let slot = NodeSlot {
            node_id: 1,
            first_out_edge: 2,
            first_in_edge: 3,
            prop_offset: 4096,
            prop_length: 128,
            out_degree: 5,
            in_degree: 3,
            label_offset: 8192,
            label_length: 20,
            flags: 0,
        };
        let mut buf = [0u8; NODE_SLOT_SIZE];
        slot.write_to(&mut buf);
        let decoded = NodeSlot::read_from(&buf);
        assert_eq!(decoded.node_id, slot.node_id);
        assert_eq!(decoded.first_out_edge, slot.first_out_edge);
        assert_eq!(decoded.prop_length, slot.prop_length);
        assert_eq!(decoded.out_degree, slot.out_degree);
        assert_eq!(decoded.label_offset, slot.label_offset);
        assert_eq!(decoded.flags, slot.flags);
    }

    #[test]
    fn test_edge_slot_roundtrip() {
        let slot = EdgeSlot {
            edge_id: 10,
            from_node_id: 1,
            to_node_id: 2,
            next_out_edge: 11,
            next_in_edge: 12,
            prop_offset: 4096,
            prop_length: 64,
            label_offset: 8192,
            label_length: 10,
            flags: 0,
        };
        let mut buf = [0u8; EDGE_SLOT_SIZE];
        slot.write_to(&mut buf);
        let decoded = EdgeSlot::read_from(&buf);
        assert_eq!(decoded.edge_id, slot.edge_id);
        assert_eq!(decoded.from_node_id, slot.from_node_id);
        assert_eq!(decoded.next_out_edge, slot.next_out_edge);
        assert_eq!(decoded.prop_length, slot.prop_length);
        assert_eq!(decoded.label_offset, slot.label_offset);
    }

    #[test]
    fn read_node_slot_from_short_buffer_returns_corrupted_file() {
        // The dynamic-length helpers must surface a typed error rather than
        // panic when the supplied slice is shorter than NODE_SLOT_SIZE.  A
        // short slice is the on-disk symptom of a torn write or a corrupt
        // table page; turning it into LielError::CorruptedFile lets the
        // caller report the issue cleanly instead of unwinding through FFI.
        let short = vec![0u8; NODE_SLOT_SIZE - 1];
        let err = read_node_slot_from_slice(&short, 0).expect_err("short slice must fail");
        assert!(matches!(err, LielError::CorruptedFile(_)));
    }

    #[test]
    fn read_edge_slot_from_short_buffer_returns_corrupted_file() {
        let short = vec![0u8; EDGE_SLOT_SIZE - 1];
        let err = read_edge_slot_from_slice(&short, 0).expect_err("short slice must fail");
        assert!(matches!(err, LielError::CorruptedFile(_)));
    }
}

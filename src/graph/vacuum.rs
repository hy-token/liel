use std::path::Path;

use crate::error::{LielError, Result};
use crate::graph::fault_inject::crash_at;
use crate::storage::atomic_rename::atomic_replace;
use crate::storage::pager::{ExtentKind, Pager, EDGES_PER_EXTENT, NODES_PER_EXTENT};
use crate::storage::serializer::{EdgeSlot, NodeSlot};

/// Snapshot row for one live node: (slot, label bytes, property bytes).
type NodeSnapshot = (NodeSlot, Vec<u8>, Vec<u8>);
/// Snapshot row for one live edge: (slot, label bytes, property bytes).
type EdgeSnapshot = (EdgeSlot, Vec<u8>, Vec<u8>);

/// Compact property storage and reclaim space consumed by deleted records.
///
/// Vacuum has two backends.  For an on-disk database (`db_path = Some(...)`)
/// it copies all live data into a sibling `<basename>.liel.tmp` and atomically
/// renames it over the original — the **crash-safe** copy-on-write path
/// described in [product-tradeoffs.md §5.6].  For `:memory:` databases
/// (`db_path = None`) atomic rename is meaningless, so vacuum falls back to
/// the in-place algorithm that compacts property storage inside the
/// existing pager.
///
/// On success the caller's `Pager` is no longer the right window onto the
/// data: for the file path the original file has been replaced and the
/// pager's open file descriptor now points at an unlinked inode.  The
/// caller (`GraphDB::vacuum`) is responsible for dropping the stale pager
/// and reopening from the same path.
///
/// [product-tradeoffs.md §5.6]: https://github.com/hy-token/liel/blob/main/docs/design/product-tradeoffs.md
pub fn vacuum(pager: &mut Pager, db_path: Option<&Path>) -> Result<()> {
    match db_path {
        None => vacuum_in_place(pager),
        Some(path) => {
            let tmp_path = build_file_vacuum_tmp(pager, path)?;
            install_file_vacuum_tmp(&tmp_path, path)
        }
    }
}

pub fn install_file_vacuum_tmp(tmp_path: &Path, db_path: &Path) -> Result<()> {
    atomic_replace(tmp_path, db_path)?;
    crash_at("AFTER_RENAME");
    Ok(())
}

/// In-place vacuum used for `:memory:` databases.
///
/// This is the original (pre-0.3) algorithm: snapshot live blobs, reset the
/// prop extent chain, then re-append blobs and re-write slots in place.
/// **Not crash-safe** if the process is killed mid-run; for in-memory
/// databases that does not matter because there is no on-disk state to
/// corrupt.
///
/// Returns `Ok(())` on success.
fn vacuum_in_place(pager: &mut Pager) -> Result<()> {
    let (node_data, edge_data) = snapshot_live_records(pager)?;

    // Reset the property extent chain so the next `append_prop` allocates a
    // fresh extent at the file tail and writes from offset 0 onward.  Old
    // prop extents become orphaned space in the existing file; the on-disk
    // copy-on-write path avoids this entirely because it writes a fresh
    // file from scratch.
    pager.reset_prop_storage()?;

    for (mut slot, label_bytes, prop_bytes) in node_data {
        let (label_offset, label_length) = append_blob(pager, &label_bytes)?;
        let (prop_offset, prop_length) = append_blob(pager, &prop_bytes)?;
        slot.label_offset = label_offset;
        slot.label_length = label_length;
        slot.prop_offset = prop_offset;
        slot.prop_length = prop_length;
        pager.write_node_slot(&slot)?;
    }

    for (mut slot, label_bytes, prop_bytes) in edge_data {
        let (label_offset, label_length) = append_blob(pager, &label_bytes)?;
        let (prop_offset, prop_length) = append_blob(pager, &prop_bytes)?;
        slot.label_offset = label_offset;
        slot.label_length = label_length;
        slot.prop_offset = prop_offset;
        slot.prop_length = prop_length;
        pager.write_edge_slot(&slot)?;
    }

    pager.commit()?;
    Ok(())
}

/// Copy-on-write vacuum used for on-disk databases.
///
/// Steps:
///
/// 1. Force a `commit()` so the WAL is empty before we snapshot.  Anything
///    pending would otherwise be invisible to a fresh-file rebuild.
/// 2. Snapshot every live `NodeSlot` / `EdgeSlot` together with the raw
///    label/property blob bytes they reference.
/// 3. Open a fresh sibling `<basename>.liel.tmp` as a brand-new pager and
///    pre-allocate enough node/edge extents to address the highest live ID.
/// 4. Pre-set `next_node_id` / `next_edge_id` from the source so vacuum
///    preserves the **ID-stability invariant** documented in
///    `format-spec.md §7.2`.
/// 5. Replay every live slot into the new pager, appending blobs through
///    `append_prop` so the new file is densely packed.
/// 6. `commit()` the new pager — this is the durability boundary.
/// 7. Drop the new pager so the OS file handle is released, then call
///    [`atomic_replace`] to swap the tmp file over the live one.  After
///    rename, this function returns and the caller (`GraphDB::vacuum`)
///    reopens its pager against the now-replaced path.
///
/// Crash-safety contract:
/// - A crash before step 7 leaves the original `<basename>.liel`
///   untouched and a partial `<basename>.liel.tmp` on disk; the next
///   `GraphDB::open` unconditionally unlinks the tmp.
/// - A crash during step 7 (the rename itself) is atomic on POSIX and on
///   NTFS — the path resolves to either the old or new file.
/// - A crash after step 7 is indistinguishable from a successful vacuum
///   followed by an unrelated termination.
pub fn build_file_vacuum_tmp(pager: &mut Pager, db_path: &Path) -> Result<std::path::PathBuf> {
    // Step 1: drain any uncommitted transaction.  After this call the WAL
    // is `wal_length = 0` and every dirty page has been flushed, so the
    // snapshot reads in step 2 see exactly what is on disk.
    pager.commit()?;

    // Step 2: snapshot live slots and their referenced blobs.
    let next_node_id = pager.header.next_node_id;
    let next_edge_id = pager.header.next_edge_id;
    let (node_data, edge_data) = snapshot_live_records(pager)?;

    // The tmp path follows the same convention `GraphDB::open` uses to
    // sweep stale leftovers.
    let tmp_path = vacuum_tmp_path(db_path);

    crash_at("BEFORE_TMP_OPEN");

    // Defensive: if a previous vacuum exited mid-write between
    // `GraphDB::open`'s sweep and us, drop the partial here.  ENOENT is
    // the common clean-tree path.
    match std::fs::remove_file(&tmp_path) {
        Ok(()) | Err(_) => {}
    }

    // Step 3: open a brand-new pager backed by the tmp file.  The header
    // is freshly initialised with `next_node_id = 1` etc. — we override
    // the ID counters in step 4 to honour the ID-stability invariant.
    let tmp_path_str = tmp_path.to_str().ok_or_else(|| {
        LielError::InvalidArgument(
            "vacuum: temporary path is not valid UTF-8 — \
             retry from a directory whose name is encodable as UTF-8"
                .into(),
        )
    })?;
    let result = vacuum_to_tmp_inner(
        tmp_path_str,
        next_node_id,
        next_edge_id,
        node_data,
        edge_data,
    );

    match result {
        Ok(()) => {
            crash_at("AFTER_TMP_FSYNC");
            Ok(tmp_path)
        }
        Err(err) => {
            // Best-effort cleanup so the next `GraphDB::open` does not
            // need to do it.  If the unlink itself fails the file
            // remains and the open-time sweep takes care of it.
            let _ = std::fs::remove_file(&tmp_path);
            Err(err)
        }
    }
}

/// Inner body of [`build_file_vacuum_tmp`] split out so the cleanup
/// `match` above can apply uniformly to every internal failure.
fn vacuum_to_tmp_inner(
    tmp_path: &str,
    next_node_id: u64,
    next_edge_id: u64,
    node_data: Vec<NodeSnapshot>,
    edge_data: Vec<EdgeSnapshot>,
) -> Result<()> {
    let mut new_pager = Pager::open(tmp_path)?;

    // Step 4: preserve ID counters.  The new file's header otherwise
    // starts at `next_node_id = 1`, which would silently violate the
    // ID-stability invariant of format-spec §7.2.
    new_pager.header.next_node_id = next_node_id;
    new_pager.header.next_edge_id = next_edge_id;

    // Pre-allocate enough node/edge extents to cover the highest live ID.
    // `write_node_slot` would otherwise CapacityExceeded for a slot whose
    // extent has not yet been allocated by `alloc_node_id`.
    let max_node_id = next_node_id.saturating_sub(1);
    if max_node_id > 0 {
        new_pager.ensure_extent_for(max_node_id, ExtentKind::Node, NODES_PER_EXTENT)?;
    }
    let max_edge_id = next_edge_id.saturating_sub(1);
    if max_edge_id > 0 {
        new_pager.ensure_extent_for(max_edge_id, ExtentKind::Edge, EDGES_PER_EXTENT)?;
    }

    // Step 5: replay live records.  Slot positions follow the same
    // `id → file offset` formula as the source, so node IDs land at the
    // same logical position in the new file and `next_node_id` continues
    // monotonically.
    for (mut slot, label_bytes, prop_bytes) in node_data {
        let (label_offset, label_length) = append_blob(&mut new_pager, &label_bytes)?;
        let (prop_offset, prop_length) = append_blob(&mut new_pager, &prop_bytes)?;
        slot.label_offset = label_offset;
        slot.label_length = label_length;
        slot.prop_offset = prop_offset;
        slot.prop_length = prop_length;
        new_pager.write_node_slot(&slot)?;
        new_pager.increment_node_count();
    }

    for (mut slot, label_bytes, prop_bytes) in edge_data {
        let (label_offset, label_length) = append_blob(&mut new_pager, &label_bytes)?;
        let (prop_offset, prop_length) = append_blob(&mut new_pager, &prop_bytes)?;
        slot.label_offset = label_offset;
        slot.label_length = label_length;
        slot.prop_offset = prop_offset;
        slot.prop_length = prop_length;
        new_pager.write_edge_slot(&slot)?;
        new_pager.increment_edge_count();
    }

    crash_at("AFTER_TMP_WRITES");

    // Step 6: durability boundary — `commit()` writes the WAL, fsyncs,
    // flushes data pages, fsyncs again, and clears the WAL.
    new_pager.commit()?;

    Ok(())
}

/// Read every active node and edge slot together with the raw bytes of
/// their referenced label and property blobs.  Used by both the in-place
/// and copy-on-write paths so the snapshot cost is unified.
fn snapshot_live_records(pager: &mut Pager) -> Result<(Vec<NodeSnapshot>, Vec<EdgeSnapshot>)> {
    let max_node_id = pager.max_node_id();
    let mut node_data = Vec::new();
    for id in 1..=max_node_id {
        let slot = pager.read_node_slot(id)?;
        if slot.node_id == 0 || slot.is_deleted() {
            continue;
        }
        let label_bytes = read_blob(pager, slot.label_offset, slot.label_length)?;
        let prop_bytes = read_blob(pager, slot.prop_offset, slot.prop_length)?;
        node_data.push((slot, label_bytes, prop_bytes));
    }

    let max_edge_id = pager.max_edge_id();
    let mut edge_data = Vec::new();
    for id in 1..=max_edge_id {
        let slot = pager.read_edge_slot(id)?;
        if slot.edge_id == 0 || slot.is_deleted() {
            continue;
        }
        let label_bytes = read_blob(pager, slot.label_offset, slot.label_length)?;
        let prop_bytes = read_blob(pager, slot.prop_offset, slot.prop_length)?;
        edge_data.push((slot, label_bytes, prop_bytes));
    }

    Ok((node_data, edge_data))
}

fn read_blob(pager: &mut Pager, offset: u64, length: u32) -> Result<Vec<u8>> {
    if length == 0 {
        Ok(Vec::new())
    } else {
        pager.read_prop(offset, length)
    }
}

fn append_blob(pager: &mut Pager, bytes: &[u8]) -> Result<(u64, u32)> {
    if bytes.is_empty() {
        Ok((0, 0))
    } else {
        let off = pager.append_prop(bytes)?;
        Ok((off, bytes.len() as u32))
    }
}

/// Sibling path used by vacuum's copy-on-write rewrite: append `.tmp` to
/// the full canonical path so we never strip a user-supplied extension.
/// Mirrors `crate::db::vacuum_tmp_path`; defined here as well so this
/// module does not depend on `db.rs`.
pub(crate) fn vacuum_tmp_path(db_path: &Path) -> std::path::PathBuf {
    let mut s = db_path.as_os_str().to_owned();
    s.push(".tmp");
    std::path::PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::{add_edge, delete_edge};
    use crate::graph::node::add_node;
    use crate::storage::prop_codec::PropValue;
    use std::collections::HashMap;

    fn make_db() -> Pager {
        Pager::open(":memory:").unwrap()
    }

    #[test]
    fn test_vacuum_empty_db() {
        let mut pager = make_db();
        vacuum(&mut pager, None).unwrap();
    }

    #[test]
    fn test_vacuum_preserves_data() {
        let mut pager = make_db();
        let mut props = HashMap::new();
        props.insert("name".into(), PropValue::String("Alice".into()));
        let alice = add_node(&mut pager, vec!["Person".into()], props).unwrap();

        let bob = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let mut eprops = HashMap::new();
        eprops.insert("since".into(), PropValue::Int(2020));
        add_edge(&mut pager, alice.id, "KNOWS".into(), bob.id, eprops).unwrap();

        vacuum(&mut pager, None).unwrap();

        let n = crate::graph::node::get_node(&mut pager, alice.id)
            .unwrap()
            .unwrap();
        assert_eq!(n.properties["name"], PropValue::String("Alice".into()));
        assert_eq!(n.labels, vec!["Person"]);
    }

    #[test]
    fn test_vacuum_removes_deleted_prop_data() {
        let mut pager = Pager::open(":memory:").unwrap();
        let mut big_props = HashMap::new();
        big_props.insert("data".into(), PropValue::String("x".repeat(1000)));

        let n1 = add_node(&mut pager, vec![], big_props.clone()).unwrap();
        let n2 = add_node(&mut pager, vec![], big_props.clone()).unwrap();
        let n3 = add_node(&mut pager, vec![], big_props).unwrap();
        pager.commit().unwrap();

        // Byte distance between the first prop extent and the live cursor
        // represents everything `append_prop` has consumed so far.
        let first_extent_before = pager.prop_extents_for_test().first().copied().unwrap_or(0);
        let pre_vacuum_prop_bytes = pager.header.next_prop_write_offset - first_extent_before;
        delete_edge(&mut pager, 0).unwrap_err(); // dummy call to avoid unused warning
        crate::graph::node::delete_node(&mut pager, n1.id).unwrap();
        crate::graph::node::delete_node(&mut pager, n2.id).unwrap();
        pager.commit().unwrap();

        vacuum(&mut pager, None).unwrap();

        // After vacuum only n3's blobs should be live, so the bytes actually
        // written into the current prop extent must be smaller than the high
        // watermark observed before vacuum started.
        let live_prop_bytes = pager
            .prop_extents_for_test()
            .last()
            .map(|start| pager.header.next_prop_write_offset - *start)
            .unwrap_or(0);
        assert!(
            live_prop_bytes < pre_vacuum_prop_bytes,
            "vacuum should reclaim prop space (live={live_prop_bytes}, \
             pre-vacuum={pre_vacuum_prop_bytes})",
        );

        // n3 should still be accessible and have its data intact after vacuum.
        let node = crate::graph::node::get_node(&mut pager, n3.id)
            .unwrap()
            .unwrap();
        assert!(node.properties.contains_key("data"));
    }
}

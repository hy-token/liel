use super::crc32::crc32;
use super::pager::{Pager, PAGE_SIZE, WAL_RESERVED};
use super::traits::Storage;
use crate::error::{LielError, Result};

/// Operation type byte embedded in every WAL entry header.
const OP_WRITE: u8 = 0x01;
const OP_COMMIT: u8 = 0x02;

// ─── WAL entry layout constants ────────────────────────────────────
//
// The WAL is a sequence of length-prefixed entries, each terminated by a
// CRC32.  Every entry begins with the same 17-byte header shown below; a
// Commit entry stops there and appends the CRC, while a Write entry
// additionally carries a 4096-byte page image before the CRC.
//
//   entry_length  u32 le  (size of this entry INCLUDING the trailing CRC)
//   op_type       u8      0x01=Write, 0x02=Commit
//   page_offset   u64 le  Write: target page; Commit: 0
//   data_length   u32 le  Write: PAGE_SIZE; Commit: 0
//   [page image]          only present on Write entries (4096 bytes)
//   crc32         u32 le  ISO-HDLC over all preceding bytes of this entry

/// Byte offset of the `entry_length` field within a WAL entry.
const ENTRY_LENGTH_FIELD_SIZE: usize = 4;
/// Byte offset of the `op_type` field within a WAL entry.
const OP_TYPE_FIELD_SIZE: usize = 1;
/// Byte size of the `page_offset` field within a WAL entry.
const PAGE_OFFSET_FIELD_SIZE: usize = 8;
/// Byte size of the `data_length` field within a WAL entry.
const DATA_LENGTH_FIELD_SIZE: usize = 4;

/// WAL entry header size in bytes: entry_length(4) + op_type(1) + page_offset(8) + data_length(4) = 17.
const WAL_ENTRY_HEADER_SIZE: usize =
    ENTRY_LENGTH_FIELD_SIZE + OP_TYPE_FIELD_SIZE + PAGE_OFFSET_FIELD_SIZE + DATA_LENGTH_FIELD_SIZE;
/// Size of the CRC32 checksum appended to the end of each WAL entry (4 bytes).
const WAL_CRC_SIZE: usize = 4;
/// Total size of a Write WAL entry: header + 4096-byte page data + CRC.
const WAL_WRITE_ENTRY_SIZE: usize = WAL_ENTRY_HEADER_SIZE + PAGE_SIZE + WAL_CRC_SIZE;
/// Total size of a Commit WAL entry (no page data, just header + CRC).
const WAL_COMMIT_ENTRY_SIZE: usize = WAL_ENTRY_HEADER_SIZE + WAL_CRC_SIZE;

/// Lower-bound length for any parseable entry, used by [`Wal::parse_wal_entries`]
/// to short-circuit on a bad length prefix.  Non-Write entries don't strictly
/// need the `data_length` slot of the header, so the bound is
/// `entry_length + op_type + page_offset + CRC` rather than the full header.
const MIN_ENTRY_LEN: usize =
    ENTRY_LENGTH_FIELD_SIZE + OP_TYPE_FIELD_SIZE + PAGE_OFFSET_FIELD_SIZE + WAL_CRC_SIZE;

/// Minimum `entry_length` that can still describe a well-formed Write entry
/// (full header + page image + CRC).  A shorter Write entry would have us read
/// past the page-image slice in [`Wal::parse_wal_entries`], so we treat
/// anything smaller as torn and stop recovery.
const OP_WRITE_MIN_LEN: usize = WAL_WRITE_ENTRY_SIZE;

pub struct Wal;

impl Wal {
    /// Write all dirty pages to the WAL, append a Commit entry, then flush the
    /// data pages to their final locations in the storage.
    ///
    /// Crash-safety guarantee:
    ///
    /// 1. Build WAL byte blob in memory.
    /// 2. Write + fsync the WAL section.
    /// 3. Copy each dirty page to its data-page offset.
    /// 4. Clear the WAL (set wal_length = 0 in header).
    ///
    /// A crash between steps 2 and 3 is safe: on the next open, [`Wal::recover`] replays the
    /// WAL and completes the flush.
    pub fn write_and_commit(pager: &mut Pager) -> Result<()> {
        let dirty_offsets = pager.dirty_page_offsets();
        if dirty_offsets.is_empty() {
            return Ok(());
        }

        // Reject commits that would overflow the reserved WAL region before
        // we perform any I/O: otherwise the write would spill into the
        // node/edge data pages and corrupt the database.  The total size
        // is exactly predictable from the number of dirty pages because every
        // Write entry is the same length.
        let required_bytes =
            dirty_offsets.len() as u64 * WAL_WRITE_ENTRY_SIZE as u64 + WAL_COMMIT_ENTRY_SIZE as u64;
        if required_bytes > WAL_RESERVED {
            return Err(LielError::WalOverflow(format!(
                "transaction needs {} bytes of WAL but only {} bytes are reserved \
                 ({} dirty pages; split the work across multiple commits)",
                required_bytes,
                WAL_RESERVED,
                dirty_offsets.len()
            )));
        }

        // Build the full WAL byte blob in memory before any I/O
        let mut wal_bytes: Vec<u8> = Vec::with_capacity(required_bytes as usize);
        for &page_offset in &dirty_offsets {
            let page = pager
                .get_dirty_page(page_offset)
                .ok_or_else(|| {
                    LielError::TransactionError(
                        "internal error: dirty page missing during WAL commit (please report as a bug)"
                            .into(),
                    )
                })?
                .to_owned();
            let entry = Self::build_write_entry(page_offset, &page);
            wal_bytes.extend_from_slice(&entry);
        }
        // Append the Commit marker entry
        let commit_entry = Self::build_commit_entry();
        wal_bytes.extend_from_slice(&commit_entry);

        // Write the entire WAL blob to the WAL section in storage
        let wal_offset = pager.header.wal_offset;
        Self::write_wal_to_storage(pager.storage_mut(), wal_offset, &wal_bytes)?;

        // Record wal_length in the header and fsync — this makes the WAL durable
        pager.header.wal_length = wal_bytes.len() as u64;
        pager.write_header()?;
        pager.flush_storage()?;

        // Safe to write data pages now; WAL is durable so a crash here is recoverable
        pager.flush_dirty_pages()?;
        pager.flush_storage()?;

        // Clear the WAL: set wal_length = 0 so it is not replayed on next open
        pager.header.wal_length = 0;
        pager.write_header()?;
        pager.flush_storage()?;

        Ok(())
    }

    /// Replay the WAL on startup to recover from a crash.
    ///
    /// If the WAL contains a complete Write+Commit sequence, the pages are applied
    /// (roll-forward). If there is no Commit entry, the WAL is discarded (roll-back).
    /// Called automatically by `Pager::open()`.
    pub fn recover(pager: &mut Pager) -> Result<()> {
        let wal_length = pager.header.wal_length;
        if wal_length == 0 {
            return Ok(());
        }

        let wal_offset = pager.header.wal_offset;
        let wal_bytes =
            Self::read_wal_from_storage(pager.storage_mut(), wal_offset, wal_length as usize)?;

        // Parse all WAL entries from the stored byte blob
        let entries = Self::parse_wal_entries(&wal_bytes)?;

        // Check whether a Commit entry is present in the parsed WAL entries
        let has_commit = entries.iter().any(|e| e.op_type == OP_COMMIT);
        if !has_commit {
            // No Commit: discard the WAL — data pages are already in their last committed state
            pager.header.wal_length = 0;
            pager.write_header()?;
            return Ok(());
        }

        // Roll forward: apply each Write entry's page to its data-page offset
        for entry in &entries {
            if entry.op_type == OP_WRITE {
                let mut page = [0u8; PAGE_SIZE];
                page.copy_from_slice(&entry.data);
                pager.storage_mut().write_page(entry.page_offset, &page)?;
            }
        }
        pager.storage_mut().flush()?;

        // Clear the WAL now that all pages have been applied
        pager.header.wal_length = 0;
        pager.write_header()?;

        // Reload the header — it may have been overwritten during roll-forward
        // (If the header page was in the WAL it has already been written to storage above)
        pager.load_header_from_storage()?;

        Ok(())
    }

    fn build_write_entry(page_offset: u64, page: &[u8; PAGE_SIZE]) -> Vec<u8> {
        let entry_length = WAL_WRITE_ENTRY_SIZE as u32;
        let mut entry = Vec::with_capacity(WAL_WRITE_ENTRY_SIZE);
        entry.extend_from_slice(&entry_length.to_le_bytes());
        entry.push(OP_WRITE);
        entry.extend_from_slice(&page_offset.to_le_bytes());
        entry.extend_from_slice(&(PAGE_SIZE as u32).to_le_bytes());
        entry.extend_from_slice(page);
        let crc = crc32(&entry);
        entry.extend_from_slice(&crc.to_le_bytes());
        entry
    }

    fn build_commit_entry() -> Vec<u8> {
        let entry_length = WAL_COMMIT_ENTRY_SIZE as u32;
        let mut entry = Vec::with_capacity(WAL_COMMIT_ENTRY_SIZE);
        entry.extend_from_slice(&entry_length.to_le_bytes());
        entry.push(OP_COMMIT);
        entry.extend_from_slice(&0u64.to_le_bytes()); // page_offset (unused)
        entry.extend_from_slice(&0u32.to_le_bytes()); // data_length (unused)
                                                      // CRC is over the entry so far
        let crc = crc32(&entry);
        entry.extend_from_slice(&crc.to_le_bytes());
        entry
    }

    fn write_wal_to_storage(
        storage: &mut dyn Storage,
        wal_offset: u64,
        wal_bytes: &[u8],
    ) -> Result<()> {
        // Write to the WAL section in page-aligned chunks
        let mut written = 0;
        while written < wal_bytes.len() {
            let current_offset = wal_offset + written as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_write = (PAGE_SIZE - offset_in_page).min(wal_bytes.len() - written);

            let mut page = [0u8; PAGE_SIZE];
            let file_size = storage.file_size();
            if page_offset + PAGE_SIZE as u64 <= file_size {
                page = storage.read_page(page_offset)?;
            }
            page[offset_in_page..offset_in_page + can_write]
                .copy_from_slice(&wal_bytes[written..written + can_write]);
            storage.write_page(page_offset, &page)?;
            written += can_write;
        }
        Ok(())
    }

    fn read_wal_from_storage(
        storage: &mut dyn Storage,
        wal_offset: u64,
        wal_length: usize,
    ) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(wal_length);
        let mut read = 0;
        while read < wal_length {
            let current_offset = wal_offset + read as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_read = (PAGE_SIZE - offset_in_page).min(wal_length - read);
            let page = storage.read_page(page_offset)?;
            result.extend_from_slice(&page[offset_in_page..offset_in_page + can_read]);
            read += can_read;
        }
        Ok(result)
    }

    /// Walk the WAL byte blob and return one [`WalEntry`] per parseable
    /// entry, stopping early on torn writes / corruption so any preceding
    /// good entries can still be honoured by recovery.
    ///
    /// The parser is deliberately split into three named helpers so each
    /// failure mode has a single, individually-testable home:
    ///
    /// - [`Self::read_entry_length`]  : sanity-check the length prefix
    ///   (truncated buffer, undersized header).
    /// - [`Self::verify_entry_crc`]   : confirm the trailing CRC32 covers
    ///   everything before it.
    /// - [`Self::decode_entry_body`]  : extract `op_type`, `page_offset`,
    ///   and the page payload (only Write entries carry one).
    ///
    /// Each helper returns `ControlFlow`-style values that the loop maps
    /// to `break` (stop parsing, recovery treats it as "no further good
    /// entries"), `continue` (impossible by construction here), or
    /// `Err(LielError::CorruptedFile(...))` for structural violations
    /// the writer should never produce.
    fn parse_wal_entries(wal_bytes: &[u8]) -> Result<Vec<WalEntry>> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < wal_bytes.len() {
            let entry_length = match Self::read_entry_length(wal_bytes, pos) {
                Some(len) => len,
                None => break,
            };

            let entry_data = &wal_bytes[pos..pos + entry_length];

            if !Self::verify_entry_crc(entry_data) {
                // CRC mismatch: torn write or corruption — stop and let
                // recovery fall back on whatever entries preceded this one.
                break;
            }

            match Self::decode_entry_body(entry_data)? {
                Some(entry) => entries.push(entry),
                None => break,
            }

            pos += entry_length;
        }

        Ok(entries)
    }

    /// Read the 4-byte little-endian `entry_length` prefix at `pos` and
    /// validate that the byte buffer can plausibly contain an entry of
    /// that length.  Returns `None` (caller breaks) for any of:
    ///
    /// - the buffer ends before the 4-byte prefix is complete,
    /// - the prefix claims a length below `MIN_ENTRY_LEN` (header alone
    ///   would be malformed),
    /// - the prefix claims more bytes than the buffer actually holds.
    ///
    /// Returns `Some(entry_length)` when the entry is at least
    /// **structurally** parseable; the caller still has to verify CRC and
    /// op-specific fields.
    #[inline]
    fn read_entry_length(wal_bytes: &[u8], pos: usize) -> Option<usize> {
        if pos + ENTRY_LENGTH_FIELD_SIZE > wal_bytes.len() {
            // Truncated entry — stop parsing (treat as no Commit).
            return None;
        }
        let entry_length = u32::from_le_bytes(
            wal_bytes[pos..pos + ENTRY_LENGTH_FIELD_SIZE]
                .try_into()
                .expect("BUG: wal slice indices bounds-checked above"),
        ) as usize;

        // A shorter entry_length means the WAL entry header itself is
        // corrupt; treat as end-of-WAL so recovery can still succeed for
        // any preceding good entries.  See [`MIN_ENTRY_LEN`] for the
        // derivation of the 17-byte floor.
        if entry_length < MIN_ENTRY_LEN {
            return None;
        }
        if pos + entry_length > wal_bytes.len() {
            // The prefix promises more bytes than the WAL actually holds.
            return None;
        }
        Some(entry_length)
    }

    /// Recompute CRC32 over the entry's bytes excluding the trailing
    /// 4-byte checksum field, and compare against the stored value.
    /// Returns `true` iff the two match.
    #[inline]
    fn verify_entry_crc(entry_data: &[u8]) -> bool {
        // Layout invariant established by [`Self::read_entry_length`]:
        // `entry_data.len() >= MIN_ENTRY_LEN >= WAL_CRC_SIZE`, so both
        // slices below are in-bounds.
        let crc_start = entry_data.len() - WAL_CRC_SIZE;
        let stored_crc = u32::from_le_bytes(
            entry_data[crc_start..]
                .try_into()
                .expect("BUG: entry_length >= MIN_ENTRY_LEN guarantees >= WAL_CRC_SIZE"),
        );
        let computed_crc = crc32(&entry_data[..crc_start]);
        stored_crc == computed_crc
    }

    /// Decode the per-op fields of a CRC-validated entry.
    ///
    /// Returns:
    /// - `Ok(Some(entry))` for a well-formed entry the caller should keep.
    /// - `Ok(None)` for an OP_WRITE entry whose `entry_length` is too
    ///   small to hold a full page payload (treat as torn write — stop
    ///   parsing, do not return an error).
    /// - `Err(LielError::CorruptedFile(_))` for structural violations
    ///   the writer should never emit (e.g. `data_length != PAGE_SIZE`
    ///   on an OP_WRITE entry).  Surfacing these as a hard error rather
    ///   than silently truncating recovery is intentional: a wrong
    ///   `data_length` indicates a writer bug, not a power loss.
    #[inline]
    fn decode_entry_body(entry_data: &[u8]) -> Result<Option<WalEntry>> {
        let op_type = entry_data[ENTRY_LENGTH_FIELD_SIZE];
        let page_offset_start = ENTRY_LENGTH_FIELD_SIZE + OP_TYPE_FIELD_SIZE;
        let page_offset_end = page_offset_start + PAGE_OFFSET_FIELD_SIZE;
        let page_offset = u64::from_le_bytes(
            entry_data[page_offset_start..page_offset_end]
                .try_into()
                .expect("BUG: entry_length >= MIN_ENTRY_LEN guarantees 13 bytes"),
        );

        let data = if op_type == OP_WRITE {
            // A Write entry carries a full PAGE_SIZE payload between the
            // header and the CRC; anything shorter than
            // [`OP_WRITE_MIN_LEN`] would have us read past the buffer.
            if entry_data.len() < OP_WRITE_MIN_LEN {
                return Ok(None);
            }
            let data_length_start = page_offset_end;
            let data_length_end = data_length_start + DATA_LENGTH_FIELD_SIZE;
            let data_length = u32::from_le_bytes(
                entry_data[data_length_start..data_length_end]
                    .try_into()
                    .expect("BUG: OP_WRITE entry_length checked >= OP_WRITE_MIN_LEN"),
            ) as usize;
            if data_length != PAGE_SIZE {
                return Err(LielError::CorruptedFile(format!(
                    "WAL: unexpected data_length {}",
                    data_length
                )));
            }
            entry_data[WAL_ENTRY_HEADER_SIZE..WAL_ENTRY_HEADER_SIZE + PAGE_SIZE].to_vec()
        } else {
            Vec::new()
        };

        Ok(Some(WalEntry {
            op_type,
            page_offset,
            data,
        }))
    }

    /// Public wrapper around [`build_write_entry`] for tests and tooling.
    pub fn build_write_entry_pub(page_offset: u64, page: &[u8; PAGE_SIZE]) -> Vec<u8> {
        Self::build_write_entry(page_offset, page)
    }
}

struct WalEntry {
    op_type: u8,
    page_offset: u64,
    data: Vec<u8>,
}

// Extension trait that adds WAL helper methods to Pager
impl Pager {
    /// Reload the file header directly from storage.
    /// Called after WAL recovery to pick up the freshly-written header.
    pub fn load_header_from_storage(&mut self) -> Result<()> {
        use crate::storage::pager::StorageExt;
        let mut buf = [0u8; 128];
        self.storage.seek_and_read(0, &mut buf)?;
        self.header = crate::storage::pager::FileHeader::from_bytes(&buf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::node::{add_node, get_node};
    use crate::storage::pager::{Pager, PAGE_HEADER_SIZE};
    use crate::storage::prop_codec::PropValue;
    use crate::storage::serializer::NodeSlot;
    use std::collections::HashMap;

    // ── Unit tests for the parser helpers (B1 split) ───────────────────────
    //
    // These exercise `read_entry_length`, `verify_entry_crc`, and
    // `decode_entry_body` in isolation so a regression in one helper does
    // not require a full file-backed WAL recovery path to surface.

    #[test]
    fn read_entry_length_returns_none_when_buffer_truncated_before_prefix() {
        // Only 3 of the 4 prefix bytes are present.
        let buf = [0u8; 3];
        assert!(Wal::read_entry_length(&buf, 0).is_none());
    }

    #[test]
    fn read_entry_length_returns_none_when_prefix_below_min_entry_len() {
        // Prefix declares 10 bytes, well below MIN_ENTRY_LEN (17).
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&10u32.to_le_bytes());
        assert!(Wal::read_entry_length(&buf, 0).is_none());
    }

    #[test]
    fn read_entry_length_returns_none_when_prefix_exceeds_buffer() {
        // Prefix declares 4096 bytes but only 32 are present.
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&4096u32.to_le_bytes());
        assert!(Wal::read_entry_length(&buf, 0).is_none());
    }

    #[test]
    fn read_entry_length_accepts_a_well_formed_commit_entry() {
        let entry = Wal::build_commit_entry();
        assert_eq!(
            Wal::read_entry_length(&entry, 0),
            Some(WAL_COMMIT_ENTRY_SIZE)
        );
    }

    #[test]
    fn verify_entry_crc_passes_for_well_formed_entry() {
        let entry = Wal::build_commit_entry();
        assert!(Wal::verify_entry_crc(&entry));
    }

    #[test]
    fn verify_entry_crc_rejects_a_flipped_byte_inside_payload() {
        let mut entry = Wal::build_commit_entry();
        entry[5] ^= 0xFF; // op_type byte
        assert!(!Wal::verify_entry_crc(&entry));
    }

    #[test]
    fn verify_entry_crc_rejects_a_flipped_crc_byte() {
        let mut entry = Wal::build_commit_entry();
        let last = entry.len() - 1;
        entry[last] ^= 0xFF;
        assert!(!Wal::verify_entry_crc(&entry));
    }

    #[test]
    fn decode_entry_body_returns_a_well_formed_commit_entry() {
        let entry = Wal::build_commit_entry();
        let parsed = Wal::decode_entry_body(&entry).unwrap().unwrap();
        assert_eq!(parsed.op_type, OP_COMMIT);
        assert_eq!(parsed.page_offset, 0);
        assert!(parsed.data.is_empty());
    }

    #[test]
    fn decode_entry_body_returns_a_well_formed_write_entry() {
        let page = [0xAAu8; PAGE_SIZE];
        let entry = Wal::build_write_entry_pub(0x1234, &page);
        let parsed = Wal::decode_entry_body(&entry).unwrap().unwrap();
        assert_eq!(parsed.op_type, OP_WRITE);
        assert_eq!(parsed.page_offset, 0x1234);
        assert_eq!(parsed.data.len(), PAGE_SIZE);
        assert_eq!(parsed.data[0], 0xAA);
    }

    #[test]
    fn decode_entry_body_returns_corrupted_file_when_data_length_wrong() {
        // Take a real Write entry, smash `data_length` to a non-PAGE_SIZE
        // value, and recompute the CRC so we hit the `data_length`
        // check rather than the CRC check.
        let page = [0u8; PAGE_SIZE];
        let mut entry = Wal::build_write_entry_pub(0, &page);
        let bad_len: u32 = (PAGE_SIZE as u32) - 1;
        entry[13..17].copy_from_slice(&bad_len.to_le_bytes());
        let crc_start = entry.len() - WAL_CRC_SIZE;
        let crc = crc32(&entry[..crc_start]);
        entry[crc_start..].copy_from_slice(&crc.to_le_bytes());

        match Wal::decode_entry_body(&entry) {
            Err(LielError::CorruptedFile(msg)) => {
                assert!(msg.contains("data_length"), "got {msg:?}");
            }
            Err(other) => panic!("expected CorruptedFile, got error {other:?}"),
            Ok(_) => panic!("expected CorruptedFile, got Ok(...)"),
        }
    }

    #[test]
    fn decode_entry_body_returns_none_when_write_entry_too_short() {
        // Forge a Write entry whose entry_length claims OP_WRITE but is
        // shorter than OP_WRITE_MIN_LEN.  The helper must signal "stop
        // parsing" via Ok(None) rather than reading past the buffer.
        let mut buf = vec![0u8; MIN_ENTRY_LEN];
        let entry_length = MIN_ENTRY_LEN as u32;
        buf[0..4].copy_from_slice(&entry_length.to_le_bytes());
        buf[4] = OP_WRITE;
        // Recompute CRC so we get past `verify_entry_crc` if anyone calls it
        // — irrelevant for this unit test that targets `decode_entry_body`
        // directly, but keeps the buffer self-consistent for human inspection.
        let crc_start = buf.len() - WAL_CRC_SIZE;
        let crc = crc32(&buf[..crc_start]);
        buf[crc_start..].copy_from_slice(&crc.to_le_bytes());

        assert!(Wal::decode_entry_body(&buf).unwrap().is_none());
    }

    /// Patch `wal_length` (bytes 96..104) in the file header and recompute
    /// the XOR checksum (bytes 104..112) so the header still validates.
    ///
    /// Tests that inject a raw, uncommitted WAL entry need to do this by hand
    /// because they bypass the normal `Pager::commit` → `write_header` path.
    fn patch_header_wal_length(path_str: &str, wal_length: u64) {
        use std::fs::OpenOptions;
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path_str)
            .unwrap();
        let mut header = [0u8; 128];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut header).unwrap();
        header[96..104].copy_from_slice(&wal_length.to_le_bytes());
        let checksum: u64 = header[0..104].chunks(8).fold(0u64, |acc, chunk| {
            let mut bytes = [0u8; 8];
            bytes[..chunk.len()].copy_from_slice(chunk);
            acc ^ u64::from_le_bytes(bytes)
        });
        header[104..112].copy_from_slice(&checksum.to_le_bytes());
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&header).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn test_wal_write_and_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.liel");
        let path_str = path.to_str().unwrap();

        // Write a node slot and commit so the data reaches the file
        {
            let mut pager = Pager::open(path_str).unwrap();
            let id = pager.alloc_node_id().unwrap();
            pager.increment_node_count();
            let slot = NodeSlot {
                node_id: id,
                first_out_edge: 999,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
            pager.commit().unwrap();
        }

        // Reopen and verify the committed data survived
        {
            let mut pager = Pager::open(path_str).unwrap();
            assert_eq!(pager.node_count(), 1);
            let slot = pager.read_node_slot(1).unwrap();
            assert_eq!(slot.first_out_edge, 999);
        }
    }

    #[test]
    fn test_crash_recovery_no_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.liel");
        let path_str = path.to_str().unwrap();

        // Extent allocation is lazy, so capture the offset of the first node
        // extent as soon as the initial commit lands.  The dirty WAL entry we
        // inject below targets this page, which is guaranteed to exist on
        // disk and to be covered by the node area.
        let target_offset;

        // First write and commit good data
        let wal_offset = {
            let mut pager = Pager::open(path_str).unwrap();
            let id = pager.alloc_node_id().unwrap();
            pager.increment_node_count();
            let slot = NodeSlot {
                node_id: id,
                first_out_edge: 111,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
            pager.commit().unwrap();
            target_offset = pager.node_extents_for_test()[0];
            pager.header.wal_offset
        };

        // Write WAL entries WITHOUT a Commit entry (simulate a crash mid-txn)
        {
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};
            let mut file = OpenOptions::new().write(true).open(path_str).unwrap();
            let dummy_page = [0xABu8; PAGE_SIZE];
            let entry = Wal::build_write_entry_pub(target_offset, &dummy_page);
            // Write the entry at the real WAL offset read from the header
            // (constant in Phase 1 but we fetch it dynamically for robustness).
            file.seek(SeekFrom::Start(wal_offset)).unwrap();
            file.write_all(&entry).unwrap();
            file.flush().unwrap();
            drop(file);
            // Re-write the header with an updated wal_length AND a matching
            // checksum so `Pager::open` accepts the file.
            patch_header_wal_length(path_str, entry.len() as u64);
        }
        // Update the header wal_length + checksum so the reader sees the entry
        patch_header_wal_length(path_str, 0_u64 /* placeholder */);
        // We actually need the real length; recompute it and re-patch.
        let entry_len = WAL_WRITE_ENTRY_SIZE as u64;
        patch_header_wal_length(path_str, entry_len);

        // Reopen: no Commit in WAL → roll back (dirty WAL entry is discarded)
        {
            let mut pager = Pager::open(path_str).unwrap();
            // Data from the first committed write must still be present
            assert_eq!(pager.node_count(), 1);
            // The uncommitted WAL change (dummy page) must NOT have been applied
            let slot = pager.read_node_slot(1).unwrap();
            assert_eq!(slot.first_out_edge, 111); // value from the first commit
        }
    }

    #[test]
    fn test_wal_checksum_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.liel");
        let path_str = path.to_str().unwrap();

        let target_offset;

        // Commit good data first
        let wal_offset = {
            let mut pager = Pager::open(path_str).unwrap();
            let id = pager.alloc_node_id().unwrap();
            pager.increment_node_count();
            let slot = NodeSlot {
                node_id: id,
                first_out_edge: 42,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
            pager.commit().unwrap();
            target_offset = pager.node_extents_for_test()[0];
            pager.header.wal_offset
        };

        // Now write a WAL entry with a deliberately corrupted CRC
        {
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};
            let dummy_page = [0xFFu8; PAGE_SIZE];
            let mut entry = Wal::build_write_entry_pub(target_offset, &dummy_page);
            // Flip the last byte of the CRC so parse_wal_entries rejects it
            let last = entry.len() - 1;
            entry[last] ^= 0xFF;
            let mut file = OpenOptions::new().write(true).open(path_str).unwrap();
            file.seek(SeekFrom::Start(wal_offset)).unwrap();
            file.write_all(&entry).unwrap();
            file.flush().unwrap();
            drop(file);
            patch_header_wal_length(path_str, entry.len() as u64);
        }
        patch_header_wal_length(path_str, WAL_WRITE_ENTRY_SIZE as u64);

        // Reopen: CRC mismatch → WAL is discarded, data rolls back to last commit
        {
            let mut pager = Pager::open(path_str).unwrap();
            let slot = pager.read_node_slot(1).unwrap();
            assert_eq!(slot.first_out_edge, 42);
        }
    }

    #[test]
    fn test_wal_overflow_rejects_oversized_commit() {
        // Fabricate a dirty-page set large enough to exceed WAL_RESERVED and
        // assert that write_and_commit refuses the commit instead of silently
        // corrupting the node/edge data region.
        use crate::storage::pager::{
            ExtentKind, Pager, NODES_PER_EXTENT, NODES_PER_PAGE, WAL_RESERVED,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("overflow.liel");
        let path_str = path.to_str().unwrap();

        let mut pager = Pager::open(path_str).unwrap();

        // WAL_RESERVED / WAL_WRITE_ENTRY_SIZE is the exact page budget; add a
        // few extra pages on top so the overflow check must trip.
        let capacity_pages = (WAL_RESERVED / WAL_WRITE_ENTRY_SIZE as u64) as usize;
        let extra_pages = 4;
        let needed_pages = capacity_pages + extra_pages;

        // Pre-allocate enough node extents to cover `needed_pages`.  Each
        // extent carries 256 pages, so ceil(needed_pages / 256) extents are
        // enough; we pre-seed them via `ensure_extent_for` so that the
        // subsequent slot writes (which bypass `alloc_node_id`) don't trip
        // the CapacityExceeded guard.
        let max_node_id = (needed_pages * NODES_PER_PAGE) as u64 + 1;
        pager
            .ensure_extent_for(max_node_id, ExtentKind::Node, NODES_PER_EXTENT)
            .unwrap();

        // Write one node slot per target page: NODES_PER_PAGE apart so every
        // write dirties a distinct 4 KiB page.
        for page_idx in 0..needed_pages {
            let node_id = (page_idx * NODES_PER_PAGE + 1) as u64;
            pager.header.next_node_id = pager.header.next_node_id.max(node_id + 1);
            let slot = NodeSlot {
                node_id,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
        }
        assert!(
            pager.dirty_page_offsets().len() > capacity_pages,
            "test setup must exceed WAL capacity (dirty={}, capacity={})",
            pager.dirty_page_offsets().len(),
            capacity_pages,
        );

        match Wal::write_and_commit(&mut pager) {
            Err(crate::error::LielError::WalOverflow(_)) => { /* expected */ }
            other => panic!("expected WalOverflow, got {:?}", other),
        }
    }

    /// Helper: seed the file with one committed node slot and return the
    /// node-extent offset along with the WAL offset from the header.  Used by
    /// the fault-injection tests below as a known-good starting point.
    fn seed_committed_node(path_str: &str, sentinel: u64) -> (u64, u64) {
        let mut pager = Pager::open(path_str).unwrap();
        let id = pager.alloc_node_id().unwrap();
        pager.increment_node_count();
        let slot = NodeSlot {
            node_id: id,
            first_out_edge: sentinel,
            ..Default::default()
        };
        pager.write_node_slot(&slot).unwrap();
        pager.commit().unwrap();
        let target = pager.node_extents_for_test()[0];
        let wal_off = pager.header.wal_offset;
        (target, wal_off)
    }

    /// Helper: write raw bytes at the given absolute file offset, extending
    /// the file if needed.  Used to inject hand-crafted WAL byte streams.
    fn write_raw_at(path_str: &str, offset: u64, bytes: &[u8]) {
        use std::fs::OpenOptions;
        use std::io::{Seek, SeekFrom, Write};
        let mut file = OpenOptions::new().write(true).open(path_str).unwrap();
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(bytes).unwrap();
        file.flush().unwrap();
    }

    /// A WAL whose last committed entry is truncated halfway through its
    /// payload must be treated as "no commit found" and roll back to the
    /// previous on-disk state instead of applying garbage.
    #[test]
    fn test_wal_truncated_in_middle_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("truncated.liel");
        let path_str = path.to_str().unwrap();

        let (target_offset, wal_offset) = seed_committed_node(path_str, 555);

        // Build a Write+Commit byte stream targeting `target_offset` with a
        // sentinel page payload, then truncate it in the middle of the Write
        // entry's data section so the commit marker is missing entirely.
        let payload = [0xCDu8; PAGE_SIZE];
        let mut blob = Wal::build_write_entry_pub(target_offset, &payload);
        let commit = Wal::build_commit_entry();
        blob.extend_from_slice(&commit);
        let truncated_len = WAL_ENTRY_HEADER_SIZE + (PAGE_SIZE / 2);
        blob.truncate(truncated_len);

        write_raw_at(path_str, wal_offset, &blob);
        // Header still advertises the truncated length (a real torn-write
        // would record the writer's intent here, while the bytes on disk
        // are short).
        patch_header_wal_length(path_str, blob.len() as u64);

        // Reopen: parser should bail out at the missing CRC, no commit
        // marker is found, recovery rolls back to the seed value.
        let mut pager = Pager::open(path_str).unwrap();
        assert_eq!(pager.node_count(), 1);
        let slot = pager.read_node_slot(1).unwrap();
        assert_eq!(slot.first_out_edge, 555);
        assert_eq!(pager.header.wal_length, 0);
    }

    /// Trailing garbage past a valid Commit entry must not prevent recovery
    /// from applying the committed write.  The parser stops at the bogus
    /// trailing bytes (CRC mismatch / unparseable length) but the prior
    /// commit is still honored.
    #[test]
    fn test_wal_trailing_garbage_after_commit_is_discarded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trailing.liel");
        let path_str = path.to_str().unwrap();

        let (target_offset, wal_offset) = seed_committed_node(path_str, 111);

        // Build a complete Write+Commit then append junk bytes after the
        // commit marker.  The parser will halt at the junk but `has_commit`
        // remains true, so the Write entry is applied.
        let mut payload = [0u8; PAGE_SIZE];
        // Place a recognisable byte pattern at the slot's `first_out_edge`
        // field so we can assert the write actually landed.
        // Page header + first slot's `first_out_edge` field (offset 8 within
        // a NodeSlot — see format-spec §3.2).
        let slot_offset = PAGE_HEADER_SIZE + 8;
        payload[slot_offset..slot_offset + 8].copy_from_slice(&777u64.to_le_bytes());
        let mut blob = Wal::build_write_entry_pub(target_offset, &payload);
        blob.extend_from_slice(&Wal::build_commit_entry());
        let garbage = [0xEFu8; 64];
        blob.extend_from_slice(&garbage);

        write_raw_at(path_str, wal_offset, &blob);
        patch_header_wal_length(path_str, blob.len() as u64);

        let mut pager = Pager::open(path_str).unwrap();
        let slot = pager.read_node_slot(1).unwrap();
        assert_eq!(
            slot.first_out_edge, 777,
            "committed write must be applied even when trailing bytes are garbage"
        );
        assert_eq!(pager.header.wal_length, 0);
    }

    /// A Write entry whose `entry_length` field claims more bytes than the
    /// WAL actually contains must not be applied; recovery should roll back.
    /// This guards against header-level length corruption (e.g. a single
    /// flipped bit in the length prefix) that would otherwise cause
    /// `parse_wal_entries` to read past the WAL region.
    #[test]
    fn test_wal_oversized_entry_length_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized.liel");
        let path_str = path.to_str().unwrap();

        let (target_offset, wal_offset) = seed_committed_node(path_str, 222);

        let payload = [0xAAu8; PAGE_SIZE];
        let mut entry = Wal::build_write_entry_pub(target_offset, &payload);
        // Inflate the entry_length field to claim 4× the real length.  The
        // parser must notice that pos + entry_length > wal_bytes.len() and
        // bail out before touching the data section.
        let inflated = (entry.len() as u32).saturating_mul(4);
        entry[0..4].copy_from_slice(&inflated.to_le_bytes());

        let blob = entry;
        write_raw_at(path_str, wal_offset, &blob);
        patch_header_wal_length(path_str, blob.len() as u64);

        let mut pager = Pager::open(path_str).unwrap();
        let slot = pager.read_node_slot(1).unwrap();
        assert_eq!(slot.first_out_edge, 222);
        assert_eq!(pager.header.wal_length, 0);
    }

    /// An `entry_length` field smaller than the minimum byte count for a
    /// well-formed entry (length + op_type + page_offset + CRC = 17) is a
    /// hard signal that the WAL prefix is corrupt.  Recovery must treat it as
    /// end-of-WAL and fall back to the previous committed state, not panic on
    /// a slice out of bounds.
    #[test]
    fn test_wal_undersized_entry_length_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("undersized.liel");
        let path_str = path.to_str().unwrap();

        let (_target_offset, wal_offset) = seed_committed_node(path_str, 444);

        // Forge an entry whose length field claims 10 bytes (< MIN_ENTRY_LEN).
        // Padded with zeros so the header checksum rewrite below still sees a
        // deterministic region.
        let mut blob = vec![0u8; 32];
        let bogus_len: u32 = 10;
        blob[0..4].copy_from_slice(&bogus_len.to_le_bytes());
        write_raw_at(path_str, wal_offset, &blob);
        patch_header_wal_length(path_str, blob.len() as u64);

        // Should not panic; should preserve the previous committed state.
        let mut pager = Pager::open(path_str).unwrap();
        let slot = pager.read_node_slot(1).unwrap();
        assert_eq!(slot.first_out_edge, 444);
        assert_eq!(pager.header.wal_length, 0);
    }

    /// A Write entry whose `data_length` field is not exactly `PAGE_SIZE` is
    /// a structural corruption of the format (the WAL invariant is that every
    /// Write payload is a full page).  Recovery surfaces this as
    /// `CorruptedFile` rather than silently rolling back, because a non-page
    /// payload length indicates a bug in the writer, not a torn log.
    #[test]
    fn test_wal_wrong_data_length_returns_corrupted_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baddatalen.liel");
        let path_str = path.to_str().unwrap();

        let (target_offset, wal_offset) = seed_committed_node(path_str, 666);

        // Build a full Write entry, then overwrite the `data_length` field
        // (bytes 13..17) with a non-PAGE_SIZE value.  CRC still matches the
        // original bytes so the parser reaches the `data_length != PAGE_SIZE`
        // check and surfaces `CorruptedFile` instead of silently recovering.
        let payload = [0xDDu8; PAGE_SIZE];
        let mut entry = Wal::build_write_entry_pub(target_offset, &payload);
        let bad_data_len: u32 = (PAGE_SIZE as u32) - 1;
        entry[13..17].copy_from_slice(&bad_data_len.to_le_bytes());
        // Recompute the CRC so the parser gets past the CRC check and hits
        // the data_length branch we're actually exercising.
        let new_crc = crc32(&entry[..entry.len() - WAL_CRC_SIZE]);
        let crc_start = entry.len() - WAL_CRC_SIZE;
        entry[crc_start..].copy_from_slice(&new_crc.to_le_bytes());

        write_raw_at(path_str, wal_offset, &entry);
        patch_header_wal_length(path_str, entry.len() as u64);

        match Pager::open(path_str) {
            Err(LielError::CorruptedFile(msg)) => {
                assert!(
                    msg.contains("data_length"),
                    "expected data_length mention, got {msg:?}"
                );
            }
            Err(other) => panic!("expected CorruptedFile, got error {other:?}"),
            Ok(_) => panic!("expected CorruptedFile, got Ok(Pager)"),
        }
    }

    /// A valid Write entry followed by a Commit entry whose CRC is broken
    /// must NOT be applied: the commit marker is unparseable, so
    /// `has_commit` stays false and recovery rolls back.
    #[test]
    fn test_wal_corrupt_commit_after_valid_write_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("badcommit.liel");
        let path_str = path.to_str().unwrap();

        let (target_offset, wal_offset) = seed_committed_node(path_str, 333);

        let payload = [0xBBu8; PAGE_SIZE];
        let write_entry = Wal::build_write_entry_pub(target_offset, &payload);
        let mut commit_entry = Wal::build_commit_entry();
        // Flip every CRC byte so the commit cannot validate.
        let crc_start = commit_entry.len() - WAL_CRC_SIZE;
        for byte in commit_entry[crc_start..].iter_mut() {
            *byte ^= 0xFF;
        }

        let mut blob = Vec::with_capacity(write_entry.len() + commit_entry.len());
        blob.extend_from_slice(&write_entry);
        blob.extend_from_slice(&commit_entry);
        write_raw_at(path_str, wal_offset, &blob);
        patch_header_wal_length(path_str, blob.len() as u64);

        let mut pager = Pager::open(path_str).unwrap();
        let slot = pager.read_node_slot(1).unwrap();
        assert_eq!(
            slot.first_out_edge, 333,
            "write must not be applied when the matching commit fails CRC"
        );
        assert_eq!(pager.header.wal_length, 0);
    }

    #[test]
    fn test_recovery_invalidates_cached_extent_index_pages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery-cache.liel");
        let path_str = path.to_str().unwrap();

        let mut target_node_id = 0;
        let mut target_name = String::new();

        {
            let mut pager = Pager::open(path_str).unwrap();
            for i in 0..=crate::storage::pager::NODES_PER_EXTENT {
                let mut props = HashMap::new();
                let name = format!("node-{i}");
                props.insert("name".into(), PropValue::String(name.clone()));
                let node = add_node(&mut pager, vec![], props).unwrap();
                if i == crate::storage::pager::NODES_PER_EXTENT {
                    target_node_id = node.id;
                    target_name = name;
                }
            }

            // Persist the full transaction through the WAL but stop before the
            // data pages are flushed to their final locations, simulating a
            // crash after the WAL became durable.
            let dirty_offsets = pager.dirty_page_offsets();
            assert!(
                !dirty_offsets.is_empty(),
                "test setup must dirty pages so recovery has work to do"
            );

            let required_bytes = dirty_offsets.len() as u64 * WAL_WRITE_ENTRY_SIZE as u64
                + WAL_COMMIT_ENTRY_SIZE as u64;
            assert!(required_bytes <= WAL_RESERVED);

            let mut wal_bytes: Vec<u8> = Vec::with_capacity(required_bytes as usize);
            for &page_offset in &dirty_offsets {
                let page = pager.get_dirty_page(page_offset).unwrap().to_owned();
                let entry = Wal::build_write_entry_pub(page_offset, &page);
                wal_bytes.extend_from_slice(&entry);
            }
            wal_bytes.extend_from_slice(&Wal::build_commit_entry());

            let wal_offset = pager.header.wal_offset;
            Wal::write_wal_to_storage(pager.storage_mut(), wal_offset, &wal_bytes).unwrap();
            pager.header.wal_length = wal_bytes.len() as u64;
            pager.write_header().unwrap();
            pager.flush_storage().unwrap();
        }

        {
            let mut pager = Pager::open(path_str).unwrap();
            let node = get_node(&mut pager, target_node_id)
                .unwrap()
                .expect("recovered node allocated in the WAL-backed extent should be visible");

            assert_eq!(pager.header.wal_length, 0);
            match node.properties.get("name") {
                Some(PropValue::String(name)) => assert_eq!(name, &target_name),
                other => panic!("expected recovered name property, got {other:?}"),
            }
        }
    }
}

use std::collections::HashMap;

use super::cache::{PageCache, DEFAULT_CACHE_CAPACITY};
use super::file::{FileStorage, MemoryStorage};
use super::serializer::{EdgeSlot, NodeSlot, EDGE_SLOT_SIZE, NODE_SLOT_SIZE};
use super::traits::Storage;
use super::wal::Wal;
use crate::error::{LielError, Result};

// ─── Page / slot constants ──────────────────────────────────────────

/// Fixed page size.  Every read/write goes through a page of this size and all
/// section boundaries (WAL, extent metadata pages, data extents) are aligned
/// to a multiple of `PAGE_SIZE`.
pub const PAGE_SIZE: usize = 4096;

/// Bytes reserved at the start of each data page for the node/edge page
/// header (page type + slot count + used count + reserved).  Slot payloads
/// begin at `page_offset + PAGE_HEADER_SIZE`.  See the §3.1 "Node / Edge page
/// common header" table in `docs/reference/format-spec.ja.md`.
pub const PAGE_HEADER_SIZE: usize = 8;

/// Node slots per data page: each page reserves [`PAGE_HEADER_SIZE`] bytes at
/// the start, after which node slots are packed back-to-back.
pub const NODES_PER_PAGE: usize = (PAGE_SIZE - PAGE_HEADER_SIZE) / NODE_SLOT_SIZE; // = 63

/// Edge slots per data page (same layout rationale as [`NODES_PER_PAGE`]).
pub const EDGES_PER_PAGE: usize = (PAGE_SIZE - PAGE_HEADER_SIZE) / EDGE_SLOT_SIZE; // = 51

// ─── On-disk file layout constants ─────────────────────────────────

const MAGIC: &[u8; 16] = b"LIEL\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
const HEADER_SIZE: u64 = 128;
const FORMAT_VERSION_MAJOR: u16 = 1;
const FORMAT_VERSION_MINOR: u16 = 0;

/// File offset of the start of the WAL section.  The WAL occupies a fixed-size
/// reservation immediately after the 4 KiB header page.
pub const WAL_OFFSET: u64 = PAGE_SIZE as u64; // 4096

/// Reserved size of the WAL section (4 MiB).  A single transaction must fit
/// into this reservation; larger transactions must be split by the caller.
pub const WAL_RESERVED: u64 = 1024 * PAGE_SIZE as u64;

/// First byte offset at which the pager may allocate data extents and index
/// pages.  Equal to the end of the WAL reservation.
pub const FIRST_ALLOCATABLE_OFFSET: u64 = WAL_OFFSET + WAL_RESERVED;

// ─── Extent allocator constants ────────────────────────────────────

/// Pages per data extent.  Each time the pager runs out of slot space for a
/// given kind (node / edge / prop), it allocates a fresh extent of this many
/// pages at the end of the file and registers it in that kind's extent chain.
pub const EXTENT_PAGES: u64 = 256;

/// Bytes per data extent (1 MiB at 4 KiB pages).
pub const EXTENT_BYTES: u64 = EXTENT_PAGES * PAGE_SIZE as u64;

/// Number of node slots a single extent can hold.  Used for O(1) address
/// computation inside [`Pager::node_slot_file_offset`].
pub const NODES_PER_EXTENT: u64 = EXTENT_PAGES * NODES_PER_PAGE as u64; // 16 128

/// Number of edge slots a single extent can hold.
pub const EDGES_PER_EXTENT: u64 = EXTENT_PAGES * EDGES_PER_PAGE as u64; // 13 056

/// Maximum data bytes a single property-blob write can consume.  A blob must
/// fit entirely inside one extent; larger blobs are rejected with
/// [`LielError::InvalidArgument`].  The practical payload limit is a few bytes
/// shy of [`EXTENT_BYTES`] because we reserve [`PAGE_HEADER_SIZE`] bytes at the
/// head of each page to keep the layout homogeneous with slot pages.
pub const MAX_PROP_BLOB_BYTES: u64 = EXTENT_BYTES - PAGE_HEADER_SIZE as u64;

/// Extent-offset entries per index page.  An index page lays out its 4 KiB as
/// a 16-byte header (next-pointer + count + reserved) followed by this many
/// 8-byte offsets.
pub const INDEX_ENTRIES_PER_PAGE: usize = (PAGE_SIZE - 16) / 8; // 510

/// Hard ceiling on the number of extents of a single kind, purely as a
/// defence-in-depth guard against a runaway counter walking out of `u64`.
/// At 1 MiB per extent this allows up to ~2^32 MiB per kind, far beyond any
/// realistic workload and still safely within u64 arithmetic.
const MAX_EXTENTS_PER_KIND: u64 = u32::MAX as u64;

// ─── File header ───────────────────────────────────────────────────

/// Strongly-typed view of the 128-byte file header at offset 0.
///
/// ```text
///   bytes   field
///   0..16   magic "LIEL\0...\0"
///  16..18   version major (currently 1)
///  18..20   version minor (currently 0)
///  20..24   page size (4096)
///  24..32   node_count       -- live (non-deleted) node count
///  32..40   edge_count
///  40..48   next_node_id     -- monotonically increasing allocator
///  48..56   next_edge_id
///  56..64   node_table_head  -- file offset of the first node extent-index page (0 = empty)
///  64..72   edge_table_head
///  72..80   prop_table_head
///  80..88   next_prop_write_offset -- byte offset where the next property blob goes (0 = no prop extent yet)
///  88..96   wal_offset       -- fixed = 4096, kept in the header for WAL-recovery tools
///  96..104  wal_length       -- bytes of live WAL entries
/// 104..112  xor checksum of bytes 0..104
/// 112..128  reserved (zero-filled)
/// ```
///
/// The three `*_table_head` fields form the entry point into a linked list of
/// index pages, one chain per kind.  Walking the chain at open time rebuilds
/// the in-memory `Vec<u64>` of data-extent offsets that addresses slot pages
/// in O(1).
#[derive(Debug, Clone)]
pub struct FileHeader {
    pub node_count: u64,
    pub edge_count: u64,
    pub next_node_id: u64,
    pub next_edge_id: u64,
    pub node_table_head: u64,
    pub edge_table_head: u64,
    pub prop_table_head: u64,
    /// Byte offset in the file where the next property-blob byte will be
    /// written.  Zero is a sentinel meaning "no prop extent has been allocated
    /// yet" — the next `append_prop` call will allocate one.
    pub next_prop_write_offset: u64,
    pub wal_offset: u64,
    pub wal_length: u64,
}

impl FileHeader {
    pub fn new_empty() -> Self {
        Self {
            node_count: 0,
            edge_count: 0,
            next_node_id: 1,
            next_edge_id: 1,
            node_table_head: 0,
            edge_table_head: 0,
            prop_table_head: 0,
            next_prop_write_offset: 0,
            wal_offset: WAL_OFFSET,
            wal_length: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE as usize] {
        let mut buf = [0u8; HEADER_SIZE as usize];
        buf[0..16].copy_from_slice(MAGIC);
        buf[16..18].copy_from_slice(&FORMAT_VERSION_MAJOR.to_le_bytes());
        buf[18..20].copy_from_slice(&FORMAT_VERSION_MINOR.to_le_bytes());
        buf[20..24].copy_from_slice(&(PAGE_SIZE as u32).to_le_bytes());
        buf[24..32].copy_from_slice(&self.node_count.to_le_bytes());
        buf[32..40].copy_from_slice(&self.edge_count.to_le_bytes());
        buf[40..48].copy_from_slice(&self.next_node_id.to_le_bytes());
        buf[48..56].copy_from_slice(&self.next_edge_id.to_le_bytes());
        buf[56..64].copy_from_slice(&self.node_table_head.to_le_bytes());
        buf[64..72].copy_from_slice(&self.edge_table_head.to_le_bytes());
        buf[72..80].copy_from_slice(&self.prop_table_head.to_le_bytes());
        buf[80..88].copy_from_slice(&self.next_prop_write_offset.to_le_bytes());
        buf[88..96].copy_from_slice(&self.wal_offset.to_le_bytes());
        buf[96..104].copy_from_slice(&self.wal_length.to_le_bytes());
        let checksum: u64 = buf[0..104].chunks(8).fold(0u64, |acc, chunk| {
            let mut bytes = [0u8; 8];
            bytes[..chunk.len()].copy_from_slice(chunk);
            acc ^ u64::from_le_bytes(bytes)
        });
        buf[104..112].copy_from_slice(&checksum.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; HEADER_SIZE as usize]) -> Result<Self> {
        if &buf[0..16] != MAGIC {
            return Err(LielError::CorruptedFile("invalid magic bytes".into()));
        }
        let version_major = u16::from_le_bytes(
            buf[16..18]
                .try_into()
                .expect("BUG: pager slice indices statically within fixed-size buf"),
        );
        let version_minor = u16::from_le_bytes(
            buf[18..20]
                .try_into()
                .expect("BUG: pager slice indices statically within fixed-size buf"),
        );
        if version_major != FORMAT_VERSION_MAJOR || version_minor != FORMAT_VERSION_MINOR {
            return Err(LielError::CorruptedFile(format!(
                "unsupported file format version: {version_major}.{version_minor} \
                 (supported: {FORMAT_VERSION_MAJOR}.{FORMAT_VERSION_MINOR})"
            )));
        }
        let page_size = u32::from_le_bytes(
            buf[20..24]
                .try_into()
                .expect("BUG: pager slice indices statically within fixed-size buf"),
        );
        if page_size != PAGE_SIZE as u32 {
            return Err(LielError::CorruptedFile(format!(
                "unsupported page size: {page_size}"
            )));
        }
        // Validate the XOR checksum over the meaningful header bytes.  If the
        // file was written by an older layout (different byte positions) the
        // checksum almost certainly won't match and the user gets a clean
        // CorruptedFile error instead of silently using garbage offsets.
        let expected: u64 = buf[0..104].chunks(8).fold(0u64, |acc, chunk| {
            let mut bytes = [0u8; 8];
            bytes[..chunk.len()].copy_from_slice(chunk);
            acc ^ u64::from_le_bytes(bytes)
        });
        let stored = u64::from_le_bytes(
            buf[104..112]
                .try_into()
                .expect("BUG: pager slice indices statically within fixed-size buf"),
        );
        if expected != stored {
            return Err(LielError::CorruptedFile(format!(
                "header checksum mismatch: expected 0x{expected:016x}, got 0x{stored:016x}",
            )));
        }
        Ok(Self {
            node_count: u64::from_le_bytes(
                buf[24..32]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            edge_count: u64::from_le_bytes(
                buf[32..40]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            next_node_id: u64::from_le_bytes(
                buf[40..48]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            next_edge_id: u64::from_le_bytes(
                buf[48..56]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            node_table_head: u64::from_le_bytes(
                buf[56..64]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            edge_table_head: u64::from_le_bytes(
                buf[64..72]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            prop_table_head: u64::from_le_bytes(
                buf[72..80]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            next_prop_write_offset: u64::from_le_bytes(
                buf[80..88]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            wal_offset: u64::from_le_bytes(
                buf[88..96]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
            wal_length: u64::from_le_bytes(
                buf[96..104]
                    .try_into()
                    .expect("BUG: pager slice indices statically within fixed-size buf"),
            ),
        })
    }
}

// ─── Extent chain state ────────────────────────────────────────────

/// Per-kind metadata for the chain of data extents.
///
/// Walking the on-disk chain at open time populates the `extents` vector;
/// subsequent slot lookups use `extents[extent_idx]` as a direct jump target.
/// The `tail_page_offset` and `tail_page_count` fields cache the write cursor
/// so appending a new extent does not have to walk the chain again.
#[derive(Debug, Default, Clone)]
struct ExtentChain {
    /// Offsets of every data extent belonging to this kind, in allocation
    /// order.  Indexed by `(id - 1) / slots_per_extent`.
    extents: Vec<u64>,
    /// Offset of the current tail index page, or 0 when no index page has been
    /// allocated yet.  `header.*_table_head` points at the first index page
    /// and each index page stores a `next_page_offset` pointer to the next.
    tail_page_offset: u64,
    /// Number of entries used in the tail index page.  A fresh index page has
    /// `tail_page_count == 0`; once it reaches [`INDEX_ENTRIES_PER_PAGE`] the
    /// next `push` allocates a new index page and chains into it.
    tail_page_count: u32,
}

/// Identifies the three extent chains the pager manages.  Used to route
/// allocation and persistence calls to the correct in-memory state and
/// on-disk head pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtentKind {
    Node,
    Edge,
    Prop,
}

impl ExtentKind {
    fn as_str(self) -> &'static str {
        match self {
            ExtentKind::Node => "node",
            ExtentKind::Edge => "edge",
            ExtentKind::Prop => "prop",
        }
    }

    fn capacity_limit(self) -> (u64, &'static str) {
        match self {
            ExtentKind::Node => (MAX_EXTENTS_PER_KIND * NODES_PER_EXTENT, "nodes"),
            ExtentKind::Edge => (MAX_EXTENTS_PER_KIND * EDGES_PER_EXTENT, "edges"),
            ExtentKind::Prop => (MAX_EXTENTS_PER_KIND * EXTENT_BYTES, "bytes of prop storage"),
        }
    }
}

/// Decode an index page into (next_page_offset, count, entries).
fn decode_index_page(page: &[u8; PAGE_SIZE]) -> (u64, u32, Vec<u64>) {
    let next = u64::from_le_bytes(
        page[0..8]
            .try_into()
            .expect("BUG: pager slice indices statically within fixed-size buf"),
    );
    let count = u32::from_le_bytes(
        page[8..12]
            .try_into()
            .expect("BUG: pager slice indices statically within fixed-size buf"),
    );
    let capped = (count as usize).min(INDEX_ENTRIES_PER_PAGE);
    let mut entries = Vec::with_capacity(capped);
    for i in 0..capped {
        let base = 16 + i * 8;
        entries.push(u64::from_le_bytes(
            page[base..base + 8]
                .try_into()
                .expect("BUG: pager slice indices statically within fixed-size buf"),
        ));
    }
    (next, count, entries)
}

// ─── Pager ─────────────────────────────────────────────────────────

/// Page manager: coordinates all reads and writes to the underlying storage.
pub struct Pager {
    pub(crate) storage: Box<dyn Storage>,
    cache: PageCache,
    pub header: FileHeader,
    /// Dirty-page write buffer: maps file offset → 4096-byte page data awaiting commit.
    dirty: HashMap<u64, Box<[u8; PAGE_SIZE]>>,

    // ─── Extent allocator state ─────────────────────────────
    node_chain: ExtentChain,
    edge_chain: ExtentChain,
    prop_chain: ExtentChain,
    /// Byte offset at which the next file-tail allocation will place the new
    /// extent / index page.  Monotonically increases; rollback reloads it from
    /// the persisted extent chains.  Kept page-aligned at all times.
    allocated_eof: u64,
}

impl Pager {
    /// Open a database file at `path`, or create it if it does not exist.
    /// Pass `":memory:"` to use a volatile in-memory store (no file I/O).
    pub fn open(path: &str) -> Result<Self> {
        let (storage, is_new): (Box<dyn Storage>, bool) = if path == ":memory:" {
            (Box::new(MemoryStorage::new()), true)
        } else {
            let exists = std::path::Path::new(path).exists();
            (Box::new(FileStorage::open(path)?), !exists)
        };

        let mut pager = Self {
            storage,
            cache: PageCache::new(DEFAULT_CACHE_CAPACITY),
            header: FileHeader::new_empty(),
            dirty: HashMap::new(),
            node_chain: ExtentChain::default(),
            edge_chain: ExtentChain::default(),
            prop_chain: ExtentChain::default(),
            allocated_eof: FIRST_ALLOCATABLE_OFFSET,
        };

        if is_new {
            pager.write_header()?;
        } else {
            pager.load_header()?;
            pager.load_extent_chains()?;
            Wal::recover(&mut pager)?;
            // WAL recovery writes pages directly to storage, so any pages read
            // before recovery may now be stale in the page cache.
            pager.cache.clear();
            // WAL recovery may have rewritten extent-index pages; refresh the
            // in-memory chains so tail pointers reflect the latest on-disk state.
            pager.load_extent_chains()?;
        }

        Ok(pager)
    }

    fn load_header(&mut self) -> Result<()> {
        let size = self.storage.file_size();
        if size < HEADER_SIZE {
            return Err(LielError::CorruptedFile(
                "file too small to contain header".into(),
            ));
        }
        let mut buf = [0u8; HEADER_SIZE as usize];
        self.storage.seek_and_read(0, &mut buf)?;
        self.header = FileHeader::from_bytes(&buf)?;
        Ok(())
    }

    pub fn write_header(&mut self) -> Result<()> {
        let buf = self.header.to_bytes();
        self.storage.seek_and_write(0, &buf)?;
        Ok(())
    }

    // ─── Extent-chain loading ───────────────────────────────

    /// Walk all three extent-index chains and populate the in-memory
    /// `ExtentChain` state from the persisted linked-list of index pages.
    ///
    /// Also derives [`Pager::allocated_eof`] so that the next file-tail
    /// allocation lands past every known extent and index page.
    fn load_extent_chains(&mut self) -> Result<()> {
        let node_chain = self.walk_chain(self.header.node_table_head, ExtentKind::Node)?;
        let edge_chain = self.walk_chain(self.header.edge_table_head, ExtentKind::Edge)?;
        let prop_chain = self.walk_chain(self.header.prop_table_head, ExtentKind::Prop)?;

        let mut eof = self.storage.file_size().max(FIRST_ALLOCATABLE_OFFSET);
        for chain in [&node_chain, &edge_chain, &prop_chain] {
            for &ext in &chain.extents {
                eof = eof.max(ext + EXTENT_BYTES);
            }
            if chain.tail_page_offset != 0 {
                eof = eof.max(chain.tail_page_offset + PAGE_SIZE as u64);
            }
        }
        // Round up to a page boundary so every future allocation is aligned.
        // (`u64::is_multiple_of` would be cleaner here but is only stable on
        // Rust 1.87+; we keep the modulo form to honour the MSRV in
        // Cargo.toml.)
        if eof % (PAGE_SIZE as u64) != 0 {
            eof = (eof / PAGE_SIZE as u64 + 1) * PAGE_SIZE as u64;
        }

        self.node_chain = node_chain;
        self.edge_chain = edge_chain;
        self.prop_chain = prop_chain;
        self.allocated_eof = eof;
        Ok(())
    }

    fn walk_chain(&mut self, head: u64, kind: ExtentKind) -> Result<ExtentChain> {
        let mut chain = ExtentChain::default();
        if head == 0 {
            return Ok(chain);
        }
        let mut next_offset = head;
        let mut last_offset = 0;
        let mut last_count: u32 = 0;
        let mut visited: std::collections::HashSet<u64> = std::collections::HashSet::new();
        while next_offset != 0 {
            if !visited.insert(next_offset) {
                return Err(LielError::CorruptedFile(format!(
                    "{kind_str} extent-index chain contains a cycle at 0x{offset:x}",
                    kind_str = kind.as_str(),
                    offset = next_offset,
                )));
            }
            let page = self.read_page(next_offset)?;
            let (next, count, entries) = decode_index_page(&page);
            if (count as usize) > INDEX_ENTRIES_PER_PAGE {
                return Err(LielError::CorruptedFile(format!(
                    "{} extent-index page at 0x{:x} reports count={} > max {}",
                    kind.as_str(),
                    next_offset,
                    count,
                    INDEX_ENTRIES_PER_PAGE,
                )));
            }
            chain.extents.extend(entries);
            last_offset = next_offset;
            last_count = count;
            next_offset = next;
        }
        chain.tail_page_offset = last_offset;
        chain.tail_page_count = last_count;
        Ok(chain)
    }

    // ─── Slot-offset calculation (extent-aware) ────────────

    fn slot_file_offset(
        &self,
        id: u64,
        kind: ExtentKind,
        chain: &ExtentChain,
        slots_per_extent: u64,
        slots_per_page: u64,
        slot_size: u64,
    ) -> Result<u64> {
        assert!(id >= 1, "id must be >= 1");
        let idx = id - 1;
        let extent_idx = idx / slots_per_extent;
        if extent_idx >= chain.extents.len() as u64 {
            // The caller asked for a slot whose extent has never been
            // allocated.  This is a corruption-grade signal (e.g. the header
            // claims `next_node_id` is larger than what the extent chain can
            // address).  We surface it as CapacityExceeded so the error is
            // actionable without being mistaken for a normal "not found".
            return Err(LielError::CapacityExceeded {
                kind: kind.as_str(),
                limit: chain.extents.len() as u64 * slots_per_extent,
                unit: match kind {
                    ExtentKind::Node => "nodes",
                    ExtentKind::Edge => "edges",
                    ExtentKind::Prop => "bytes of prop storage",
                },
            });
        }
        let extent_offset = chain.extents[extent_idx as usize];
        let in_extent = idx % slots_per_extent;
        let page_in_extent = in_extent / slots_per_page;
        let slot_in_page = in_extent % slots_per_page;
        Ok(extent_offset
            + page_in_extent * PAGE_SIZE as u64
            + PAGE_HEADER_SIZE as u64
            + slot_in_page * slot_size)
    }

    /// Return the byte offset in the file for the slot of `node_id` (1-based).
    pub fn node_slot_file_offset(&self, node_id: u64) -> Result<u64> {
        self.slot_file_offset(
            node_id,
            ExtentKind::Node,
            &self.node_chain,
            NODES_PER_EXTENT,
            NODES_PER_PAGE as u64,
            NODE_SLOT_SIZE as u64,
        )
    }

    /// Return the byte offset in the file for the slot of `edge_id` (1-based).
    pub fn edge_slot_file_offset(&self, edge_id: u64) -> Result<u64> {
        self.slot_file_offset(
            edge_id,
            ExtentKind::Edge,
            &self.edge_chain,
            EDGES_PER_EXTENT,
            EDGES_PER_PAGE as u64,
            EDGE_SLOT_SIZE as u64,
        )
    }

    // ─── Node slot read / write ───────────────────────────────────

    pub fn read_node_slot(&mut self, node_id: u64) -> Result<NodeSlot> {
        let file_offset = self.node_slot_file_offset(node_id)?;
        let page_offset = file_offset - (file_offset % PAGE_SIZE as u64);
        let offset_in_page = (file_offset % PAGE_SIZE as u64) as usize;

        let page = self.read_page(page_offset)?;
        let arr: &[u8; NODE_SLOT_SIZE] = page[offset_in_page..offset_in_page + NODE_SLOT_SIZE]
            .try_into()
            .expect("BUG: page is PAGE_SIZE bytes and slot offset is bounds-checked");
        Ok(NodeSlot::read_from(arr))
    }

    pub fn write_node_slot(&mut self, slot: &NodeSlot) -> Result<()> {
        let file_offset = self.node_slot_file_offset(slot.node_id)?;
        let page_offset = file_offset - (file_offset % PAGE_SIZE as u64);
        let offset_in_page = (file_offset % PAGE_SIZE as u64) as usize;

        let mut page = self.read_page_for_write(page_offset)?;
        let mut slot_buf = [0u8; NODE_SLOT_SIZE];
        slot.write_to(&mut slot_buf);
        page[offset_in_page..offset_in_page + NODE_SLOT_SIZE].copy_from_slice(&slot_buf);
        self.mark_dirty(page_offset, page);
        Ok(())
    }

    // ─── Edge slot read / write ──────────────────────────────────

    pub fn read_edge_slot(&mut self, edge_id: u64) -> Result<EdgeSlot> {
        let file_offset = self.edge_slot_file_offset(edge_id)?;
        let page_offset = file_offset - (file_offset % PAGE_SIZE as u64);
        let offset_in_page = (file_offset % PAGE_SIZE as u64) as usize;

        let page = self.read_page(page_offset)?;
        let arr: &[u8; EDGE_SLOT_SIZE] = page[offset_in_page..offset_in_page + EDGE_SLOT_SIZE]
            .try_into()
            .expect("BUG: page is PAGE_SIZE bytes and slot offset is bounds-checked");
        Ok(EdgeSlot::read_from(arr))
    }

    pub fn write_edge_slot(&mut self, slot: &EdgeSlot) -> Result<()> {
        let file_offset = self.edge_slot_file_offset(slot.edge_id)?;
        let page_offset = file_offset - (file_offset % PAGE_SIZE as u64);
        let offset_in_page = (file_offset % PAGE_SIZE as u64) as usize;

        let mut page = self.read_page_for_write(page_offset)?;
        let mut slot_buf = [0u8; EDGE_SLOT_SIZE];
        slot.write_to(&mut slot_buf);
        page[offset_in_page..offset_in_page + EDGE_SLOT_SIZE].copy_from_slice(&slot_buf);
        self.mark_dirty(page_offset, page);
        Ok(())
    }

    // ─── Property / label read / write ──────────────────────────

    /// Append a raw byte slice to **property storage** (prop extents) and return
    /// the **absolute file offset** it was written to.  The returned offset is
    /// stored in the node/edge slot so the blob can be retrieved with
    /// [`Pager::read_prop`].
    ///
    /// Internally this maintains a per-property-extent write cursor and
    /// allocates fresh prop extents at end-of-file when a blob does not fit in
    /// the remaining space of the current one.  Blobs larger than one extent
    /// are rejected with [`LielError::InvalidArgument`].
    pub fn append_prop(&mut self, data: &[u8]) -> Result<u64> {
        if data.is_empty() {
            return Ok(0);
        }
        if data.len() as u64 > MAX_PROP_BLOB_BYTES {
            return Err(LielError::InvalidArgument(format!(
                "property blob of {} bytes exceeds the per-extent limit of {} bytes",
                data.len(),
                MAX_PROP_BLOB_BYTES,
            )));
        }
        let needed = data.len() as u64;
        let current_extent_end = self
            .prop_chain
            .extents
            .last()
            .map(|off| *off + EXTENT_BYTES);
        let cursor_fits = self.header.next_prop_write_offset != 0
            && current_extent_end
                .map(|end| self.header.next_prop_write_offset + needed <= end)
                .unwrap_or(false);
        if !cursor_fits {
            let new_extent = self.allocate_extent(ExtentKind::Prop)?;
            self.header.next_prop_write_offset = new_extent;
        }
        let offset = self.header.next_prop_write_offset;
        self.write_bytes_at(offset, data)?;
        self.header.next_prop_write_offset = offset + needed;
        Ok(offset)
    }

    /// Read `length` bytes of property data from the given `offset`.
    pub fn read_prop(&mut self, offset: u64, length: u32) -> Result<Vec<u8>> {
        if length == 0 {
            return Ok(Vec::new());
        }
        self.read_bytes_at(offset, length as usize)
    }

    // ─── Generic page read / write ─────────────────────────────────

    fn read_page(&mut self, page_offset: u64) -> Result<[u8; PAGE_SIZE]> {
        if let Some(page) = self.dirty.get(&page_offset) {
            return Ok(**page);
        }
        if let Some(cached) = self.cache.get(page_offset) {
            return Ok(*cached);
        }
        let file_size = self.storage.file_size();
        if page_offset + PAGE_SIZE as u64 > file_size {
            return Ok([0u8; PAGE_SIZE]);
        }
        let page = self.storage.read_page(page_offset)?;
        self.cache.put(page_offset, page);
        Ok(page)
    }

    fn read_page_for_write(&mut self, page_offset: u64) -> Result<[u8; PAGE_SIZE]> {
        if let Some(page) = self.dirty.get(&page_offset) {
            return Ok(**page);
        }
        self.read_page(page_offset)
    }

    fn mark_dirty(&mut self, page_offset: u64, page: [u8; PAGE_SIZE]) {
        self.cache.put(page_offset, page);
        self.dirty.insert(page_offset, Box::new(page));
    }

    /// Write `data` starting at `offset`, spanning page boundaries if necessary.
    fn write_bytes_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < data.len() {
            let current_offset = offset + written as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_write = (PAGE_SIZE - offset_in_page).min(data.len() - written);

            let mut page = self.read_page_for_write(page_offset)?;
            page[offset_in_page..offset_in_page + can_write]
                .copy_from_slice(&data[written..written + can_write]);
            self.mark_dirty(page_offset, page);
            written += can_write;
        }
        Ok(())
    }

    /// Read `length` bytes starting at `offset`, spanning page boundaries if necessary.
    fn read_bytes_at(&mut self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(length);
        let mut read = 0;
        while read < length {
            let current_offset = offset + read as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_read = (PAGE_SIZE - offset_in_page).min(length - read);

            let page = self.read_page(page_offset)?;
            result.extend_from_slice(&page[offset_in_page..offset_in_page + can_read]);
            read += can_read;
        }
        Ok(result)
    }

    // ─── Extent allocation (private) ───────────────────────

    /// Allocate a fresh data extent of [`EXTENT_BYTES`] for `kind`, append it
    /// to the in-memory chain, and stage the matching index-page update into
    /// the dirty buffer so a subsequent commit makes it durable.
    fn allocate_extent(&mut self, kind: ExtentKind) -> Result<u64> {
        // The in-memory `extents` length is authoritative for the kind's
        // capacity-exceeded check; tail/count are used only for persistence.
        let len = self.chain_ref(kind).extents.len() as u64;
        if len >= MAX_EXTENTS_PER_KIND {
            let (limit, unit) = kind.capacity_limit();
            return Err(LielError::CapacityExceeded {
                kind: kind.as_str(),
                limit,
                unit,
            });
        }

        let new_offset = self.allocated_eof;
        self.allocated_eof = new_offset + EXTENT_BYTES;

        self.chain_mut(kind).extents.push(new_offset);
        self.persist_extent_entry(kind, new_offset)?;
        Ok(new_offset)
    }

    /// Persist a new extent offset into the tail index page of `kind`.
    /// Allocates and chains a new index page on the fly when the current tail
    /// is full (or when no index page exists for the kind yet).
    fn persist_extent_entry(&mut self, kind: ExtentKind, extent_offset: u64) -> Result<()> {
        let need_new_page = {
            let chain = self.chain_ref(kind);
            chain.tail_page_offset == 0 || chain.tail_page_count as usize >= INDEX_ENTRIES_PER_PAGE
        };
        if need_new_page {
            self.allocate_index_page(kind)?;
        }
        let (tail_offset, insert_idx) = {
            let chain = self.chain_ref(kind);
            (chain.tail_page_offset, chain.tail_page_count as usize)
        };

        let mut page = self.read_page_for_write(tail_offset)?;
        let entry_pos = 16 + insert_idx * 8;
        page[entry_pos..entry_pos + 8].copy_from_slice(&extent_offset.to_le_bytes());
        let new_count = (insert_idx as u32) + 1;
        page[8..12].copy_from_slice(&new_count.to_le_bytes());
        self.mark_dirty(tail_offset, page);

        self.chain_mut(kind).tail_page_count = new_count;
        Ok(())
    }

    /// Allocate a new (empty) index page at the file tail and chain it into
    /// the kind's linked list.  Updates `header.*_table_head` when the kind
    /// had no index page yet.
    fn allocate_index_page(&mut self, kind: ExtentKind) -> Result<()> {
        let new_offset = self.allocated_eof;
        self.allocated_eof = new_offset + PAGE_SIZE as u64;

        // Zero-fill the freshly allocated page (next=0, count=0, entries=0).
        let page = [0u8; PAGE_SIZE];
        self.mark_dirty(new_offset, page);

        let previous_tail = self.chain_ref(kind).tail_page_offset;
        if previous_tail == 0 {
            // First index page for this kind: record the head in the file header.
            match kind {
                ExtentKind::Node => self.header.node_table_head = new_offset,
                ExtentKind::Edge => self.header.edge_table_head = new_offset,
                ExtentKind::Prop => self.header.prop_table_head = new_offset,
            }
        } else {
            // Chain by writing the new offset into the previous tail's
            // `next_page_offset` slot (bytes 0..8 of its index page).
            let mut prev_page = self.read_page_for_write(previous_tail)?;
            prev_page[0..8].copy_from_slice(&new_offset.to_le_bytes());
            self.mark_dirty(previous_tail, prev_page);
        }

        let chain = self.chain_mut(kind);
        chain.tail_page_offset = new_offset;
        chain.tail_page_count = 0;
        Ok(())
    }

    fn chain_ref(&self, kind: ExtentKind) -> &ExtentChain {
        match kind {
            ExtentKind::Node => &self.node_chain,
            ExtentKind::Edge => &self.edge_chain,
            ExtentKind::Prop => &self.prop_chain,
        }
    }

    fn chain_mut(&mut self, kind: ExtentKind) -> &mut ExtentChain {
        match kind {
            ExtentKind::Node => &mut self.node_chain,
            ExtentKind::Edge => &mut self.edge_chain,
            ExtentKind::Prop => &mut self.prop_chain,
        }
    }

    // ─── Commit / rollback ──────────────────────────────────────

    /// Flush all dirty pages through the WAL and update the file header.
    ///
    /// Write order: WAL entries → WAL fsync → data pages → WAL clear → header update.
    pub fn commit(&mut self) -> Result<()> {
        if self.dirty.is_empty() {
            self.write_header()?;
            return Ok(());
        }
        if self.storage.is_memory() {
            return self.commit_memory();
        }
        Wal::write_and_commit(self)?;
        Ok(())
    }

    fn commit_memory(&mut self) -> Result<()> {
        let dirty: Vec<(u64, Box<[u8; PAGE_SIZE]>)> = self.dirty.drain().collect();
        for (offset, page) in dirty {
            self.storage.write_page(offset, &page)?;
        }
        self.write_header()?;
        Ok(())
    }

    pub fn flush_dirty_pages(&mut self) -> Result<()> {
        let dirty: Vec<(u64, Box<[u8; PAGE_SIZE]>)> = self.dirty.drain().collect();
        for (offset, page) in dirty {
            self.storage.write_page(offset, &page)?;
        }
        Ok(())
    }

    pub fn flush_storage(&mut self) -> Result<()> {
        self.storage.flush()
    }

    /// Reset the pager to the same logical state as a brand-new database.
    ///
    /// This discards every in-flight dirty page, clears the page cache, resets
    /// the header and all in-memory extent metadata, truncates the backing
    /// store to the fixed header + WAL reservation, then writes a fresh empty
    /// header. After this call the next allocated node/edge ID is 1 and a
    /// later `commit()` cannot flush stale pre-clear pages from the old dirty
    /// buffer.
    pub fn clear_to_empty(&mut self) -> Result<()> {
        self.dirty.clear();
        self.cache.clear();
        self.header = FileHeader::new_empty();
        self.node_chain = ExtentChain::default();
        self.edge_chain = ExtentChain::default();
        self.prop_chain = ExtentChain::default();
        self.allocated_eof = FIRST_ALLOCATABLE_OFFSET;
        self.storage.set_len(FIRST_ALLOCATABLE_OFFSET)?;
        self.write_header()?;
        self.flush_storage()
    }

    pub fn storage_mut(&mut self) -> &mut dyn Storage {
        self.storage.as_mut()
    }

    // ─── ID management ────────────────────────────────────────────

    /// Reserve the next available node ID.
    ///
    /// Grows the node extent chain by one extent when the new ID would cross
    /// an extent boundary.  Returns [`LielError::CapacityExceeded`] only in
    /// the degenerate case where the number of extents would exceed the
    /// defence-in-depth guard [`MAX_EXTENTS_PER_KIND`] (roughly four billion
    /// extents = `2^32 * NODES_PER_EXTENT` node IDs).  On failure the counter
    /// is left untouched so a retry after freeing space stays consistent.
    pub fn alloc_node_id(&mut self) -> Result<u64> {
        let id = self.header.next_node_id;
        self.ensure_extent_for(id, ExtentKind::Node, NODES_PER_EXTENT)?;
        self.header.next_node_id = id + 1;
        Ok(id)
    }

    /// Reserve the next available edge ID.  See [`alloc_node_id`] for the
    /// overflow-protection and growth semantics.
    pub fn alloc_edge_id(&mut self) -> Result<u64> {
        let id = self.header.next_edge_id;
        self.ensure_extent_for(id, ExtentKind::Edge, EDGES_PER_EXTENT)?;
        self.header.next_edge_id = id + 1;
        Ok(id)
    }

    /// Ensure that a data extent covering `id` exists, allocating new ones at
    /// end-of-file until the chain is long enough.  The allocation writes go
    /// through the normal dirty-page / WAL pipeline and are durable on the
    /// next `commit()`.
    pub(crate) fn ensure_extent_for(
        &mut self,
        id: u64,
        kind: ExtentKind,
        slots_per_extent: u64,
    ) -> Result<()> {
        let needed = (id - 1) / slots_per_extent + 1;
        while (self.chain_ref(kind).extents.len() as u64) < needed {
            self.allocate_extent(kind)?;
        }
        Ok(())
    }

    pub fn node_count(&self) -> u64 {
        self.header.node_count
    }

    pub fn edge_count(&self) -> u64 {
        self.header.edge_count
    }

    pub fn increment_node_count(&mut self) {
        self.header.node_count += 1;
    }

    pub fn decrement_node_count(&mut self) {
        self.header.node_count = self.header.node_count.saturating_sub(1);
    }

    pub fn increment_edge_count(&mut self) {
        self.header.edge_count += 1;
    }

    pub fn decrement_edge_count(&mut self) {
        self.header.edge_count = self.header.edge_count.saturating_sub(1);
    }

    /// Return all allocated node IDs in the range [1, next_node_id).
    /// Includes IDs of deleted nodes; callers must check the active flag themselves.
    pub fn max_node_id(&self) -> u64 {
        self.header.next_node_id.saturating_sub(1)
    }

    pub fn max_edge_id(&self) -> u64 {
        self.header.next_edge_id.saturating_sub(1)
    }

    pub fn dirty_page_offsets(&self) -> Vec<u64> {
        self.dirty.keys().copied().collect()
    }

    pub fn get_dirty_page(&self, offset: u64) -> Option<&[u8; PAGE_SIZE]> {
        self.dirty.get(&offset).map(|b| b.as_ref())
    }

    /// Reset property storage so `vacuum` can re-append all live blobs from
    /// scratch.  The prop extent-index chain is cleared both in memory and on
    /// disk (via a dirty write of the old head page's header to zero), and
    /// the write cursor is zeroed so the very next `append_prop` allocates a
    /// fresh prop extent.
    ///
    /// Previous prop extents remain allocated in the file as **orphaned** space.
    /// [`crate::graph::vacuum::vacuum`] does not currently call
    /// [`Pager::truncate_to`] to reclaim that tail; see the `vacuum` module docs.
    pub fn reset_prop_storage(&mut self) -> Result<()> {
        // Clear the on-disk head page (count=0, next=0) so a crash mid-vacuum
        // leaves the chain empty rather than half-updated.
        if self.header.prop_table_head != 0 {
            let zero = [0u8; PAGE_SIZE];
            self.mark_dirty(self.header.prop_table_head, zero);
            // Truncate the chain to a single empty head page; subsequent
            // allocations will reuse this page until it fills up again.
            self.prop_chain.extents.clear();
            self.prop_chain.tail_page_offset = self.header.prop_table_head;
            self.prop_chain.tail_page_count = 0;
        } else {
            self.prop_chain = ExtentChain::default();
        }
        self.header.next_prop_write_offset = 0;
        Ok(())
    }

    /// Truncate the underlying file to `new_size` bytes.
    ///
    /// Provided for future tail reclamation after compaction; the current
    /// [`crate::graph::vacuum::vacuum`] implementation does not invoke this yet.
    pub fn truncate_to(&mut self, size: u64) -> Result<()> {
        self.storage.set_len(size)?;
        if self.allocated_eof > size {
            self.allocated_eof = size;
        }
        Ok(())
    }

    /// Roll back all uncommitted changes and restore the last committed state.
    pub fn rollback(&mut self) -> Result<()> {
        self.dirty.clear();
        self.cache = PageCache::new(DEFAULT_CACHE_CAPACITY);
        let size = self.storage.file_size();
        if size >= HEADER_SIZE {
            self.load_header()?;
            self.load_extent_chains()?;
        } else {
            self.header = FileHeader::new_empty();
            self.node_chain = ExtentChain::default();
            self.edge_chain = ExtentChain::default();
            self.prop_chain = ExtentChain::default();
            self.allocated_eof = FIRST_ALLOCATABLE_OFFSET;
        }
        Ok(())
    }

    pub fn file_size(&self) -> u64 {
        self.storage.file_size()
    }

    /// Test-only accessor for the node-extent offsets.  Exposed behind
    /// `#[cfg(test)]` callers via `pub(crate)`.
    #[allow(dead_code)]
    pub(crate) fn node_extents_for_test(&self) -> &[u64] {
        &self.node_chain.extents
    }

    /// Test-only accessor for the edge-extent offsets.
    #[allow(dead_code)]
    pub(crate) fn edge_extents_for_test(&self) -> &[u64] {
        &self.edge_chain.extents
    }

    /// Test-only accessor for the prop-extent offsets.
    #[allow(dead_code)]
    pub(crate) fn prop_extents_for_test(&self) -> &[u64] {
        &self.prop_chain.extents
    }
}

// Extension trait that adds seek_and_read / seek_and_write convenience helpers to Storage
pub trait StorageExt {
    fn seek_and_read(&mut self, offset: u64, buf: &mut [u8]) -> Result<()>;
    fn seek_and_write(&mut self, offset: u64, data: &[u8]) -> Result<()>;
}

impl StorageExt for dyn Storage {
    fn seek_and_read(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        let mut read = 0;
        while read < buf.len() {
            let current_offset = offset + read as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_read = (PAGE_SIZE - offset_in_page).min(buf.len() - read);
            let page = self.read_page(page_offset)?;
            buf[read..read + can_read]
                .copy_from_slice(&page[offset_in_page..offset_in_page + can_read]);
            read += can_read;
        }
        Ok(())
    }

    fn seek_and_write(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < data.len() {
            let current_offset = offset + written as u64;
            let page_offset = current_offset - (current_offset % PAGE_SIZE as u64);
            let offset_in_page = (current_offset % PAGE_SIZE as u64) as usize;
            let can_write = (PAGE_SIZE - offset_in_page).min(data.len() - written);
            let mut page = [0u8; PAGE_SIZE];
            let file_size = self.file_size();
            if page_offset + PAGE_SIZE as u64 <= file_size {
                page = self.read_page(page_offset)?;
            }
            page[offset_in_page..offset_in_page + can_write]
                .copy_from_slice(&data[written..written + can_write]);
            self.write_page(page_offset, &page)?;
            written += can_write;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_extent_lazy_allocation() {
        let mut pager = Pager::open(":memory:").unwrap();
        assert!(pager.node_chain.extents.is_empty());
        let id = pager.alloc_node_id().unwrap();
        assert_eq!(id, 1);
        assert_eq!(pager.node_chain.extents.len(), 1);
        let first_extent = pager.node_chain.extents[0];
        assert!(first_extent >= FIRST_ALLOCATABLE_OFFSET);
        assert_eq!(first_extent % PAGE_SIZE as u64, 0);
    }

    #[test]
    fn test_node_slot_offset_within_first_extent() {
        let mut pager = Pager::open(":memory:").unwrap();
        pager.alloc_node_id().unwrap();
        let ext0 = pager.node_chain.extents[0];
        // NodeId=1 → page_index=0, slot_index=0
        assert_eq!(
            pager.node_slot_file_offset(1).unwrap(),
            ext0 + PAGE_HEADER_SIZE as u64
        );
        // NodeId=64 → first slot on page_index=1
        // (we need the extent to address id=64; it already does, 1 extent holds 16128)
        assert_eq!(
            pager.node_slot_file_offset(64).unwrap(),
            ext0 + PAGE_SIZE as u64 + PAGE_HEADER_SIZE as u64
        );
    }

    #[test]
    fn test_write_and_read_node_slot() {
        let mut pager = Pager::open(":memory:").unwrap();
        let id = pager.alloc_node_id().unwrap();
        let slot = NodeSlot {
            node_id: id,
            first_out_edge: 0,
            first_in_edge: 0,
            prop_offset: 0,
            prop_length: 0,
            out_degree: 0,
            in_degree: 0,
            label_offset: 0,
            label_length: 0,
            flags: 0,
        };
        pager.write_node_slot(&slot).unwrap();
        let read_back = pager.read_node_slot(id).unwrap();
        assert_eq!(read_back.node_id, id);
        assert_eq!(read_back.first_out_edge, 0);
    }

    #[test]
    fn test_in_memory_no_file() {
        let pager = Pager::open(":memory:").unwrap();
        assert!(pager.storage.is_memory());
    }

    #[test]
    fn test_node_extent_boundary_grows_chain() {
        let mut pager = Pager::open(":memory:").unwrap();
        // Fast-forward the counter so that the very next alloc lands in
        // extent #2 (extent indices are 0-based).
        pager.header.next_node_id = NODES_PER_EXTENT + 1;
        // The first extent still needs to exist so the header is consistent;
        // pre-allocate it so the invariants `extents[extent_idx]` will hold
        // for any prior ID the test might also reference.
        pager
            .ensure_extent_for(1, ExtentKind::Node, NODES_PER_EXTENT)
            .unwrap();
        assert_eq!(pager.node_chain.extents.len(), 1);

        let id = pager.alloc_node_id().unwrap();
        assert_eq!(id, NODES_PER_EXTENT + 1);
        // Allocating that ID must have grown the chain to 2 extents so the
        // slot offset is addressable.
        assert_eq!(pager.node_chain.extents.len(), 2);
        let off = pager.node_slot_file_offset(id).unwrap();
        // First slot of the second extent sits at extent[1] + 8.
        assert_eq!(off, pager.node_chain.extents[1] + 8);
    }

    #[test]
    fn test_edge_extent_lazy_allocation() {
        let mut pager = Pager::open(":memory:").unwrap();
        assert!(pager.edge_chain.extents.is_empty());
        let id = pager.alloc_edge_id().unwrap();
        assert_eq!(id, 1);
        assert_eq!(pager.edge_chain.extents.len(), 1);
    }

    #[test]
    fn test_extent_chain_round_trip_via_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extent-roundtrip.liel");
        let path_str = path.to_str().unwrap();

        // Open → allocate IDs that span >1 node extent and >1 edge extent → commit.
        {
            let mut pager = Pager::open(path_str).unwrap();
            // Jump straight past the first node extent.
            pager.header.next_node_id = NODES_PER_EXTENT;
            pager
                .ensure_extent_for(1, ExtentKind::Node, NODES_PER_EXTENT)
                .unwrap();
            let id = pager.alloc_node_id().unwrap(); // consumes the last slot of extent 0
            let slot = NodeSlot {
                node_id: id,
                first_out_edge: 7,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
            let id2 = pager.alloc_node_id().unwrap(); // triggers extent #1
            let slot2 = NodeSlot {
                node_id: id2,
                first_out_edge: 9,
                ..Default::default()
            };
            pager.write_node_slot(&slot2).unwrap();
            pager.increment_node_count();
            pager.increment_node_count();
            pager.commit().unwrap();
            assert_eq!(pager.node_chain.extents.len(), 2);
        }

        // Reopen → the extent chain must rebuild exactly as before and slot
        // reads must return the committed values.
        {
            let mut pager = Pager::open(path_str).unwrap();
            assert_eq!(pager.node_chain.extents.len(), 2);
            let s1 = pager.read_node_slot(NODES_PER_EXTENT).unwrap();
            assert_eq!(s1.first_out_edge, 7);
            let s2 = pager.read_node_slot(NODES_PER_EXTENT + 1).unwrap();
            assert_eq!(s2.first_out_edge, 9);
        }
    }

    #[test]
    fn test_header_checksum_mismatch_is_rejected() {
        // Writing a header then flipping a single content byte must trigger a
        // CorruptedFile error on the next `Pager::open`, proving that the
        // checksum at bytes 104..112 is actually being verified on read.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("header.liel");
        let path_str = path.to_str().unwrap();

        // Create a fresh file so Pager writes a valid header and then drop it.
        {
            let _ = Pager::open(path_str).unwrap();
        }

        // Corrupt the node_count field (bytes 24..32) without updating the
        // checksum; this simulates a torn write.
        {
            use std::fs::OpenOptions;
            use std::io::{Seek, SeekFrom, Write};
            let mut f = OpenOptions::new().write(true).open(path_str).unwrap();
            f.seek(SeekFrom::Start(24)).unwrap();
            f.write_all(&0xDEAD_BEEF_u64.to_le_bytes()).unwrap();
        }

        match Pager::open(path_str) {
            Err(LielError::CorruptedFile(msg)) => {
                assert!(msg.contains("checksum"), "unexpected message: {msg}");
            }
            Err(other) => panic!("expected CorruptedFile, got Err({:?})", other),
            Ok(_) => panic!("expected CorruptedFile, got Ok(_)"),
        }
    }

    #[test]
    fn test_unsupported_format_version_is_rejected() {
        let mut header = FileHeader::new_empty().to_bytes();
        header[16..18].copy_from_slice(&2u16.to_le_bytes());

        match FileHeader::from_bytes(&header) {
            Err(LielError::CorruptedFile(msg)) => {
                assert!(
                    msg.contains("unsupported file format version"),
                    "unexpected message: {msg}"
                );
            }
            Err(other) => panic!("expected CorruptedFile, got Err({:?})", other),
            Ok(_) => panic!("expected CorruptedFile, got Ok(_)"),
        }
    }

    #[test]
    fn test_multiple_pages() {
        let mut pager = Pager::open(":memory:").unwrap();
        for i in 1..=64u64 {
            let _ = pager.alloc_node_id().unwrap();
            let slot = NodeSlot {
                node_id: i,
                first_out_edge: i * 10,
                ..Default::default()
            };
            pager.write_node_slot(&slot).unwrap();
        }
        // The 64th node must land on the second page (63 slots / page).
        let slot64 = pager.read_node_slot(64).unwrap();
        assert_eq!(slot64.node_id, 64);
        assert_eq!(slot64.first_out_edge, 640);
        let slot1 = pager.read_node_slot(1).unwrap();
        assert_eq!(slot1.first_out_edge, 10);
    }
}

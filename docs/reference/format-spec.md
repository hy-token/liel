# On-disk format

The public English reference for reading and writing `.liel` files at the byte level. It is the English counterpart of the maintainer-facing Japanese source, `format-spec.ja.md`; both documents share the same basename and cover the same concern.

For the high-level picture and data model, see **[architecture overview](../design/architecture.md)**. For the rationale of fixed decisions and the explicit product trade-offs, see **[product trade-offs](../design/product-tradeoffs.md)**.

> **format version**: 1.0 (`0.x` Beta series). The byte layout is documented
> as the current contract. Breaking format changes may still happen before
> `1.0`, but they must be recorded in the changelog and paired with explicit
> version/fail-closed handling.

---

## 1. Overall file layout

```
Offset 0        : [Header]           128 bytes   fixed
Offset 128      : Reserved           3968 bytes  (rest of page 0; unused)
Offset 4096     : [WAL Segment]      4 MiB       1024-page reservation (live length is variable)
Offset 4198400  : [Extent / Index]   variable     node / edge / property extents and
                                                  extent-index pages, allocated as needed
```

```
Byte offset
        0  ┌─────────────────────────────────┐ ─┐
           │ File Header             (128 B)  │  │
      128  ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤  │ Page 0 (4096 B)
           │ (unused padding)       (3968 B)  │  │
     4096  ├─────────────────────────────────┤ ─┘ ← WAL_OFFSET
           │                                 │
           │ WAL Segment          (≤ 4 MiB)  │
           │ (1024 pages reserved)           │
           │                                 │
  4198400  ├─────────────────────────────────┤  ← first extent after WAL
           │ Node Extent 0           (1 MiB)  │
           ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
           │ Node Index Page 0       (4 KiB)  │
           ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
           │ Edge Extent 0           (1 MiB)  │
           ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
           │ Edge Index Page 0       (4 KiB)  │
           ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
           │ Prop Extent 0           (1 MiB)  │
           ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
           │ ... (grows toward EOF)           │
           └─────────────────────────────────┘
```

- A **page** is always a **4096-byte block**. **Page 0** spans offsets `0..4096`; only its first 128 bytes are the file header (§2). The WAL starts at offset **4096** — the equivalent of "page 1" (`pager.rs`'s `WAL_OFFSET`).
- Page size is fixed at **4096 bytes**.
- A **4 MiB** region (1024 pages) starting at offset **4096** is reserved for the WAL. The WAL is a **fixed in-file region** — its location does not move even though its live length varies at runtime.
- Immediately after the WAL reservation (byte offset `4096 + 4 MiB = 4 198 400`), 1 MiB **extents** and 4 KiB **extent-index pages** are allocated toward EOF as they are needed. Nodes, edges, and properties each have their own independent chains; they do not overlap (§5).

The legacy layout (fixed 256-page node region / edge region) has been retired. As a result, the old "silent corruption past 16 128 nodes / 13 056 edges" bug cannot occur.

---

## 2. File header (128 bytes)

```
Bytes  0-15  : Magic number           b"LIEL\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
Bytes 16-17  : Version major          u16 le  (= 1)
Bytes 18-19  : Version minor          u16 le  (= 0)
Bytes 20-23  : Page size              u32 le  (= 4096)
Bytes 24-31  : Node count             u64 le
Bytes 32-39  : Edge count             u64 le
Bytes 40-47  : Next node ID           u64 le  (auto-assigned)
Bytes 48-55  : Next edge ID           u64 le
Bytes 56-63  : Node table head        u64 le  head page of the node extent-index chain
                                              (0 = nothing has been allocated yet)
Bytes 64-71  : Edge table head        u64 le  head page of the edge extent-index chain
Bytes 72-79  : Prop table head        u64 le  head page of the property extent-index chain
Bytes 80-87  : Next prop write offset u64 le  file offset where the next property byte goes
                                              (0 = no prop extent yet)
Bytes 88-95  : WAL offset             u64 le  start of the WAL segment (= 4096)
Bytes 96-103 : WAL length             u64 le  current live WAL byte count (0 ≤ x ≤ 4 MiB)
Bytes 104-111: Checksum               u64 le  XOR of bytes 0..=103 in 8-byte chunks
Bytes 112-127: Reserved               16 bytes
```

```
Byte offset
   0  ┌─────────────────────────────────────────────────────┐
      │ Magic: b"LIEL\x00\x00\x00\x00\x00\x00\x00\x00      │
      │             \x00\x00\x00\x00"            (16 B)    │
  16  ├───────────────────────┬─────────────────────────────┤
      │ Version major  u16 le │ Version minor  u16 le       │
      │ (= 1)  (2 B)          │ (= 0)  (2 B)               │
  20  ├───────────────────────┴─────────────────────────────┤
      │ Page size: u32 le (= 4096)                  (4 B)   │
  24  ├─────────────────────────────────────────────────────┤
      │ Node count: u64 le                          (8 B)   │
  32  ├─────────────────────────────────────────────────────┤
      │ Edge count: u64 le                          (8 B)   │
  40  ├─────────────────────────────────────────────────────┤
      │ Next node ID: u64 le                        (8 B)   │
  48  ├─────────────────────────────────────────────────────┤
      │ Next edge ID: u64 le                        (8 B)   │
  56  ├─────────────────────────────────────────────────────┤
      │ Node table head: u64 le  (0 = none)         (8 B)   │
  64  ├─────────────────────────────────────────────────────┤
      │ Edge table head: u64 le  (0 = none)         (8 B)   │
  72  ├─────────────────────────────────────────────────────┤
      │ Prop table head: u64 le  (0 = none)         (8 B)   │
  80  ├─────────────────────────────────────────────────────┤
      │ Next prop write offset: u64 le  (0 = none)  (8 B)   │
  88  ├─────────────────────────────────────────────────────┤
      │ WAL offset: u64 le (= 4096)                 (8 B)   │
  96  ├─────────────────────────────────────────────────────┤
      │ WAL length: u64 le                          (8 B)   │
 104  ├─────────────────────────────────────────────────────┤
      │ Checksum: u64 le  (XOR of bytes 0..103      (8 B)   │
      │                    in 8-byte chunks)                │
 112  ├─────────────────────────────────────────────────────┤
      │ Reserved                                   (16 B)   │
 128  └─────────────────────────────────────────────────────┘
```

The checksum is just an XOR over 8-byte chunks, but it is **enough to detect a file in the legacy layout** as a first line of defence. `Pager::open` validates this on read and returns `CorruptedFile("header checksum mismatch: ...")` if it does not match.

---

## 3. Slot structures

Slot widths remain **64 bytes / 80 bytes** (NodeSlot / EdgeSlot). The page header is 8 bytes, so per-page slot counts are:

- Node: `(4096 - 8) / 64 = 63` slots
- Edge: `(4096 - 8) / 80 = 51` slots

### 3.1 Node / Edge page common header (8 bytes)

```
Bytes 0-1   : Page type    u16  (Node = 0x0001 / Edge = 0x0002)
Bytes 2-3   : Slot count   u16  total slots in this page
Bytes 4-5   : Used slots   u16
Bytes 6-7   : Reserved
```

```
Byte  0           2           4           6           8
      ┌───────────┬───────────┬───────────┬───────────┐
      │ Page type │ Slot count│ Used slots│ Reserved  │
      │ u16 le    │ u16 le    │ u16 le    │           │
      │ 0x0001 or │ (63 / 51) │           │           │
      │ 0x0002    │           │           │           │
      └───────────┴───────────┴───────────┴───────────┘
```

### 3.2 NodeSlot (64 bytes, fixed)

```
Bytes  0-7  : node_id          u64 le
Bytes  8-15 : first_out_edge   u64 le  first outgoing edge ID (0 = none)
Bytes 16-23 : first_in_edge    u64 le  first incoming edge ID (0 = none)
Bytes 24-31 : prop_offset      u64 le  absolute file offset of the property blob
Bytes 32-39 : prop_length      u32 le  property byte length
Bytes 40-43 : out_degree       u32 le
Bytes 44-47 : in_degree        u32 le
Bytes 48-55 : label_offset     u64 le  absolute file offset of the label string
Bytes 56-59 : label_length     u32 le
Bytes 60    : flags            u8      bit0 = deleted, bit1 = has_props
Bytes 61-63 : Reserved         3 bytes
```

```
Byte offset
  0  ┌──────────────────────────────────────────────────┐
     │ node_id                               u64 le     │  8 B
  8  ├──────────────────────────────────────────────────┤
     │ first_out_edge  (0 = none)            u64 le     │  8 B
 16  ├──────────────────────────────────────────────────┤
     │ first_in_edge   (0 = none)            u64 le     │  8 B
 24  ├──────────────────────────────────────────────────┤
     │ prop_offset  (absolute file offset)   u64 le     │  8 B
 32  ├────────────────────────────┬─────────────────────┤
     │ prop_length      u32 le    │ (padding)           │  8 B
     │                  (4 B)     │ (4 B)               │
 40  ├─────────────────────────────────────────────────┤
     │ out_degree  u32 le (4 B)  │ in_degree  u32 le   │  8 B
     │                           │ (4 B)               │
 48  ├──────────────────────────────────────────────────┤
     │ label_offset (absolute file offset)   u64 le     │  8 B
 56  ├────────────────────────────┬─────┬───────────────┤
     │ label_length     u32 le    │flags│ Reserved      │  8 B
     │                  (4 B)     │ u8  │ (3 B)         │
 64  └────────────────────────────┴─────┴───────────────┘
                                   flags:
                                     bit0 = deleted
                                     bit1 = has_props
```

### 3.3 EdgeSlot (80 bytes, fixed)

```
Bytes  0-7  : edge_id           u64 le
Bytes  8-15 : from_node_id      u64 le
Bytes 16-23 : to_node_id        u64 le
Bytes 24-31 : next_out_edge     u64 le  next outgoing edge of from_node (linked list)
Bytes 32-39 : next_in_edge      u64 le  next incoming edge of to_node (linked list)
Bytes 40-47 : prop_offset       u64 le
Bytes 48-55 : prop_length       u32 le
Bytes 56-63 : label_offset      u64 le  absolute file offset of the edge label string
Bytes 64-67 : label_length      u32 le
Bytes 68    : flags             u8
Bytes 69-79 : Reserved
```

```
Byte offset
  0  ┌──────────────────────────────────────────────────┐
     │ edge_id                               u64 le     │  8 B
  8  ├──────────────────────────────────────────────────┤
     │ from_node_id                          u64 le     │  8 B
 16  ├──────────────────────────────────────────────────┤
     │ to_node_id                            u64 le     │  8 B
 24  ├──────────────────────────────────────────────────┤
     │ next_out_edge  (linked list, 0 = end) u64 le     │  8 B
 32  ├──────────────────────────────────────────────────┤
     │ next_in_edge   (linked list, 0 = end) u64 le     │  8 B
 40  ├──────────────────────────────────────────────────┤
     │ prop_offset  (absolute file offset)   u64 le     │  8 B
 48  ├────────────────────────────┬─────────────────────┤
     │ prop_length      u32 le    │ (padding)           │  8 B
     │                  (4 B)     │ (4 B)               │
 56  ├──────────────────────────────────────────────────┤
     │ label_offset (absolute file offset)   u64 le     │  8 B
 64  ├────────────────────────────┬─────┬───────────────┤
     │ label_length     u32 le    │flags│ Reserved      │ 16 B
     │                  (4 B)     │ u8  │ (11 B)        │
 80  └────────────────────────────┴─────┴───────────────┘
                                   flags: bit0 = deleted
```

In the legacy layout, property and label offsets were relative to a single prop region. In the current layout they are **absolute file offsets**, because extents may be scattered and a relative form would be undecodable.

---

## 4. Extent chains

Nodes, edges, and properties each live in an independent **linked list of extents**.

- **extent**: a contiguous 1 MiB block (= 256 pages). One extent is dedicated to one kind (node / edge / property).
- **extent-index page**: a 4 KiB management page that holds up to 510 extent absolute offsets and a pointer to the next index page.
- **head of the chain**: `FileHeader.node_table_head` / `edge_table_head` / `prop_table_head` point at the first index page. If nothing of that kind is allocated yet, the head is `0` (uninitialised sentinel).

### 4.1 Extent-index page layout

```
Bytes  0-7   : next_page_offset  u64 le  offset of the next index page
                                         (0 = end of chain)
Bytes  8-11  : count              u32 le  number of extents recorded in this page
                                          (0 ≤ x ≤ 510)
Bytes 12-15  : Reserved           4 bytes
Bytes 16+    : entries            count × u64 le (absolute offset of each extent);
                                  unused slots remain zero-filled
```

```
Byte offset
  0  ┌──────────────────────────────────────────────────┐
     │ next_page_offset  u64 le  (0 = end of chain)    │  8 B
  8  ├────────────────────────────┬─────────────────────┤
     │ count  u32 le (0–510)      │ Reserved  (4 B)     │  8 B
 16  ├──────────────────────────────────────────────────┤
     │ entries[0]   absolute file offset of extent 0   │  8 B
 24  ├──────────────────────────────────────────────────┤
     │ entries[1]   absolute file offset of extent 1   │  8 B
     ├──────────────────────────────────────────────────┤
     │ ...                                             │
     ├──────────────────────────────────────────────────┤
     │ entries[509] absolute file offset of extent 509 │  8 B
4096 └──────────────────────────────────────────────────┘
     ◀─ 16 B header + 510 × 8 B entries = 4096 B total ─▶
```

One index page holds **510 entries**; one entry is one 1 MiB extent, so a single index page tracks **510 MiB** of data extents. The chain extends up to `MAX_EXTENTS_PER_KIND = u32::MAX`, which is theoretically several exabytes per kind.

### 4.2 Allocation order

Right after a fresh file is created, the end of the WAL reservation (offset `0x400800`) is the initial value of `allocated_eof`. The first `add_node` extends the file like this:

1. `allocate_extent(Node)`: reserve a 1 MiB node extent at offset `allocated_eof`. `extents.push(...)`, `allocated_eof += 1 MiB`.
2. `persist_extent_entry(Node, ...)`: there is no node-side index page yet, so `allocate_index_page(Node)` reserves another 4 KiB index page at `allocated_eof`. `header.node_table_head` is updated to point at this page.
3. The new index page is initialised with `next=0, count=1, entries=[new_extent_offset]`.

For subsequent extents, if there is a free slot in the index page they are appended to the same page; once full (`count == 510`) a new index page is allocated at the file end in the same way, and the previous index page's `next_page_offset` is updated to chain it.

### 4.3 Slot → file offset conversion

The formula to derive a file offset from a `node_id`:

```
extent_idx = (node_id - 1) / NODES_PER_EXTENT          // NODES_PER_EXTENT = 16 128
slot_in_ext = (node_id - 1) % NODES_PER_EXTENT
page_idx   = slot_in_ext / NODES_PER_PAGE              // NODES_PER_PAGE   = 63
slot_in_pg = slot_in_ext % NODES_PER_PAGE
file_offset = node_extents[extent_idx]
            + page_idx * 4096
            + 8                                        // page header
            + slot_in_pg * 64                          // NodeSlot size
```

Edges follow the same shape with constants `EDGES_PER_EXTENT = 13 056`, `EDGES_PER_PAGE = 51`, slot size 80 bytes. Properties are written compactly inside an extent from the front, with `next_prop_write_offset` tracking the cursor.

### 4.4 Capacity errors

- If the number of extents for one kind exceeds `u32::MAX` → `CapacityExceeded`.
- If `append_prop` is asked to write a single blob larger than `MAX_PROP_BLOB_BYTES = 1 MiB - 8 B` → `CapacityExceeded { kind: "prop", limit: MAX_PROP_BLOB_BYTES }`.

On the Python side these surface as `liel.CapacityExceededError`. They are not silently turned into a downstream `CorruptedFileError`.

---

## 5. Property serialization format

Properties are serialized in a **custom fixed format** with no external crate dependency. The rationale is in [product trade-offs §6.4](../design/product-tradeoffs.md).

```
type tag (1 byte) + data:
  0x00 = Null    (no data)
  0x01 = Bool    (1 byte: 0x00 = false, 0x01 = true)
  0x02 = Int64   (8 bytes, little endian)
  0x03 = Float64 (8 bytes, IEEE 754 little endian)
  0x04 = String  (4 bytes length + UTF-8 bytes)
  0x05 = List    (4 bytes count + each element encoded recursively)
  0x06 = Map     (4 bytes count + repeated key (in 0x04 form) + value)

Map encoding overall:
  4 bytes: entry count (u32 le)
  per entry: key (0x04 form) + value (any of the above)
  Note: map entries are written in UTF-8 lexicographic key order
        (deterministic encoding)
```

```
── Null ──────────────────────────────────────────────────────────
  ┌──────┐
  │ 0x00 │  (no data follows)
  └──────┘

── Bool ──────────────────────────────────────────────────────────
  ┌──────┬──────────────────────────────┐
  │ 0x01 │ value: 0x00=false, 0x01=true │
  └──────┴──────────────────────────────┘
          1 B

── Int64 ─────────────────────────────────────────────────────────
  ┌──────┬──────────────────────────────────────────────────────┐
  │ 0x02 │ i64 value  (8 bytes, little endian)                  │
  └──────┴──────────────────────────────────────────────────────┘
          8 B

── Float64 ───────────────────────────────────────────────────────
  ┌──────┬──────────────────────────────────────────────────────┐
  │ 0x03 │ f64 value  (8 bytes, IEEE 754 little endian)         │
  └──────┴──────────────────────────────────────────────────────┘
          8 B

── String ────────────────────────────────────────────────────────
  ┌──────┬────────────────┬─────────────────────────────────────┐
  │ 0x04 │ length  u32 le │ UTF-8 bytes  (length bytes)         │
  └──────┴────────────────┴─────────────────────────────────────┘
          4 B              variable

── List ──────────────────────────────────────────────────────────
  ┌──────┬────────────────┬────────────┬────────────┬───────────┐
  │ 0x05 │ count  u32 le  │ element[0] │ element[1] │    ...    │
  └──────┴────────────────┴────────────┴────────────┴───────────┘
          4 B              each element recursively encoded

── Map ───────────────────────────────────────────────────────────
  ┌──────┬────────────────┬──────────────────────────────────────┐
  │ 0x06 │ count  u32 le  │ key[0] (0x04 form) + value[0]  ...  │
  └──────┴────────────────┴──────────────────────────────────────┘
          4 B              entries in UTF-8 lexicographic key order
```

```rust
// Value enum
enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<PropValue>),
    Map(HashMap<String, PropValue>),
}
```

Implementation: `src/storage/prop_codec.rs`.

---

## 6. WAL (Write-Ahead Log)

Minimal implementation for crash safety. Recorded **at page granularity** (simpler than record granularity, with a much shorter recovery path).

Common header (Write and Commit share the same first 17 bytes):

```
  Bytes 0-3   : entry_length  u32 le  total entry length including the trailing 4-byte CRC
  Bytes 4     : op_type       u8      0x01 = Write, 0x02 = Commit
  Bytes 5-12  : page_offset   u64 le  Write: file offset of the target data page / Commit: 0
  Bytes 13-16 : data_length   u32 le  Write: 4096 (= PAGE_SIZE) / Commit: 0
```

```
── Write entry  (entry_length = 4117 bytes) ──────────────────────
Byte offset
  0  ┌──────────────────────────────────────────────────┐
     │ entry_length = 4117           u32 le             │  4 B
  4  ├──────────────────────────────────────────────────┤
     │ op_type = 0x01 (Write)        u8                 │  1 B
  5  ├──────────────────────────────────────────────────┤
     │ page_offset  (target page)    u64 le             │  8 B
 13  ├──────────────────────────────────────────────────┤
     │ data_length = 4096            u32 le             │  4 B
 17  ├──────────────────────────────────────────────────┤
     │                                                  │
     │ Page image                                4096 B │
     │                                                  │
4113 ├──────────────────────────────────────────────────┤
     │ CRC32  (ISO-HDLC, over bytes 0..4112)    u32 le  │  4 B
4117 └──────────────────────────────────────────────────┘

── Commit entry  (entry_length = 21 bytes) ───────────────────────
  0  ┌──────────────────────────────────────────────────┐
     │ entry_length = 21             u32 le             │  4 B
  4  ├──────────────────────────────────────────────────┤
     │ op_type = 0x02 (Commit)       u8                 │  1 B
  5  ├──────────────────────────────────────────────────┤
     │ page_offset = 0               u64 le             │  8 B
 13  ├──────────────────────────────────────────────────┤
     │ data_length = 0               u32 le             │  4 B
 17  ├──────────────────────────────────────────────────┤
     │ CRC32  (ISO-HDLC, over bytes 0..16)     u32 le   │  4 B
 21  └──────────────────────────────────────────────────┘
```

- **Write entry** (`entry_length` = 4117): the 17-byte header followed by **4096 bytes** of page image, then **CRC32 (4 bytes)**.
- **Commit entry** (`entry_length` = 21): the 17-byte header followed by **CRC32 (4 bytes)** only (no page image).

CRC32 is **ISO-HDLC** (polynomial 0xEDB88320, identical to the `crc32fast` crate's definition). The implementation lives in **`src/storage/crc32.rs`** and has no external dependency. The CRC is computed over **everything in the entry except the CRC field itself** (`entry_length - 4` bytes).

WAL policy:

- Writes first append the full page to the WAL.
- On `commit()` the affected pages are flushed to their canonical location and the header's `wal_length` is reset to 0. The **on-file region reserved for the WAL** (the 4 MiB at offset 4096) **never moves or changes size**; commit does not shrink the file or relocate the WAL. The next transaction starts again at the beginning of the same region.
- On startup, if `wal_length > 0` we roll forward to recover (any entry that fails CRC truncates recovery there).
- A 1-byte change still writes 4 KiB. This is an accepted trade-off for the
  current `0.x` Beta series.
- If a single transaction would exceed the WAL reservation (4 MiB), `commit()` returns a `TransactionError` (`LielError::WalOverflow` on the Rust side). `Wal::write_and_commit` computes the total bytes before writing and returns the error while leaving dirty pages in place if `WAL_RESERVED` would be exceeded. The caller should split the transaction and retry.

---

## 7. Adjacency-list and vacuum invariants

This section is not about byte layouts but about **invariants that any implementation reading or writing the format must uphold**. We list them here so higher layers don't break semantics that callers already rely on.

### 7.1 Adjacency-list traversal order

- The singly-linked lists anchored at `NodeSlot.first_out_edge` / `first_in_edge` are built by **head-insertion**. Therefore `out_edges` / `in_edges` / `neighbors` / `bfs` / `dfs` return edges in **reverse insertion order**.
- This order is a stable, observable property of the format, but it is **not sorted by value, ID, or label**. Callers that need a specific order must sort the result themselves.
- Any future change that introduces a different order must keep the current default and opt-in through an explicit parameter (e.g. `neighbors(n, order=...)`). See [product-tradeoffs.md §5](../design/product-tradeoffs.md) for the rationale and `python/liel/liel.pyi` for API docstrings.

### 7.2 Vacuum invariants

- `vacuum` **preserves node and edge IDs**. `FileHeader.next_node_id` / `next_edge_id` are unchanged. Application-side caches and external ID references remain valid across a vacuum.
- `vacuum` does not modify `FileHeader.magic`, `version_major`, `version_minor`, `page_size`, or `wal_offset`.
- `vacuum` **does change** absolute blob offsets (`NodeSlot.prop_offset`, `label_offset`, and similar fields on edges) because property and label blobs are repacked. External code must reference data by ID, never by cached blob offset.
- The 0.3 copy-on-write + atomic-rename variant (see [product-tradeoffs.md §5.6](../design/product-tradeoffs.md)) keeps the old `.liel` intact if the process is killed mid-vacuum; the next `open()` unconditionally removes any stale `.liel.tmp`. The new file's format is fully compatible with the previous layout, and the ID-preservation invariant above continues to hold.

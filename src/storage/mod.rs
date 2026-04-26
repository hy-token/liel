//! Storage subsystem for the liel graph database engine.
//!
//! Suggested reading order for new contributors:
//!
//! 1. [`traits`] — `Storage` trait for disk and in-memory backends.
//! 2. [`file`] — `FileStorage` (disk) and `MemoryStorage` (`:memory:`).
//! 3. [`cache`] — LRU page cache between the pager and storage.
//! 4. [`serializer`] — fixed-size `NodeSlot` / `EdgeSlot` binary layouts.
//! 5. [`prop_codec`] — custom property encoding (no external serde crates).
//! 6. [`pager`] — page manager, header, dirty pages, ID allocation.
//! 7. [`wal`] — write-ahead log for crash-safe commits.

pub mod atomic_rename;
pub mod cache;
pub mod crc32;
pub mod file;
pub mod lock;
pub mod pager;
pub mod prop_codec;
pub mod serializer;
pub mod traits;
pub mod wal;

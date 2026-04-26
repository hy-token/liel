use crate::error::Result;

/// Page-granularity I/O abstraction for the liel storage engine.
///
/// All storage backends — real files on disk and the in-memory `:memory:` mode —
/// implement this trait. The engine always reads and writes exactly one page
/// (4096 bytes) at a time; it never issues partial-page reads or writes through
/// this interface.
///
/// Implementations must be `Send + Sync` so they can be placed inside an
/// `Arc<Mutex<>>` and used safely from PyO3 callbacks that may arrive on
/// different OS threads.
pub trait Storage: Send + Sync {
    /// Read a single 4096-byte page starting at the given byte `offset`.
    ///
    /// # Parameters
    /// - `offset`: The byte offset in the backing store where the page begins.
    ///   Must be a multiple of 4096 in practice, though the trait itself does
    ///   not enforce alignment.
    ///
    /// # Returns
    /// A fixed-size 4096-byte array containing the page contents.
    ///
    /// # Errors
    /// Returns an I/O error if the read fails (e.g. the offset is beyond the
    /// end of the file, or an OS-level error occurs).
    fn read_page(&mut self, offset: u64) -> Result<[u8; 4096]>;

    /// Write a single 4096-byte page at the given byte `offset`.
    ///
    /// Writing does **not** guarantee durability until `flush` is called.
    /// The implementation should update any internal size tracking so that
    /// `file_size` reflects the new extent after a write that extends the file.
    ///
    /// # Parameters
    /// - `offset`: Destination byte offset. Must be a multiple of 4096 in
    ///   practice.
    /// - `data`: The full 4096-byte page to write.
    ///
    /// # Errors
    /// Returns an I/O error if the write fails.
    fn write_page(&mut self, offset: u64, data: &[u8; 4096]) -> Result<()>;

    /// Flush all buffered writes to durable storage (equivalent to `fsync`).
    ///
    /// For `FileStorage` this calls `File::flush` followed by the OS sync.
    /// For `MemoryStorage` this is a no-op because all data already lives in
    /// RAM and there is nothing to persist.
    ///
    /// The WAL commit protocol requires two `flush` calls:
    /// 1. After writing WAL entries — guarantees the WAL is durable before
    ///    touching the main data pages.
    /// 2. After writing data pages — guarantees the data is durable before
    ///    clearing the WAL.
    ///
    /// # Errors
    /// Returns an I/O error if the underlying `fsync` fails.
    fn flush(&mut self) -> Result<()>;

    /// Return the current size of the backing store in bytes.
    ///
    /// For `FileStorage` this mirrors the OS file size.  For `MemoryStorage`
    /// this is the length of the in-memory `Vec<u8>`.  The value is updated
    /// eagerly after every `write_page` or `set_len` call so callers can rely
    /// on it without issuing a stat syscall.
    fn file_size(&self) -> u64;

    /// Resize the backing store to exactly `size` bytes.
    ///
    /// This can either truncate (remove bytes beyond `size`) or extend (pad
    /// with zeroes up to `size`).  Used by the vacuum routine and by
    /// `MemoryStorage` to allocate space before a `write_page` call.
    ///
    /// # Parameters
    /// - `size`: The desired new size in bytes.
    ///
    /// # Errors
    /// Returns an I/O error if the resize fails.
    fn set_len(&mut self, size: u64) -> Result<()>;

    /// Return `true` if this storage backend is purely in-memory.
    ///
    /// When `true`, the `Pager` skips the WAL entirely and commits dirty pages
    /// directly to the `MemoryStorage` buffer.  This avoids pointless fsync
    /// overhead for the `:memory:` mode where durability is irrelevant.
    fn is_memory(&self) -> bool;
}

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use super::traits::Storage;
use crate::error::{LielError, Result};

/// A `Storage` implementation backed by a real file on disk.
///
/// `FileStorage` wraps a single `std::fs::File` opened for both reading and
/// writing.  It caches the file size locally so that `file_size()` never
/// needs a syscall.
///
/// # Durability
/// Writes are buffered by the OS page cache until `flush` is called.  The WAL
/// commit protocol in `wal.rs` calls `flush` at carefully chosen points to
/// guarantee crash-safety; callers must not bypass that protocol.
pub struct FileStorage {
    /// The underlying OS file handle, opened with read+write+create flags.
    file: File,
    /// Locally tracked file size in bytes, updated after every write that
    /// extends the file.  Avoids repeated `metadata()` syscalls.
    size: u64,
}

impl FileStorage {
    /// Open (or create) the `.liel` file at the given filesystem path.
    ///
    /// If the file does not exist it is created as an empty file.  If it
    /// already exists it is opened for reading and writing without truncation,
    /// so existing data is preserved.
    ///
    /// # Parameters
    /// - `path`: Filesystem path to the `.liel` database file.
    ///
    /// # Returns
    /// A ready-to-use `FileStorage`.  The caller (the `Pager`) is responsible
    /// for reading or writing the file header afterwards.
    ///
    /// # Errors
    /// Returns a wrapped I/O error if the file cannot be opened or its
    /// metadata cannot be queried.
    pub fn open(path: &str) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;

            const FILE_SHARE_READ: u32 = 0x00000001;
            const FILE_SHARE_WRITE: u32 = 0x00000002;
            const FILE_SHARE_DELETE: u32 = 0x00000004;

            options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
        }
        let file = options.open(path)?;
        let size = file.metadata()?.len();
        Ok(Self { file, size })
    }
}

/// `Storage` implementation for on-disk files.
///
/// Each method seeks to the requested byte offset before reading or writing,
/// making the implementation straightforward but not optimised for sequential
/// access patterns (the `Pager`'s dirty-page map and LRU cache handle that).
impl Storage for FileStorage {
    /// Seek to `offset` and read exactly 4096 bytes into a fixed-size buffer.
    ///
    /// Returns `UnexpectedEof` if the file is shorter than `offset + 4096`.
    fn read_page(&mut self, offset: u64) -> Result<[u8; 4096]> {
        let mut buf = [0u8; 4096];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Seek to `offset` and write the full 4096-byte page.
    ///
    /// Updates the cached `size` field if the write extends the file beyond
    /// its previous end.
    fn write_page(&mut self, offset: u64, data: &[u8; 4096]) -> Result<()> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;
        // Keep the cached size in sync if this write extended the file.
        if offset + 4096 > self.size {
            self.size = offset + 4096;
        }
        Ok(())
    }

    /// Flush all buffered data and metadata to durable storage (`fsync`).
    ///
    /// Calls `File::sync_all`, which forces both the file's contents **and**
    /// its metadata through the OS page cache to the physical device.  This is
    /// required by the WAL commit protocol: after writing WAL entries we must
    /// guarantee they have actually reached the disk before we touch the main
    /// data pages, and after writing data pages we must guarantee durability
    /// before clearing the WAL.
    ///
    /// Note: `File::flush` alone is effectively a no-op for `std::fs::File`
    /// because there is no userspace buffer to drain — the write already went
    /// through to the kernel.  The crash-safety guarantees of liel depend on
    /// the stronger `sync_all` semantics, so we use it even though it is
    /// substantially slower than a plain `flush`.
    fn flush(&mut self) -> Result<()> {
        self.file.sync_all()?;
        Ok(())
    }

    /// Return the cached file size in bytes.
    ///
    /// The value is updated whenever `write_page` extends the file, so it
    /// stays accurate without additional syscalls.
    fn file_size(&self) -> u64 {
        self.size
    }

    /// Truncate or extend the file to exactly `size` bytes.
    ///
    /// Updates the cached `size` field to match.  Used by the vacuum routine
    /// when reclaiming space.
    ///
    /// # Errors
    /// Returns an I/O error if the underlying `set_len` syscall fails.
    fn set_len(&mut self, size: u64) -> Result<()> {
        self.file.set_len(size)?;
        self.size = size;
        Ok(())
    }

    /// Always returns `false` — this is an on-disk backend, not in-memory.
    fn is_memory(&self) -> bool {
        false
    }
}

/// A `Storage` implementation backed entirely by an in-process `Vec<u8>`.
///
/// Used when the caller opens a database with the special path `":memory:"`.
/// Data exists only for the lifetime of the `Pager` and is discarded when it
/// is dropped.  There is no WAL, no fsync, and no crash-recovery — the pager
/// writes dirty pages directly to this buffer on commit.
///
/// This mode is useful for unit tests and for applications that only need a
/// transient in-memory graph.
pub struct MemoryStorage {
    /// The raw byte store.  Grows automatically as pages are written beyond
    /// the current end.
    data: Vec<u8>,
}

impl MemoryStorage {
    /// Create a new, empty `MemoryStorage` with no pre-allocated capacity.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// `Storage` implementation for the in-memory `:memory:` mode.
///
/// `read_page` returns an error if the requested range lies beyond the current
/// buffer end (unlike `FileStorage` where the OS would return `UnexpectedEof`).
/// `write_page` extends the buffer with zeroes if needed before copying the
/// page in.
impl Storage for MemoryStorage {
    /// Copy 4096 bytes from the in-memory buffer at `offset` into a fixed-size
    /// array.
    ///
    /// # Errors
    /// Returns `UnexpectedEof` if `offset + 4096 > self.data.len()`.  The
    /// `Pager` avoids this by returning a zero page for reads beyond the
    /// current file extent, so this error is only raised for truly invalid
    /// accesses.
    fn read_page(&mut self, offset: u64) -> Result<[u8; 4096]> {
        let offset = offset as usize;
        let end = offset + 4096;
        if end > self.data.len() {
            return Err(LielError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "read_page: offset={} out of range (size={})",
                    offset,
                    self.data.len()
                ),
            )));
        }
        let mut buf = [0u8; 4096];
        buf.copy_from_slice(&self.data[offset..end]);
        Ok(buf)
    }

    /// Write a 4096-byte page into the in-memory buffer at `offset`.
    ///
    /// If `offset + 4096` is beyond the current buffer length, the buffer is
    /// resized (padded with zeroes) before the copy.
    fn write_page(&mut self, offset: u64, data: &[u8; 4096]) -> Result<()> {
        let offset = offset as usize;
        let end = offset + 4096;
        // Extend the buffer if the write would go past the current end.
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// No-op: in-memory data is never flushed to disk.
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Return the current length of the in-memory byte buffer.
    fn file_size(&self) -> u64 {
        self.data.len() as u64
    }

    /// Resize the in-memory byte buffer to exactly `size` bytes.
    ///
    /// Truncation discards bytes beyond `size`; extension pads with zeroes.
    fn set_len(&mut self, size: u64) -> Result<()> {
        self.data.resize(size as usize, 0);
        Ok(())
    }

    /// Always returns `true` — this is the in-memory backend.
    ///
    /// The `Pager` uses this flag to skip WAL logic entirely when committing
    /// in `:memory:` mode.
    fn is_memory(&self) -> bool {
        true
    }
}

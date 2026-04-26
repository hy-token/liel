//! Cross-platform atomic file replacement.
//!
//! Vacuum's copy-on-write strategy ([product-tradeoffs.md §5.6]) writes a
//! sibling `<basename>.liel.tmp`, fsyncs it, and then **atomically replaces**
//! the live `<basename>.liel`.  The atomicity is what makes a mid-vacuum
//! crash safe: every observer of the path either sees the previous file
//! intact or the new file complete — never a half-finished state.
//!
//! This module hides the platform differences:
//!
//! - **Unix** uses `rename(2)`, which is atomic across replace within the
//!   same filesystem.  Durability of the rename itself is then forced via
//!   an `fsync` on the *parent directory* — the OS may otherwise lose the
//!   directory entry update independently of the file data.
//!
//! - **Windows** uses `MoveFileExW` with
//!   `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`.  NTFS treats this
//!   as atomic and write-through, so the equivalent of "directory fsync"
//!   is implicit.  FAT and SMB shares are *not* covered by this guarantee
//!   and we document that limitation rather than try to paper over it.
//!
//! ## Verification status
//!
//! The Unix path is exercised by the unit tests in this file, by every
//! `cargo test` run on Linux/macOS, and end-to-end by
//! `tests/python/test_vacuum_crash_safety.py`.  The Windows path is
//! shipped as written but has **not been verified on a real NTFS
//! filesystem** in CI — the crash-safety harness uses `os.fork()` and
//! is skipped on Windows runners, so vacuum's atomic rename only gets
//! the surface-level coverage of `cargo test` (which builds the
//! crate but exits before exercising it on disk).  Anyone hitting
//! vacuum on Windows for the first time should plan to run the Linux
//! crash-safety harness's logic by hand to confirm the contract still
//! holds.
//!
//! [product-tradeoffs.md §5.6]: https://github.com/hy-token/liel/blob/main/docs/design/product-tradeoffs.md

use std::path::Path;

use crate::error::Result;

/// Atomically replace `dst` with `src`.
///
/// On success the contents formerly at `dst` are gone and the file
/// previously at `src` now lives at `dst` (with `src` removed).  Both paths
/// must be on the same filesystem; cross-filesystem atomic rename is not
/// supported by any OS we target.
///
/// Crash safety:
///
/// - Before this function returns, the caller must have already
///   `fsync`-ed the data of `src` (so the *contents* are durable).  This
///   function takes care of making the rename itself durable.
/// - On a crash mid-call, the path `dst` resolves to either the old file
///   or the new one — never a partial state.  The temporary path `src`
///   may or may not still exist; `Pager::open` is responsible for
///   sweeping any leftover sibling `.tmp` on the next start.
///
/// # Errors
///
/// Returns `LielError::Io` for any underlying syscall failure.
pub fn atomic_replace(src: &Path, dst: &Path) -> Result<()> {
    platform::atomic_replace(src, dst)
}

#[cfg(unix)]
mod platform {
    use std::fs::File;
    use std::path::Path;

    use crate::error::{LielError, Result};

    pub(super) fn atomic_replace(src: &Path, dst: &Path) -> Result<()> {
        // `rename(2)` on POSIX is atomic across replace as long as both
        // paths are on the same filesystem.  We do NOT fall back to a
        // copy+unlink on EXDEV because that breaks the atomicity guarantee
        // the rest of vacuum relies on.
        std::fs::rename(src, dst).map_err(LielError::Io)?;

        // The rename system call updates the *directory entry*, which is
        // metadata.  Most filesystems journal data and metadata separately,
        // so the inode contents survive a crash but the entry pointing at
        // the new inode might not — until we fsync the directory itself.
        let parent = dst.parent().unwrap_or_else(|| Path::new("."));
        // Some filesystems (e.g. tmpfs) do not support fsync on a directory
        // file descriptor and return EINVAL; treat that as a no-op rather
        // than failing the whole vacuum.
        match File::open(parent) {
            Ok(dir) => match dir.sync_all() {
                Ok(()) => Ok(()),
                Err(err) if err.raw_os_error() == Some(libc_einval()) => Ok(()),
                Err(err) => Err(LielError::Io(err)),
            },
            Err(err) => Err(LielError::Io(err)),
        }
    }

    /// Hard-coded `EINVAL` (22 on Linux/macOS) so we don't need a `libc`
    /// dependency just for this single constant.  The numeric value is part
    /// of the POSIX/SUSv4 standard and stable across the platforms we ship
    /// to (Linux, macOS, BSDs).
    fn libc_einval() -> i32 {
        22
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use crate::error::{LielError, Result};

    // We avoid pulling in a Win32 binding crate just for one syscall.  The
    // declarations below match `winbase.h` exactly; if Windows ever changes
    // them we'd already need a bigger fix everywhere.
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    extern "system" {
        fn MoveFileExW(
            lpExistingFileName: *const u16,
            lpNewFileName: *const u16,
            dwFlags: u32,
        ) -> i32;
    }

    pub(super) fn atomic_replace(src: &Path, dst: &Path) -> Result<()> {
        let src_w = to_wide(src);
        let dst_w = to_wide(dst);
        let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
        // SAFETY: pointers are NUL-terminated UTF-16 sequences owned for the
        // call's duration; the function returns nonzero on success.
        let ok = unsafe { MoveFileExW(src_w.as_ptr(), dst_w.as_ptr(), flags) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            return Err(LielError::Io(std::io::Error::new(
                err.kind(),
                format!(
                    "atomic_replace: MoveFileExW failed replacing {} with {}: {}",
                    dst.display(),
                    src.display(),
                    err
                ),
            )));
        }
        // `MOVEFILE_WRITE_THROUGH` already forces NTFS to flush the rename
        // through to the device; no separate directory fsync is needed.
        Ok(())
    }

    fn to_wide(path: &Path) -> Vec<u16> {
        let mut buf: Vec<u16> = path.as_os_str().encode_wide().collect();
        buf.push(0);
        buf
    }
}

// On platforms that are neither Unix nor Windows (e.g. WASI) we don't
// support atomic rename and refuse the call so vacuum can fail loudly
// instead of corrupting data.
#[cfg(not(any(unix, windows)))]
mod platform {
    use std::path::Path;

    use crate::error::{LielError, Result};

    pub(super) fn atomic_replace(_src: &Path, _dst: &Path) -> Result<()> {
        Err(LielError::Io(std::io::Error::other(
            "atomic_replace: unsupported platform",
        )))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::error::LielError;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn replaces_existing_destination() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("new.bin");
        let dst = dir.path().join("live.bin");

        fs::write(&dst, b"old contents").unwrap();
        fs::write(&src, b"new contents").unwrap();

        atomic_replace(&src, &dst).unwrap();

        assert!(!src.exists(), "src must be moved away");
        let after = fs::read(&dst).unwrap();
        assert_eq!(after, b"new contents");
    }

    #[test]
    fn creates_destination_when_absent() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("only.bin");
        let dst = dir.path().join("missing.bin");

        fs::write(&src, b"first time").unwrap();
        assert!(!dst.exists());

        atomic_replace(&src, &dst).unwrap();

        assert!(!src.exists());
        let after = fs::read(&dst).unwrap();
        assert_eq!(after, b"first time");
    }

    #[test]
    fn returns_io_error_when_source_missing() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("no-such.bin");
        let dst = dir.path().join("dst.bin");
        let err = atomic_replace(&src, &dst).expect_err("must fail when src is missing");
        match err {
            LielError::Io(_) => {}
            other => panic!("expected Io error, got {other:?}"),
        }
    }
}

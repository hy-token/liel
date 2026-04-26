use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{LielError, Result};

/// Cross-process writer guard backed by an atomic lock directory.
///
/// The directory itself is the lock. `owner.json` is only diagnostic metadata
/// used to decide whether a leftover lock can be reclaimed after a crash.
///
/// [`WriterLock::acquire`] retries [`create_dir`](fs::create_dir) at most
/// [`ACQUIRE_RETRY_CYCLES`] times after a successful stale-lock reclaim. That
/// bounds worst-case spinning when another process races on the same path, and
/// matches the `rename → delete → recreate` stale-recovery protocol (see
/// `docs/design/single-writer-guard.md`).
#[derive(Debug)]
pub struct WriterLock {
    path: PathBuf,
}

/// Upper bound on `create_dir` attempts after `AlreadyExists`, including passes
/// where stale recovery succeeds and we retry acquiring the lock directory.
const ACQUIRE_RETRY_CYCLES: usize = 4;

impl WriterLock {
    pub fn acquire(db_path: &Path) -> Result<Self> {
        let lock_path = lock_path_for(db_path);

        for _ in 0..ACQUIRE_RETRY_CYCLES {
            match fs::create_dir(&lock_path) {
                Ok(()) => {
                    if let Err(err) = write_owner_file(&lock_path, db_path) {
                        match fs::remove_dir_all(&lock_path) {
                            Ok(()) => return Err(err),
                            Err(cleanup_err) => {
                                return Err(LielError::Io(io::Error::new(
                                    cleanup_err.kind(),
                                    format!(
                                        "failed to write lock owner metadata for {} and failed to remove partially-created lock directory {}: original error: {}; cleanup error: {}",
                                        db_path.display(),
                                        lock_path.display(),
                                        err,
                                        cleanup_err
                                    ),
                                )))
                            }
                        }
                    }
                    return Ok(Self { path: lock_path });
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    if try_reclaim_stale_lock(&lock_path)? {
                        continue;
                    }
                    return Err(LielError::AlreadyOpen(format!(
                        "{} (lock directory: {})",
                        db_path.display(),
                        lock_path.display()
                    )));
                }
                Err(err) => return Err(LielError::Io(err)),
            }
        }

        Err(LielError::AlreadyOpen(format!(
            "{} (lock directory: {}; another process may be reclaiming a stale lock)",
            db_path.display(),
            lock_path.display()
        )))
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.path) {
            eprintln!(
                "liel: warning: failed to remove writer lock directory {}: {}",
                self.path.display(),
                err
            );
        }
    }
}

fn lock_path_for(db_path: &Path) -> PathBuf {
    let mut raw: OsString = db_path.as_os_str().to_os_string();
    raw.push(".lock");
    PathBuf::from(raw)
}

fn write_owner_file(lock_path: &Path, db_path: &Path) -> Result<()> {
    let owner_path = lock_path.join("owner.json");
    let now_ms = now_unix_ms();
    let body = format!(
        "{{\n  \"pid\": {},\n  \"created_at_unix_ms\": {},\n  \"path\": \"{}\"\n}}\n",
        std::process::id(),
        now_ms,
        json_escape(&db_path.display().to_string())
    );
    fs::write(owner_path, body).map_err(LielError::Io)
}

fn try_reclaim_stale_lock(lock_path: &Path) -> Result<bool> {
    let owner = match fs::read_to_string(lock_path.join("owner.json")) {
        Ok(text) => text,
        Err(_) => return Ok(false),
    };
    let Some(pid) = parse_owner_pid(&owner) else {
        return Ok(false);
    };
    if pid == std::process::id() || platform::process_is_alive(pid) {
        Ok(false)
    } else {
        reclaim_lock_dir(lock_path)
    }
}

fn reclaim_lock_dir(lock_path: &Path) -> Result<bool> {
    let reap_path = reap_path_for(lock_path);
    match fs::rename(lock_path, &reap_path) {
        Ok(()) => {
            fs::remove_dir_all(reap_path).map_err(LielError::Io)?;
            Ok(true)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(true),
        Err(err) => Err(LielError::Io(err)),
    }
}

fn reap_path_for(lock_path: &Path) -> PathBuf {
    let mut raw = lock_path.as_os_str().to_os_string();
    raw.push(format!(".reap.{}.{}", std::process::id(), now_unix_ms()));
    PathBuf::from(raw)
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn parse_owner_pid(owner: &str) -> Option<u32> {
    let key_pos = owner.find("\"pid\"")?;
    let after_key = &owner[key_pos + "\"pid\"".len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let digits: String = after_colon
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const ERROR_INVALID_PARAMETER: u32 = 87;

    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> Handle;
        fn GetExitCodeProcess(hProcess: Handle, lpExitCode: *mut u32) -> i32;
        fn CloseHandle(hObject: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn process_is_alive(pid: u32) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return GetLastError() != ERROR_INVALID_PARAMETER;
            }

            let mut exit_code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut exit_code);
            if CloseHandle(handle) == 0 {
                return true;
            }
            ok == 0 || exit_code == STILL_ACTIVE
        }
    }
}

#[cfg(unix)]
mod platform {
    const ESRCH: i32 = 3;

    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn errno() -> i32 {
        extern "C" {
            fn __errno_location() -> *mut i32;
        }
        *__errno_location()
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe fn errno() -> i32 {
        extern "C" {
            fn __error() -> *mut i32;
        }
        *__error()
    }

    pub fn process_is_alive(pid: u32) -> bool {
        unsafe {
            if kill(pid as i32, 0) == 0 {
                return true;
            }
            errno() != ESRCH
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    pub fn process_is_alive(_pid: u32) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn acquire_creates_and_drop_removes_lock_dir() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sample.liel");
        let lock_path = lock_path_for(&db_path);

        {
            let _lock = WriterLock::acquire(&db_path).unwrap();
            assert!(lock_path.is_dir());
            let owner = fs::read_to_string(lock_path.join("owner.json")).unwrap();
            assert!(owner.contains("\"pid\""));
            assert!(owner.contains("\"created_at_unix_ms\""));
            assert!(owner.contains("\"path\""));
        }

        assert!(!lock_path.exists());
    }

    #[test]
    fn live_owner_blocks_second_lock() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("live.liel");
        let _lock = WriterLock::acquire(&db_path).unwrap();

        match WriterLock::acquire(&db_path) {
            Err(LielError::AlreadyOpen(_)) => {}
            other => panic!("expected AlreadyOpen, got {other:?}"),
        }
    }

    #[test]
    fn stale_owner_is_reclaimed() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("stale.liel");
        let lock_path = lock_path_for(&db_path);
        fs::create_dir(&lock_path).unwrap();
        fs::write(
            lock_path.join("owner.json"),
            "{\n  \"pid\": 4294967295,\n  \"created_at_unix_ms\": 1,\n  \"path\": \"stale.liel\"\n}\n",
        )
        .unwrap();

        let _lock = WriterLock::acquire(&db_path).unwrap();
        assert!(lock_path.is_dir());
    }
}

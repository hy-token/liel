//! Deterministic crash points for vacuum copy-on-write testing.
//!
//! Gated entirely behind the `test-fault-injection` Cargo feature.  When the
//! feature is **off** (the default for release builds and ordinary
//! `cargo test`), [`crash_at`] is a `#[inline]` no-op and no env-var lookup
//! happens at all — the linker drops it.
//!
//! When the feature is **on**, `crash_at("AFTER_TMP_FSYNC")` reads the
//! `LIEL_VACUUM_CRASH_AT` environment variable and, if its value matches the
//! injection point name, calls `_exit(1)` immediately — bypassing destructors,
//! finalisers, and Rust panic unwinding.  The Python crash-safety harness
//! `fork`s the worker, sets the variable in the child, and observes the
//! resulting on-disk state from the parent.
//!
//! The names form the contract between this module and the test harness:
//!
//! - `BEFORE_TMP_OPEN`     — before the sibling `.tmp` is created
//! - `AFTER_TMP_WRITES`    — after every slot/blob is written, before commit
//! - `AFTER_TMP_FSYNC`     — after `commit()` durably flushed the new file
//! - `AFTER_RENAME`        — after the atomic `rename(tmp, original)`
//!
//! Adding a new injection point: add the name to the list above, call
//! `crash_at("NEW_NAME")` at the desired point in `vacuum.rs`, and document
//! it in the test harness.

#[cfg(feature = "test-fault-injection")]
const ENV_VAR: &str = "LIEL_VACUUM_CRASH_AT";

/// Conditionally terminate the process at a labelled injection point.
///
/// The call is a no-op unless every condition holds:
/// 1. The crate was built with `--features test-fault-injection`.
/// 2. The `LIEL_VACUUM_CRASH_AT` environment variable is set.
/// 3. Its value matches `name` byte-for-byte.
///
/// On a match the process exits via `_exit(1)`, which skips all destructors,
/// stdio buffers, and `atexit` handlers — exactly the semantics a kernel
/// `kill -9` would deliver, so the test exercises the on-disk state our
/// crash-safety design must tolerate.
#[cfg(feature = "test-fault-injection")]
pub fn crash_at(name: &str) {
    if let Ok(target) = std::env::var(ENV_VAR) {
        if target == name {
            // SAFETY: `_exit` is async-signal-safe and has no preconditions.
            // We deliberately bypass Rust unwinding because real crashes do
            // not run destructors either.
            unsafe { libc_exit(1) };
        }
    }
}

#[cfg(not(feature = "test-fault-injection"))]
#[inline(always)]
pub fn crash_at(_name: &str) {
    // The compiler is expected to fold this away when the caller passes a
    // string literal; nothing references the name at runtime.
}

#[cfg(feature = "test-fault-injection")]
unsafe fn libc_exit(code: i32) -> ! {
    extern "C" {
        fn _exit(status: i32) -> !;
    }
    _exit(code)
}

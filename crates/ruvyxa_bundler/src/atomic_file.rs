//! One durable file publish, used by every cache that writes to disk.
//!
//! Writing a cache entry means the same four steps everywhere: write a temporary
//! file, rename it over the target, survive the platforms where that rename can
//! fail, and never leave the temporary behind. Four call sites had grown their own
//! version of those steps — the bundler's compile cache and graph manifest, the
//! CLI's client-artifact cache and image cache — and they had drifted apart in
//! exactly the places a copy drifts:
//!
//!   - Two named the temporary after a fixed extension, so two writers publishing
//!     the same entry at once used one temporary file for both payloads.
//!   - One skipped removing the temporary when the *first* write failed, so a full
//!     disk left `.tmp` files behind on every attempt.
//!   - One recovered from a failed rename by reading the temporary back and
//!     writing whatever it got — `unwrap_or_default()` on that read, so a
//!     recovery that itself failed replaced a good cache entry with zero bytes.
//!
//! The bytes are already in memory at every call site, which is what makes a
//! shared helper possible: recovery re-writes the buffer it was given rather than
//! reading anything back, so no failure path can publish content that was never
//! passed in.
//!
//! Every caller here writes content-addressed entries — the key is a hash of the
//! bytes — so replacing an entry that another writer just published writes the
//! same content and the race has no observable outcome.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes temporaries created by the same process at the same instant.
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Temporary path that no other writer can pick.
///
/// The process id separates concurrent builds sharing a cache directory and the
/// counter separates threads inside one build. A name derived only from the
/// target path — the previous `with_extension("json.tmp")` — gave two writers
/// publishing the same entry one temporary between them, so each could rename a
/// file the other was still writing.
fn temporary_path(path: &Path) -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{sequence}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Publish `bytes` at `path`, replacing any existing file.
///
/// Readers see either the previous contents or the new ones, never a partial
/// write, on every platform where `rename` is atomic. Where it is not — a
/// cross-device target, or a Windows replacement the OS refuses — the bytes are
/// written directly rather than lost.
///
/// The temporary file is removed on every path out of this function, including
/// the ones that return an error.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path(path);
    if let Err(error) = fs::write(&temporary, bytes) {
        // The write may have created the file before failing partway.
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let result = fs::write(path, bytes);
            let _ = fs::remove_file(&temporary);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_content_and_leaves_no_temporary_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("entry.js");

        write_atomic(&target, b"compiled").expect("write must succeed");

        assert_eq!(fs::read(&target).expect("target exists"), b"compiled");
        assert_eq!(
            leftover_temporaries(dir.path()),
            0,
            "a published entry must leave no .tmp files"
        );
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("nested/deeper/entry.js");

        write_atomic(&target, b"compiled").expect("write must create parents");
        assert_eq!(fs::read(&target).expect("target exists"), b"compiled");
    }

    #[test]
    fn replaces_an_existing_entry_without_a_window_of_empty_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("entry.js");
        fs::write(&target, b"old").expect("seed");

        write_atomic(&target, b"new").expect("replace must succeed");

        assert_eq!(fs::read(&target).expect("target exists"), b"new");
        assert_eq!(leftover_temporaries(dir.path()), 0);
    }

    /// The temporary name must depend on more than the target path, or two
    /// writers publishing one entry share a single temporary file.
    #[test]
    fn temporaries_are_unique_per_call() {
        let path = Path::new("cache/entry.js");
        let first = temporary_path(path);
        let second = temporary_path(path);

        assert_ne!(first, second);
        assert_eq!(
            first.parent(),
            path.parent(),
            "temporary stays beside target"
        );
    }

    /// A directory in place of the target makes both the rename and the direct
    /// write fail. The error must surface and the temporary must still be gone.
    #[test]
    fn a_failed_publish_reports_the_error_and_still_cleans_up() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = dir.path().join("entry.js");
        fs::create_dir(&target).expect("occupy the target path with a directory");

        assert!(
            write_atomic(&target, b"compiled").is_err(),
            "publishing over a directory cannot succeed"
        );
        assert_eq!(
            leftover_temporaries(dir.path()),
            0,
            "a failed publish must not leave a .tmp file behind"
        );
    }

    fn leftover_temporaries(dir: &Path) -> usize {
        fs::read_dir(dir)
            .expect("readable directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count()
    }
}

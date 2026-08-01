//! Writing build output atomically.
//!
//! A build renders into a staging directory and is moved into place only once
//! it has fully succeeded, so an interrupted or failing build cannot leave a
//! half-written `dist/` that the next `start` would happily serve. If the move
//! fails partway, the previously moved outputs are restored.
//!
//! [`rename_with_windows_retry`] exists because a rename on Windows can fail
//! transiently while a virus scanner or indexer holds a handle on a
//! just-written file; a short bounded retry turns a spurious build failure back
//! into a success without hiding a real one.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Context;

use crate::*;

pub(crate) fn canonical_route_file(root: &Path, file: &Path) -> PathBuf {
    if file.is_absolute() {
        return ruvyxa_diagnostics::normalized_canonical_path(file);
    }

    let direct = ruvyxa_diagnostics::normalized_canonical_path(file);
    if direct.is_absolute() {
        return direct;
    }
    ruvyxa_diagnostics::normalized_canonical_path(&root.join(file))
}

pub(crate) fn resolve_layout_file(
    root: &Path,
    app_dir: &Path,
    layout_path: &str,
) -> Option<PathBuf> {
    let path = PathBuf::from(layout_path);
    let mut candidates = Vec::new();

    if path.is_absolute() {
        candidates.push(path);
    } else {
        candidates.push(root.join(&path));

        let app_relative = path
            .strip_prefix("app")
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        candidates.push(app_dir.join(app_relative));
    }

    let mut expanded = Vec::new();
    for candidate in candidates {
        expanded.push(candidate.clone());
        if candidate.extension().is_none() {
            for extension in ["tsx", "jsx", "ts", "js"] {
                expanded.push(candidate.with_extension(extension));
            }
        }
    }

    expanded
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| ruvyxa_diagnostics::normalized_canonical_path(&candidate))
}

pub(crate) fn create_build_staging_dir(out_dir: &Path) -> anyhow::Result<PathBuf> {
    create_build_temp_dir(out_dir, ".build-staging")
}

pub(crate) fn create_build_temp_dir(out_dir: &Path, prefix: &str) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(out_dir)?;
    let created_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp_dir = out_dir.join(format!("{prefix}-{}-{created_at}", std::process::id()));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

pub(crate) fn commit_staged_build_outputs(
    staging_dir: &Path,
    out_dir: &Path,
) -> anyhow::Result<()> {
    let backup_dir = create_build_temp_dir(out_dir, ".build-rollback")?;
    let moved_existing = match move_named_build_outputs(out_dir, &backup_dir) {
        Ok(moved) => moved,
        Err(error) => {
            let _ = fs::remove_dir_all(&backup_dir);
            return Err(error);
        }
    };
    let commit_result = move_named_build_outputs(staging_dir, out_dir);

    match commit_result {
        Ok(_) => {
            fs::remove_dir_all(&backup_dir)?;
            if staging_dir.exists() {
                fs::remove_dir_all(staging_dir)?;
            }
            Ok(())
        }
        Err(error) => {
            let _ = remove_named_build_outputs(out_dir);
            let rollback_result =
                restore_named_build_outputs(&backup_dir, out_dir, &moved_existing);
            let _ = fs::remove_dir_all(&backup_dir);
            if let Err(rollback_error) = rollback_result {
                return Err(error).with_context(|| {
                    format!(
                        "rollback also failed while restoring previous output: {rollback_error}"
                    )
                });
            }
            Err(error)
        }
    }
}

pub(crate) fn move_named_build_outputs(from: &Path, to: &Path) -> anyhow::Result<Vec<String>> {
    fs::create_dir_all(to)?;
    let mut moved = Vec::new();

    for name in BUILD_OUTPUT_DIRS.into_iter().chain(BUILD_OUTPUT_FILES) {
        let source = from.join(name);
        if !source.exists() {
            continue;
        }
        let destination = to.join(name);
        if destination.exists() {
            remove_path(&destination)?;
        }
        if let Err(error) = rename_with_windows_retry(&source, &destination) {
            let rollback_result = restore_named_build_outputs(to, from, &moved);
            let mut move_error: anyhow::Error = error.into();
            move_error = move_error.context(format!(
                "failed to move {} to {}",
                source.display(),
                destination.display()
            ));
            if let Err(rollback_error) = rollback_result {
                return Err(move_error).with_context(|| {
                    format!("rollback of partially moved outputs also failed: {rollback_error}")
                });
            }
            return Err(move_error);
        }
        moved.push(name.to_string());
    }

    Ok(moved)
}

pub(crate) fn restore_named_build_outputs(
    backup_dir: &Path,
    out_dir: &Path,
    moved_existing: &[String],
) -> anyhow::Result<()> {
    for name in moved_existing {
        let source = backup_dir.join(name);
        if !source.exists() {
            continue;
        }
        let destination = out_dir.join(name);
        if destination.exists() {
            remove_path(&destination)?;
        }
        rename_with_windows_retry(&source, &destination).with_context(|| {
            format!(
                "failed to restore {} to {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

pub(crate) fn rename_with_windows_retry(source: &Path, destination: &Path) -> std::io::Result<()> {
    let mut delay = Duration::from_millis(25);

    for attempt in 0..WINDOWS_RENAME_RETRY_COUNT {
        match fs::rename(source, destination) {
            Ok(()) => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && error.kind() == std::io::ErrorKind::PermissionDenied
                    && attempt + 1 < WINDOWS_RENAME_RETRY_COUNT =>
            {
                std::thread::sleep(delay);
                delay = delay.saturating_mul(2);
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("the retry loop returns on its final attempt")
}

pub(crate) fn remove_named_build_outputs(out_dir: &Path) -> anyhow::Result<()> {
    for name in BUILD_OUTPUT_DIRS.into_iter().chain(BUILD_OUTPUT_FILES) {
        let path = out_dir.join(name);
        if path.exists() {
            remove_path(&path)?;
        }
    }

    Ok(())
}

pub(crate) fn remove_path(path: &Path) -> anyhow::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

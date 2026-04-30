//! Cross-platform tick lock — `fs2`-backed exclusive file lock so
//! multiple gateway / daemon processes don't double-fire jobs.
//!
//! Hermes' `cron/scheduler.py` uses `fcntl` on Unix and `msvcrt`
//! on Windows; `fs2` wraps both behind a single `try_lock_exclusive`
//! API so we don't need conditional compilation here.
//!
//! Convention: the lock file lives at
//! `<workspace>/cron/.tick.lock`. The first daemon to acquire it
//! holds the file open for the lifetime of the process; competing
//! daemons see `Err(TickLockError::AlreadyHeld)` and back off.
use fs2::FileExt;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Cross-platform tick lock guard. Drop releases the lock by
/// closing the underlying [`File`].
#[derive(Debug)]
pub struct TickLock {
    /// Path the lock was acquired against.
    pub path: PathBuf,
    file: File,
}

impl TickLock {
    /// Try to acquire the lock at `path`. Returns
    /// [`TickLockError::AlreadyHeld`] if another process holds it.
    ///
    /// # Errors
    /// Returns the underlying I/O error wrapped in
    /// [`TickLockError::Io`] for create/open failures, or
    /// [`TickLockError::AlreadyHeld`] for contention.
    pub fn try_acquire(path: &Path) -> Result<Self, TickLockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| TickLockError::Io {
                detail: format!("create_dir_all {}: {e}", parent.display()),
            })?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)
            .map_err(|e| TickLockError::Io {
                detail: format!("open {}: {e}", path.display()),
            })?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Self {
                path: path.to_path_buf(),
                file,
            }),
            Err(_) => Err(TickLockError::AlreadyHeld {
                path: path.to_path_buf(),
            }),
        }
    }
}

impl Drop for TickLock {
    fn drop(&mut self) {
        // Best-effort unlock; closing the file releases the lock
        // even if `unlock` returns an error.
        let _ = FileExt::unlock(&self.file);
    }
}

/// Errors from [`TickLock::try_acquire`].
#[derive(Debug, ::thiserror::Error)]
pub enum TickLockError {
    /// Another process already holds the lock.
    #[error("tick lock at {path} is already held")]
    AlreadyHeld {
        /// Lock path.
        path: PathBuf,
    },
    /// Filesystem error opening or locking the file.
    #[error("tick lock i/o: {detail}")]
    Io {
        /// What failed.
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_acquire_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".tick.lock");
        let _lock = TickLock::try_acquire(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn second_acquire_returns_already_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".tick.lock");
        let _lock = TickLock::try_acquire(&path).unwrap();
        // Re-acquiring without dropping the first should fail.
        let result = TickLock::try_acquire(&path);
        match result {
            Err(TickLockError::AlreadyHeld { .. }) => {}
            other => panic!("expected AlreadyHeld, got {other:?}"),
        }
    }

    #[test]
    fn drop_releases_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".tick.lock");
        {
            let _lock = TickLock::try_acquire(&path).unwrap();
        }
        // After drop, a fresh acquire should succeed.
        let _again = TickLock::try_acquire(&path).unwrap();
    }
}

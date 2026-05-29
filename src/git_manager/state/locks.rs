//! Two-level locks and access modes for `GitManager`.

#![allow(clippy::missing_const_for_fn, clippy::unused_self)]

use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static PROCESS_LOCAL_LOCKS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
static LOCK_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Typed repository access mode for a Git operation.
///
/// `ReadOnly` is used for snapshot/descriptor/overview scenarios and does not create write-like
/// filesystem lock artifacts. `Mutating` preserves the current exclusive semantics via the
/// process-local and filesystem lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitLockMode {
    /// Read-only access without write-like lock artifacts.
    ReadOnly,
    /// Mutating access with an exclusive process-local and filesystem lock.
    Mutating,
}

impl GitLockMode {
    /// Returns the stable machine-readable identifier for the access mode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Mutating => "mutating",
        }
    }

    const fn creates_filesystem_lock(self) -> bool {
        matches!(self, Self::Mutating)
    }
}

/// Two-level lock manager for `GitManager` operations.
#[derive(Debug, Clone, Default)]
pub struct GitLockManager;

impl GitLockManager {
    /// Creates a lock manager with process-local and filesystem locks.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Acquires an access guard for the repository according to the typed lock policy.
    ///
    /// In [`GitLockMode::ReadOnly`] mode the guard does not create a filesystem lock and does not
    /// block refresh under the lock-manager policy. In [`GitLockMode::Mutating`] mode the canonical
    /// exclusive path with a process-local and filesystem lock is used.
    ///
    /// # Errors
    /// Returns `INVALID_REPO_PATH` if the repository path is empty or invalid.
    /// Returns `LOCK_CONTENTION` or `LOCK_IO` for the exclusive mutating path.
    #[cfg_attr(all(debug_assertions, feature = "trace_logs"), tracing::instrument(skip_all, fields(access_mode = mode.as_str())))]
    pub fn access(&self, repository_path: &Path, mode: GitLockMode) -> GitResult<GitLockGuard> {
        validate_non_empty_path(repository_path, "repository path")?;

        tracing::trace!(
            access_mode = mode.as_str(),
            repository_path = %repository_path.display(),
            creates_filesystem_lock = mode.creates_filesystem_lock(),
            "preparing git repository access"
        );

        if matches!(mode, GitLockMode::ReadOnly) {
            return Ok(GitLockGuard::read_only(repository_path.to_path_buf()));
        }

        let lock_path = self.lock_path_for_repository(repository_path)?;
        self.acquire_exclusive(lock_path.as_path())
    }

    /// Builds the lock file path for the repository.
    ///
    /// # Errors
    /// Returns `INVALID_REPO_PATH` if the repository path is empty.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn lock_path_for_repository(&self, repository_path: &Path) -> GitResult<PathBuf> {
        validate_non_empty_path(repository_path, "repository path")?;
        let mut lock_path = repository_path.to_path_buf();
        let _ = lock_path.set_extension("avacana-gm.lock");
        Ok(lock_path)
    }

    /// Acquires a lock using the two-level scheme (`process-local` + filesystem).
    ///
    /// # Errors
    /// Returns `LOCK_CONTENTION` if the lock is already held.
    /// Returns `INVALID_REPO_PATH` if the lock path is invalid.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn acquire(&self, lock_path: &Path) -> GitResult<GitLockGuard> {
        self.acquire_exclusive(lock_path)
    }

    fn acquire_exclusive(&self, lock_path: &Path) -> GitResult<GitLockGuard> {
        validate_non_empty_path(lock_path, "lock path")?;
        let process_key = normalize_process_key(lock_path)?;

        tracing::trace!(
            access_mode = GitLockMode::Mutating.as_str(),
            lock_path = %lock_path.display(),
            creates_filesystem_lock = true,
            "acquiring exclusive git repository lock"
        );

        acquire_process_local_lock(&process_key)?;

        let lock_token = build_lock_token();
        if let Err(error) = create_filesystem_lock(lock_path, &lock_token) {
            release_process_local_lock(&process_key);
            return Err(error);
        }

        Ok(GitLockGuard::exclusive(process_key, lock_path, lock_token))
    }
}

/// RAII guard for an acquired lock.
#[derive(Debug)]
pub struct GitLockGuard {
    mode: GitLockMode,
    state: GitLockGuardState,
    released: bool,
}

#[derive(Debug)]
enum GitLockGuardState {
    ReadOnly {
        repository_path: PathBuf,
    },
    Exclusive {
        process_key: PathBuf,
        lock_path: PathBuf,
        lock_token: String,
    },
}

impl GitLockGuard {
    fn read_only(repository_path: PathBuf) -> Self {
        Self {
            mode: GitLockMode::ReadOnly,
            state: GitLockGuardState::ReadOnly { repository_path },
            released: false,
        }
    }

    fn exclusive(process_key: PathBuf, lock_path: &Path, lock_token: String) -> Self {
        Self {
            mode: GitLockMode::Mutating,
            state: GitLockGuardState::Exclusive {
                process_key,
                lock_path: lock_path.to_path_buf(),
                lock_token,
            },
            released: false,
        }
    }

    /// Explicitly releases the lock before the guard goes out of scope.
    ///
    /// # Errors
    /// Returns `LOCK_IO` if the lock file could not be processed.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn release(mut self) -> GitResult<()> {
        self.release_internal()
    }

    fn release_internal(&mut self) -> GitResult<()> {
        if self.released {
            return Ok(());
        }
        self.released = true;

        match &self.state {
            GitLockGuardState::ReadOnly { repository_path } => {
                tracing::trace!(
                    access_mode = self.mode.as_str(),
                    repository_path = %repository_path.display(),
                    creates_filesystem_lock = false,
                    "releasing read-only git repository access"
                );
                Ok(())
            }
            GitLockGuardState::Exclusive {
                process_key,
                lock_path,
                lock_token,
            } => {
                let release_file_result = remove_owned_filesystem_lock(lock_path, lock_token);
                release_process_local_lock(process_key);

                tracing::trace!(
                    access_mode = self.mode.as_str(),
                    lock_path = %lock_path.display(),
                    creates_filesystem_lock = true,
                    "releasing exclusive git repository lock"
                );

                release_file_result
            }
        }
    }
}

impl Drop for GitLockGuard {
    fn drop(&mut self) {
        let _ = self.release_internal();
    }
}

fn process_local_locks() -> &'static Mutex<HashSet<PathBuf>> {
    PROCESS_LOCAL_LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn validate_non_empty_path(path: &Path, label: &str) -> GitResult<()> {
    if path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRepoPath,
            format!("{label} must not be empty"),
        ));
    }
    Ok(())
}

fn normalize_process_key(lock_path: &Path) -> GitResult<PathBuf> {
    if lock_path.is_absolute() {
        return Ok(lock_path.to_path_buf());
    }

    std::env::current_dir()
        .map(|current_dir| current_dir.join(lock_path))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to resolve current directory for lock key: {error}"),
            )
        })
}

fn acquire_process_local_lock(process_key: &Path) -> GitResult<()> {
    let mut table = process_local_locks().lock().map_err(|_| {
        GitError::new(
            GitErrorCode::Internal,
            "process-local lock table is poisoned",
        )
    })?;

    if table.insert(process_key.to_path_buf()) {
        return Ok(());
    }

    drop(table);

    Err(GitError::new(
        GitErrorCode::LockContention,
        format!(
            "process-local lock contention for `{}`",
            process_key.display()
        ),
    ))
}

fn release_process_local_lock(process_key: &Path) {
    match process_local_locks().lock() {
        Ok(mut table) => {
            let _ = table.remove(process_key);
        }
        Err(poisoned) => {
            let mut table = poisoned.into_inner();
            let _ = table.remove(process_key);
        }
    }
}

fn build_lock_token() -> String {
    let sequence = LOCK_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("pid:{};seq:{sequence}", std::process::id())
}

fn create_filesystem_lock(lock_path: &Path, lock_token: &str) -> GitResult<()> {
    ensure_lock_parent_exists(lock_path)?;

    let mut lock_file = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(lock_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Err(GitError::new(
                GitErrorCode::LockContention,
                format!("filesystem lock contention for `{}`", lock_path.display()),
            ))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(GitError::new(
                GitErrorCode::InvalidRepoPath,
                format!(
                    "lock parent directory does not exist for `{}`",
                    lock_path.display()
                ),
            ))
        }
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::LockIo,
                format!(
                    "failed to create lock file `{}`: {error}",
                    lock_path.display()
                ),
            ))
        }
    };

    lock_file
        .write_all(lock_token.as_bytes())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::LockIo,
                format!(
                    "failed to write lock token to `{}`: {error}",
                    lock_path.display()
                ),
            )
        })?;
    lock_file.flush().map_err(|error| {
        GitError::new(
            GitErrorCode::LockIo,
            format!(
                "failed to flush lock file `{}`: {error}",
                lock_path.display()
            ),
        )
    })
}

fn ensure_lock_parent_exists(lock_path: &Path) -> GitResult<()> {
    let parent_dir = lock_path.parent().map_or_else(
        || PathBuf::from("."),
        |parent| {
            if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.to_path_buf()
            }
        },
    );

    if parent_dir.exists() {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::InvalidRepoPath,
        format!(
            "lock parent directory does not exist: `{}`",
            parent_dir.display()
        ),
    ))
}

fn remove_owned_filesystem_lock(lock_path: &Path, lock_token: &str) -> GitResult<()> {
    let existing_token = match std::fs::read_to_string(lock_path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::LockIo,
                format!(
                    "failed to read lock file `{}`: {error}",
                    lock_path.display()
                ),
            ))
        }
    };

    if existing_token != lock_token {
        return Ok(());
    }

    match std::fs::remove_file(lock_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GitError::new(
            GitErrorCode::LockIo,
            format!(
                "failed to remove lock file `{}`: {error}",
                lock_path.display()
            ),
        )),
    }
}


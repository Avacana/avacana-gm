//! Working-directory state policies for branch switching operations.

use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{Repository, StatusOptions};

/// Branch switching policy with respect to the worktree/index state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SwitchWorktreePolicy {
    /// Allows `switch` to run with a dirty worktree.
    pub allow_dirty: bool,
}

impl SwitchWorktreePolicy {
    /// Creates a policy for `switch`.
    #[must_use]
    pub const fn new(allow_dirty: bool) -> Self {
        Self { allow_dirty }
    }
}

/// Enforces the dirty-worktree policy for the `switch` operation.
///
/// # Errors
/// Returns `WORKTREE_DIRTY` if `allow_dirty=false` and uncommitted changes
/// are found in the `worktree/index`.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(skip_all, fields(allow_dirty = policy.allow_dirty))
)]
pub fn enforce_switch_worktree_policy(
    repository: &Repository,
    policy: SwitchWorktreePolicy,
) -> GitResult<()> {
    if policy.allow_dirty {
        return Ok(());
    }

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to inspect repository status for switch policy: {error}"),
            )
        })?;

    if statuses.is_empty() {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::WorktreeDirty,
        "switch requires a clean worktree unless allow_dirty=true",
    ))
}

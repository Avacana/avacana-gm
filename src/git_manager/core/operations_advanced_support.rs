//! Helper utilities of the `advanced` domain for `GitManager`.

#![allow(clippy::redundant_pub_crate)]

#[path = "operations_advanced_support_path.rs"]
mod path_support;
#[path = "operations_advanced_support_submodule.rs"]
mod submodule_support;
#[path = "operations_advanced_support_trace.rs"]
mod trace_support;
#[path = "operations_advanced_support_worktree.rs"]
mod worktree_support;

pub(super) use crate::git_manager::core::repository_access::open_repository;
pub(super) use path_support::{
    add_status_pathspec, resolve_repository_relative_path, resolve_request_path,
};
pub(super) use submodule_support::sync_submodule_recursive;
pub(super) use trace_support::execute_trace_set_operation;
pub(super) use worktree_support::{
    append_worktree_lock_status, derive_worktree_name, detach_worktree_head,
    ensure_unique_worktree_name, ensure_worktree_clean_before_removal, find_worktree_by_path,
    resolve_worktree_reference,
};

use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::ErrorCode;

pub(super) fn normalize_non_empty<'value>(
    value: &'value str,
    field_name: &str,
) -> GitResult<&'value str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not be empty"),
        ));
    }

    if trimmed.contains('\0') {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not contain NUL bytes"),
        ));
    }

    Ok(trimmed)
}

pub(super) fn normalize_optional_non_empty<'value>(
    value: Option<&'value str>,
    field_name: &str,
) -> GitResult<Option<&'value str>> {
    value
        .map(|value| normalize_non_empty(value, field_name))
        .transpose()
}

#[must_use]
pub(super) fn map_advanced_error(error: &git2::Error, context: impl AsRef<str>) -> GitError {
    let code = match error.code() {
        ErrorCode::NotFound
        | ErrorCode::UnbornBranch
        | ErrorCode::InvalidSpec
        | ErrorCode::Ambiguous
        | ErrorCode::Peel => GitErrorCode::RefNotFound,
        ErrorCode::Locked => GitErrorCode::LockContention,
        ErrorCode::Conflict
        | ErrorCode::MergeConflict
        | ErrorCode::ApplyFail
        | ErrorCode::Unmerged => GitErrorCode::MergeConflict,
        ErrorCode::Invalid
        | ErrorCode::User
        | ErrorCode::Directory
        | ErrorCode::Exists
        | ErrorCode::BareRepo => GitErrorCode::AdvancedInvalidInput,
        _ => GitErrorCode::AdvancedOperationFailed,
    };

    GitError::new(code, format!("{}: {error}", context.as_ref()))
}

#[must_use]
pub(super) const fn empty_advanced_result() -> AdvancedResult {
    AdvancedResult {
        changed: false,
        summary: None,
        items: Vec::new(),
    }
}

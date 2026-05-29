//! query/lifecycle/verification domain helper functions.

#![allow(clippy::redundant_pub_crate)]

#[path = "operations_query_lifecycle_support_diff.rs"]
mod diff_support;
#[path = "operations_query_lifecycle_support_history.rs"]
mod history_support;
#[path = "operations_query_lifecycle_support_message.rs"]
mod message_support;
#[path = "operations_query_lifecycle_support_tree.rs"]
mod tree_support;

pub(super) use crate::git_manager::core::repository_access::open_repository;
pub(super) use diff_support::collect_commit_change_entries;
pub(super) use history_support::{collect_revwalk_oids, commit_summary, resolve_commit_summary};
pub(super) use message_support::{
    classify_unsupported_command, extract_message_details, resolve_revspec,
    validate_blame_line_range,
};
pub(super) use tree_support::{collect_tree_entries, collect_tree_entries_walk, resolve_tree};

use crate::git_manager::core::{GitError, GitErrorCode, GitResult, QueryLifecycleResult};
use git2::ErrorCode;

const DEFAULT_LOG_MAX_COUNT: usize = 128;

pub(super) fn normalize_max_count(max_count: usize, operation: &str) -> GitResult<usize> {
    if max_count == 0 {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            format!("query_lifecycle.{operation} max_count must be greater than zero"),
        ));
    }

    Ok(max_count.min(DEFAULT_LOG_MAX_COUNT))
}

pub(super) fn normalize_non_empty<'value>(
    value: &'value str,
    field: &str,
) -> GitResult<&'value str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            format!("{field} must not be empty"),
        ));
    }

    if trimmed.contains('\0') {
        return Err(GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            format!("{field} must not contain NUL bytes"),
        ));
    }

    Ok(trimmed)
}

#[must_use]
pub(super) fn normalize_optional_revision(revision: Option<&str>) -> Option<&str> {
    revision.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

#[must_use]
pub(super) const fn empty_query_result() -> QueryLifecycleResult {
    QueryLifecycleResult {
        changed: false,
        initialized_repository: None,
        config_value: None,
        blame_hunks: Vec::new(),
        commits: Vec::new(),
        revspec: None,
        tree_entries: Vec::new(),
        merge_file_preview: None,
        shortlog_entries: Vec::new(),
        message_details: None,
        change_entries: Vec::new(),
        formatted_email: None,
        version: None,
        unsupported: None,
        summary: None,
    }
}

#[must_use]
pub(super) fn map_query_error(error: &git2::Error, context: impl AsRef<str>) -> GitError {
    let error_code = if is_absent_head_error(error) {
        GitErrorCode::RefNotFound
    } else {
        GitErrorCode::QueryLifecycleFailed
    };

    GitError::new(error_code, format!("{}: {error}", context.as_ref()))
}

fn is_absent_head_error(error: &git2::Error) -> bool {
    matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch)
}

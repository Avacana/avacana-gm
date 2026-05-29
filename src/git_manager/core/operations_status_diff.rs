use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, StatusDiffRequest, StatusDiffResult,
};
use git2::ErrorCode;

#[path = "operations_status_diff_apply.rs"]
mod apply_operations;
#[path = "operations_status_diff_diff.rs"]
mod diff_operations;
#[path = "operations_status_diff_mapping.rs"]
mod mapping_operations;
#[path = "operations_status_diff_pathspec.rs"]
mod pathspec_operations;
#[path = "operations_status_diff_payload.rs"]
mod payload_operations;
#[path = "operations_status_diff_rendering.rs"]
mod rendering;

use apply_operations::apply_patch_if_requested;
use diff_operations::collect_scope_entries;
use mapping_operations::normalize_entries;
use pathspec_operations::validate_pathspecs;
use payload_operations::build_diff_payload;

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            scope = ?request.scope,
            pathspec_count = request.pathspecs.len(),
            include_patch = request.include_patch,
            apply_requested = request.apply.is_some()
        )
    )
)]
pub(super) fn execute_status_diff_operation(
    request: &StatusDiffRequest,
) -> GitResult<StatusDiffResult> {
    validate_pathspecs(request.pathspecs.as_slice())?;
    let repository = open_repository(&request.repository_path, "status_diff")?;
    apply_patch_if_requested(&repository, request)?;
    let mut entries =
        collect_scope_entries(&repository, request.scope, request.pathspecs.as_slice())?;
    normalize_entries(&mut entries);
    let (diff_summary, diff_details) = if request.include_patch {
        let (summary, details) = build_diff_payload(
            &repository,
            request.scope,
            request.pathspecs.as_slice(),
            entries.as_slice(),
            request.render_diff_format,
        )?;
        (Some(summary), Some(details))
    } else {
        (None, None)
    };
    Ok(StatusDiffResult {
        scope: request.scope,
        entries,
        diff_summary,
        diff_details,
    })
}

pub(super) fn status_diff_error(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::StatusDiffFailed, message)
}

pub(super) fn invalid_pathspec_error(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::StatusDiffInvalidPathspec, message)
}

pub(super) fn apply_patch_error(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::ApplyPatchFailed, message)
}

pub(super) fn is_absent_head_error(error: &git2::Error) -> bool {
    matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch)
}

//! `merge` operation for `GitManager`.

use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, MergeRequest, MergeResult};

#[path = "operations_merge_execution.rs"]
mod execution;
#[path = "operations_merge_resolution.rs"]
mod resolution;

use execution::{perform_fast_forward_merge, perform_non_fast_forward_merge};
use resolution::{
    normalize_non_empty, resolve_attached_head_reference_name, resolve_source_annotated_commit,
    resolve_target_reference_name,
};

/// Executes a `merge` of the current branch with the given source ref.
///
/// # Errors
/// Returns a typed `GitError`, including `MERGE_CONFLICT`,
/// `DETACHED_HEAD`, `REF_NOT_FOUND`, and repository/index state errors.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            source_ref = request.source_ref,
            target_ref = ?request.target_ref
        )
    )
)]
pub fn execute_merge_operation(request: &MergeRequest) -> GitResult<MergeResult> {
    let source_ref = normalize_non_empty(request.source_ref.as_str(), "merge source_ref")?;
    let repository = open_repository(&request.repository_path, "merge")?;
    execution::ensure_clean_repository_state(&repository, "merge")?;
    let head_reference_name = resolve_attached_head_reference_name(&repository)?;
    let target_reference_name = resolve_target_reference_name(
        &repository,
        request.target_ref.as_deref(),
        head_reference_name.as_str(),
    )?;

    if target_reference_name != head_reference_name {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!(
                "merge target `{target_reference_name}` is not currently checked out (HEAD is `{head_reference_name}`); switch first"
            ),
        ));
    }

    let source_annotated_commit = resolve_source_annotated_commit(&repository, source_ref)?;
    let (merge_analysis, _) = repository
        .merge_analysis(&[&source_annotated_commit])
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("merge-analysis failed for source `{source_ref}`: {error}"),
            )
        })?;

    tracing::trace!(
        up_to_date = merge_analysis.is_up_to_date(),
        fast_forward = merge_analysis.is_fast_forward(),
        normal_merge = merge_analysis.is_normal(),
        "merge analysis result"
    );

    if merge_analysis.is_up_to_date() {
        return Ok(MergeResult {
            merged: false,
            fast_forward: false,
        });
    }

    if merge_analysis.is_fast_forward() {
        perform_fast_forward_merge(
            &repository,
            head_reference_name.as_str(),
            &source_annotated_commit,
            source_ref,
        )?;
        return Ok(MergeResult {
            merged: true,
            fast_forward: true,
        });
    }

    if merge_analysis.is_normal() {
        perform_non_fast_forward_merge(
            &repository,
            &source_annotated_commit,
            source_ref,
            head_reference_name.as_str(),
            request.file_favor,
        )?;
        return Ok(MergeResult {
            merged: true,
            fast_forward: false,
        });
    }

    Err(GitError::new(
        GitErrorCode::Internal,
        "merge-analysis produced unsupported outcome flags",
    ))
}

//! `repository_descriptor` operation for `GitManager`.

use crate::git_manager::core::repository_access::{
    describe_opened_repository, open_repository_context,
};
use crate::git_manager::core::{
    GitResult, RepositoryDescriptorRequest, RepositoryDescriptorResult,
};
use std::time::Instant;

/// Runs the canonical discovery/open/describe path for a typed repository descriptor.
///
/// # Errors
/// Returns a typed `GitError` if repository discovery/open/canonicalize is not possible.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "repository_descriptor",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_repository_descriptor_operation(
    request: &RepositoryDescriptorRequest,
) -> GitResult<RepositoryDescriptorResult> {
    let started_at = Instant::now();
    let opened_repository =
        open_repository_context(&request.repository_path, "repository_descriptor")?;
    let repository = describe_opened_repository(&opened_repository);
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(repository.repo_root.display()),
    );
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "repository_descriptor",
        requested_path = %request.repository_path.display(),
        repo_root = %repository.repo_root.display(),
        elapsed_ms,
        "resolved typed repository descriptor"
    );

    Ok(RepositoryDescriptorResult { repository })
}


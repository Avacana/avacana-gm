//! `pull` operations for `GitManager`.

use crate::git_manager::core::operations_clone_fetch_push::{
    execute_fetch_operation, FetchOperationRequest,
};
use crate::git_manager::core::repository_access::{
    describe_opened_repository, open_repository_context,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, PullMode, PullRequest, PullResult,
};
use crate::git_manager::transport::Git2TransportBridge;

#[path = "operations_pull_merge.rs"]
mod merge_operations;
#[path = "operations_pull_plan.rs"]
mod plan_operations;

use merge_operations::merge_fetched_head;
use plan_operations::{
    detached_head_error, ensure_pull_rebase_disabled, resolve_pull_branch, validate_pull_request,
};

/// Performs a `pull` in `fetch + merge` mode.
///
/// # Errors
/// Returns a typed `GitError`, including
/// `PULL_REBASE_NOT_SUPPORTED_YET`, `MERGE_CONFLICT`, `DETACHED_HEAD`,
/// `UPSTREAM_NOT_FOUND`, and transport codes propagated from `fetch`.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            remote = request.remote_name,
            branch = ?request.branch,
            depth = tracing::field::Empty,
            upstream = tracing::field::Empty,
            tag_mode = ?request.tag_mode,
            file_favor = ?request.file_favor,
            merge_mode = ?request.mode
        )
    )
)]
pub(super) fn execute_pull_operation(
    request: &PullRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<PullResult> {
    validate_pull_request(request)?;
    tracing::Span::current().record("depth", tracing::field::display("none"));

    if request.mode != PullMode::FetchAndMerge {
        return Err(GitError::new(
            GitErrorCode::NotImplemented,
            "only pull mode `FetchAndMerge` is supported in FR-083 MVP scope",
        ));
    }

    let opened_repository = open_repository_context(&request.repository_path, "pull")?;
    let repository = &opened_repository.repository;
    let descriptor = describe_opened_repository(&opened_repository);
    let Some(current_branch) = descriptor.current_branch.as_deref() else {
        return Err(detached_head_error());
    };

    if descriptor.head_oid.is_none() {
        #[cfg(all(debug_assertions, feature = "trace_logs"))]
        tracing::trace!(
            current_branch,
            "pull rejected unborn HEAD before merge path"
        );

        return Err(detached_head_error());
    }

    ensure_pull_rebase_disabled(repository, current_branch)?;

    let branch_to_fetch = if request.refspecs.is_empty() {
        Some(resolve_pull_branch(&descriptor, request, current_branch)?)
    } else {
        request
            .branch
            .as_deref()
            .and_then(|name| {
                let trimmed_name = name.trim();
                if trimmed_name.is_empty() {
                    None
                } else {
                    Some(trimmed_name)
                }
            })
            .map(str::to_owned)
    };

    let upstream_display = branch_to_fetch.as_deref().unwrap_or("<custom-refspec>");
    tracing::Span::current().record("upstream", tracing::field::display(upstream_display));
    tracing::trace!(
        resolved_upstream_branch = upstream_display,
        "pull pipeline resolved upstream branch"
    );

    let fetch_operation_request = FetchOperationRequest {
        repository_path: &request.repository_path,
        remote_name: request.remote_name.as_str(),
        branch: branch_to_fetch.as_deref(),
        depth: None,
        tag_mode: request.tag_mode,
        refspecs: request.refspecs.as_slice(),
        prune: request.prune,
        mirror: false,
        operation_name: "pull",
    };
    execute_fetch_operation(&fetch_operation_request, transport_bridge)?;

    merge_fetched_head(repository, request.file_favor)
}

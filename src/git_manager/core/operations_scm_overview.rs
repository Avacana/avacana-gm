//! `scm_overview` operation for `GitManager`.

#![allow(clippy::too_many_lines)]

use crate::git_manager::core::operations_query_lifecycle::execute_query_lifecycle_operation;
use crate::git_manager::core::operations_refs_meta::execute_refs_operation;
use crate::git_manager::core::operations_repository_descriptor::execute_repository_descriptor_operation;
use crate::git_manager::core::operations_working_copy_overview::execute_working_copy_overview_operation;
use crate::git_manager::core::repository_access::open_repository_context;
use crate::git_manager::core::{
    QueryLifecycleOperation, QueryLifecycleRequest, RefsOperation, RefsRequest,
    RepositoryDescriptor, RepositoryDescriptorRequest, ScmBranchTrackingSummary,
    ScmHeadCommitSummary, ScmOverview, ScmOverviewRequest, ScmOverviewResult, ScmStashEntrySummary,
    ScmStashSummary, ScmSubmoduleSummary, ScmWorktreeSummary, WorkingCopyOverviewRequest,
};
use crate::git_manager::GitResult;
use git2::Repository;
use std::collections::BTreeSet;
use std::time::Instant;

/// Assembles a typed higher-level SCM overview on top of the existing read-only domain operations.
///
/// # Errors
/// Returns a typed `GitError` if canonical repository/working-copy/refs discovery is
/// unavailable. The optional higher-level summaries (`stash`, `worktree`, `submodules`, `head_commit`)
/// degrade to `None` when the backend cannot compute them reliably.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "scm_overview",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            stash_summary_available = tracing::field::Empty,
            worktree_summary_available = tracing::field::Empty,
            submodule_summary_available = tracing::field::Empty,
            head_commit_available = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_scm_overview_operation(
    request: &ScmOverviewRequest,
) -> GitResult<ScmOverviewResult> {
    let started_at = Instant::now();
    let descriptor = execute_repository_descriptor_operation(&RepositoryDescriptorRequest {
        repository_path: request.repository_path.clone(),
    })?
    .repository;
    let working_copy = execute_working_copy_overview_operation(&WorkingCopyOverviewRequest::new(
        request.repository_path.clone(),
    ))?
    .overview;
    let refs_result = execute_refs_operation(&RefsRequest {
        repository_path: request.repository_path.clone(),
        operation: RefsOperation::List { pattern: None },
    })?;

    let mut opened_repository = open_repository_context(&request.repository_path, "scm_overview")?;
    let head_commit = try_collect_head_commit_summary(request);
    let branch_tracking = build_branch_tracking_summary(
        &descriptor,
        refs_result
            .references
            .iter()
            .map(|reference| reference.name.as_str()),
        head_commit,
    );
    let stash = try_collect_stash_summary(&mut opened_repository.repository);
    let worktree = try_collect_worktree_summary(&opened_repository.repository);
    let submodules = try_collect_submodule_summary(&opened_repository.repository);
    let overview = ScmOverview::new(working_copy, branch_tracking, stash, worktree, submodules);
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(overview.working_copy.repository.repo_root.display()),
    );
    tracing::Span::current().record(
        "stash_summary_available",
        tracing::field::display(overview.stash.is_some()),
    );
    tracing::Span::current().record(
        "worktree_summary_available",
        tracing::field::display(overview.worktree.is_some()),
    );
    tracing::Span::current().record(
        "submodule_summary_available",
        tracing::field::display(overview.submodules.is_some()),
    );
    tracing::Span::current().record(
        "head_commit_available",
        tracing::field::display(overview.branch_tracking.head_commit.is_some()),
    );
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "scm_overview",
        requested_path = %request.repository_path.display(),
        repo_root = %overview.working_copy.repository.repo_root.display(),
        current_branch = ?overview.branch_tracking.current_branch,
        upstream_branch = ?overview.branch_tracking.upstream_branch,
        stash_summary_available = overview.stash.is_some(),
        worktree_summary_available = overview.worktree.is_some(),
        submodule_summary_available = overview.submodules.is_some(),
        head_commit_available = overview.branch_tracking.head_commit.is_some(),
        elapsed_ms,
        "assembled typed scm overview from repository, working copy, refs and lifecycle data"
    );

    Ok(ScmOverviewResult::new(overview))
}

fn build_branch_tracking_summary<'a>(
    descriptor: &RepositoryDescriptor,
    reference_names: impl IntoIterator<Item = &'a str>,
    head_commit: Option<ScmHeadCommitSummary>,
) -> ScmBranchTrackingSummary {
    let reference_names = reference_names
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let current_branch_ref_present = descriptor
        .current_branch
        .as_ref()
        .map(|branch| reference_names.contains(&format!("refs/heads/{branch}")));
    let upstream_branch_ref_present = descriptor
        .upstream_branch
        .as_ref()
        .map(|branch| reference_names.contains(&format!("refs/remotes/{branch}")));

    ScmBranchTrackingSummary::new(
        descriptor.head_reference.clone(),
        descriptor.head_oid.clone(),
        descriptor.current_branch.clone(),
        current_branch_ref_present,
        descriptor.upstream_branch.clone(),
        upstream_branch_ref_present,
        descriptor.ahead,
        descriptor.behind,
        head_commit,
    )
}

fn try_collect_head_commit_summary(request: &ScmOverviewRequest) -> Option<ScmHeadCommitSummary> {
    match execute_query_lifecycle_operation(&QueryLifecycleRequest {
        repository_path: request.repository_path.clone(),
        operation: QueryLifecycleOperation::Log {
            revision_range: None,
            max_count: 1,
        },
    }) {
        Ok(result) => result.commits.into_iter().next().map(|commit| {
            ScmHeadCommitSummary::new(
                commit.oid,
                commit.summary,
                commit.author_name,
                commit.author_email,
                commit.timestamp_seconds,
                commit.parent_count,
            )
        }),
        Err(error) => {
            tracing::debug!(
                operation = "scm_overview",
                requested_path = %request.repository_path.display(),
                error_code = %error.code(),
                "head commit summary is unavailable; degrading scm_overview.head_commit to None"
            );
            None
        }
    }
}

fn try_collect_stash_summary(repository: &mut Repository) -> Option<ScmStashSummary> {
    if repository.is_bare() || repository.workdir().is_none() {
        return None;
    }

    let mut total_count = 0_usize;
    let mut latest = None;
    if let Err(error) = repository.stash_foreach(|index, message, oid| {
        total_count += 1;
        if latest.is_none() {
            latest = Some(ScmStashEntrySummary::new(
                index,
                oid.to_string(),
                message.to_string(),
            ));
        }
        true
    }) {
        tracing::debug!(
            operation = "scm_overview",
            error = %error,
            "stash summary is unavailable; degrading scm_overview.stash to None"
        );
        return None;
    }

    Some(ScmStashSummary::new(total_count, latest))
}

fn try_collect_worktree_summary(repository: &Repository) -> Option<ScmWorktreeSummary> {
    if repository.is_bare() || repository.workdir().is_none() {
        return None;
    }

    let worktree_names = match repository.worktrees() {
        Ok(worktree_names) => worktree_names,
        Err(error) => {
            tracing::debug!(
                operation = "scm_overview",
                error = %error,
                "worktree summary is unavailable; degrading scm_overview.worktree to None"
            );
            return None;
        }
    };

    let mut linked_count = 0_usize;
    let mut locked_count = 0_usize;
    for worktree_name in worktree_names.iter().flatten() {
        let worktree = match repository.find_worktree(worktree_name) {
            Ok(worktree) => worktree,
            Err(error) => {
                tracing::debug!(
                    operation = "scm_overview",
                    worktree_name,
                    error = %error,
                    "worktree summary is unavailable; degrading scm_overview.worktree to None"
                );
                return None;
            }
        };
        linked_count += 1;
        match worktree.is_locked() {
            Ok(git2::WorktreeLockStatus::Unlocked) => {}
            Ok(git2::WorktreeLockStatus::Locked(_)) => locked_count += 1,
            Err(error) => {
                tracing::debug!(
                    operation = "scm_overview",
                    worktree_name,
                    error = %error,
                    "worktree summary is unavailable; degrading scm_overview.worktree to None"
                );
                return None;
            }
        }
    }

    Some(ScmWorktreeSummary::new(true, linked_count, locked_count))
}

fn try_collect_submodule_summary(repository: &Repository) -> Option<ScmSubmoduleSummary> {
    if repository.is_bare() || repository.workdir().is_none() {
        return None;
    }

    let submodules = match repository.submodules() {
        Ok(submodules) => submodules,
        Err(error) => {
            tracing::debug!(
                operation = "scm_overview",
                error = %error,
                "submodule summary is unavailable; degrading scm_overview.submodules to None"
            );
            return None;
        }
    };

    let total_count = submodules.len();
    let initialized_count = submodules
        .iter()
        .filter(|submodule| submodule.open().is_ok())
        .count();

    Some(ScmSubmoduleSummary::new(total_count, initialized_count))
}


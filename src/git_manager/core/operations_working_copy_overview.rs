//! `working_copy_overview` operation for `GitManager`.

#![allow(clippy::struct_field_names)]

use crate::git_manager::core::operations_repository_descriptor::execute_repository_descriptor_operation;
use crate::git_manager::core::operations_working_copy_status::execute_working_copy_status_operation;
use crate::git_manager::core::{
    RepositoryDescriptor, RepositoryDescriptorRequest, WorkingCopyEntry, WorkingCopyEntryKind,
    WorkingCopyOverview, WorkingCopyOverviewRequest, WorkingCopyOverviewResult, WorkingCopyScope,
    WorkingCopyStatusRequest,
};
use crate::git_manager::GitResult;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct WorkingCopyOverviewSummary {
    staged_count: usize,
    unstaged_count: usize,
    untracked_count: usize,
    ignored_count: usize,
    conflicted_count: usize,
}

impl WorkingCopyOverviewSummary {
    fn collect(entries: &[WorkingCopyEntry]) -> Self {
        let mut summary = Self::default();

        for entry in entries {
            summary.record(entry);
        }

        summary
    }

    fn record(&mut self, entry: &WorkingCopyEntry) {
        match &entry.kind {
            WorkingCopyEntryKind::Tracked {
                index,
                worktree,
                conflicted,
                ..
            } => {
                self.staged_count += usize::from(index.is_some());
                self.unstaged_count += usize::from(worktree.is_some());
                self.conflicted_count += usize::from(*conflicted);
            }
            WorkingCopyEntryKind::Untracked => {
                self.untracked_count += 1;
            }
            WorkingCopyEntryKind::Ignored => {
                self.ignored_count += 1;
            }
        }
    }

    fn into_overview(self, repository: RepositoryDescriptor) -> WorkingCopyOverview {
        WorkingCopyOverview::new(
            repository,
            self.staged_count,
            self.unstaged_count,
            self.untracked_count,
            self.ignored_count,
            self.conflicted_count,
        )
    }
}

/// Assembles a typed read-only overview of the working tree on top of the descriptor/status operations.
///
/// # Errors
/// Returns a typed `GitError` if discovery or reading the working-copy snapshot
/// fails.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "working_copy_overview",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            staged_count = tracing::field::Empty,
            unstaged_count = tracing::field::Empty,
            untracked_count = tracing::field::Empty,
            ignored_count = tracing::field::Empty,
            conflicted_count = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_working_copy_overview_operation(
    request: &WorkingCopyOverviewRequest,
) -> GitResult<WorkingCopyOverviewResult> {
    let started_at = Instant::now();

    let repository = execute_repository_descriptor_operation(&RepositoryDescriptorRequest {
        repository_path: request.repository_path.clone(),
    })?
    .repository;

    let snapshot = execute_working_copy_status_operation(&WorkingCopyStatusRequest::new(
        request.repository_path.clone(),
        WorkingCopyScope::full(),
        true,
        true,
        true,
        true,
        true,
    ))?;

    let overview = WorkingCopyOverviewSummary::collect(&snapshot.entries).into_overview(repository);
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(overview.repository.repo_root.display()),
    );
    tracing::Span::current().record(
        "staged_count",
        tracing::field::display(overview.staged_count),
    );
    tracing::Span::current().record(
        "unstaged_count",
        tracing::field::display(overview.unstaged_count),
    );
    tracing::Span::current().record(
        "untracked_count",
        tracing::field::display(overview.untracked_count),
    );
    tracing::Span::current().record(
        "ignored_count",
        tracing::field::display(overview.ignored_count),
    );
    tracing::Span::current().record(
        "conflicted_count",
        tracing::field::display(overview.conflicted_count),
    );
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "working_copy_overview",
        requested_path = %request.repository_path.display(),
        repo_root = %overview.repository.repo_root.display(),
        current_branch = ?overview.repository.current_branch,
        upstream_branch = ?overview.repository.upstream_branch,
        ahead = ?overview.repository.ahead,
        behind = ?overview.repository.behind,
        staged_count = overview.staged_count,
        unstaged_count = overview.unstaged_count,
        untracked_count = overview.untracked_count,
        ignored_count = overview.ignored_count,
        conflicted_count = overview.conflicted_count,
        elapsed_ms,
        "assembled typed working copy overview from descriptor and snapshot operations"
    );

    Ok(WorkingCopyOverviewResult::new(overview))
}


//! Contracts of the higher-level read-only `scm_overview` domain.

#![allow(clippy::too_many_arguments)]

use super::WorkingCopyOverview;
use std::path::PathBuf;

/// Request to read a typed SCM overview for a future SCM UI.
///
/// The contract remains a read-only boundary and does not mix the summary surface with mutating
/// Git operations. If part of the higher-level summary cannot be computed reliably, the
/// corresponding result field must remain `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmOverviewRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
}

impl ScmOverviewRequest {
    /// Creates a typed request to read the higher-level SCM overview.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(repository_path: PathBuf) -> Self {
        Self { repository_path }
    }
}

/// Typed higher-level read model for a future SCM UI.
///
/// The model aggregates the canonical `working_copy_overview` and additional typed summaries for
/// the branch tracking, stash, worktree, and submodule surfaces without stringly parsing raw
/// operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmOverview {
    /// Base typed working-tree overview with the canonical repository summary and counters.
    pub working_copy: WorkingCopyOverview,
    /// Higher-level typed summary of branch tracking metadata.
    pub branch_tracking: ScmBranchTrackingSummary,
    /// Typed summary of the stash stack, or `None` if it cannot be computed reliably.
    pub stash: Option<ScmStashSummary>,
    /// Typed summary of linked worktree metadata, or `None` if it cannot be computed reliably.
    pub worktree: Option<ScmWorktreeSummary>,
    /// Typed summary of the submodule inventory, or `None` if it cannot be computed reliably.
    pub submodules: Option<ScmSubmoduleSummary>,
}

impl ScmOverview {
    /// Creates a typed SCM overview on top of `working_copy_overview` and higher-level summary fields.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        working_copy: WorkingCopyOverview,
        branch_tracking: ScmBranchTrackingSummary,
        stash: Option<ScmStashSummary>,
        worktree: Option<ScmWorktreeSummary>,
        submodules: Option<ScmSubmoduleSummary>,
    ) -> Self {
        Self {
            working_copy,
            branch_tracking,
            stash,
            worktree,
            submodules,
        }
    }
}

/// Typed summary of branch/upstream/HEAD metadata for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmBranchTrackingSummary {
    /// Full name of the symbolic HEAD reference, if HEAD is attached to a branch.
    pub head_reference: Option<String>,
    /// OID of the current HEAD, if available.
    pub head_oid: Option<String>,
    /// Short name of the current local branch, if HEAD is attached.
    pub current_branch: Option<String>,
    /// Whether the current local branch is confirmed by the refs inventory.
    pub current_branch_ref_present: Option<bool>,
    /// Short name of the upstream branch (`origin/main`), if configured.
    pub upstream_branch: Option<String>,
    /// Whether the upstream branch is confirmed by the refs inventory.
    pub upstream_branch_ref_present: Option<bool>,
    /// Number of commits ahead of upstream, if the metadata is available.
    pub ahead: Option<usize>,
    /// Number of commits behind upstream, if the metadata is available.
    pub behind: Option<usize>,
    /// Typed summary of the HEAD commit, or `None` if it cannot be computed reliably.
    pub head_commit: Option<ScmHeadCommitSummary>,
}

impl ScmBranchTrackingSummary {
    /// Creates a typed branch tracking summary for the SCM overview.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        head_reference: Option<String>,
        head_oid: Option<String>,
        current_branch: Option<String>,
        current_branch_ref_present: Option<bool>,
        upstream_branch: Option<String>,
        upstream_branch_ref_present: Option<bool>,
        ahead: Option<usize>,
        behind: Option<usize>,
        head_commit: Option<ScmHeadCommitSummary>,
    ) -> Self {
        Self {
            head_reference,
            head_oid,
            current_branch,
            current_branch_ref_present,
            upstream_branch,
            upstream_branch_ref_present,
            ahead,
            behind,
            head_commit,
        }
    }
}

/// Typed summary of HEAD commit metadata for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmHeadCommitSummary {
    /// Commit OID.
    pub oid: String,
    /// Short commit summary.
    pub summary: String,
    /// Author name.
    pub author_name: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// UNIX timestamp of the commit time.
    pub timestamp_seconds: i64,
    /// Number of commit parents.
    pub parent_count: usize,
}

impl ScmHeadCommitSummary {
    /// Creates a typed summary of HEAD commit metadata.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        oid: String,
        summary: String,
        author_name: Option<String>,
        author_email: Option<String>,
        timestamp_seconds: i64,
        parent_count: usize,
    ) -> Self {
        Self {
            oid,
            summary,
            author_name,
            author_email,
            timestamp_seconds,
            parent_count,
        }
    }
}

/// Typed summary of stash stack metadata for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmStashSummary {
    /// Number of stash entries on the stack.
    pub total_count: usize,
    /// The most recent stash entry, or `None` if the stack is empty.
    pub latest: Option<ScmStashEntrySummary>,
}

impl ScmStashSummary {
    /// Creates a typed summary of stash stack metadata.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(total_count: usize, latest: Option<ScmStashEntrySummary>) -> Self {
        Self {
            total_count,
            latest,
        }
    }
}

/// Typed summary of a single stash entry for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmStashEntrySummary {
    /// Index of the stash entry.
    pub index: usize,
    /// OID of the stash commit.
    pub commit_oid: String,
    /// Message of the stash entry.
    pub message: String,
}

impl ScmStashEntrySummary {
    /// Creates a typed summary of a single stash entry.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(index: usize, commit_oid: String, message: String) -> Self {
        Self {
            index,
            commit_oid,
            message,
        }
    }
}

/// Typed summary of the worktree inventory for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmWorktreeSummary {
    /// Whether the open repository has a main worktree.
    pub main_worktree_present: bool,
    /// Number of linked worktrees known to the repository.
    pub linked_count: usize,
    /// Number of locked linked worktrees.
    pub locked_count: usize,
}

impl ScmWorktreeSummary {
    /// Creates a typed summary of linked worktree metadata.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(main_worktree_present: bool, linked_count: usize, locked_count: usize) -> Self {
        Self {
            main_worktree_present,
            linked_count,
            locked_count,
        }
    }
}

/// Typed summary of the submodule inventory for the SCM overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmSubmoduleSummary {
    /// Total number of submodule entries.
    pub total_count: usize,
    /// Number of submodule entries that could be opened locally.
    pub initialized_count: usize,
}

impl ScmSubmoduleSummary {
    /// Creates a typed summary of the submodule inventory.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(total_count: usize, initialized_count: usize) -> Self {
        Self {
            total_count,
            initialized_count,
        }
    }
}

/// Typed result of the `scm_overview` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmOverviewResult {
    /// Higher-level typed overview for the SCM UI read path.
    pub overview: ScmOverview,
}

impl ScmOverviewResult {
    /// Creates a typed `scm_overview` result.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(overview: ScmOverview) -> Self {
        Self { overview }
    }
}

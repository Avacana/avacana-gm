//! Public `GitManager` facade.
#![allow(clippy::missing_errors_doc)]

use crate::git_manager::composition::GitManagerComponents;
use crate::git_manager::core::{
    AdvancedRequest, AdvancedResult, CloneRequest, CloneResult, CommitRequest, CommitResult,
    CreateBranchRequest, CreateBranchResult, FetchRequest, FetchResult, GitPipeline, GitResult,
    HistoryRewriteRequest, HistoryRewriteResult, LineDiffRequest, LineDiffResult, LsRemoteRequest,
    LsRemoteResult, MergeRequest, MergeResult, PlumbingRequest, PlumbingResult, PullRequest,
    PullResult, PushRequest, PushResult, QueryLifecycleRequest, QueryLifecycleResult, RefsRequest,
    RefsResult, RepositoryDescriptorRequest, RepositoryDescriptorResult, ScmOverviewRequest,
    ScmOverviewResult, StageRequest, StageResult, StatusDiffRequest, StatusDiffResult,
    SwitchBranchRequest, SwitchBranchResult, TagSummariesRequest, TagSummariesResult,
    WorkingCopyOverviewRequest, WorkingCopyOverviewResult, WorkingCopyStatusRequest,
    WorkingCopyStatusResult,
};

#[path = "facade_async.rs"]
mod facade_async;
#[path = "facade_sync.rs"]
mod facade_sync;

/// Unified public contract for domain Git operations.
pub trait GitManager: Send + Sync {
    fn clone_repository(&self, request: &CloneRequest) -> GitResult<CloneResult>;
    fn pull(&self, request: &PullRequest) -> GitResult<PullResult>;
    fn fetch(&self, request: &FetchRequest) -> GitResult<FetchResult>;
    fn ls_remote(&self, request: &LsRemoteRequest) -> GitResult<LsRemoteResult>;
    fn stage(&self, request: &StageRequest) -> GitResult<StageResult>;
    fn commit(&self, request: &CommitRequest) -> GitResult<CommitResult>;
    fn push(&self, request: &PushRequest) -> GitResult<PushResult>;
    fn create_branch(&self, request: &CreateBranchRequest) -> GitResult<CreateBranchResult>;
    fn switch_branch(&self, request: &SwitchBranchRequest) -> GitResult<SwitchBranchResult>;
    fn merge(&self, request: &MergeRequest) -> GitResult<MergeResult>;

    /// Runs a diff-oriented scenario for render/apply/pathspec semantics.
    ///
    /// This method is not a production API for UI/file-tree integrations. For a typed worktree
    /// snapshot and ignored semantics, use [`GitManager::working_copy_status`] and
    /// [`GitManager::repository_descriptor`].
    fn status_diff(&self, request: &StatusDiffRequest) -> GitResult<StatusDiffResult>;

    /// Returns typed single-path diff facts for editor-like consumers.
    ///
    /// The method stays read-only and is intended for gutter/editor-like use cases and other
    /// single-path consumers. It does not replace [`GitManager::working_copy_status`] as the
    /// file-tree snapshot API, nor [`GitManager::status_diff`] as the general
    /// render/apply/pathspec surface.
    fn line_diff(&self, request: &LineDiffRequest) -> GitResult<LineDiffResult>;

    fn history_rewrite(&self, request: &HistoryRewriteRequest) -> GitResult<HistoryRewriteResult>;
    fn refs(&self, request: &RefsRequest) -> GitResult<RefsResult>;
    fn plumbing(&self, request: &PlumbingRequest) -> GitResult<PlumbingResult>;

    /// Runs supplementary advanced operations and returns a diagnostic/utility-only surface.
    ///
    /// The returned [`AdvancedResult`] is intentionally stringly; production integrations must not
    /// use `AdvancedResult.items` as a stable machine-readable read model.
    fn advanced(&self, request: &AdvancedRequest) -> GitResult<AdvancedResult>;

    fn query_lifecycle(&self, request: &QueryLifecycleRequest) -> GitResult<QueryLifecycleResult>;

    /// Reads a typed worktree snapshot without mixing in diff-oriented models.
    fn working_copy_status(
        &self,
        request: &WorkingCopyStatusRequest,
    ) -> GitResult<WorkingCopyStatusResult>;

    /// Runs the canonical discovery/open/describe path for a typed repository descriptor.
    fn repository_descriptor(
        &self,
        request: &RepositoryDescriptorRequest,
    ) -> GitResult<RepositoryDescriptorResult>;

    /// Returns a typed commit-backed summary of all repository tags.
    ///
    /// Only tags that could be peeled to a commit are included. For each tag the result provides
    /// the canonical ref name, short name, target commit oid, and that commit's timestamp.
    fn tag_summaries(&self, request: &TagSummariesRequest) -> GitResult<TagSummariesResult>;

    /// Returns a typed read-only overview of the worktree.
    fn working_copy_overview(
        &self,
        request: &WorkingCopyOverviewRequest,
    ) -> GitResult<WorkingCopyOverviewResult>;

    /// Returns a typed higher-level SCM overview for a future SCM UI.
    ///
    /// The method aggregates `working_copy_overview` along with additional typed summaries for the
    /// branch tracking, stash, worktree, and submodule surfaces. No UI labels, colors, icons, or
    /// presentation-ready strings are returned from the backend.
    fn scm_overview(&self, request: &ScmOverviewRequest) -> GitResult<ScmOverviewResult>;
}

/// Default implementation of the public `GitManager` facade.
#[derive(Debug, Clone)]
pub struct GitManagerFacade {
    pipeline: GitPipeline,
}

const _: fn(&GitManagerFacade, &CloneRequest) -> GitResult<CloneResult> =
    <GitManagerFacade as GitManager>::clone_repository;
const _: fn(&GitManagerFacade, &PullRequest) -> GitResult<PullResult> =
    <GitManagerFacade as GitManager>::pull;
const _: fn(&GitManagerFacade, &FetchRequest) -> GitResult<FetchResult> =
    <GitManagerFacade as GitManager>::fetch;
const _: fn(&GitManagerFacade, &LsRemoteRequest) -> GitResult<LsRemoteResult> =
    <GitManagerFacade as GitManager>::ls_remote;
const _: fn(&GitManagerFacade, &StageRequest) -> GitResult<StageResult> =
    <GitManagerFacade as GitManager>::stage;
const _: fn(&GitManagerFacade, &CommitRequest) -> GitResult<CommitResult> =
    <GitManagerFacade as GitManager>::commit;
const _: fn(&GitManagerFacade, &PushRequest) -> GitResult<PushResult> =
    <GitManagerFacade as GitManager>::push;
const _: fn(&GitManagerFacade, &CreateBranchRequest) -> GitResult<CreateBranchResult> =
    <GitManagerFacade as GitManager>::create_branch;
const _: fn(&GitManagerFacade, &SwitchBranchRequest) -> GitResult<SwitchBranchResult> =
    <GitManagerFacade as GitManager>::switch_branch;
const _: fn(&GitManagerFacade, &MergeRequest) -> GitResult<MergeResult> =
    <GitManagerFacade as GitManager>::merge;
const _: fn(&GitManagerFacade, &StatusDiffRequest) -> GitResult<StatusDiffResult> =
    <GitManagerFacade as GitManager>::status_diff;
const _: fn(&GitManagerFacade, &LineDiffRequest) -> GitResult<LineDiffResult> =
    <GitManagerFacade as GitManager>::line_diff;
const _: fn(&GitManagerFacade, &HistoryRewriteRequest) -> GitResult<HistoryRewriteResult> =
    <GitManagerFacade as GitManager>::history_rewrite;
const _: fn(&GitManagerFacade, &RefsRequest) -> GitResult<RefsResult> =
    <GitManagerFacade as GitManager>::refs;
const _: fn(&GitManagerFacade, &PlumbingRequest) -> GitResult<PlumbingResult> =
    <GitManagerFacade as GitManager>::plumbing;
const _: fn(&GitManagerFacade, &AdvancedRequest) -> GitResult<AdvancedResult> =
    <GitManagerFacade as GitManager>::advanced;
const _: fn(&GitManagerFacade, &QueryLifecycleRequest) -> GitResult<QueryLifecycleResult> =
    <GitManagerFacade as GitManager>::query_lifecycle;
const _: fn(&GitManagerFacade, &WorkingCopyStatusRequest) -> GitResult<WorkingCopyStatusResult> =
    <GitManagerFacade as GitManager>::working_copy_status;
const _: fn(
    &GitManagerFacade,
    &RepositoryDescriptorRequest,
) -> GitResult<RepositoryDescriptorResult> =
    <GitManagerFacade as GitManager>::repository_descriptor;
const _: fn(&GitManagerFacade, &TagSummariesRequest) -> GitResult<TagSummariesResult> =
    <GitManagerFacade as GitManager>::tag_summaries;
const _: fn(
    &GitManagerFacade,
    &WorkingCopyOverviewRequest,
) -> GitResult<WorkingCopyOverviewResult> = <GitManagerFacade as GitManager>::working_copy_overview;
const _: fn(&GitManagerFacade, &ScmOverviewRequest) -> GitResult<ScmOverviewResult> =
    <GitManagerFacade as GitManager>::scm_overview;

impl GitManagerFacade {
    /// Creates the facade from the canonical typed dependency bundle.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "typed_components_bundle",
                legacy_default_ctor_used = false
            )
        )
    )]
    #[must_use]
    pub fn new(components: GitManagerComponents) -> Self {
        Self {
            pipeline: GitPipeline::new(components),
        }
    }

    /// Creates the facade from the legacy default bundle.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "legacy_default_ctor",
                legacy_default_ctor_used = true
            )
        )
    )]
    #[must_use]
    pub fn with_default_lock_manager() -> Self {
        Self::from_legacy_defaults()
    }

    fn from_legacy_defaults() -> Self {
        Self {
            pipeline: GitPipeline::new(GitManagerComponents::production()),
        }
    }
}

impl Default for GitManagerFacade {
    fn default() -> Self {
        Self::from_legacy_defaults()
    }
}

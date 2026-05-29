#![allow(clippy::missing_errors_doc)]

use super::{GitManager, GitManagerFacade};
use crate::git_manager::core::{
    AdvancedRequest, AdvancedResult, CloneRequest, CloneResult, CommitRequest, CommitResult,
    CreateBranchRequest, CreateBranchResult, FetchRequest, FetchResult, GitError, GitErrorCode,
    GitResult, HistoryRewriteRequest, HistoryRewriteResult, LineDiffRequest, LineDiffResult,
    LsRemoteRequest, LsRemoteResult, MergeRequest, MergeResult, PlumbingRequest, PlumbingResult,
    PullRequest, PullResult, PushRequest, PushResult, QueryLifecycleRequest, QueryLifecycleResult,
    RefsRequest, RefsResult, RepositoryDescriptorRequest, RepositoryDescriptorResult,
    ScmOverviewRequest, ScmOverviewResult, StageRequest, StageResult, StatusDiffRequest,
    StatusDiffResult, SwitchBranchRequest, SwitchBranchResult, TagSummariesRequest,
    TagSummariesResult, WorkingCopyOverviewRequest, WorkingCopyOverviewResult,
    WorkingCopyStatusRequest, WorkingCopyStatusResult,
};

impl GitManagerFacade {
    async fn execute_async<T, F>(&self, operation: &str, handler: F) -> GitResult<T>
    where
        T: Send + 'static,
        F: FnOnce(Self) -> GitResult<T> + Send + 'static,
    {
        let facade = self.clone();
        tokio::task::spawn_blocking(move || handler(facade))
            .await
            .map_err(|err| {
                GitError::new(
                    GitErrorCode::Internal,
                    format!("git async operation `{operation}` join failed: {err}"),
                )
            })?
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn clone_repository_async(&self, request: CloneRequest) -> GitResult<CloneResult> {
        self.execute_async("clone", move |facade| facade.clone_repository(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn pull_async(&self, request: PullRequest) -> GitResult<PullResult> {
        self.execute_async("pull", move |facade| facade.pull(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn fetch_async(&self, request: FetchRequest) -> GitResult<FetchResult> {
        self.execute_async("fetch", move |facade| facade.fetch(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn ls_remote_async(&self, request: LsRemoteRequest) -> GitResult<LsRemoteResult> {
        self.execute_async("ls-remote", move |facade| facade.ls_remote(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn stage_async(&self, request: StageRequest) -> GitResult<StageResult> {
        self.execute_async("stage", move |facade| facade.stage(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn commit_async(&self, request: CommitRequest) -> GitResult<CommitResult> {
        self.execute_async("commit", move |facade| facade.commit(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn push_async(&self, request: PushRequest) -> GitResult<PushResult> {
        self.execute_async("push", move |facade| facade.push(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn create_branch_async(
        &self,
        request: CreateBranchRequest,
    ) -> GitResult<CreateBranchResult> {
        self.execute_async("create_branch", move |facade| {
            facade.create_branch(&request)
        })
        .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn switch_branch_async(
        &self,
        request: SwitchBranchRequest,
    ) -> GitResult<SwitchBranchResult> {
        self.execute_async("switch", move |facade| facade.switch_branch(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn merge_async(&self, request: MergeRequest) -> GitResult<MergeResult> {
        self.execute_async("merge", move |facade| facade.merge(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn status_diff_async(
        &self,
        request: StatusDiffRequest,
    ) -> GitResult<StatusDiffResult> {
        self.execute_async("status_diff", move |facade| facade.status_diff(&request))
            .await
    }

    /// Async wrapper around [`GitManager::line_diff`] for read-only single-path diff facts.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn line_diff_async(&self, request: LineDiffRequest) -> GitResult<LineDiffResult> {
        self.execute_async("line_diff", move |facade| facade.line_diff(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn history_rewrite_async(
        &self,
        request: HistoryRewriteRequest,
    ) -> GitResult<HistoryRewriteResult> {
        self.execute_async("history_rewrite", move |facade| {
            facade.history_rewrite(&request)
        })
        .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn refs_async(&self, request: RefsRequest) -> GitResult<RefsResult> {
        self.execute_async("refs", move |facade| facade.refs(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn plumbing_async(&self, request: PlumbingRequest) -> GitResult<PlumbingResult> {
        self.execute_async("plumbing", move |facade| facade.plumbing(&request))
            .await
    }

    /// Async wrapper around [`GitManager::advanced`] for diagnostic/utility-only operations.
    ///
    /// `AdvancedResult.items` remains a supplementary stringly surface and must not be used
    /// as a production machine-readable contract.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn advanced_async(&self, request: AdvancedRequest) -> GitResult<AdvancedResult> {
        self.execute_async("advanced", move |facade| facade.advanced(&request))
            .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn query_lifecycle_async(
        &self,
        request: QueryLifecycleRequest,
    ) -> GitResult<QueryLifecycleResult> {
        self.execute_async("query_lifecycle", move |facade| {
            facade.query_lifecycle(&request)
        })
        .await
    }

    /// Async wrapper around [`GitManager::working_copy_status`] for read-only snapshot refresh.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn working_copy_status_async(
        &self,
        request: WorkingCopyStatusRequest,
    ) -> GitResult<WorkingCopyStatusResult> {
        self.execute_async("working_copy_status", move |facade| {
            facade.working_copy_status(&request)
        })
        .await
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn repository_descriptor_async(
        &self,
        request: RepositoryDescriptorRequest,
    ) -> GitResult<RepositoryDescriptorResult> {
        self.execute_async("repository_descriptor", move |facade| {
            facade.repository_descriptor(&request)
        })
        .await
    }

    /// Async wrapper around [`GitManager::tag_summaries`] for typed tag metadata.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn tag_summaries_async(
        &self,
        request: TagSummariesRequest,
    ) -> GitResult<TagSummariesResult> {
        self.execute_async("tag_summaries", move |facade| {
            facade.tag_summaries(&request)
        })
        .await
    }

    /// Async wrapper around [`GitManager::working_copy_overview`] for the read-only overview path.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn working_copy_overview_async(
        &self,
        request: WorkingCopyOverviewRequest,
    ) -> GitResult<WorkingCopyOverviewResult> {
        self.execute_async("working_copy_overview", move |facade| {
            facade.working_copy_overview(&request)
        })
        .await
    }

    /// Async wrapper around [`GitManager::scm_overview`] for the higher-level SCM read path.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub async fn scm_overview_async(
        &self,
        request: ScmOverviewRequest,
    ) -> GitResult<ScmOverviewResult> {
        self.execute_async("scm_overview", move |facade| facade.scm_overview(&request))
            .await
    }
}

use super::{GitManager, GitManagerFacade};
use crate::git_manager::core::{
    AdvancedRequest, AdvancedResult, CloneRequest, CloneResult, CommitRequest, CommitResult,
    CreateBranchRequest, CreateBranchResult, FetchRequest, FetchResult, GitResult,
    HistoryRewriteRequest, HistoryRewriteResult, LineDiffRequest, LineDiffResult, LsRemoteRequest,
    LsRemoteResult, MergeRequest, MergeResult, PlumbingRequest, PlumbingResult, PullRequest,
    PullResult, PushRequest, PushResult, QueryLifecycleRequest, QueryLifecycleResult, RefsRequest,
    RefsResult, RepositoryDescriptorRequest, RepositoryDescriptorResult, ScmOverviewRequest,
    ScmOverviewResult, StageRequest, StageResult, StatusDiffRequest, StatusDiffResult,
    SwitchBranchRequest, SwitchBranchResult, TagSummariesRequest, TagSummariesResult,
    WorkingCopyOverviewRequest, WorkingCopyOverviewResult, WorkingCopyStatusRequest,
    WorkingCopyStatusResult,
};

impl GitManager for GitManagerFacade {
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn clone_repository(&self, request: &CloneRequest) -> GitResult<CloneResult> {
        self.pipeline.clone_repository(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn pull(&self, request: &PullRequest) -> GitResult<PullResult> {
        self.pipeline.pull(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn fetch(&self, request: &FetchRequest) -> GitResult<FetchResult> {
        self.pipeline.fetch(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn ls_remote(&self, request: &LsRemoteRequest) -> GitResult<LsRemoteResult> {
        self.pipeline.ls_remote(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn stage(&self, request: &StageRequest) -> GitResult<StageResult> {
        self.pipeline.stage(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn commit(&self, request: &CommitRequest) -> GitResult<CommitResult> {
        self.pipeline.commit(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn push(&self, request: &PushRequest) -> GitResult<PushResult> {
        self.pipeline.push(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn create_branch(&self, request: &CreateBranchRequest) -> GitResult<CreateBranchResult> {
        self.pipeline.create_branch(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn switch_branch(&self, request: &SwitchBranchRequest) -> GitResult<SwitchBranchResult> {
        self.pipeline.switch_branch(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn merge(&self, request: &MergeRequest) -> GitResult<MergeResult> {
        self.pipeline.merge(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn status_diff(&self, request: &StatusDiffRequest) -> GitResult<StatusDiffResult> {
        self.pipeline.status_diff(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn line_diff(&self, request: &LineDiffRequest) -> GitResult<LineDiffResult> {
        self.pipeline.line_diff(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn history_rewrite(&self, request: &HistoryRewriteRequest) -> GitResult<HistoryRewriteResult> {
        self.pipeline.history_rewrite(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn refs(&self, request: &RefsRequest) -> GitResult<RefsResult> {
        self.pipeline.refs(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn plumbing(&self, request: &PlumbingRequest) -> GitResult<PlumbingResult> {
        self.pipeline.plumbing(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn advanced(&self, request: &AdvancedRequest) -> GitResult<AdvancedResult> {
        self.pipeline.advanced(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn query_lifecycle(&self, request: &QueryLifecycleRequest) -> GitResult<QueryLifecycleResult> {
        self.pipeline.query_lifecycle(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn working_copy_status(
        &self,
        request: &WorkingCopyStatusRequest,
    ) -> GitResult<WorkingCopyStatusResult> {
        self.pipeline.working_copy_status(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn repository_descriptor(
        &self,
        request: &RepositoryDescriptorRequest,
    ) -> GitResult<RepositoryDescriptorResult> {
        self.pipeline.repository_descriptor(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn tag_summaries(&self, request: &TagSummariesRequest) -> GitResult<TagSummariesResult> {
        self.pipeline.tag_summaries(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn working_copy_overview(
        &self,
        request: &WorkingCopyOverviewRequest,
    ) -> GitResult<WorkingCopyOverviewResult> {
        self.pipeline.working_copy_overview(request)
    }

    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    fn scm_overview(&self, request: &ScmOverviewRequest) -> GitResult<ScmOverviewResult> {
        self.pipeline.scm_overview(request)
    }
}

//! Internal pipeline of the `GitManager` framework.

#![allow(clippy::unused_self)]

use crate::git_manager::composition::GitManagerComponents;
use crate::git_manager::core::operations_advanced::execute_advanced_operation;
use crate::git_manager::core::operations_branch::execute_create_branch_operation;
use crate::git_manager::core::operations_clone_fetch_push::{
    execute_clone_operation, execute_ls_remote_operation, execute_public_fetch_operation,
};
use crate::git_manager::core::operations_commit::execute_commit_operation;
use crate::git_manager::core::operations_history_rewrite::execute_history_rewrite_operation;
use crate::git_manager::core::operations_line_diff::execute_line_diff_operation;
use crate::git_manager::core::operations_merge::execute_merge_operation;
use crate::git_manager::core::operations_plumbing::execute_plumbing_operation;
use crate::git_manager::core::operations_pull::execute_pull_operation;
use crate::git_manager::core::operations_push::execute_push_operation;
use crate::git_manager::core::operations_query_lifecycle::execute_query_lifecycle_operation;
use crate::git_manager::core::operations_refs_meta::execute_refs_operation;
use crate::git_manager::core::operations_repository_descriptor::execute_repository_descriptor_operation;
use crate::git_manager::core::operations_scm_overview::execute_scm_overview_operation;
use crate::git_manager::core::operations_stage::execute_stage_operation;
use crate::git_manager::core::operations_status_diff::execute_status_diff_operation;
use crate::git_manager::core::operations_switch::execute_switch_operation;
use crate::git_manager::core::operations_tags::execute_tag_summaries_operation;
use crate::git_manager::core::operations_working_copy_overview::execute_working_copy_overview_operation;
use crate::git_manager::core::operations_working_copy_status::execute_working_copy_status_operation;
use crate::git_manager::core::{
    AdvancedRequest, AdvancedResult, CloneRequest, CloneResult, CommitRequest, CommitResult,
    CreateBranchRequest, CreateBranchResult, FetchRequest, FetchResult, GitError, GitErrorCode,
    GitErrorRetryClassification, GitResult, HistoryRewriteRequest, HistoryRewriteResult,
    LineDiffRequest, LineDiffResult, LsRemoteRequest, LsRemoteResult, MergeRequest, MergeResult,
    PlumbingRequest, PlumbingResult, PullRequest, PullResult, PushRequest, PushResult,
    QueryLifecycleRequest, QueryLifecycleResult, RefsRequest, RefsResult,
    RepositoryDescriptorRequest, RepositoryDescriptorResult, ScmOverviewRequest, ScmOverviewResult,
    StageRequest, StageResult, StatusDiffRequest, StatusDiffResult, SwitchBranchRequest,
    SwitchBranchResult, TagSummariesRequest, TagSummariesResult, WorkingCopyOverviewRequest,
    WorkingCopyOverviewResult, WorkingCopyScope, WorkingCopyStatusRequest, WorkingCopyStatusResult,
};
use crate::git_manager::state::{GitLockGuard, GitLockMode};
use std::path::Path;

const RETRY_STRATEGY_RETRY_ONCE: &str = "retry_once";
const RETRY_STRATEGY_ESCALATE_TO_FULL_REFRESH: &str = "escalate_to_full_refresh";

#[derive(Debug, Clone)]
pub struct GitPipeline {
    components: GitManagerComponents,
}

impl GitPipeline {
    #[must_use]
    pub const fn new(components: GitManagerComponents) -> Self {
        Self { components }
    }

    pub(crate) fn clone_repository(&self, request: &CloneRequest) -> GitResult<CloneResult> {
        let _lock_guard =
            self.prepare_access("clone", &request.destination_path, GitLockMode::Mutating)?;
        let transport_bridge = self.components.transport_factory().bridge();
        execute_clone_operation(request, &transport_bridge)
    }

    pub(crate) fn pull(&self, request: &PullRequest) -> GitResult<PullResult> {
        let _lock_guard =
            self.prepare_access("pull", &request.repository_path, GitLockMode::Mutating)?;
        let transport_bridge = self.components.transport_factory().bridge();
        execute_pull_operation(request, &transport_bridge)
    }

    pub(crate) fn fetch(&self, request: &FetchRequest) -> GitResult<FetchResult> {
        let _lock_guard =
            self.prepare_access("fetch", &request.repository_path, GitLockMode::Mutating)?;
        let transport_bridge = self.components.transport_factory().bridge();
        execute_public_fetch_operation(request, &transport_bridge)
    }

    pub(crate) fn ls_remote(&self, request: &LsRemoteRequest) -> GitResult<LsRemoteResult> {
        let _lock_guard =
            self.prepare_access("ls_remote", &request.repository_path, GitLockMode::Mutating)?;
        let transport_bridge = self.components.transport_factory().bridge();
        execute_ls_remote_operation(request, &transport_bridge)
    }

    pub(crate) fn stage(&self, request: &StageRequest) -> GitResult<StageResult> {
        let _lock_guard =
            self.prepare_access("stage", &request.repository_path, GitLockMode::Mutating)?;
        execute_stage_operation(request)
    }

    pub(crate) fn commit(&self, request: &CommitRequest) -> GitResult<CommitResult> {
        let _lock_guard =
            self.prepare_access("commit", &request.repository_path, GitLockMode::Mutating)?;
        execute_commit_operation(request)
    }

    pub(crate) fn push(&self, request: &PushRequest) -> GitResult<PushResult> {
        let _lock_guard =
            self.prepare_access("push", &request.repository_path, GitLockMode::Mutating)?;
        let transport_bridge = self.components.transport_factory().bridge();
        execute_push_operation(request, &transport_bridge)
    }

    pub(crate) fn create_branch(
        &self,
        request: &CreateBranchRequest,
    ) -> GitResult<CreateBranchResult> {
        let _lock_guard = self.prepare_access(
            "create_branch",
            &request.repository_path,
            GitLockMode::Mutating,
        )?;
        execute_create_branch_operation(request)
    }

    pub(crate) fn switch_branch(
        &self,
        request: &SwitchBranchRequest,
    ) -> GitResult<SwitchBranchResult> {
        let _lock_guard =
            self.prepare_access("switch", &request.repository_path, GitLockMode::Mutating)?;
        execute_switch_operation(request)
    }

    pub(crate) fn merge(&self, request: &MergeRequest) -> GitResult<MergeResult> {
        let _lock_guard =
            self.prepare_access("merge", &request.repository_path, GitLockMode::Mutating)?;
        execute_merge_operation(request)
    }

    pub(crate) fn status_diff(&self, request: &StatusDiffRequest) -> GitResult<StatusDiffResult> {
        let _lock_guard = self.prepare_access(
            "status_diff",
            &request.repository_path,
            GitLockMode::Mutating,
        )?;
        execute_status_diff_operation(request)
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                operation = "line_diff",
                access_mode = "read_only",
                repository_path = %request.repository_path.display(),
                target_path = %request.target_path.display(),
                normalized_target_path = tracing::field::Empty,
                baseline_kind = tracing::field::Empty,
                binary = tracing::field::Empty,
                hunk_count = tracing::field::Empty,
                line_count = tracing::field::Empty
            )
        )
    )]
    pub(crate) fn line_diff(&self, request: &LineDiffRequest) -> GitResult<LineDiffResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard =
            self.prepare_access("line_diff", &request.repository_path, GitLockMode::ReadOnly)?;

        ensure_non_empty_target_path("line_diff", &request.target_path)?;

        let execution = execute_line_diff_operation(request)?;
        tracing::Span::current().record(
            "normalized_target_path",
            tracing::field::display(execution.normalized_target_path.as_str()),
        );
        tracing::Span::current().record(
            "baseline_kind",
            tracing::field::display(execution.baseline_kind),
        );
        tracing::Span::current().record("binary", tracing::field::display(execution.binary));
        tracing::Span::current()
            .record("hunk_count", tracing::field::display(execution.hunk_count));
        tracing::Span::current()
            .record("line_count", tracing::field::display(execution.line_count));

        Ok(execution.result)
    }

    pub(crate) fn history_rewrite(
        &self,
        request: &HistoryRewriteRequest,
    ) -> GitResult<HistoryRewriteResult> {
        let _lock_guard = self.prepare_access(
            "history_rewrite",
            &request.repository_path,
            GitLockMode::Mutating,
        )?;
        execute_history_rewrite_operation(request)
    }

    pub(crate) fn refs(&self, request: &RefsRequest) -> GitResult<RefsResult> {
        let _lock_guard =
            self.prepare_access("refs", &request.repository_path, GitLockMode::Mutating)?;
        execute_refs_operation(request)
    }

    pub(crate) fn plumbing(&self, request: &PlumbingRequest) -> GitResult<PlumbingResult> {
        let _lock_guard =
            self.prepare_access("plumbing", &request.repository_path, GitLockMode::Mutating)?;
        execute_plumbing_operation(request)
    }

    pub(crate) fn advanced(&self, request: &AdvancedRequest) -> GitResult<AdvancedResult> {
        let _lock_guard =
            self.prepare_access("advanced", &request.repository_path, GitLockMode::Mutating)?;
        execute_advanced_operation(request)
    }

    pub(crate) fn query_lifecycle(
        &self,
        request: &QueryLifecycleRequest,
    ) -> GitResult<QueryLifecycleResult> {
        let _lock_guard = self.prepare_access(
            "query_lifecycle",
            &request.repository_path,
            GitLockMode::Mutating,
        )?;
        execute_query_lifecycle_operation(request)
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                operation = "working_copy_status",
                access_mode = "read_only",
                retry_strategy = tracing::field::Empty
            )
        )
    )]
    pub(crate) fn working_copy_status(
        &self,
        request: &WorkingCopyStatusRequest,
    ) -> GitResult<WorkingCopyStatusResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard = self.prepare_access(
            "working_copy_status",
            &request.repository_path,
            GitLockMode::ReadOnly,
        )?;

        match execute_working_copy_status_operation(request) {
            Ok(result) => Ok(result),
            Err(error) => self.retry_working_copy_status_after_transient_race(request, error),
        }
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(operation = "repository_descriptor", access_mode = "read_only")
        )
    )]
    pub(crate) fn repository_descriptor(
        &self,
        request: &RepositoryDescriptorRequest,
    ) -> GitResult<RepositoryDescriptorResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard = self.prepare_access(
            "repository_descriptor",
            &request.repository_path,
            GitLockMode::ReadOnly,
        )?;
        execute_repository_descriptor_operation(request)
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(operation = "tag_summaries", access_mode = "read_only")
        )
    )]
    pub(crate) fn tag_summaries(
        &self,
        request: &TagSummariesRequest,
    ) -> GitResult<TagSummariesResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard = self.prepare_access(
            "tag_summaries",
            &request.repository_path,
            GitLockMode::ReadOnly,
        )?;
        execute_tag_summaries_operation(request)
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(operation = "working_copy_overview", access_mode = "read_only")
        )
    )]
    pub(crate) fn working_copy_overview(
        &self,
        request: &WorkingCopyOverviewRequest,
    ) -> GitResult<WorkingCopyOverviewResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard = self.prepare_access(
            "working_copy_overview",
            &request.repository_path,
            GitLockMode::ReadOnly,
        )?;
        execute_working_copy_overview_operation(request)
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(operation = "scm_overview", access_mode = "read_only")
        )
    )]
    pub(crate) fn scm_overview(
        &self,
        request: &ScmOverviewRequest,
    ) -> GitResult<ScmOverviewResult> {
        let _repository_access = self.components.repository_access();
        let _lock_guard = self.prepare_access(
            "scm_overview",
            &request.repository_path,
            GitLockMode::ReadOnly,
        )?;
        execute_scm_overview_operation(request)
    }

    fn prepare_access(
        &self,
        operation: &str,
        repository_path: &Path,
        access_mode: GitLockMode,
    ) -> GitResult<GitLockGuard> {
        ensure_non_empty_path(operation, repository_path)?;
        let lock_manager = self.components.lock_manager();
        tracing::trace!(
            operation,
            access_mode = access_mode.as_str(),
            requested_path = %repository_path.display(),
            creates_filesystem_lock = matches!(access_mode, GitLockMode::Mutating),
            "classified git pipeline access mode"
        );
        lock_manager.access(repository_path, access_mode)
    }

    fn retry_working_copy_status_after_transient_race(
        &self,
        request: &WorkingCopyStatusRequest,
        error: GitError,
    ) -> GitResult<WorkingCopyStatusResult> {
        if error.retry_classification() != Some(GitErrorRetryClassification::Retryable) {
            return Err(error);
        }

        let (retry_request, retry_strategy) =
            retry_request_for_transient_working_copy_race(request);
        tracing::Span::current().record("retry_strategy", tracing::field::display(retry_strategy));
        tracing::warn!(
            operation = "working_copy_status",
            access_mode = GitLockMode::ReadOnly.as_str(),
            requested_path = %request.repository_path.display(),
            scope_kind = request.scope.kind_name(),
            retry_strategy,
            error_code = %error.code(),
            retry_classification = %GitErrorRetryClassification::Retryable,
            "working_copy_status hit transient read-only race; retrying within same backend"
        );

        execute_working_copy_status_operation(&retry_request)
    }
}

fn retry_request_for_transient_working_copy_race(
    request: &WorkingCopyStatusRequest,
) -> (WorkingCopyStatusRequest, &'static str) {
    match &request.scope {
        WorkingCopyScope::Full => (request.clone(), RETRY_STRATEGY_RETRY_ONCE),
        WorkingCopyScope::Paths { .. } => {
            let mut retry_request = request.clone();
            retry_request.scope = WorkingCopyScope::full();
            (retry_request, RETRY_STRATEGY_ESCALATE_TO_FULL_REFRESH)
        }
    }
}

fn ensure_non_empty_path(operation: &str, repository_path: &Path) -> GitResult<()> {
    if repository_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRepoPath,
            format!("operation `{operation}` requires a non-empty repository path"),
        ));
    }
    Ok(())
}

fn ensure_non_empty_target_path(operation: &str, target_path: &Path) -> GitResult<()> {
    if target_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::LineDiffInvalidPath,
            format!("operation `{operation}` requires a non-empty target path"),
        ));
    }

    Ok(())
}

//! Contracts and pipeline of the `GitManager` core.

mod contracts;
mod domain_contracts;
mod operations_advanced;
mod operations_advanced_metadata;
mod operations_advanced_support;
mod operations_branch;
mod operations_clone_fetch_push;
mod operations_commit;
mod operations_diff_patch_support;
mod operations_history_rewrite;
mod operations_history_rewrite_support;
mod operations_line_diff;
mod operations_merge;
mod operations_plumbing;
mod operations_plumbing_support;
mod operations_pull;
mod operations_push;
mod operations_push_mirror_support;
mod operations_query_lifecycle;
mod operations_query_lifecycle_support;
mod operations_refs_meta;
mod operations_refs_meta_support;
mod operations_remote_transport_support;
mod operations_repository_descriptor;
mod operations_repository_path_support;
mod operations_scm_overview;
mod operations_stage;
mod operations_status_diff;
mod operations_switch;
mod operations_tags;
mod operations_working_copy_overview;
mod operations_working_copy_status;
pub(crate) mod pipeline;
mod query_lifecycle_contracts;
pub(crate) mod repository_access;

pub use contracts::{
    CloneRequest, CloneResult, CommitRequest, CommitResult, CreateBranchRequest,
    CreateBranchResult, EmptyCommitPolicy, FetchRequest, FetchResult, FetchTagMode,
    ForceWithLeasePolicy, ForceWithLeaseRef, GitError, GitErrorCode, GitErrorRetryClassification,
    GitResult, GitWarning, GitWarningCode, HooksPolicy, LsRemoteRequest, LsRemoteResult,
    MergeFileFavor, MergeRequest, MergeResult, PullMode, PullRequest, PullResult, PushRequest,
    PushResult, RemoteReference, StageRequest, StageResult, SwitchBranchRequest,
    SwitchBranchResult,
};
pub use domain_contracts::{
    normalize_repository_relative_path, AdvancedOperation, AdvancedRequest, AdvancedResult,
    ApplyPatchLocation, ApplyPatchRequest, CherryPickRequest, DiffBinaryDetails,
    DiffBinaryFileDetails, DiffBinaryKindDetails, DiffDeltaDetails, DiffFileDetails,
    DiffFileModeKind, DiffHunkDetails, DiffLineDetails, DiffLineTypeDetails, DiffOutputFormat,
    DiffPatchDetails, DiffSegmentDetails, DiffStatusCode, DiffStatusEntry, DiffSummary,
    HistoryRewriteOperation, HistoryRewriteRequest, HistoryRewriteResult, HistoryRewriteState,
    IndexerProgressSnapshot, LineDiffPayload, LineDiffRequest, LineDiffResult, PackBuildProgress,
    PackBuildStage, PathspecDiffEntryDetails, PathspecMatchDetails, PlumbingOperation,
    PlumbingRequest, PlumbingResult, RebaseAction, RebaseRequest, RefUpdateSpec,
    ReferenceDescriptor, ReferenceKind, ReflogEntry, RefsOperation, RefsRequest, RefsResult,
    RepositoryDescriptor, RepositoryDescriptorRequest, RepositoryDescriptorResult,
    RepositoryPathKind, RevertRequest, ScmBranchTrackingSummary, ScmHeadCommitSummary, ScmOverview,
    ScmOverviewRequest, ScmOverviewResult, ScmStashEntrySummary, ScmStashSummary,
    ScmSubmoduleSummary, ScmWorktreeSummary, StatusDiffDetails, StatusDiffRequest,
    StatusDiffResult, StatusScope, TagSummariesRequest, TagSummariesResult, TagSummary,
    TrackedChangeKind, TreeEntrySpec, WorkingCopyEntry, WorkingCopyEntryKind,
    WorkingCopyEntryOrigin, WorkingCopyOverview, WorkingCopyOverviewRequest,
    WorkingCopyOverviewResult, WorkingCopyScope, WorkingCopyStatusRequest, WorkingCopyStatusResult,
};
pub(crate) use pipeline::GitPipeline;
pub use query_lifecycle_contracts::{
    QueryBlameHunk, QueryChangeEntry, QueryCommitSummary, QueryConfigValue,
    QueryLifecycleOperation, QueryLifecycleRequest, QueryLifecycleResult, QueryShortlogEntry,
    QueryTreeEntry, QueryUnsupportedClassification, QueryVersionInfo,
};
pub use repository_access::RepositoryAccess;

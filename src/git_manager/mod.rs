//! `GitManager` framework for embedded Git operations.

pub mod auth;
pub mod composition;
pub mod core;
pub mod diagnostics;
pub mod facade;
pub mod state;
pub mod transport;

pub use composition::{
    GitAuthProviderFactory, GitManagerComponents, GitTransportFactory, RepositoryAccess,
};
pub use core::{
    normalize_repository_relative_path, AdvancedOperation, AdvancedRequest, AdvancedResult,
    ApplyPatchLocation, ApplyPatchRequest, CherryPickRequest, CloneRequest, CloneResult,
    CommitRequest, CommitResult, CreateBranchRequest, CreateBranchResult, DiffFileModeKind,
    DiffOutputFormat, DiffStatusCode, DiffStatusEntry, DiffSummary, EmptyCommitPolicy,
    FetchRequest, FetchResult, FetchTagMode, ForceWithLeasePolicy, ForceWithLeaseRef, GitError,
    GitErrorCode, GitErrorRetryClassification, GitResult, GitWarning, GitWarningCode,
    HistoryRewriteOperation, HistoryRewriteRequest, HistoryRewriteResult, HistoryRewriteState,
    HooksPolicy, IndexerProgressSnapshot, LineDiffPayload, LineDiffRequest, LineDiffResult,
    LsRemoteRequest, LsRemoteResult, MergeFileFavor, MergeRequest, MergeResult, PackBuildProgress,
    PackBuildStage, PlumbingOperation, PlumbingRequest, PlumbingResult, PullMode, PullRequest,
    PullResult, PushRequest, PushResult, QueryBlameHunk, QueryChangeEntry, QueryCommitSummary,
    QueryConfigValue, QueryLifecycleOperation, QueryLifecycleRequest, QueryLifecycleResult,
    QueryShortlogEntry, QueryTreeEntry, QueryUnsupportedClassification, QueryVersionInfo,
    RebaseAction, RebaseRequest, RefUpdateSpec, ReferenceDescriptor, ReferenceKind, ReflogEntry,
    RefsOperation, RefsRequest, RefsResult, RemoteReference, RepositoryDescriptor,
    RepositoryDescriptorRequest, RepositoryDescriptorResult, RepositoryPathKind, RevertRequest,
    ScmBranchTrackingSummary, ScmHeadCommitSummary, ScmOverview, ScmOverviewRequest,
    ScmOverviewResult, ScmStashEntrySummary, ScmStashSummary, ScmSubmoduleSummary,
    ScmWorktreeSummary, StageRequest, StageResult, StatusDiffRequest, StatusDiffResult,
    StatusScope, SwitchBranchRequest, SwitchBranchResult, TagSummariesRequest, TagSummariesResult,
    TagSummary, TrackedChangeKind, TreeEntrySpec, WorkingCopyEntry, WorkingCopyEntryKind,
    WorkingCopyEntryOrigin, WorkingCopyOverview, WorkingCopyOverviewRequest,
    WorkingCopyOverviewResult, WorkingCopyScope, WorkingCopyStatusRequest, WorkingCopyStatusResult,
};
pub use facade::{GitManager, GitManagerFacade};
pub use state::{GitLockGuard, GitLockManager, GitLockMode};

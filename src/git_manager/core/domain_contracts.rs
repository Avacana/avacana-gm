//! Domain contracts of the extended public `GitManager` API.

#[path = "domain_contracts_advanced.rs"]
mod domain_contracts_advanced;
#[path = "domain_contracts_history_rewrite.rs"]
mod domain_contracts_history_rewrite;
#[path = "domain_contracts_line_diff.rs"]
mod domain_contracts_line_diff;
#[path = "domain_contracts_plumbing.rs"]
mod domain_contracts_plumbing;
#[path = "domain_contracts_refs_meta.rs"]
mod domain_contracts_refs_meta;
#[path = "domain_contracts_repository.rs"]
mod domain_contracts_repository;
#[path = "domain_contracts_scm_overview.rs"]
mod domain_contracts_scm_overview;
#[path = "domain_contracts_status_diff.rs"]
mod domain_contracts_status_diff;
#[path = "domain_contracts_tags.rs"]
mod domain_contracts_tags;
#[path = "domain_contracts_working_copy.rs"]
mod domain_contracts_working_copy;

pub use domain_contracts_advanced::{AdvancedOperation, AdvancedRequest, AdvancedResult};
pub use domain_contracts_history_rewrite::{
    CherryPickRequest, HistoryRewriteOperation, HistoryRewriteRequest, HistoryRewriteResult,
    HistoryRewriteState, RebaseAction, RebaseRequest, RevertRequest,
};
pub use domain_contracts_line_diff::{LineDiffPayload, LineDiffRequest, LineDiffResult};
pub use domain_contracts_plumbing::{
    IndexerProgressSnapshot, PackBuildProgress, PackBuildStage, PlumbingOperation, PlumbingRequest,
    PlumbingResult, TreeEntrySpec,
};
pub use domain_contracts_refs_meta::{
    RefUpdateSpec, ReferenceDescriptor, ReferenceKind, ReflogEntry, RefsOperation, RefsRequest,
    RefsResult,
};
pub use domain_contracts_repository::{
    RepositoryDescriptor, RepositoryDescriptorRequest, RepositoryDescriptorResult,
};
pub use domain_contracts_scm_overview::{
    ScmBranchTrackingSummary, ScmHeadCommitSummary, ScmOverview, ScmOverviewRequest,
    ScmOverviewResult, ScmStashEntrySummary, ScmStashSummary, ScmSubmoduleSummary,
    ScmWorktreeSummary,
};
pub use domain_contracts_status_diff::{
    ApplyPatchLocation, ApplyPatchRequest, DiffBinaryDetails, DiffBinaryFileDetails,
    DiffBinaryKindDetails, DiffDeltaDetails, DiffFileDetails, DiffFileModeKind, DiffHunkDetails,
    DiffLineDetails, DiffLineTypeDetails, DiffOutputFormat, DiffPatchDetails, DiffSegmentDetails,
    DiffStatusCode, DiffStatusEntry, DiffSummary, PathspecDiffEntryDetails, PathspecMatchDetails,
    StatusDiffDetails, StatusDiffRequest, StatusDiffResult, StatusScope,
};
pub use domain_contracts_tags::{TagSummariesRequest, TagSummariesResult, TagSummary};
pub use domain_contracts_working_copy::{
    normalize_repository_relative_path, RepositoryPathKind, TrackedChangeKind, WorkingCopyEntry,
    WorkingCopyEntryKind, WorkingCopyEntryOrigin, WorkingCopyOverview, WorkingCopyOverviewRequest,
    WorkingCopyOverviewResult, WorkingCopyScope, WorkingCopyStatusRequest, WorkingCopyStatusResult,
};

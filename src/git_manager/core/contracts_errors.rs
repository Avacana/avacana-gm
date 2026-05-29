//! Error and warning types of the public `GitManager` API.

use std::fmt;

/// A `GitManager`-level error with a machine-readable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    code: GitErrorCode,
    message: String,
    retry_classification: Option<GitErrorRetryClassification>,
}

impl GitError {
    /// Creates a typed `GitManager` error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(code: GitErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_classification: None,
        }
    }

    /// Returns the machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> GitErrorCode {
        self.code
    }

    /// Attaches a retryability classification to the typed error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_retry_classification(
        mut self,
        retry_classification: GitErrorRetryClassification,
    ) -> Self {
        self.retry_classification = Some(retry_classification);
        self
    }

    /// Returns the error's retryability classification, if the domain assigned one.
    #[must_use]
    pub const fn retry_classification(&self) -> Option<GitErrorRetryClassification> {
        self.retry_classification
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for GitError {}

/// Machine-readable `GitManager` error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitErrorCode {
    /// The repository path is empty or invalid at the repository input level.
    InvalidRepoPath,
    /// The scoped path/pathspec is empty, contains a NUL, or escapes `repo_root`.
    InvalidPathspec,
    LockContention,
    LockIo,
    Internal,
    NotImplemented,
    StatusDiffNotSupportedYet,
    StatusDiffFailed,
    StatusDiffInvalidPathspec,
    LineDiffNotSupportedYet,
    LineDiffInvalidPath,
    LineDiffFailed,
    ApplyPatchFailed,
    HistoryRewriteNotSupportedYet,
    RefsNotSupportedYet,
    PlumbingNotSupportedYet,
    PlumbingInvalidInput,
    PlumbingObjectNotFound,
    PlumbingOperationFailed,
    PlumbingPackBuildFailed,
    PlumbingIndexerFailed,
    AdvancedNotSupportedYet,
    AdvancedInvalidInput,
    AdvancedOperationFailed,
    QueryLifecycleFailed,
    QueryLifecycleInvalidInput,
    TagSummariesFailed,
    /// Repository discovery/open/canonicalization failed.
    RepositoryOpenFailed,
    /// Collecting the typed working copy snapshot via `libgit2` failed.
    WorkingCopySnapshotFailed,
    IndexUpdateFailed,
    StagePathspecEmpty,
    CommitMessageEmpty,
    EmptyCommitNotAllowed,
    InvalidSignatureContext,
    MergeInProgress,
    HooksPresent,
    HookDiscoveryFailed,
    PullRebaseNotSupportedYet,
    MergeConflict,
    DetachedHead,
    UpstreamNotFound,
    WorktreeDirty,
    BranchAlreadyExists,
    RefNotFound,
    AuthDenied,
    AuthNoCredentials,
    AuthTimeout,
    AuthUnsupportedSshDirective,
    AuthHostKeyUnknown,
    AuthHostKeyMismatch,
    TransportTimeout,
    TransportTlsError,
    TransportTemporaryNetwork,
    TransportNetworkFailure,
    TransportFailure,
    InvalidRefspec,
    LsRemoteFailed,
    PushRejectedRefs,
    PushForceWithLeaseRejected,
}

impl GitErrorCode {
    /// Returns the stable code in string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRepoPath => "INVALID_REPO_PATH",
            Self::InvalidPathspec => "INVALID_PATHSPEC",
            Self::LockContention => "LOCK_CONTENTION",
            Self::LockIo => "LOCK_IO",
            Self::Internal => "INTERNAL",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::StatusDiffNotSupportedYet => "STATUS_DIFF_NOT_SUPPORTED_YET",
            Self::StatusDiffFailed => "STATUS_DIFF_FAILED",
            Self::StatusDiffInvalidPathspec => "STATUS_DIFF_INVALID_PATHSPEC",
            Self::LineDiffNotSupportedYet => "LINE_DIFF_NOT_SUPPORTED_YET",
            Self::LineDiffInvalidPath => "LINE_DIFF_INVALID_PATH",
            Self::LineDiffFailed => "LINE_DIFF_FAILED",
            Self::ApplyPatchFailed => "APPLY_PATCH_FAILED",
            Self::HistoryRewriteNotSupportedYet => "HISTORY_REWRITE_NOT_SUPPORTED_YET",
            Self::RefsNotSupportedYet => "REFS_NOT_SUPPORTED_YET",
            Self::PlumbingNotSupportedYet => "PLUMBING_NOT_SUPPORTED_YET",
            Self::PlumbingInvalidInput => "PLUMBING_INVALID_INPUT",
            Self::PlumbingObjectNotFound => "PLUMBING_OBJECT_NOT_FOUND",
            Self::PlumbingOperationFailed => "PLUMBING_OPERATION_FAILED",
            Self::PlumbingPackBuildFailed => "PLUMBING_PACK_BUILD_FAILED",
            Self::PlumbingIndexerFailed => "PLUMBING_INDEXER_FAILED",
            Self::AdvancedNotSupportedYet => "ADVANCED_NOT_SUPPORTED_YET",
            Self::AdvancedInvalidInput => "ADVANCED_INVALID_INPUT",
            Self::AdvancedOperationFailed => "ADVANCED_OPERATION_FAILED",
            Self::QueryLifecycleFailed => "QUERY_LIFECYCLE_FAILED",
            Self::QueryLifecycleInvalidInput => "QUERY_LIFECYCLE_INVALID_INPUT",
            Self::TagSummariesFailed => "TAG_SUMMARIES_FAILED",
            Self::RepositoryOpenFailed => "REPOSITORY_OPEN_FAILED",
            Self::WorkingCopySnapshotFailed => "WORKING_COPY_SNAPSHOT_FAILED",
            Self::IndexUpdateFailed => "INDEX_UPDATE_FAILED",
            Self::StagePathspecEmpty => "STAGE_PATHSPEC_EMPTY",
            Self::CommitMessageEmpty => "COMMIT_MESSAGE_EMPTY",
            Self::EmptyCommitNotAllowed => "EMPTY_COMMIT_NOT_ALLOWED",
            Self::InvalidSignatureContext => "INVALID_SIGNATURE_CONTEXT",
            Self::MergeInProgress => "MERGE_IN_PROGRESS",
            Self::HooksPresent => "HOOKS_PRESENT",
            Self::HookDiscoveryFailed => "HOOK_DISCOVERY_FAILED",
            Self::PullRebaseNotSupportedYet => "PULL_REBASE_NOT_SUPPORTED_YET",
            Self::MergeConflict => "MERGE_CONFLICT",
            Self::DetachedHead => "DETACHED_HEAD",
            Self::UpstreamNotFound => "UPSTREAM_NOT_FOUND",
            Self::WorktreeDirty => "WORKTREE_DIRTY",
            Self::BranchAlreadyExists => "BRANCH_ALREADY_EXISTS",
            Self::RefNotFound => "REF_NOT_FOUND",
            Self::AuthDenied => "AUTH_DENIED",
            Self::AuthNoCredentials => "AUTH_NO_CREDENTIALS",
            Self::AuthTimeout => "AUTH_TIMEOUT",
            Self::AuthUnsupportedSshDirective => "AUTH_UNSUPPORTED_SSH_DIRECTIVE",
            Self::AuthHostKeyUnknown => "AUTH_HOSTKEY_UNKNOWN",
            Self::AuthHostKeyMismatch => "AUTH_HOSTKEY_MISMATCH",
            Self::TransportTimeout => "TRANSPORT_TIMEOUT",
            Self::TransportTlsError => "TRANSPORT_TLS_ERROR",
            Self::TransportTemporaryNetwork => "TRANSPORT_TEMPORARY_NETWORK",
            Self::TransportNetworkFailure => "TRANSPORT_NETWORK_FAILURE",
            Self::TransportFailure => "TRANSPORT_FAILURE",
            Self::InvalidRefspec => "INVALID_REFSPEC",
            Self::LsRemoteFailed => "LS_REMOTE_FAILED",
            Self::PushRejectedRefs => "PUSH_REJECTED_REFS",
            Self::PushForceWithLeaseRejected => "PUSH_FORCE_WITH_LEASE_REJECTED",
        }
    }
}

/// Retryability classification of a typed `GitManager` error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitErrorRetryClassification {
    /// The error stems from a transient scenario and the operation may be retried.
    Retryable,
    /// The error stems from a hard-fail scenario and must not be masked by retry/fallback.
    Permanent,
}

impl GitErrorRetryClassification {
    /// Returns the stable machine-readable code for the retryability classification.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Retryable => "RETRYABLE",
            Self::Permanent => "PERMANENT",
        }
    }
}

impl fmt::Display for GitErrorRetryClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for GitErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Standard result alias for `GitManager` operations.
pub type GitResult<T> = Result<T, GitError>;

/// Warning codes for `GitManager` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitWarningCode {
    /// Hook scripts were not executed because of the `NO_SUBPROCESS` policy.
    HooksNotExecuted,
}

impl GitWarningCode {
    /// Returns the stable warning code in string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HooksNotExecuted => "HOOKS_NOT_EXECUTED",
        }
    }
}

impl fmt::Display for GitWarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed `GitManager` warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWarning {
    code: GitWarningCode,
    message: String,
}

impl GitWarning {
    /// Creates a typed warning.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(code: GitWarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the machine-readable warning code.
    #[must_use]
    pub const fn code(&self) -> GitWarningCode {
        self.code
    }
}

impl fmt::Display for GitWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

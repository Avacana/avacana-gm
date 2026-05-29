//! Contracts of the `history rewrite` domain.

use std::path::PathBuf;

/// Request for the history-rewrite domain (`rebase/cherry-pick/revert`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRewriteRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// The rewrite operation to perform.
    pub operation: HistoryRewriteOperation,
}

/// Typed operations of the history-rewrite domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryRewriteOperation {
    /// Request for a `rebase` flow.
    Rebase(RebaseRequest),
    /// Request for a `cherry-pick`.
    CherryPick(CherryPickRequest),
    /// Continue an active `cherry-pick` after conflicts are resolved.
    CherryPickContinue,
    /// Abort an active `cherry-pick` and roll back the state.
    CherryPickAbort,
    /// Skip the current step of an active `cherry-pick`.
    CherryPickSkip,
    /// Request for a `revert`.
    Revert(RevertRequest),
    /// Continue an active `revert` after conflicts are resolved.
    RevertContinue,
    /// Abort an active `revert` and roll back the state.
    RevertAbort,
    /// Skip the current step of an active `revert`.
    RevertSkip,
}

/// Parameters for a `rebase` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseRequest {
    /// Lifecycle action of the `rebase`.
    pub action: RebaseAction,
    /// Upstream revision to start the process from.
    pub upstream: Option<String>,
    /// Target `onto` revision.
    pub onto: Option<String>,
    /// Explicit branch target (if it differs from HEAD).
    pub branch: Option<String>,
}

/// Execution mode of a `rebase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseAction {
    /// Start a new `rebase`.
    Start,
    /// Continue after conflicts are resolved.
    Continue,
    /// Abort the `rebase` and roll back.
    Abort,
    /// Skip the current patch.
    Skip,
}

/// Parameters for a `cherry-pick` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CherryPickRequest {
    /// Source commit.
    pub commit: String,
    /// Mainline number for merge commits.
    pub mainline: Option<u32>,
}

/// Parameters for a `revert` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevertRequest {
    /// Commit to revert.
    pub commit: String,
    /// Mainline number for merge commits.
    pub mainline: Option<u32>,
}

/// Result of the history-rewrite domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRewriteResult {
    /// Final lifecycle state.
    pub state: HistoryRewriteState,
    /// The new `HEAD`, if it was changed.
    pub resulting_head: Option<String>,
    /// List of conflicted paths, if applicable.
    pub conflicted_paths: Vec<String>,
}

/// Machine-readable state of a history-rewrite flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryRewriteState {
    /// The process has started.
    Started,
    /// The process is in an intermediate state.
    InProgress,
    /// A conflict occurred.
    Conflict,
    /// The process continued after a conflict.
    Continued,
    /// The process was aborted.
    Aborted,
    /// The current patch/commit was skipped.
    Skipped,
    /// The process completed successfully.
    Completed,
}

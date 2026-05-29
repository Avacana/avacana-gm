//! Operations of the `history rewrite` domain for `GitManager`.

use crate::git_manager::core::operations_history_rewrite_support::open_repository;
use crate::git_manager::core::{
    GitResult, HistoryRewriteOperation, HistoryRewriteRequest, HistoryRewriteResult,
};

#[path = "operations_history_rewrite_cherry_pick.rs"]
mod cherry_pick_operations;
#[path = "operations_history_rewrite_rebase.rs"]
mod rebase_operations;
#[path = "operations_history_rewrite_revert.rs"]
mod revert_operations;
#[path = "operations_history_rewrite_sequence.rs"]
mod sequence_operations;

use cherry_pick_operations::{
    execute_cherry_pick_abort, execute_cherry_pick_continue, execute_cherry_pick_operation,
    execute_cherry_pick_skip,
};
use rebase_operations::execute_rebase_operation;
use revert_operations::{
    execute_revert_abort, execute_revert_continue, execute_revert_operation, execute_revert_skip,
};

/// Executes operations of the `history rewrite` domain (`rebase/cherry-pick/revert`).
///
/// # Errors
/// Returns a typed `GitError` if the history rewrite operation
/// cannot be executed because the repository is unavailable, the revisions are invalid,
/// the repository is in a conflicting state, or libgit2 fails.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            operation = ?request.operation
        )
    )
)]
pub(super) fn execute_history_rewrite_operation(
    request: &HistoryRewriteRequest,
) -> GitResult<HistoryRewriteResult> {
    let repository = open_repository(&request.repository_path, "history_rewrite")?;

    match &request.operation {
        HistoryRewriteOperation::Rebase(rebase_request) => {
            execute_rebase_operation(&repository, rebase_request)
        }
        HistoryRewriteOperation::CherryPick(cherry_pick_request) => {
            execute_cherry_pick_operation(&repository, cherry_pick_request)
        }
        HistoryRewriteOperation::CherryPickContinue => execute_cherry_pick_continue(&repository),
        HistoryRewriteOperation::CherryPickAbort => execute_cherry_pick_abort(&repository),
        HistoryRewriteOperation::CherryPickSkip => execute_cherry_pick_skip(&repository),
        HistoryRewriteOperation::Revert(revert_request) => {
            execute_revert_operation(&repository, revert_request)
        }
        HistoryRewriteOperation::RevertContinue => execute_revert_continue(&repository),
        HistoryRewriteOperation::RevertAbort => execute_revert_abort(&repository),
        HistoryRewriteOperation::RevertSkip => execute_revert_skip(&repository),
    }
}

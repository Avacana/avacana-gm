//! Operations of the `advanced` domain for `GitManager`.
use crate::git_manager::core::operations_advanced_metadata::{
    execute_check_ignore_operation, execute_describe_revision_operation,
    execute_query_attribute_operation, execute_resolve_mailmap_operation,
    execute_status_scan_operation, execute_submodule_update_operation,
    execute_worktree_lock_operation,
};
use crate::git_manager::core::operations_advanced_support::open_repository;
use crate::git_manager::core::{AdvancedOperation, AdvancedRequest, AdvancedResult, GitResult};

#[path = "operations_advanced_stash.rs"]
mod stash_operations;
#[path = "operations_advanced_submodules.rs"]
mod submodule_operations;
#[path = "operations_advanced_worktree.rs"]
mod worktree_operations;

use stash_operations::{
    execute_stash_apply_operation, execute_stash_drop_operation, execute_stash_list_operation,
    execute_stash_save_operation,
};
use submodule_operations::execute_sync_submodule_operation;
use worktree_operations::{execute_add_worktree_operation, execute_remove_worktree_operation};

const DEFAULT_DESCRIBE_REVISION: &str = "HEAD";

/// Executes operations of the `advanced` domain.
///
/// # Errors
/// Returns a typed `GitError` if the advanced operation cannot be
/// executed because the repository is unavailable, the parameters are invalid,
/// the required stash/ref/worktree entities are missing, or libgit2 fails.
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
pub(super) fn execute_advanced_operation(request: &AdvancedRequest) -> GitResult<AdvancedResult> {
    let mut repository = open_repository(&request.repository_path, "advanced")?;

    match &request.operation {
        AdvancedOperation::StashSave {
            message,
            include_untracked,
            keep_index,
        } => execute_stash_save_operation(
            &mut repository,
            message.as_deref(),
            *include_untracked,
            *keep_index,
        ),
        AdvancedOperation::StashList => execute_stash_list_operation(&mut repository),
        AdvancedOperation::StashApply {
            index,
            reinstate_index,
            pop,
        } => execute_stash_apply_operation(&mut repository, *index, *reinstate_index, *pop),
        AdvancedOperation::StashDrop { index } => {
            execute_stash_drop_operation(&mut repository, *index)
        }
        AdvancedOperation::SyncSubmodule { name, recursive } => {
            execute_sync_submodule_operation(&repository, name.as_deref(), *recursive)
        }
        AdvancedOperation::SubmoduleUpdate {
            name,
            recursive,
            init,
            allow_fetch,
        } => execute_submodule_update_operation(
            &repository,
            name.as_deref(),
            *recursive,
            *init,
            *allow_fetch,
        ),
        AdvancedOperation::AddWorktree {
            path,
            reference,
            detach,
        } => execute_add_worktree_operation(
            &repository,
            &request.repository_path,
            path,
            reference.as_deref(),
            *detach,
        ),
        AdvancedOperation::RemoveWorktree { path, force } => {
            execute_remove_worktree_operation(&repository, &request.repository_path, path, *force)
        }
        AdvancedOperation::WorktreeLock {
            path,
            action,
            reason,
        } => execute_worktree_lock_operation(
            &repository,
            &request.repository_path,
            path,
            action,
            reason.as_deref(),
        ),
        AdvancedOperation::QueryAttribute { path, name } => {
            execute_query_attribute_operation(&repository, path, name)
        }
        AdvancedOperation::StatusScan {
            show,
            pathspec,
            include_untracked,
        } => execute_status_scan_operation(
            &repository,
            show,
            pathspec.as_deref(),
            *include_untracked,
        ),
        AdvancedOperation::TraceSet { level } => {
            crate::git_manager::core::operations_advanced_support::execute_trace_set_operation(
                level,
            )
        }
        AdvancedOperation::CheckIgnore { path } => {
            execute_check_ignore_operation(&repository, path)
        }
        AdvancedOperation::ResolveMailmap { name, email } => {
            execute_resolve_mailmap_operation(&repository, name.as_deref(), email.as_deref())
        }
        AdvancedOperation::DescribeRevision { revision } => {
            execute_describe_revision_operation(&repository, revision.as_deref())
        }
    }
}

pub(super) fn push_boolean_flag(items: &mut Vec<String>, name: &str, value: bool) {
    items.push(format!("{name}:{value}"));
}

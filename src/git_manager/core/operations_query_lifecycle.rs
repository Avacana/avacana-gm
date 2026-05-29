//! query/lifecycle/verification domain operations for `GitManager`.
use crate::git_manager::core::operations_query_lifecycle_support::open_repository;
use crate::git_manager::core::{
    GitResult, QueryLifecycleOperation, QueryLifecycleRequest, QueryLifecycleResult,
};

#[path = "operations_query_lifecycle_history.rs"]
mod history_operations;
#[path = "operations_query_lifecycle_identity.rs"]
mod identity_operations;
#[path = "operations_query_lifecycle_merge.rs"]
mod merge_operations;
#[path = "operations_query_lifecycle_tree.rs"]
mod tree_operations;

use history_operations::{
    execute_format_email_operation, execute_log_operation, execute_message_trailers_operation,
    execute_shortlog_operation, execute_show_operation, execute_whatchanged_operation,
};
use identity_operations::{
    execute_blame_like_operation, execute_config_get_operation, execute_init_operation,
    execute_unsupported_command_operation, execute_version_operation,
};
use merge_operations::execute_merge_file_preview_operation;
use tree_operations::{
    execute_ls_tree_operation, execute_revparse_operation, execute_tree_walk_operation,
};

const DEFAULT_TREE_REVISION: &str = "HEAD^\u{7b}tree\u{7d}";

/// Executes query/lifecycle/verification domain operations.
///
/// # Errors
/// Returns a typed `GitError` if the operation cannot be carried out
/// because of an unavailable repository, invalid parameters,
/// a missing revision, or libgit2 failures.
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
pub(super) fn execute_query_lifecycle_operation(
    request: &QueryLifecycleRequest,
) -> GitResult<QueryLifecycleResult> {
    match &request.operation {
        QueryLifecycleOperation::Version => Ok(execute_version_operation()),
        QueryLifecycleOperation::Init {
            bare,
            initial_branch,
        } => execute_init_operation(&request.repository_path, *bare, initial_branch.as_deref()),
        QueryLifecycleOperation::UnsupportedCommand { command } => {
            execute_unsupported_command_operation(command)
        }
        QueryLifecycleOperation::Annotate {
            path,
            min_line,
            max_line,
            use_mailmap,
        }
        | QueryLifecycleOperation::Blame {
            path,
            min_line,
            max_line,
            use_mailmap,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_blame_like_operation(&repository, path, *min_line, *max_line, *use_mailmap)
        }
        QueryLifecycleOperation::ConfigGet { key } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_config_get_operation(&repository, key)
        }
        QueryLifecycleOperation::Log {
            revision_range,
            max_count,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_log_operation(&repository, revision_range.as_deref(), *max_count)
        }
        QueryLifecycleOperation::Revparse { spec } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_revparse_operation(&repository, spec)
        }
        QueryLifecycleOperation::LsTree {
            revision,
            recursive,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_ls_tree_operation(&repository, revision.as_deref(), *recursive)
        }
        QueryLifecycleOperation::TreeWalk {
            revision,
            post_order,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_tree_walk_operation(&repository, revision.as_deref(), *post_order)
        }
        QueryLifecycleOperation::Shortlog {
            revision_range,
            max_count,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_shortlog_operation(&repository, revision_range.as_deref(), *max_count)
        }
        QueryLifecycleOperation::Show { revision } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_show_operation(&repository, revision.as_deref())
        }
        QueryLifecycleOperation::MessageTrailers { revision } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_message_trailers_operation(&repository, revision.as_deref())
        }
        QueryLifecycleOperation::MergeFilePreview { path, ours, theirs } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_merge_file_preview_operation(&repository, path, ours, theirs)
        }
        QueryLifecycleOperation::FormatEmail {
            revision,
            subject_prefix,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_format_email_operation(
                &repository,
                revision.as_deref(),
                subject_prefix.as_deref(),
            )
        }
        QueryLifecycleOperation::Whatchanged {
            revision_range,
            max_count,
        } => {
            let repository = open_repository(&request.repository_path, "query_lifecycle")?;
            execute_whatchanged_operation(&repository, revision_range.as_deref(), *max_count)
        }
    }
}

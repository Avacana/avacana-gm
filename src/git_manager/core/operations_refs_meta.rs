//! `refs/meta` domain operations for `GitManager`.

use crate::git_manager::core::operations_refs_meta_support::{
    map_refs_error, normalize_reference_name, open_repository,
};
use crate::git_manager::core::{GitResult, ReflogEntry, RefsOperation, RefsRequest, RefsResult};
use git2::Repository;

#[path = "operations_refs_meta_listing.rs"]
mod listing;
#[path = "operations_refs_meta_mutation.rs"]
mod mutation;
#[path = "operations_refs_meta_notes.rs"]
mod notes;
#[path = "operations_refs_meta_transaction.rs"]
mod transaction;

/// Executes `refs/meta` domain operations.
///
/// # Errors
/// Returns a typed `GitError` if the refs/meta operation cannot be carried out
/// because of an unavailable repository, invalid arguments,
/// an expected-target mismatch, or libgit2 failures.
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
pub(super) fn execute_refs_operation(request: &RefsRequest) -> GitResult<RefsResult> {
    let repository = open_repository(&request.repository_path, "refs")?;
    match &request.operation {
        RefsOperation::List { pattern } => {
            listing::execute_list_operation(&repository, pattern.as_deref())
        }
        RefsOperation::ListBranches {
            include_local,
            include_remote,
        } => listing::execute_list_branches_operation(&repository, *include_local, *include_remote),
        RefsOperation::ListReferenceNames { pattern } => {
            listing::execute_list_reference_names_operation(&repository, pattern.as_deref())
        }
        RefsOperation::ListConfigEntries { glob } => {
            listing::execute_list_config_entries_operation(&repository, glob.as_deref())
        }
        RefsOperation::CreateBranch { name, start_point } => {
            mutation::execute_create_branch_refs_operation(
                &request.repository_path,
                &repository,
                name,
                start_point.as_deref(),
            )
        }
        RefsOperation::CreateTag {
            name,
            target,
            message,
            force,
        } => mutation::execute_create_tag_operation(
            &repository,
            name,
            target,
            message.as_deref(),
            *force,
        ),
        RefsOperation::DeleteReference {
            name,
            expected_target,
        } => mutation::execute_delete_reference_operation(
            &repository,
            name,
            expected_target.as_deref(),
        ),
        RefsOperation::UpdateReference {
            name,
            new_target,
            expected_old_target,
            reflog_message,
        } => mutation::execute_update_reference_operation(
            &repository,
            name,
            new_target,
            expected_old_target.as_deref(),
            reflog_message.as_deref(),
        ),
        RefsOperation::ReadReflog {
            reference,
            limit,
            newest_first,
        } => execute_read_reflog_operation(&repository, reference, *limit, *newest_first),
        RefsOperation::WriteNote {
            target,
            namespace,
            message,
            force,
        } => notes::execute_write_note_operation(
            &repository,
            target,
            namespace.as_deref(),
            message,
            *force,
        ),
        RefsOperation::ReadNote { target, namespace } => {
            notes::execute_read_note_operation(&repository, target, namespace.as_deref())
        }
        RefsOperation::Transaction {
            updates,
            reflog_message,
        } => transaction::execute_transaction_operation(
            &repository,
            updates.as_slice(),
            reflog_message.as_deref(),
        ),
    }
}

fn execute_read_reflog_operation(
    repository: &Repository,
    reference_name: &str,
    limit: usize,
    newest_first: bool,
) -> GitResult<RefsResult> {
    let reference_name = normalize_reference_name(reference_name, "refs.read_reflog.reference")?;
    let reflog: git2::Reflog = repository.reflog(reference_name).map_err(|error| {
        map_refs_error(
            &error,
            format!("refs.read_reflog failed to read `{reference_name}`"),
        )
    })?;
    let reflog_iter: git2::ReflogIter<'_> = reflog.iter();
    let mut reflog_entries = reflog_iter
        .take(limit)
        .map(|entry: git2::ReflogEntry<'_>| ReflogEntry {
            old_oid: entry.id_old().to_string(),
            new_oid: entry.id_new().to_string(),
            message: entry.message().map_or_else(String::new, str::to_owned),
        })
        .collect::<Vec<_>>();
    if !newest_first {
        reflog_entries.reverse();
    }
    Ok(RefsResult {
        changed: false,
        reflog_entries,
        ..RefsResult::default()
    })
}

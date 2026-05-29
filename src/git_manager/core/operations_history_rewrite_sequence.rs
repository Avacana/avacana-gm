use crate::git_manager::core::operations_history_rewrite_support::{
    checkout_head_safe, cleanup_repository_state, resolve_head_commit, resolve_signature,
};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::Repository;

pub(super) fn finalize_sequence_commit(
    repository: &Repository,
    author: Option<&git2::Signature<'_>>,
    message: &str,
    operation: &str,
) -> GitResult<()> {
    let mut index = repository.index().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("{operation} failed to open git index after apply: {error}"),
        )
    })?;

    if index.has_conflicts() {
        return Err(GitError::new(
            GitErrorCode::MergeConflict,
            format!("{operation} cannot finalize commit while index has unresolved conflicts"),
        ));
    }

    let tree_oid = index.write_tree_to(repository).map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("{operation} failed to write resulting tree to repository: {error}"),
        )
    })?;
    let tree = repository.find_tree(tree_oid).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("{operation} failed to resolve resulting tree `{tree_oid}`: {error}"),
        )
    })?;

    let head_commit = resolve_head_commit(repository, operation)?;
    let committer = resolve_signature(repository, operation)?;
    let commit_author = author.unwrap_or(&committer);

    repository
        .commit(
            Some("HEAD"),
            commit_author,
            &committer,
            message,
            &tree,
            &[&head_commit],
        )
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("{operation} failed to create resulting commit: {error}"),
            )
        })?;

    cleanup_repository_state(repository, operation)?;
    checkout_head_safe(repository, operation)
}

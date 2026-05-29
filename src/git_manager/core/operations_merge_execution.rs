#![allow(clippy::too_many_lines)]

use crate::git_manager::core::{GitError, GitErrorCode, GitResult, MergeFileFavor};
use git2::{
    build::CheckoutBuilder, FileFavor, MergeOptions, Repository, RepositoryState, ResetType,
};

pub(super) fn perform_fast_forward_merge(
    repository: &Repository,
    head_reference_name: &str,
    source_annotated_commit: &git2::AnnotatedCommit<'_>,
    source_ref: &str,
) -> GitResult<()> {
    let mut head_reference = repository
        .find_reference(head_reference_name)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to resolve target reference `{head_reference_name}` for fast-forward merge: {error}"
                ),
            )
        })?;

    head_reference
        .set_target(
            source_annotated_commit.id(),
            format!("merge: fast-forward from `{source_ref}`").as_str(),
        )
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to update `{head_reference_name}` during fast-forward merge: {error}"
                ),
            )
        })?;

    repository.set_head(head_reference_name).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!(
                "failed to set HEAD to `{head_reference_name}` after fast-forward merge: {error}"
            ),
        )
    })?;

    checkout_head_safe(repository)
}

pub(super) fn perform_non_fast_forward_merge(
    repository: &Repository,
    source_annotated_commit: &git2::AnnotatedCommit<'_>,
    source_ref: &str,
    head_reference_name: &str,
    file_favor: MergeFileFavor,
) -> GitResult<()> {
    let mut merge_options = MergeOptions::new();
    merge_options.file_favor(map_merge_file_favor(file_favor));
    repository
        .merge(&[source_annotated_commit], Some(&mut merge_options), None)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to start merge for source `{source_ref}`: {error}"),
            )
        })?;

    let merge_result = (|| {
        let mut index = repository.index().map_err(|error| {
            GitError::new(
                GitErrorCode::IndexUpdateFailed,
                format!("failed to read index after merge attempt: {error}"),
            )
        })?;

        if index.has_conflicts() {
            let conflict_paths = collect_conflict_paths(&index);
            tracing::trace!(
                conflict_count = conflict_paths.len(),
                conflict_paths = ?conflict_paths,
                "merge produced index conflicts"
            );
            return Err(GitError::new(
                GitErrorCode::MergeConflict,
                format!(
                    "merge `{source_ref}` into `{head_reference_name}` produced conflicts: {}",
                    summarize_conflicts(conflict_paths.as_slice())
                ),
            ));
        }

        let tree_oid = index.write_tree_to(repository).map_err(|error| {
            GitError::new(
                GitErrorCode::IndexUpdateFailed,
                format!("failed to write merge tree to repository: {error}"),
            )
        })?;
        let merge_tree = repository.find_tree(tree_oid).map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to resolve merge tree `{tree_oid}`: {error}"),
            )
        })?;

        let head_commit = repository
            .find_reference(head_reference_name)
            .and_then(|reference| reference.peel_to_commit())
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::DetachedHead,
                    format!(
                        "failed to resolve target HEAD commit `{head_reference_name}` for merge commit: {error}"
                    ),
                )
            })?;
        let source_commit = repository
            .find_commit(source_annotated_commit.id())
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::RefNotFound,
                    format!(
                        "failed to resolve source commit `{}` for merge commit: {error}",
                        source_annotated_commit.id()
                    ),
                )
            })?;

        let signature = repository.signature().map_err(|error| {
            GitError::new(
                GitErrorCode::InvalidSignatureContext,
                format!("failed to resolve signature for merge commit: {error}"),
            )
        })?;

        let target_display = head_reference_name
            .strip_prefix("refs/heads/")
            .unwrap_or(head_reference_name);
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                format!("Merge `{source_ref}` into `{target_display}`").as_str(),
                &merge_tree,
                &[&head_commit, &source_commit],
            )
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::Internal,
                    format!("failed to create merge commit for `{source_ref}`: {error}"),
                )
            })?;

        repository.cleanup_state().map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to cleanup merge state after commit: {error}"),
            )
        })?;

        checkout_head_safe(repository)
    })();

    match merge_result {
        Ok(()) => Ok(()),
        Err(error) => rollback_failed_merge(repository, source_ref, error),
    }
}

pub(super) fn ensure_clean_repository_state(
    repository: &Repository,
    operation: &str,
) -> GitResult<()> {
    if repository.state() == RepositoryState::Clean {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::MergeInProgress,
        format!(
            "{operation} requires clean repository state, current state is `{:?}`",
            repository.state()
        ),
    ))
}

fn rollback_failed_merge(
    repository: &Repository,
    source_ref: &str,
    original_error: GitError,
) -> GitResult<()> {
    let mut recovery_failures = Vec::new();

    if let Err(error) = repository.cleanup_state() {
        log_best_effort_cleanup_failure("merge.rollback", &error);
        recovery_failures.push(format!("cleanup_state failed: {error}"));
    }

    let head_commit = repository
        .head()
        .and_then(|head_reference| head_reference.peel_to_commit());
    match head_commit {
        Ok(commit) => {
            if let Err(error) = repository.reset(commit.as_object(), ResetType::Hard, None) {
                recovery_failures.push(format!("reset --hard failed: {error}"));
            }
        }
        Err(error) => {
            recovery_failures.push(format!(
                "failed to resolve HEAD for rollback reset: {error}"
            ));
        }
    }

    if recovery_failures.is_empty() {
        return Err(original_error);
    }

    Err(GitError::new(
        GitErrorCode::Internal,
        format!(
            "{original_error}; merge rollback after `{source_ref}` also failed: {}",
            recovery_failures.join("; ")
        ),
    ))
}

fn collect_conflict_paths(index: &git2::Index) -> Vec<String> {
    let mut conflict_paths = Vec::new();
    let Ok(conflicts) = index.conflicts() else {
        return conflict_paths;
    };

    for conflict in conflicts.flatten() {
        let path = conflict
            .our
            .as_ref()
            .or(conflict.their.as_ref())
            .or(conflict.ancestor.as_ref())
            .and_then(|entry| std::str::from_utf8(&entry.path).ok())
            .and_then(non_empty)
            .unwrap_or("<unknown-path>");
        conflict_paths.push(path.to_owned());
    }

    conflict_paths.sort();
    conflict_paths.dedup();
    conflict_paths
}

fn summarize_conflicts(conflict_paths: &[String]) -> String {
    const MAX_LISTED_CONFLICTS: usize = 8;

    if conflict_paths.is_empty() {
        return "unknown conflict set".to_string();
    }

    if conflict_paths.len() <= MAX_LISTED_CONFLICTS {
        return conflict_paths.join(", ");
    }

    let listed = conflict_paths
        .iter()
        .take(MAX_LISTED_CONFLICTS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{listed}, … ({} more)",
        conflict_paths.len() - MAX_LISTED_CONFLICTS
    )
}

fn checkout_head_safe(repository: &Repository) -> GitResult<()> {
    let mut checkout_builder = CheckoutBuilder::new();
    checkout_builder.safe();

    repository
        .checkout_head(Some(&mut checkout_builder))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to checkout HEAD after merge operation: {error}"),
            )
        })
}

pub(super) const fn map_merge_file_favor(file_favor: MergeFileFavor) -> FileFavor {
    match file_favor {
        MergeFileFavor::Normal => FileFavor::Normal,
        MergeFileFavor::Ours => FileFavor::Ours,
        MergeFileFavor::Theirs => FileFavor::Theirs,
        MergeFileFavor::Union => FileFavor::Union,
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn log_best_effort_cleanup_failure(context: &str, error: &git2::Error) {
    tracing::warn!(
        context = context,
        cleanup_action = "repository.cleanup_state",
        error = %error,
        "best_effort_cleanup_failed"
    );
}

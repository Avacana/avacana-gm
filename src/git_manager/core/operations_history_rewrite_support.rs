//! Helper utilities of the `history rewrite` domain.

pub(super) use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, HistoryRewriteResult, HistoryRewriteState,
};
use git2::{
    build::CheckoutBuilder, ErrorCode, Rebase, RebaseOptions, Repository, RepositoryState,
    ResetType,
};
use std::io::ErrorKind;

pub(super) fn cleanup_repository_state(repository: &Repository, operation: &str) -> GitResult<()> {
    repository.cleanup_state().map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("{operation} failed to cleanup repository state: {error}"),
        )
    })
}

pub(super) fn checkout_head_safe(repository: &Repository, operation: &str) -> GitResult<()> {
    let mut checkout_builder = CheckoutBuilder::new();
    checkout_builder.safe();

    repository
        .checkout_head(Some(&mut checkout_builder))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("{operation} failed to checkout HEAD after completion: {error}"),
            )
        })
}

pub(super) fn reset_head_hard(repository: &Repository, operation: &str) -> GitResult<()> {
    let head_commit = resolve_head_commit(repository, operation)?;
    repository
        .reset(head_commit.as_object(), ResetType::Hard, None)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("{operation} failed to hard-reset repository state: {error}"),
            )
        })
}

pub(super) fn open_active_rebase<'repo>(
    repository: &'repo Repository,
    context: &str,
) -> GitResult<Rebase<'repo>> {
    let mut rebase_options = RebaseOptions::new();
    rebase_options.quiet(true);
    repository
        .open_rebase(Some(&mut rebase_options))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("{context} failed to open active rebase session: {error}"),
            )
        })
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

pub(super) fn ensure_active_rebase_state(
    repository: &Repository,
    operation: &str,
) -> GitResult<()> {
    if is_rebase_state(repository.state()) {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::MergeInProgress,
        format!(
            "{operation} requires active rebase state, current state is `{:?}`",
            repository.state()
        ),
    ))
}

pub(super) fn ensure_active_cherry_pick_state(
    repository: &Repository,
    operation: &str,
) -> GitResult<()> {
    if is_cherry_pick_state(repository.state()) {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::MergeInProgress,
        format!(
            "{operation} requires active cherry-pick state, current state is `{:?}`",
            repository.state()
        ),
    ))
}

pub(super) fn ensure_active_revert_state(
    repository: &Repository,
    operation: &str,
) -> GitResult<()> {
    if is_revert_state(repository.state()) {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::MergeInProgress,
        format!(
            "{operation} requires active revert state, current state is `{:?}`",
            repository.state()
        ),
    ))
}

pub(super) const fn is_rebase_state(state: RepositoryState) -> bool {
    matches!(
        state,
        RepositoryState::Rebase | RepositoryState::RebaseInteractive | RepositoryState::RebaseMerge
    )
}

pub(super) const fn is_cherry_pick_state(state: RepositoryState) -> bool {
    matches!(
        state,
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence
    )
}

pub(super) const fn is_revert_state(state: RepositoryState) -> bool {
    matches!(
        state,
        RepositoryState::Revert | RepositoryState::RevertSequence
    )
}

pub(super) const fn is_conflict_error_code(error_code: ErrorCode) -> bool {
    matches!(
        error_code,
        ErrorCode::Conflict
            | ErrorCode::MergeConflict
            | ErrorCode::Unmerged
            | ErrorCode::ApplyFail
            | ErrorCode::IndexDirty
    )
}

pub(super) fn history_result(
    repository: &Repository,
    state: HistoryRewriteState,
    conflicted_paths: Vec<String>,
) -> HistoryRewriteResult {
    HistoryRewriteResult {
        state,
        resulting_head: resolve_head_oid(repository),
        conflicted_paths,
    }
}

pub(super) fn resolve_head_oid(repository: &Repository) -> Option<String> {
    repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string())
}

pub(super) fn resolve_head_commit<'repo>(
    repository: &'repo Repository,
    operation: &str,
) -> GitResult<git2::Commit<'repo>> {
    repository
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::DetachedHead,
                format!("{operation} requires an attached HEAD commit: {error}"),
            )
        })
}

pub(super) fn resolve_signature<'repo>(
    repository: &'repo Repository,
    operation: &str,
) -> GitResult<git2::Signature<'repo>> {
    repository.signature().map_err(|error| {
        GitError::new(
            GitErrorCode::InvalidSignatureContext,
            format!("{operation} failed to resolve commit signature context: {error}"),
        )
    })
}

pub(super) fn resolve_commit<'repo>(
    repository: &'repo Repository,
    revision: &str,
    field_name: &str,
) -> GitResult<git2::Commit<'repo>> {
    let revision = normalize_non_empty(revision, field_name)?;
    let object = repository.revparse_single(revision).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} `{revision}` cannot be resolved: {error}"),
        )
    })?;

    object.peel_to_commit().map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} `{revision}` cannot be resolved to a commit: {error}"),
        )
    })
}

pub(super) fn resolve_sequence_head_commit<'repo>(
    repository: &'repo Repository,
    pseudo_ref_name: &str,
    operation: &str,
) -> GitResult<Option<git2::Commit<'repo>>> {
    let pseudo_ref_path = repository.path().join(pseudo_ref_name);
    let raw_oid = match std::fs::read_to_string(&pseudo_ref_path) {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::Internal,
                format!(
                    "{operation} failed to read sequence marker `{}`: {error}",
                    pseudo_ref_path.display()
                ),
            ))
        }
    };

    let oid_text = non_empty(raw_oid.trim()).ok_or_else(|| {
        GitError::new(
            GitErrorCode::Internal,
            format!(
                "{operation} sequence marker `{}` is empty",
                pseudo_ref_path.display()
            ),
        )
    })?;
    let oid = git2::Oid::from_str(oid_text).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!(
                "{operation} sequence marker `{}` has invalid object id `{oid_text}`: {error}",
                pseudo_ref_path.display()
            ),
        )
    })?;

    repository.find_commit(oid).map(Some).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!(
                "{operation} sequence marker `{pseudo_ref_name}` points to unknown commit `{oid}`: {error}"
            ),
        )
    })
}

pub(super) fn read_sequence_message(
    repository: &Repository,
    operation: &str,
    fallback_message: &str,
) -> GitResult<String> {
    let merge_message_path = repository.path().join("MERGE_MSG");
    let content = match std::fs::read_to_string(&merge_message_path) {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(fallback_message.to_owned());
        }
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::Internal,
                format!(
                    "{operation} failed to read sequence message `{}`: {error}",
                    merge_message_path.display()
                ),
            ))
        }
    };

    Ok(non_empty(content.trim()).map_or_else(|| fallback_message.to_owned(), str::to_owned))
}

pub(super) fn resolve_annotated_commit<'repo>(
    repository: &'repo Repository,
    revision: &str,
    field_name: &str,
) -> GitResult<git2::AnnotatedCommit<'repo>> {
    let revision = normalize_non_empty(revision, field_name)?;
    let object = repository.revparse_single(revision).map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} `{revision}` cannot be resolved: {error}"),
        )
    })?;

    repository
        .find_annotated_commit(object.id())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "{field_name} `{revision}` cannot be converted to annotated commit: {error}"
                ),
            )
        })
}

pub(super) fn resolve_optional_annotated_commit<'repo>(
    repository: &'repo Repository,
    revision: Option<&str>,
    field_name: &str,
) -> GitResult<Option<git2::AnnotatedCommit<'repo>>> {
    revision.map_or_else(
        || Ok(None),
        |revision| resolve_annotated_commit(repository, revision, field_name).map(Some),
    )
}

pub(super) fn collect_conflict_paths(repository: &Repository) -> GitResult<Vec<String>> {
    let index = repository.index().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("history rewrite failed to open git index for conflict inspection: {error}"),
        )
    })?;

    if !index.has_conflicts() {
        return Ok(Vec::new());
    }

    let conflicts = index.conflicts().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("history rewrite failed to enumerate conflict entries: {error}"),
        )
    })?;

    let mut conflict_paths = Vec::new();
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
    Ok(conflict_paths)
}

pub(super) fn normalize_non_empty<'a>(value: &'a str, field_name: &str) -> GitResult<&'a str> {
    non_empty(value).ok_or_else(|| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("{field_name} must not be empty"),
        )
    })
}

pub(super) fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

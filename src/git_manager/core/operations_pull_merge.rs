use super::plan_operations::detached_head_error;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, MergeFileFavor, PullResult};
use git2::{build::CheckoutBuilder, FileFavor, MergeOptions, Repository};

pub(super) fn merge_fetched_head(
    repository: &Repository,
    file_favor: MergeFileFavor,
) -> GitResult<PullResult> {
    let fetch_head_reference = repository.find_reference("FETCH_HEAD").map_err(|error| {
        GitError::new(
            GitErrorCode::UpstreamNotFound,
            format!("pull failed to resolve FETCH_HEAD after fetch: {error}"),
        )
    })?;
    let fetch_head_commit = repository
        .reference_to_annotated_commit(&fetch_head_reference)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("pull failed to resolve annotated commit from FETCH_HEAD: {error}"),
            )
        })?;

    let (merge_analysis, _) =
        repository
            .merge_analysis(&[&fetch_head_commit])
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::Internal,
                    format!("pull merge-analysis failed for FETCH_HEAD: {error}"),
                )
            })?;

    tracing::trace!(
        up_to_date = merge_analysis.is_up_to_date(),
        fast_forward = merge_analysis.is_fast_forward(),
        normal_merge = merge_analysis.is_normal(),
        "pull merge-analysis result"
    );

    if merge_analysis.is_up_to_date() {
        return Ok(PullResult {
            updated: false,
            fast_forward: false,
        });
    }

    if merge_analysis.is_fast_forward() {
        perform_fast_forward_merge(repository, &fetch_head_commit)?;
        return Ok(PullResult {
            updated: true,
            fast_forward: true,
        });
    }

    if merge_analysis.is_normal() {
        perform_non_fast_forward_merge(repository, &fetch_head_commit, file_favor)?;
        return Ok(PullResult {
            updated: true,
            fast_forward: false,
        });
    }

    Err(GitError::new(
        GitErrorCode::Internal,
        "pull merge-analysis produced unsupported outcome flags",
    ))
}

fn perform_fast_forward_merge(
    repository: &Repository,
    fetch_head_commit: &git2::AnnotatedCommit<'_>,
) -> GitResult<()> {
    let head_reference = repository.head().map_err(|error| {
        GitError::new(
            GitErrorCode::DetachedHead,
            format!("pull fast-forward requires branch HEAD: {error}"),
        )
    })?;
    if !head_reference.is_branch() {
        return Err(detached_head_error());
    }

    let head_name = head_reference
        .name()
        .and_then(non_empty)
        .ok_or_else(detached_head_error)?
        .to_string();

    let mut branch_reference = repository.find_reference(&head_name).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve branch reference `{head_name}` for pull: {error}"),
        )
    })?;

    branch_reference
        .set_target(fetch_head_commit.id(), "pull: fast-forward")
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to update HEAD ref for pull fast-forward: {error}"),
            )
        })?;
    repository.set_head(&head_name).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to set HEAD after pull fast-forward: {error}"),
        )
    })?;
    checkout_head_safe(repository)
}

fn perform_non_fast_forward_merge(
    repository: &Repository,
    fetch_head_commit: &git2::AnnotatedCommit<'_>,
    file_favor: MergeFileFavor,
) -> GitResult<()> {
    let mut merge_options = MergeOptions::new();
    merge_options.file_favor(map_merge_file_favor(file_favor));
    repository
        .merge(&[fetch_head_commit], Some(&mut merge_options), None)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to start non-fast-forward pull merge: {error}"),
            )
        })?;

    let mut index = repository.index().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("failed to read index after pull merge attempt: {error}"),
        )
    })?;
    if index.has_conflicts() {
        if let Err(error) = repository.cleanup_state() {
            log_best_effort_cleanup_failure("pull.merge_conflict", &error);
            return Err(GitError::new(
                GitErrorCode::MergeConflict,
                format!(
                    "pull merge resulted in index conflicts; cleanup_state also failed: {error}"
                ),
            ));
        }

        return Err(GitError::new(
            GitErrorCode::MergeConflict,
            "pull merge resulted in index conflicts",
        ));
    }

    let tree_oid = index.write_tree_to(repository).map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!("failed to write merge tree for pull commit: {error}"),
        )
    })?;
    let merge_tree = repository.find_tree(tree_oid).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve merge tree `{tree_oid}` for pull: {error}"),
        )
    })?;

    let head_commit = repository
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::DetachedHead,
                format!("failed to resolve HEAD commit for pull merge commit: {error}"),
            )
        })?;
    let fetched_commit = repository
        .find_commit(fetch_head_commit.id())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to resolve fetched commit `{}` for pull merge commit: {error}",
                    fetch_head_commit.id()
                ),
            )
        })?;

    let signature = repository.signature().map_err(|error| {
        GitError::new(
            GitErrorCode::InvalidSignatureContext,
            format!("failed to resolve signature for pull merge commit: {error}"),
        )
    })?;

    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Merge FETCH_HEAD",
            &merge_tree,
            &[&head_commit, &fetched_commit],
        )
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to create merge commit for pull: {error}"),
            )
        })?;

    repository.cleanup_state().map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to cleanup merge state after pull commit: {error}"),
        )
    })?;
    checkout_head_safe(repository)
}

fn checkout_head_safe(repository: &Repository) -> GitResult<()> {
    let mut checkout_builder = CheckoutBuilder::new();
    checkout_builder.safe();

    repository
        .checkout_head(Some(&mut checkout_builder))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to checkout HEAD after pull merge: {error}"),
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

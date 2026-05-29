//! `commit` operations for `GitManager`.

use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    CommitRequest, CommitResult, EmptyCommitPolicy, GitError, GitErrorCode, GitResult, GitWarning,
    GitWarningCode,
};
use git2::{ErrorCode, Repository, RepositoryState, Signature};
use std::ffi::OsStr;
use std::io::ErrorKind;

/// Executes a `commit`, applying the empty-commit and hooks policies.
///
/// # Errors
/// Returns a typed `GitError` on signature/context errors, an
/// empty commit (when disallowed by policy), merge state, and index errors.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            message_len = request.message.len(),
            fail_if_hooks_present = request.hooks_policy.fail_if_hooks_present
        )
    )
)]
pub(super) fn execute_commit_operation(request: &CommitRequest) -> GitResult<CommitResult> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(GitError::new(
            GitErrorCode::CommitMessageEmpty,
            "commit message must not be empty",
        ));
    }

    let repository = open_repository(&request.repository_path, "commit")?;
    ensure_commit_not_blocked_by_repository_state(&repository)?;

    let hooks_present = has_non_sample_hooks(&repository)?;
    if request.hooks_policy.fail_if_hooks_present && hooks_present {
        return Err(GitError::new(
            GitErrorCode::HooksPresent,
            "commit aborted because hooks are present and fail_if_hooks_present policy is enabled",
        ));
    }

    let mut index = repository.index().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!(
                "failed to read index for commit in repository `{}`: {error}",
                request.repository_path.display()
            ),
        )
    })?;

    let tree_oid = index.write_tree().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!(
                "failed to write tree from index for repository `{}`: {error}",
                request.repository_path.display()
            ),
        )
    })?;

    let tree = repository.find_tree(tree_oid).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!(
                "failed to load written tree `{tree_oid}` in repository `{}`: {error}",
                request.repository_path.display()
            ),
        )
    })?;

    let empty_commit = is_empty_commit(&repository, tree_oid)?;
    if empty_commit && request.empty_commit_policy == EmptyCommitPolicy::Reject {
        return Err(GitError::new(
            GitErrorCode::EmptyCommitNotAllowed,
            "commit aborted because index tree is unchanged and empty commits are rejected by policy",
        ));
    }

    let signature = resolve_commit_signature(&repository, request)?;
    let parent_commits = collect_parent_commits(&repository)?;
    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();

    let commit_oid = repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to create commit in repository `{}`: {error}",
                    request.repository_path.display()
                ),
            )
        })?;

    Ok(CommitResult {
        commit_oid: commit_oid.to_string(),
        empty_commit,
        warnings: vec![hooks_not_executed_warning()],
    })
}

fn ensure_commit_not_blocked_by_repository_state(repository: &Repository) -> GitResult<()> {
    if repository.state() != RepositoryState::Clean {
        return Err(GitError::new(
            GitErrorCode::MergeInProgress,
            format!(
                "commit requires clean repository state, current state is `{:?}`",
                repository.state()
            ),
        ));
    }

    let merge_head_path = repository.path().join("MERGE_HEAD");
    if merge_head_path.exists() {
        return Err(GitError::new(
            GitErrorCode::MergeInProgress,
            "commit requires clean repository state, found MERGE_HEAD marker",
        ));
    }

    Ok(())
}

fn has_non_sample_hooks(repository: &Repository) -> GitResult<bool> {
    let hooks_dir = repository.path().join("hooks");
    let read_dir = match std::fs::read_dir(&hooks_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::HookDiscoveryFailed,
                format!(
                    "failed to read hooks directory `{}`: {error}",
                    hooks_dir.display()
                ),
            ))
        }
    };

    for entry in read_dir {
        let path = entry
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::HookDiscoveryFailed,
                    format!(
                        "failed to inspect entry in hooks directory `{}`: {error}",
                        hooks_dir.display()
                    ),
                )
            })?
            .path();

        if !path.is_file() {
            continue;
        }

        let is_sample = path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sample"));
        if is_sample {
            continue;
        }

        return Ok(true);
    }

    Ok(false)
}

fn is_empty_commit(repository: &Repository, tree_oid: git2::Oid) -> GitResult<bool> {
    match repository.head() {
        Ok(head_reference) => {
            let head_commit = head_reference.peel_to_commit().map_err(|error| {
                GitError::new(
                    GitErrorCode::Internal,
                    format!("failed to resolve HEAD commit for empty-commit check: {error}"),
                )
            })?;
            Ok(head_commit.tree_id() == tree_oid)
        }
        Err(error) if is_absent_head_error(&error) => {
            let tree = repository.find_tree(tree_oid).map_err(|find_error| {
                GitError::new(
                    GitErrorCode::Internal,
                    format!(
                        "failed to resolve tree `{tree_oid}` for initial empty-commit check: {find_error}"
                    ),
                )
            })?;
            Ok(tree.is_empty())
        }
        Err(error) => Err(GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve HEAD for empty-commit check: {error}"),
        )),
    }
}

fn resolve_commit_signature(
    repository: &Repository,
    request: &CommitRequest,
) -> GitResult<Signature<'static>> {
    match (request.author_name.as_deref(), request.author_email.as_deref()) {
        (Some(author_name), Some(author_email)) => {
            let author_name = author_name.trim();
            let author_email = author_email.trim();
            if author_name.is_empty() || author_email.is_empty() {
                return Err(GitError::new(
                    GitErrorCode::InvalidSignatureContext,
                    "author_name and author_email must be non-empty when explicitly provided",
                ));
            }

            Signature::now(author_name, author_email).map_err(|error| {
                GitError::new(
                    GitErrorCode::InvalidSignatureContext,
                    format!(
                        "failed to construct commit signature from request context: {error}"
                    ),
                )
            })
        }
        (None, None) => repository.signature().map_err(|error| {
            GitError::new(
                GitErrorCode::InvalidSignatureContext,
                format!(
                    "failed to resolve git signature from repository config (user.name/user.email): {error}"
                ),
            )
        }),
        _ => Err(GitError::new(
            GitErrorCode::InvalidSignatureContext,
            "author_name and author_email must be provided together",
        )),
    }
}

fn collect_parent_commits(repository: &Repository) -> GitResult<Vec<git2::Commit<'_>>> {
    match repository.head() {
        Ok(head_reference) => head_reference.peel_to_commit().map_or_else(
            |error| {
                Err(GitError::new(
                    GitErrorCode::Internal,
                    format!("failed to resolve HEAD commit for parent list: {error}"),
                ))
            },
            |commit| Ok(vec![commit]),
        ),
        Err(error) if is_absent_head_error(&error) => Ok(Vec::new()),
        Err(error) => Err(GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve HEAD for parent list: {error}"),
        )),
    }
}

fn is_absent_head_error(error: &git2::Error) -> bool {
    matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch)
}

fn hooks_not_executed_warning() -> GitWarning {
    GitWarning::new(
        GitWarningCode::HooksNotExecuted,
        "git hooks were not executed due to NO_SUBPROCESS policy",
    )
}

//! `switch` operations for `GitManager`.

use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, SwitchBranchRequest, SwitchBranchResult,
};
use crate::git_manager::state::worktree_policy::{
    enforce_switch_worktree_policy, SwitchWorktreePolicy,
};
use git2::{build::CheckoutBuilder, BranchType, ErrorCode, Repository};

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(repository = %request.repository_path.display(), branch = request.branch_name, force = request.force, allow_dirty = request.allow_dirty)
    )
)]
pub(super) fn execute_switch_operation(
    request: &SwitchBranchRequest,
) -> GitResult<SwitchBranchResult> {
    let branch_name = normalize_branch_name(request.branch_name.as_str(), "switch")?;
    let repository = open_repository(&request.repository_path, "switch")?;
    let previous_branch = current_branch_name(&repository)?;

    enforce_switch_worktree_policy(&repository, SwitchWorktreePolicy::new(request.allow_dirty))?;

    let target_reference_name = resolve_local_branch_reference_name(&repository, branch_name)?;
    checkout_local_branch(&repository, target_reference_name.as_str(), request.force)?;

    Ok(SwitchBranchResult {
        current_branch: branch_name.to_string(),
        previous_branch,
    })
}

fn normalize_branch_name<'a>(branch_name: &'a str, operation: &str) -> GitResult<&'a str> {
    non_empty(branch_name).ok_or_else(|| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("operation `{operation}` requires a non-empty branch name"),
        )
    })
}

fn resolve_local_branch_reference_name(
    repository: &Repository,
    branch_name: &str,
) -> GitResult<String> {
    let branch = repository
        .find_branch(branch_name, BranchType::Local)
        .map_err(|error| {
            if error.code() == ErrorCode::NotFound {
                return GitError::new(
                    GitErrorCode::RefNotFound,
                    format!("local branch `{branch_name}` not found"),
                );
            }
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to resolve local branch `{branch_name}` for switch: {error}"),
            )
        })?;

    branch
        .get()
        .name()
        .and_then(non_empty)
        .map(str::to_owned)
        .ok_or_else(|| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "local branch `{branch_name}` has an empty reference name and cannot be switched"
                ),
            )
        })
}

fn checkout_local_branch(
    repository: &Repository,
    reference_name: &str,
    force: bool,
) -> GitResult<()> {
    let target_object = repository
        .revparse_single(reference_name)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("failed to resolve switch target reference `{reference_name}`: {error}"),
            )
        })?;

    let mut checkout_builder = CheckoutBuilder::new();
    if force {
        checkout_builder.force();
    } else {
        checkout_builder.safe();
    }

    repository
        .checkout_tree(&target_object, Some(&mut checkout_builder))
        .map_err(|error| {
            if !force && error.code() == ErrorCode::Conflict {
                return GitError::new(
                    GitErrorCode::WorktreeDirty,
                    format!(
                        "switch to `{reference_name}` requires clean worktree or explicit force=true: {error}"
                    ),
                );
            }

            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to checkout target reference `{reference_name}` during switch: {error}"
                ),
            )
        })?;

    repository.set_head(reference_name).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to set HEAD to `{reference_name}` after switch checkout: {error}"),
        )
    })
}

fn current_branch_name(repository: &Repository) -> GitResult<Option<String>> {
    match repository.head() {
        Ok(head) => {
            if !head.is_branch() {
                return Ok(None);
            }
            Ok(head.shorthand().and_then(non_empty).map(str::to_owned))
        }
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            Ok(None)
        }
        Err(error) => Err(GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve HEAD branch for repository operation: {error}"),
        )),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

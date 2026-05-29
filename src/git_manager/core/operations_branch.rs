//! `create_branch` operations for `GitManager`.

use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    CreateBranchRequest, CreateBranchResult, GitError, GitErrorCode, GitResult,
};
use git2::{BranchType, ErrorCode, Repository};

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(repository = %request.repository_path.display(), branch = request.branch_name, start_point = ?request.start_point)
    )
)]
pub(super) fn execute_create_branch_operation(
    request: &CreateBranchRequest,
) -> GitResult<CreateBranchResult> {
    let branch_name = normalize_branch_name(request.branch_name.as_str(), "create_branch")?;
    let repository = open_repository(&request.repository_path, "create_branch")?;
    ensure_local_branch_absent(&repository, branch_name)?;
    let start_commit = resolve_start_commit(&repository, request.start_point.as_deref())?;

    repository
        .branch(branch_name, &start_commit, false)
        .map_err(|error| {
            if error.code() == ErrorCode::Exists {
                return branch_already_exists_error(branch_name);
            }
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to create local branch `{branch_name}`: {error}"),
            )
        })?;

    Ok(CreateBranchResult {
        branch_name: branch_name.to_string(),
    })
}

fn normalize_branch_name<'a>(branch_name: &'a str, operation: &str) -> GitResult<&'a str> {
    let branch_name = non_empty(branch_name).ok_or_else(|| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("operation `{operation}` requires a non-empty branch name"),
        )
    })?;
    let fully_qualified_name = format!("refs/heads/{branch_name}");
    if !git2::Reference::is_valid_name(fully_qualified_name.as_str()) {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("operation `{operation}` received invalid branch name `{branch_name}`"),
        ));
    }

    Ok(branch_name)
}

fn resolve_start_commit<'repo>(
    repository: &'repo Repository,
    start_point: Option<&str>,
) -> GitResult<git2::Commit<'repo>> {
    match start_point {
        Some(start_point) => {
            let start_point = non_empty(start_point).ok_or_else(|| {
                GitError::new(
                    GitErrorCode::RefNotFound,
                    "create_branch start_point must not be empty",
                )
            })?;
            resolve_commitish(repository, start_point)
        }
        None => repository
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::RefNotFound,
                    format!("failed to resolve HEAD commit for create_branch: {error}"),
                )
            }),
    }
}

fn resolve_commitish<'repo>(
    repository: &'repo Repository,
    commitish: &str,
) -> GitResult<git2::Commit<'repo>> {
    repository
        .revparse_single(commitish)
        .and_then(|object| object.peel_to_commit())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("failed to resolve start point `{commitish}` to commit: {error}"),
            )
        })
}

fn ensure_local_branch_absent(repository: &Repository, branch_name: &str) -> GitResult<()> {
    match repository.find_branch(branch_name, BranchType::Local) {
        Ok(_) => Err(branch_already_exists_error(branch_name)),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(()),
        Err(error) => Err(GitError::new(
            GitErrorCode::Internal,
            format!("failed to inspect local branch `{branch_name}`: {error}"),
        )),
    }
}

fn branch_already_exists_error(branch_name: &str) -> GitError {
    GitError::new(
        GitErrorCode::BranchAlreadyExists,
        format!("local branch `{branch_name}` already exists"),
    )
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

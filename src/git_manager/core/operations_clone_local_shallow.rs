use super::open_repository;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use crate::git_manager::transport::{TransportError, TransportErrorCode};
use git2::{ErrorCode, Repository};
use std::path::Path;

pub(super) fn should_retry_local_shallow_transport(
    depth: Option<i32>,
    is_local_remote: bool,
    error: &TransportError,
) -> bool {
    const LOCAL_SHALLOW_UNSUPPORTED: &str = "shallow fetch is not supported by the local transport";

    depth.is_some()
        && is_local_remote
        && matches!(
            error.code(),
            TransportErrorCode::TransportNetworkFailure
                | TransportErrorCode::TransportFailure
                | TransportErrorCode::TransportTemporaryNetwork
        )
        && format!("{error}")
            .to_ascii_lowercase()
            .contains(LOCAL_SHALLOW_UNSUPPORTED)
}

pub(super) fn apply_local_shallow_metadata(repository_path: &Path, depth: i32) -> GitResult<()> {
    let repository = open_repository(repository_path, "local shallow fallback")?;
    let mut boundary_commit = resolve_shallow_boundary_commit(&repository)?;
    for _ in 1..depth {
        if boundary_commit.parent_count() == 0 {
            break;
        }
        boundary_commit = boundary_commit.parent(0).map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!("failed to resolve shallow boundary commit for local fallback: {error}"),
            )
        })?;
    }

    let shallow_file_path = repository.path().join("shallow");
    std::fs::write(&shallow_file_path, format!("{}\n", boundary_commit.id())).map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!(
                "failed to write shallow metadata `{}` for local fallback: {error}",
                shallow_file_path.display()
            ),
        )
    })?;

    let reopened_repository = open_repository(repository_path, "local shallow verification")?;
    if !reopened_repository.is_shallow() {
        return Err(GitError::new(
            GitErrorCode::Internal,
            "local shallow fallback did not produce a shallow repository marker",
        ));
    }

    Ok(())
}

fn resolve_shallow_boundary_commit(repository: &Repository) -> GitResult<git2::Commit<'_>> {
    match repository.head().and_then(|head| head.peel_to_commit()) {
        Ok(commit) => Ok(commit),
        Err(error) if error.code() == ErrorCode::UnbornBranch => {
            resolve_fetch_head_commit(repository)
        }
        Err(error) => Err(GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve HEAD commit for local shallow fallback: {error}"),
        )),
    }
}

fn resolve_fetch_head_commit(repository: &Repository) -> GitResult<git2::Commit<'_>> {
    let fetch_head_reference = repository.find_reference("FETCH_HEAD").map_err(|error| {
        GitError::new(
            GitErrorCode::Internal,
            format!("failed to resolve FETCH_HEAD for local shallow fallback: {error}"),
        )
    })?;
    let fetch_head_commit = repository
        .reference_to_annotated_commit(&fetch_head_reference)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                    "failed to convert FETCH_HEAD to annotated commit for local shallow fallback: {error}"
                ),
            )
        })?;

    repository
        .find_commit(fetch_head_commit.id())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::Internal,
                format!(
                "failed to resolve commit `{}` from FETCH_HEAD for local shallow fallback: {error}",
                fetch_head_commit.id()
            ),
            )
        })
}

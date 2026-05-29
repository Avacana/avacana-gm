//! `stage` operations for `GitManager`.

use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, StageRequest, StageResult};
use git2::{Index, IndexAddOption, Repository, StatusOptions};
use std::path::Path;

/// Updates the index in either stage-all mode or selective pathspec mode.
///
/// # Errors
/// Returns a typed `GitError` if the repository is unavailable,
/// a pathspec is empty, or the index could not be updated.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            stage_all = request.is_stage_all(),
            pathspec_count = request.staged_pathspec_count()
        )
    )
)]
pub(super) fn execute_stage_operation(request: &StageRequest) -> GitResult<StageResult> {
    let staged_pathspec_count = request.staged_pathspec_count();

    let repository = open_repository(&request.repository_path, "stage")?;
    let mut index = repository.index().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!(
                "failed to read index for repository `{}`: {error}",
                request.repository_path.display()
            ),
        )
    })?;

    if request.is_stage_all() {
        let deleted_paths = collect_workdir_deletions(&repository, None, &request.repository_path)?;
        index
            .add_all(["*"], IndexAddOption::DEFAULT, None)
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::IndexUpdateFailed,
                    format!(
                        "failed to stage all paths for repository `{}`: {error}",
                        request.repository_path.display()
                    ),
                )
            })?;
        index.update_all(["*"], None).map_err(|error| {
            GitError::new(
                GitErrorCode::IndexUpdateFailed,
                format!(
                    "failed to refresh tracked paths for repository `{}`: {error}",
                    request.repository_path.display()
                ),
            )
        })?;
        remove_deleted_paths_from_index(&mut index, &deleted_paths, &request.repository_path)?;
    } else {
        let Some(pathspecs) = request.selective_pathspecs() else {
            unreachable!("selective_pathspecs must exist when stage mode is selective")
        };

        if pathspecs.is_empty() {
            return Err(GitError::new(
                GitErrorCode::StagePathspecEmpty,
                "stage operation requires at least one pathspec in selective mode",
            ));
        }
        validate_stage_pathspecs(pathspecs)?;
        let deleted_paths =
            collect_workdir_deletions(&repository, Some(pathspecs), &request.repository_path)?;

        index
            .add_all(
                pathspecs.iter().map(String::as_str),
                IndexAddOption::DEFAULT,
                None,
            )
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::IndexUpdateFailed,
                    format!(
                        "failed to stage pathspecs for repository `{}`: {error}",
                        request.repository_path.display()
                    ),
                )
            })?;
        index
            .update_all(pathspecs.iter().map(String::as_str), None)
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::IndexUpdateFailed,
                    format!(
                        "failed to refresh staged pathspecs for repository `{}`: {error}",
                        request.repository_path.display()
                    ),
                )
            })?;
        remove_deleted_paths_from_index(&mut index, &deleted_paths, &request.repository_path)?;
    }

    index.write().map_err(|error| {
        GitError::new(
            GitErrorCode::IndexUpdateFailed,
            format!(
                "failed to persist index for repository `{}`: {error}",
                request.repository_path.display()
            ),
        )
    })?;

    Ok(StageResult {
        staged_pathspec_count,
        index_entry_count: index.len(),
    })
}

fn validate_stage_pathspecs(pathspecs: &[String]) -> GitResult<()> {
    for pathspec in pathspecs {
        if pathspec.is_empty() {
            return Err(GitError::new(
                GitErrorCode::StagePathspecEmpty,
                "stage operation pathspec values must not be empty",
            ));
        }

        if pathspec.contains('\0') {
            return Err(GitError::new(
                GitErrorCode::StagePathspecEmpty,
                "stage operation pathspec values must not contain NUL bytes",
            ));
        }
    }

    Ok(())
}

fn collect_workdir_deletions(
    repository: &Repository,
    pathspecs: Option<&[String]>,
    repository_path: &Path,
) -> GitResult<Vec<String>> {
    let mut status_options = StatusOptions::new();
    status_options
        .include_unmodified(false)
        .include_untracked(false)
        .include_ignored(false);
    if let Some(pathspecs) = pathspecs {
        for pathspec in pathspecs {
            status_options.pathspec(pathspec);
        }
    }

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::IndexUpdateFailed,
                format!(
                    "failed to evaluate staged deletions for repository `{}`: {error}",
                    repository_path.display()
                ),
            )
        })?;
    let mut deleted_paths = Vec::new();
    for status_entry in statuses.iter() {
        if !status_entry.status().is_wt_deleted() {
            continue;
        }
        if let Some(path) = status_entry.path() {
            deleted_paths.push(path.to_string());
        }
    }

    Ok(deleted_paths)
}

fn remove_deleted_paths_from_index(
    index: &mut Index,
    deleted_paths: &[String],
    repository_path: &Path,
) -> GitResult<()> {
    for path in deleted_paths {
        if index.get_path(Path::new(path), 0).is_none() {
            continue;
        }
        index.remove_path(Path::new(path)).map_err(|error| {
            GitError::new(
                GitErrorCode::IndexUpdateFailed,
                format!(
                    "failed to stage deleted path `{path}` for repository `{}`: {error}",
                    repository_path.display()
                ),
            )
        })?;
    }

    Ok(())
}

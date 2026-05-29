use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{Repository, StatusOptions};
use std::path::{Component, Path, PathBuf};

pub(crate) fn add_status_pathspec<T: git2::IntoCString>(
    status_options: &mut StatusOptions,
    pathspec: T,
) {
    status_options.pathspec(pathspec);
}

pub(crate) fn resolve_request_path(
    repository_path: &Path,
    requested_path: &Path,
    field_name: &str,
) -> GitResult<PathBuf> {
    if requested_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not be empty"),
        ));
    }

    let resolved_path = if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        repository_path.join(requested_path)
    };

    Ok(resolved_path)
}

pub(crate) fn resolve_repository_relative_path(
    repository: &Repository,
    requested_path: &Path,
    field_name: &str,
) -> GitResult<PathBuf> {
    if requested_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not be empty"),
        ));
    }

    let repository_relative_path = if requested_path.is_absolute() {
        let workdir = repository.workdir().ok_or_else(|| {
            GitError::new(
                GitErrorCode::AdvancedInvalidInput,
                format!("{field_name} requires non-bare repository workdir"),
            )
        })?;

        requested_path.strip_prefix(workdir).map_err(|_| {
            GitError::new(
                GitErrorCode::AdvancedInvalidInput,
                format!(
                    "{field_name} path `{}` must be inside repository workdir `{}`",
                    requested_path.display(),
                    workdir.display()
                ),
            )
        })?
    } else {
        requested_path
    };

    if repository_relative_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not contain parent path traversals"),
        ));
    }

    if repository_relative_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            format!("{field_name} must not resolve to repository root"),
        ));
    }

    Ok(repository_relative_path.to_path_buf())
}

#[must_use]
pub(crate) fn canonicalize_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[must_use]
pub(crate) fn paths_equivalent(left: &Path, right: &Path) -> bool {
    canonicalize_or_original(left) == canonicalize_or_original(right)
}

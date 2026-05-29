use crate::git_manager::core::normalize_repository_relative_path;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NormalizedRepositoryPath {
    RepositoryRoot,
    RepositoryRelative(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FilesystemPathValidationFailure {
    Empty,
    ContainsNul,
    EscapesRepository,
    OutsideRepository,
    CannotNormalize,
}

pub(super) fn normalize_filesystem_path_to_repository_relative(
    path: &Path,
    repo_root: &Path,
) -> Result<NormalizedRepositoryPath, FilesystemPathValidationFailure> {
    validate_filesystem_input_path(path)?;
    let absolute_path = if path.is_absolute() {
        lexical_normalize_absolute_path(path)
    } else {
        lexical_normalize_absolute_path(&repo_root.join(path))
    }
    .ok_or(FilesystemPathValidationFailure::EscapesRepository)?;

    if absolute_path == repo_root {
        return Ok(NormalizedRepositoryPath::RepositoryRoot);
    }

    let relative_path = absolute_path
        .strip_prefix(repo_root)
        .map_err(|_| FilesystemPathValidationFailure::OutsideRepository)?;

    if relative_path.as_os_str().is_empty() {
        return Ok(NormalizedRepositoryPath::RepositoryRoot);
    }

    let normalized_path = normalize_repository_relative_path(relative_path.to_string_lossy())
        .ok_or(FilesystemPathValidationFailure::CannotNormalize)?;

    Ok(NormalizedRepositoryPath::RepositoryRelative(
        normalized_path,
    ))
}

fn validate_filesystem_input_path(path: &Path) -> Result<(), FilesystemPathValidationFailure> {
    if path.as_os_str().is_empty() {
        return Err(FilesystemPathValidationFailure::Empty);
    }

    if path.to_string_lossy().contains('\0') {
        return Err(FilesystemPathValidationFailure::ContainsNul);
    }

    Ok(())
}

fn lexical_normalize_absolute_path(path: &Path) -> Option<PathBuf> {
    let mut prefix = None::<OsString>;
    let mut has_root = false;
    let mut segments = Vec::<OsString>::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix_component) => {
                prefix = Some(prefix_component.as_os_str().to_os_string());
            }
            Component::RootDir => {
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                segments.pop()?;
            }
            Component::Normal(segment) => {
                segments.push(segment.to_os_string());
            }
        }
    }

    let mut normalized = PathBuf::new();
    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }
    if has_root {
        normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR));
    }
    for segment in segments {
        normalized.push(segment);
    }

    Some(normalized)
}

use super::map_advanced_error;
use crate::git_manager::core::GitResult;
use git2::{ErrorCode, Repository, Submodule};
use std::path::{Path, PathBuf};

pub(crate) fn sync_submodule_recursive(
    submodule: &mut Submodule<'_>,
    parent_path: &Path,
    recursive: bool,
    synced_submodules: &mut Vec<String>,
    skipped_submodules: &mut Vec<String>,
) -> GitResult<()> {
    let submodule_path = join_submodule_path(parent_path, submodule.path());
    let submodule_path_display = submodule_path.display().to_string();

    submodule.sync().map_err(|error| {
        map_advanced_error(
            &error,
            format!("advanced.sync_submodule failed to sync `{submodule_path_display}`"),
        )
    })?;
    synced_submodules.push(submodule_path_display.clone());

    if !recursive {
        return Ok(());
    }

    let nested_repository = match submodule.open() {
        Ok(repository) => repository,
        Err(error) if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) => {
            skipped_submodules.push(format!("{submodule_path_display} (not initialized)"));
            return Ok(());
        }
        Err(error) => {
            return Err(map_advanced_error(
                &error,
                format!("advanced.sync_submodule failed to open `{submodule_path_display}`"),
            ));
        }
    };

    sync_all_nested_submodules(
        &nested_repository,
        submodule_path.as_path(),
        synced_submodules,
        skipped_submodules,
    )
}

fn sync_all_nested_submodules(
    repository: &Repository,
    parent_path: &Path,
    synced_submodules: &mut Vec<String>,
    skipped_submodules: &mut Vec<String>,
) -> GitResult<()> {
    let mut nested_submodules = repository.submodules().map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.sync_submodule failed to list nested submodules for `{}`",
                parent_path.display()
            ),
        )
    })?;

    for nested_submodule in &mut nested_submodules {
        sync_submodule_recursive(
            nested_submodule,
            parent_path,
            true,
            synced_submodules,
            skipped_submodules,
        )?;
    }

    Ok(())
}

fn join_submodule_path(parent_path: &Path, current_path: &Path) -> PathBuf {
    if parent_path.as_os_str().is_empty() {
        current_path.to_path_buf()
    } else {
        parent_path.join(current_path)
    }
}

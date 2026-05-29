use crate::git_manager::core::operations_advanced_support::{
    empty_advanced_result, map_advanced_error, normalize_optional_non_empty,
    sync_submodule_recursive,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{ErrorCode, Repository};
use std::path::Path;

use super::push_boolean_flag;

pub(super) fn execute_sync_submodule_operation(
    repository: &Repository,
    submodule_name: Option<&str>,
    recursive: bool,
) -> GitResult<AdvancedResult> {
    let submodule_name =
        normalize_optional_non_empty(submodule_name, "advanced.sync_submodule.name")?;

    let mut synced_submodules = Vec::new();
    let mut skipped_submodules = Vec::new();

    if let Some(submodule_name) = submodule_name {
        let mut submodule = repository.find_submodule(submodule_name).map_err(|error| {
            if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) {
                GitError::new(
                    GitErrorCode::RefNotFound,
                    format!("advanced.sync_submodule did not find submodule `{submodule_name}`"),
                )
            } else {
                map_advanced_error(
                    &error,
                    format!("advanced.sync_submodule failed to load `{submodule_name}`"),
                )
            }
        })?;

        sync_submodule_recursive(
            &mut submodule,
            Path::new(""),
            recursive,
            &mut synced_submodules,
            &mut skipped_submodules,
        )?;
    } else {
        let mut submodules = repository.submodules().map_err(|error| {
            map_advanced_error(
                &error,
                "advanced.sync_submodule failed to list repository submodules",
            )
        })?;

        for submodule in &mut submodules {
            sync_submodule_recursive(
                submodule,
                Path::new(""),
                recursive,
                &mut synced_submodules,
                &mut skipped_submodules,
            )?;
        }
    }

    let mut result = empty_advanced_result();
    result.changed = !synced_submodules.is_empty();
    push_boolean_flag(&mut result.items, "recursive", recursive);
    if let Some(submodule_name) = submodule_name {
        result.items.push(format!("target:{submodule_name}"));
    } else {
        result.items.push("target:all".to_string());
    }
    result.items.extend(
        synced_submodules
            .iter()
            .map(|path| format!("synced:{path}")),
    );
    result.items.extend(
        skipped_submodules
            .iter()
            .map(|path| format!("skipped:{path}")),
    );

    let recursion_label = if recursive { "recursive " } else { "" };
    result.summary = Some(format!(
        "{recursion_label}submodule sync completed: {} synced, {} skipped",
        synced_submodules.len(),
        skipped_submodules.len()
    ));

    Ok(result)
}

use crate::git_manager::core::operations_advanced_support::{
    empty_advanced_result, map_advanced_error, normalize_optional_non_empty,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{ErrorCode, Repository, Submodule, SubmoduleIgnore, SubmoduleUpdateOptions};
use std::path::{Path, PathBuf};

pub(crate) fn execute_submodule_update_operation(
    repository: &Repository,
    submodule_name: Option<&str>,
    recursive: bool,
    init: bool,
    allow_fetch: bool,
) -> GitResult<AdvancedResult> {
    let submodule_name =
        normalize_optional_non_empty(submodule_name, "advanced.submodule_update.name")?;

    let mut updated_submodules = Vec::new();
    let mut skipped_submodules = Vec::new();
    let mut ignore_rules = Vec::new();

    if let Some(submodule_name) = submodule_name {
        let mut submodule = repository.find_submodule(submodule_name).map_err(|error| {
            if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) {
                GitError::new(
                    GitErrorCode::RefNotFound,
                    format!("advanced.submodule_update did not find submodule `{submodule_name}`"),
                )
            } else {
                map_advanced_error(
                    &error,
                    format!("advanced.submodule_update failed to load `{submodule_name}`"),
                )
            }
        })?;

        update_submodule_recursive(
            &mut submodule,
            Path::new(""),
            recursive,
            init,
            allow_fetch,
            &mut updated_submodules,
            &mut skipped_submodules,
            &mut ignore_rules,
        )?;
    } else {
        let mut submodules = repository.submodules().map_err(|error| {
            map_advanced_error(
                &error,
                "advanced.submodule_update failed to list repository submodules",
            )
        })?;
        for submodule in &mut submodules {
            update_submodule_recursive(
                submodule,
                Path::new(""),
                recursive,
                init,
                allow_fetch,
                &mut updated_submodules,
                &mut skipped_submodules,
                &mut ignore_rules,
            )?;
        }
    }

    let mut result = empty_advanced_result();
    result.changed = !updated_submodules.is_empty();
    result.items.push(format!("recursive:{recursive}"));
    result.items.push(format!("init:{init}"));
    result.items.push(format!("allow_fetch:{allow_fetch}"));
    if let Some(submodule_name) = submodule_name {
        result.items.push(format!("target:{submodule_name}"));
    } else {
        result.items.push("target:all".to_string());
    }
    result.items.extend(
        updated_submodules
            .iter()
            .map(|path| format!("updated:{path}")),
    );
    result.items.extend(
        skipped_submodules
            .iter()
            .map(|path| format!("skipped:{path}")),
    );
    result
        .items
        .extend(ignore_rules.iter().map(|entry| format!("ignore:{entry}")));
    result.summary = Some(format!(
        "submodule update completed: {} updated, {} skipped, {} ignore-rules",
        updated_submodules.len(),
        skipped_submodules.len(),
        ignore_rules.len()
    ));
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn update_submodule_recursive(
    submodule: &mut Submodule<'_>,
    parent_path: &Path,
    recursive: bool,
    init: bool,
    allow_fetch: bool,
    updated_submodules: &mut Vec<String>,
    skipped_submodules: &mut Vec<String>,
    ignore_rules: &mut Vec<String>,
) -> GitResult<()> {
    let submodule_path = join_nested_submodule_path(parent_path, submodule.path());
    let submodule_path_display = submodule_path.display().to_string();
    ignore_rules.push(format!(
        "{submodule_path_display}:{}",
        map_submodule_ignore(submodule.ignore_rule())
    ));

    let mut update_options = SubmoduleUpdateOptions::new();
    update_options.allow_fetch(allow_fetch);
    match submodule.update(init, Some(&mut update_options)) {
        Ok(()) => {
            updated_submodules.push(submodule_path_display.clone());
        }
        Err(error)
            if !init
                && (matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch)
                    || is_submodule_not_initialized(&error)) =>
        {
            skipped_submodules.push(format!("{submodule_path_display} (not initialized)"));
            return Ok(());
        }
        Err(error) => {
            return Err(map_advanced_error(
                &error,
                format!("advanced.submodule_update failed to update `{submodule_path_display}`"),
            ));
        }
    }

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
                format!("advanced.submodule_update failed to open `{submodule_path_display}`"),
            ));
        }
    };

    let mut nested_submodules = nested_repository.submodules().map_err(|error| {
        map_advanced_error(
            &error,
            format!("advanced.submodule_update failed to list nested submodules for `{submodule_path_display}`"),
        )
    })?;
    for nested_submodule in &mut nested_submodules {
        update_submodule_recursive(
            nested_submodule,
            submodule_path.as_path(),
            true,
            init,
            allow_fetch,
            updated_submodules,
            skipped_submodules,
            ignore_rules,
        )?;
    }
    Ok(())
}

fn join_nested_submodule_path(parent_path: &Path, current_path: &Path) -> PathBuf {
    if parent_path.as_os_str().is_empty() {
        current_path.to_path_buf()
    } else {
        parent_path.join(current_path)
    }
}

fn is_submodule_not_initialized(error: &git2::Error) -> bool {
    error
        .message()
        .to_ascii_lowercase()
        .contains("not initialized")
}

pub(crate) const fn map_submodule_ignore(ignore: SubmoduleIgnore) -> &'static str {
    match ignore {
        SubmoduleIgnore::Unspecified => "unspecified",
        SubmoduleIgnore::None => "none",
        SubmoduleIgnore::Untracked => "untracked",
        SubmoduleIgnore::Dirty => "dirty",
        SubmoduleIgnore::All => "all",
    }
}

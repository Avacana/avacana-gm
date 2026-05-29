use crate::git_manager::core::operations_advanced_support::{
    derive_worktree_name, detach_worktree_head, empty_advanced_result, ensure_unique_worktree_name,
    ensure_worktree_clean_before_removal, find_worktree_by_path, map_advanced_error,
    normalize_optional_non_empty, resolve_request_path, resolve_worktree_reference,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{Repository, WorktreeAddOptions, WorktreePruneOptions};
use std::path::Path;

use super::{push_boolean_flag, DEFAULT_DESCRIBE_REVISION};

pub(super) fn execute_add_worktree_operation(
    repository: &Repository,
    repository_path: &Path,
    requested_path: &Path,
    reference: Option<&str>,
    detach: bool,
) -> GitResult<AdvancedResult> {
    let requested_path = resolve_request_path(
        repository_path,
        requested_path,
        "advanced.add_worktree.path",
    )?;
    let reference = normalize_optional_non_empty(reference, "advanced.add_worktree.reference")?;

    if let Some(existing_worktree) = find_worktree_by_path(repository, requested_path.as_path())? {
        let mut result = empty_advanced_result();
        result.summary = Some(format!(
            "worktree already exists at `{}`",
            existing_worktree.path().display()
        ));
        push_boolean_flag(&mut result.items, "detached", detach);
        if let Some(reference) = reference {
            result.items.push(format!("reference:{reference}"));
        }
        if let Some(name) = existing_worktree.name() {
            result.items.push(name.to_string());
        }
        result
            .items
            .push(existing_worktree.path().display().to_string());
        return Ok(result);
    }

    let resolved_reference = match reference {
        Some(reference) => match resolve_worktree_reference(
            repository,
            reference,
            "advanced.add_worktree.reference",
        ) {
            Ok(reference) => Some(reference),
            Err(error) if detach && matches!(error.code(), GitErrorCode::RefNotFound) => None,
            Err(error) => return Err(error),
        },
        None => None,
    };

    let base_name = derive_worktree_name(requested_path.as_path());
    let worktree_name = ensure_unique_worktree_name(repository, base_name.as_str())?;

    let mut add_options = WorktreeAddOptions::new();
    if let Some(reference) = resolved_reference.as_ref() {
        add_options.reference(Some(reference));
    }

    let worktree = repository
        .worktree(
            worktree_name.as_str(),
            requested_path.as_path(),
            Some(&add_options),
        )
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!(
                    "advanced.add_worktree failed to create `{}`",
                    requested_path.display()
                ),
            )
        })?;

    if detach {
        detach_worktree_head(
            repository,
            worktree.path(),
            reference,
            DEFAULT_DESCRIBE_REVISION,
        )?;
    }

    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some(format!(
        "worktree `{worktree_name}` added at `{}`",
        worktree.path().display()
    ));
    result.items.push(worktree_name);
    result.items.push(worktree.path().display().to_string());
    push_boolean_flag(&mut result.items, "detached", detach);
    if let Some(reference) = reference {
        result.items.push(format!("reference:{reference}"));
    }

    Ok(result)
}

pub(super) fn execute_remove_worktree_operation(
    repository: &Repository,
    repository_path: &Path,
    requested_path: &Path,
    force: bool,
) -> GitResult<AdvancedResult> {
    let requested_path = resolve_request_path(
        repository_path,
        requested_path,
        "advanced.remove_worktree.path",
    )?;

    let worktree =
        find_worktree_by_path(repository, requested_path.as_path())?.ok_or_else(|| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "advanced.remove_worktree could not resolve worktree by path `{}`",
                    requested_path.display()
                ),
            )
        })?;

    let worktree_name = worktree
        .name()
        .map_or_else(|| "<unknown>".to_string(), str::to_owned);
    ensure_worktree_clean_before_removal(&worktree, force)?;
    let mut prune_options = WorktreePruneOptions::new();
    prune_options.valid(true).working_tree(true);
    if force {
        prune_options.locked(true);
    }
    worktree.prune(Some(&mut prune_options)).map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.remove_worktree failed for `{}`",
                requested_path.display()
            ),
        )
    })?;
    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some(format!("worktree `{worktree_name}` removed"));
    result.items.push(worktree_name);
    result.items.push(requested_path.display().to_string());
    push_boolean_flag(&mut result.items, "force", force);
    Ok(result)
}

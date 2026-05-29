use super::map_advanced_error;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{Repository, StatusOptions, Worktree, WorktreeLockStatus};
use std::path::Path;

pub(crate) fn append_worktree_lock_status(
    result: &mut crate::git_manager::core::AdvancedResult,
    lock_status: WorktreeLockStatus,
) {
    match lock_status {
        WorktreeLockStatus::Unlocked => {
            result.items.push("lock_status:unlocked".to_string());
        }
        WorktreeLockStatus::Locked(reason) => {
            result.items.push("lock_status:locked".to_string());
            if let Some(reason) = reason {
                result.items.push(format!("lock_reason:{reason}"));
            }
        }
    }
}

pub(crate) fn find_worktree_by_path(
    repository: &Repository,
    target_path: &Path,
) -> GitResult<Option<Worktree>> {
    let worktree_names = repository.worktrees().map_err(|error| {
        map_advanced_error(
            &error,
            "advanced.worktree lookup failed to list known worktrees",
        )
    })?;

    for worktree_name in worktree_names.iter().flatten() {
        let worktree = repository.find_worktree(worktree_name).map_err(|error| {
            map_advanced_error(
                &error,
                format!("advanced.worktree lookup failed to open `{worktree_name}`"),
            )
        })?;

        if super::path_support::paths_equivalent(worktree.path(), target_path) {
            return Ok(Some(worktree));
        }
    }

    Ok(None)
}

pub(crate) fn ensure_worktree_clean_before_removal(
    worktree: &Worktree,
    force: bool,
) -> GitResult<()> {
    if force {
        return Ok(());
    }

    let worktree_path = worktree.path();
    let worktree_repository = Repository::open(worktree_path).map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.remove_worktree failed to open target worktree `{}`",
                worktree_path.display()
            ),
        )
    })?;

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = worktree_repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!(
                    "advanced.remove_worktree failed to inspect worktree status for `{}`",
                    worktree_path.display()
                ),
            )
        })?;

    if statuses.is_empty() {
        return Ok(());
    }

    Err(GitError::new(
        GitErrorCode::WorktreeDirty,
        format!(
            "advanced.remove_worktree refused to remove dirty worktree `{}` ({} change(s)); retry with force=true",
            worktree_path.display(),
            statuses.len()
        ),
    ))
}

#[must_use]
pub(crate) fn derive_worktree_name(path: &Path) -> String {
    let base_name = path
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .unwrap_or("worktree");

    let mut normalized_name = String::with_capacity(base_name.len());
    for character in base_name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            normalized_name.push(character);
        } else {
            normalized_name.push('-');
        }
    }

    let normalized_name = normalized_name.trim_matches('-');
    if normalized_name.is_empty() {
        "worktree".to_string()
    } else {
        normalized_name.to_string()
    }
}

pub(crate) fn ensure_unique_worktree_name(
    repository: &Repository,
    base_name: &str,
) -> GitResult<String> {
    match repository.find_worktree(base_name) {
        Ok(_) => {}
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::NotFound | git2::ErrorCode::UnbornBranch
            ) =>
        {
            return Ok(base_name.to_string());
        }
        Err(error) => {
            return Err(map_advanced_error(
                &error,
                format!("advanced.add_worktree failed to inspect candidate name `{base_name}`"),
            ));
        }
    }

    let mut suffix = 1_u32;
    loop {
        let candidate_name = format!("{base_name}-{suffix}");
        match repository.find_worktree(candidate_name.as_str()) {
            Ok(_) => {
                suffix += 1;
            }
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::NotFound | git2::ErrorCode::UnbornBranch
                ) =>
            {
                return Ok(candidate_name);
            }
            Err(error) => {
                return Err(map_advanced_error(
                    &error,
                    format!(
                        "advanced.add_worktree failed to inspect candidate name `{candidate_name}`"
                    ),
                ));
            }
        }
    }
}

pub(crate) fn resolve_worktree_reference<'repo>(
    repository: &'repo Repository,
    reference: &str,
    field_name: &str,
) -> GitResult<git2::Reference<'repo>> {
    if let Ok(resolved_reference) = repository.find_reference(reference) {
        return Ok(resolved_reference);
    }

    repository
        .resolve_reference_from_short_name(reference)
        .map_err(|error| {
            let code = if matches!(
                error.code(),
                git2::ErrorCode::NotFound
                    | git2::ErrorCode::UnbornBranch
                    | git2::ErrorCode::InvalidSpec
                    | git2::ErrorCode::Ambiguous
            ) {
                GitErrorCode::RefNotFound
            } else {
                GitErrorCode::AdvancedOperationFailed
            };

            GitError::new(
                code,
                format!("{field_name} failed to resolve worktree reference `{reference}`: {error}"),
            )
        })
}

fn resolve_revision_to_commit_oid(
    repository: &Repository,
    revision: &str,
    field_name: &str,
) -> GitResult<git2::Oid> {
    let object = repository.revparse_single(revision).map_err(|error| {
        map_advanced_error(
            &error,
            format!("{field_name} failed to resolve revision `{revision}`"),
        )
    })?;

    object
        .peel_to_commit()
        .map(|commit| commit.id())
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!("{field_name} revision `{revision}` is not commit-like"),
            )
        })
}

pub(crate) fn detach_worktree_head(
    repository: &Repository,
    worktree_path: &Path,
    reference: Option<&str>,
    fallback_revision: &str,
) -> GitResult<()> {
    let reference_spec = reference.unwrap_or(fallback_revision);
    let target_commit_oid = resolve_revision_to_commit_oid(
        repository,
        reference_spec,
        "advanced.add_worktree.reference",
    )?;

    let worktree_repository = Repository::open(worktree_path).map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.add_worktree failed to open newly created worktree `{}`",
                worktree_path.display()
            ),
        )
    })?;

    worktree_repository
        .set_head_detached(target_commit_oid)
        .map_err(|error| {
            map_advanced_error(
                &error,
                format!(
                    "advanced.add_worktree failed to detach HEAD for `{}`",
                    worktree_path.display()
                ),
            )
        })?;

    worktree_repository.checkout_head(None).map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.add_worktree failed to checkout detached HEAD in `{}`",
                worktree_path.display()
            ),
        )
    })?;

    Ok(())
}

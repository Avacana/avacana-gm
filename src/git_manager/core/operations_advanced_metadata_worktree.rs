use crate::git_manager::core::operations_advanced_support::{
    append_worktree_lock_status, empty_advanced_result, find_worktree_by_path, map_advanced_error,
    normalize_non_empty, normalize_optional_non_empty, resolve_request_path,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::Repository;
use std::path::Path;

pub(crate) fn execute_worktree_lock_operation(
    repository: &Repository,
    repository_path: &Path,
    requested_path: &Path,
    action: &str,
    reason: Option<&str>,
) -> GitResult<AdvancedResult> {
    let requested_path = resolve_request_path(
        repository_path,
        requested_path,
        "advanced.worktree_lock.path",
    )?;
    let action = normalize_non_empty(action, "advanced.worktree_lock.action")?;
    let reason = normalize_optional_non_empty(reason, "advanced.worktree_lock.reason")?;

    let worktree =
        find_worktree_by_path(repository, requested_path.as_path())?.ok_or_else(|| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!(
                    "advanced.worktree_lock could not resolve worktree by path `{}`",
                    requested_path.display()
                ),
            )
        })?;
    let worktree_name = worktree
        .name()
        .map_or_else(|| "<unknown>".to_string(), str::to_owned);
    let action = action.to_ascii_lowercase();

    let mut result = empty_advanced_result();
    result.items.push(format!("action:{action}"));
    result.items.push(worktree_name.clone());
    result.items.push(worktree.path().display().to_string());

    match action.as_str() {
        "lock" => {
            worktree.lock(reason).map_err(|error| {
                map_advanced_error(
                    &error,
                    format!(
                        "advanced.worktree_lock failed to lock `{}`",
                        worktree.path().display()
                    ),
                )
            })?;
            result.changed = true;
            if let Some(reason) = reason {
                result.items.push(format!("reason:{reason}"));
            }
            result.summary = Some(format!("worktree `{worktree_name}` locked"));
        }
        "unlock" => {
            worktree.unlock().map_err(|error| {
                map_advanced_error(
                    &error,
                    format!(
                        "advanced.worktree_lock failed to unlock `{}`",
                        worktree.path().display()
                    ),
                )
            })?;
            result.changed = true;
            result.summary = Some(format!("worktree `{worktree_name}` unlocked"));
        }
        "query" => {
            result.summary = Some(format!("worktree `{worktree_name}` lock status queried"));
        }
        _ => {
            return Err(GitError::new(
                GitErrorCode::AdvancedInvalidInput,
                "advanced.worktree_lock.action must be `lock`, `unlock`, or `query`",
            ));
        }
    }

    let lock_status = worktree.is_locked().map_err(|error| {
        map_advanced_error(
            &error,
            format!(
                "advanced.worktree_lock failed to read lock status for `{}`",
                worktree.path().display()
            ),
        )
    })?;
    append_worktree_lock_status(&mut result, lock_status);

    Ok(result)
}

use crate::git_manager::core::operations_advanced_support::{
    empty_advanced_result, map_advanced_error, normalize_optional_non_empty,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{
    ErrorCode, Repository, StashApplyOptions, StashFlags, StashSaveOptions, Status, StatusOptions,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::push_boolean_flag;

pub(super) fn execute_stash_save_operation(
    repository: &mut Repository,
    message: Option<&str>,
    include_untracked: bool,
    keep_index: bool,
) -> GitResult<AdvancedResult> {
    let message = normalize_optional_non_empty(message, "advanced.stash_save.message")?;
    let signature = repository.signature().map_err(|error| {
        GitError::new(
            GitErrorCode::InvalidSignatureContext,
            format!("advanced.stash_save failed to resolve repository signature: {error}"),
        )
    })?;

    let mut stash_flags = StashFlags::empty();
    if include_untracked {
        stash_flags |= StashFlags::INCLUDE_UNTRACKED;
    }
    if keep_index {
        stash_flags |= StashFlags::KEEP_INDEX;
    }

    let stash_result = if let Some(message) = message {
        repository.stash_save2(&signature, Some(message), Some(stash_flags))
    } else {
        let mut save_options = StashSaveOptions::new(signature);
        save_options.flags(Some(stash_flags));
        repository.stash_save_ext(Some(&mut save_options))
    };

    let stash_oid = match stash_result {
        Ok(stash_oid) => stash_oid,
        Err(error) if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) => {
            let mut result = empty_advanced_result();
            result.summary = Some("stash save skipped: no local changes".to_string());
            push_boolean_flag(&mut result.items, "include_untracked", include_untracked);
            push_boolean_flag(&mut result.items, "keep_index", keep_index);
            if let Some(message) = message {
                result.items.push(format!("message:{message}"));
            }
            return Ok(result);
        }
        Err(error) => {
            return Err(map_advanced_error(
                &error,
                "advanced.stash_save failed to create stash entry",
            ));
        }
    };

    if include_untracked {
        cleanup_untracked_workdir_after_stash(repository)?;
    }

    let mut stash_index = None;
    repository
        .stash_foreach(|index, _, oid| {
            if *oid == stash_oid {
                stash_index = Some(index);
                false
            } else {
                true
            }
        })
        .map_err(|error| {
            map_advanced_error(&error, "advanced.stash_save failed to inspect stash stack")
        })?;

    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some("stash entry saved".to_string());
    result.items.push(stash_oid.to_string());
    push_boolean_flag(&mut result.items, "include_untracked", include_untracked);
    push_boolean_flag(&mut result.items, "keep_index", keep_index);
    if let Some(message) = message {
        result.items.push(format!("message:{message}"));
    }
    if let Some(stash_index) = stash_index {
        result.items.push(format!("stash@{{{stash_index}}}"));
    }

    Ok(result)
}

fn cleanup_untracked_workdir_after_stash(repository: &Repository) -> GitResult<()> {
    let workdir = repository.workdir().ok_or_else(|| {
        GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            "advanced.stash_save include_untracked requires non-bare repository workdir",
        )
    })?;

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            map_advanced_error(
                &error,
                "advanced.stash_save failed to inspect workdir after include_untracked stash",
            )
        })?;

    let mut untracked_paths = statuses
        .iter()
        .filter_map(|entry| {
            if entry.status().contains(Status::WT_NEW) {
                entry.path().map(|path| workdir.join(path))
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>();
    untracked_paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    untracked_paths.dedup();

    for path in untracked_paths {
        if !path.exists() {
            continue;
        }

        let removal_result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if let Err(error) = removal_result {
            return Err(GitError::new(
                GitErrorCode::AdvancedOperationFailed,
                format!(
                    "advanced.stash_save failed to remove untracked path `{}` after include_untracked stash: {error}",
                    path.display()
                ),
            ));
        }
    }

    Ok(())
}

pub(super) fn execute_stash_list_operation(
    repository: &mut Repository,
) -> GitResult<AdvancedResult> {
    let mut result = empty_advanced_result();
    let stash_callback: &mut git2::StashCb<'_> = &mut |index, message, oid| {
        result
            .items
            .push(format!("stash@{{{index}}}: {oid} {message}"));
        true
    };

    repository.stash_foreach(stash_callback).map_err(|error| {
        map_advanced_error(
            &error,
            "advanced.stash_list failed to iterate stash entries",
        )
    })?;

    result.summary = Some(format!("stash entries listed: {}", result.items.len()));
    Ok(result)
}

pub(super) fn execute_stash_apply_operation(
    repository: &mut Repository,
    index: usize,
    reinstate_index: bool,
    pop: bool,
) -> GitResult<AdvancedResult> {
    let mut apply_options = StashApplyOptions::new();
    let progress_stages = Rc::new(RefCell::new(Vec::new()));
    {
        let progress_stages = Rc::clone(&progress_stages);
        apply_options.progress_cb(move |progress| {
            progress_stages.borrow_mut().push(format!("{progress:?}"));
            true
        });
    }
    if reinstate_index {
        apply_options.reinstantiate_index();
    }

    if pop {
        repository
            .stash_pop(index, Some(&mut apply_options))
            .map_err(|error| {
                map_advanced_error(
                    &error,
                    format!("advanced.stash_pop failed for stash@{{{index}}}"),
                )
            })?;
    } else {
        repository
            .stash_apply(index, Some(&mut apply_options))
            .map_err(|error| {
                map_advanced_error(
                    &error,
                    format!("advanced.stash_apply failed for stash@{{{index}}}"),
                )
            })?;
    }

    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some(if pop {
        format!("stash@{{{index}}} popped")
    } else {
        format!("stash@{{{index}}} applied")
    });
    result.items.push(format!("stash@{{{index}}}"));
    push_boolean_flag(&mut result.items, "reinstate_index", reinstate_index);
    result.items.extend(
        progress_stages
            .borrow()
            .iter()
            .map(|stage| format!("progress:{stage}")),
    );
    result
        .items
        .push(format!("mode:{}", if pop { "pop" } else { "apply" }));
    Ok(result)
}

pub(super) fn execute_stash_drop_operation(
    repository: &mut Repository,
    index: usize,
) -> GitResult<AdvancedResult> {
    repository.stash_drop(index).map_err(|error| {
        map_advanced_error(
            &error,
            format!("advanced.stash_drop failed for stash@{{{index}}}"),
        )
    })?;

    let mut result = empty_advanced_result();
    result.changed = true;
    result.summary = Some(format!("stash@{{{index}}} dropped"));
    result.items.push(format!("stash@{{{index}}}"));
    result.items.push("mode:drop".to_string());
    Ok(result)
}

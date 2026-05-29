use super::is_absent_head_error;
use super::mapping_operations::{delta_path, map_delta_to_diff_status_code, status_entry_path};
use super::pathspec_operations::{
    apply_pathspecs_to_diff_options, apply_pathspecs_to_status_options,
};
use super::status_diff_error;
use crate::git_manager::core::{DiffStatusCode, DiffStatusEntry, GitResult, StatusScope};
use git2::{Deltas, Diff, DiffFindOptions, DiffOptions, Repository, StatusOptions};

pub(super) fn collect_scope_entries(
    repository: &Repository,
    scope: StatusScope,
    pathspecs: &[String],
) -> GitResult<Vec<DiffStatusEntry>> {
    match scope {
        StatusScope::Staged => collect_staged_entries(repository, pathspecs),
        StatusScope::Unstaged => collect_unstaged_entries(repository, pathspecs),
        StatusScope::Untracked => collect_untracked_entries(repository, pathspecs),
        StatusScope::All => {
            let mut entries = collect_staged_entries(repository, pathspecs)?;
            entries.extend(collect_unstaged_entries(repository, pathspecs)?);
            entries.extend(collect_untracked_entries(repository, pathspecs)?);
            entries.extend(collect_conflicted_entries(repository, pathspecs)?);
            Ok(entries)
        }
    }
}

fn collect_staged_entries(
    repository: &Repository,
    pathspecs: &[String],
) -> GitResult<Vec<DiffStatusEntry>> {
    Ok(collect_diff_entries(&build_staged_diff(
        repository, pathspecs,
    )?))
}

fn collect_unstaged_entries(
    repository: &Repository,
    pathspecs: &[String],
) -> GitResult<Vec<DiffStatusEntry>> {
    Ok(collect_diff_entries(&build_unstaged_diff(
        repository, pathspecs,
    )?))
}

fn collect_untracked_entries(
    repository: &Repository,
    pathspecs: &[String],
) -> GitResult<Vec<DiffStatusEntry>> {
    Ok(collect_statuses(repository, pathspecs)?
        .iter()
        .filter(|entry| entry.status().is_wt_new())
        .map(|entry| DiffStatusEntry {
            path: status_entry_path(&entry),
            code: DiffStatusCode::Untracked,
        })
        .collect())
}

fn collect_conflicted_entries(
    repository: &Repository,
    pathspecs: &[String],
) -> GitResult<Vec<DiffStatusEntry>> {
    Ok(collect_statuses(repository, pathspecs)?
        .iter()
        .filter(|entry| entry.status().is_conflicted())
        .map(|entry| DiffStatusEntry {
            path: status_entry_path(&entry),
            code: DiffStatusCode::Conflicted,
        })
        .collect())
}

fn collect_statuses<'repo>(
    repository: &'repo Repository,
    pathspecs: &[String],
) -> GitResult<git2::Statuses<'repo>> {
    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .renames_from_rewrites(true)
        .rename_threshold(50);
    apply_pathspecs_to_status_options(&mut status_options, pathspecs);
    repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            status_diff_error(format!(
                "failed to collect status list for repository `{}`: {error}",
                repository.path().display()
            ))
        })
}

pub(super) fn build_staged_diff<'repo>(
    repository: &'repo Repository,
    pathspecs: &[String],
) -> GitResult<Diff<'repo>> {
    let head_tree = resolve_head_tree(repository)?;
    let mut diff_options = DiffOptions::new();
    diff_options.include_typechange(true);
    apply_pathspecs_to_diff_options(&mut diff_options, pathspecs);
    let mut staged_diff = repository
        .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_options))
        .map_err(|error| {
            status_diff_error(format!(
                "failed to build staged diff (`HEAD -> index`) for repository `{}`: {error}",
                repository.path().display()
            ))
        })?;
    enable_rename_detection(&mut staged_diff)?;
    Ok(staged_diff)
}

pub(super) fn build_unstaged_diff<'repo>(
    repository: &'repo Repository,
    pathspecs: &[String],
) -> GitResult<Diff<'repo>> {
    let mut diff_options = DiffOptions::new();
    diff_options.include_typechange(true);
    apply_pathspecs_to_diff_options(&mut diff_options, pathspecs);
    let mut unstaged_diff = repository
        .diff_index_to_workdir(None, Some(&mut diff_options))
        .map_err(|error| {
            status_diff_error(format!(
                "failed to build unstaged diff (`index -> workdir`) for repository `{}`: {error}",
                repository.path().display()
            ))
        })?;
    enable_rename_detection(&mut unstaged_diff)?;
    Ok(unstaged_diff)
}

fn enable_rename_detection(diff: &mut Diff<'_>) -> GitResult<()> {
    let mut find_options = DiffFindOptions::new();
    find_options
        .renames(true)
        .copies(true)
        .copies_from_unmodified(true)
        .renames_from_rewrites(true)
        .rename_threshold(50);
    diff.find_similar(Some(&mut find_options)).map_err(|error| {
        status_diff_error(format!(
            "failed to run rename detection for diff result: {error}"
        ))
    })
}

fn collect_diff_entries(diff: &Diff<'_>) -> Vec<DiffStatusEntry> {
    let deltas: Deltas<'_> = diff.deltas();
    deltas
        .filter_map(|delta| {
            let code = map_delta_to_diff_status_code(delta.status())?;
            let path = delta_path(&delta)?;
            Some(DiffStatusEntry { path, code })
        })
        .collect()
}

fn resolve_head_tree(repository: &Repository) -> GitResult<Option<git2::Tree<'_>>> {
    match repository.head() {
        Ok(head) => head.peel_to_tree().map(Some).map_err(|error| {
            status_diff_error(format!(
                "failed to resolve HEAD tree for repository `{}`: {error}",
                repository.path().display()
            ))
        }),
        Err(error) if is_absent_head_error(&error) => Ok(None),
        Err(error) => Err(status_diff_error(format!(
            "failed to resolve HEAD for repository `{}`: {error}",
            repository.path().display()
        ))),
    }
}

use super::map_query_error;
use crate::git_manager::core::{DiffStatusCode, GitResult, QueryChangeEntry};
use git2::{Delta, Diff, DiffFindOptions, DiffOptions, Repository};

pub(crate) fn collect_commit_change_entries(
    repository: &Repository,
    commit: &git2::Commit<'_>,
) -> GitResult<Vec<QueryChangeEntry>> {
    let commit_tree = commit.tree().map_err(|error| {
        map_query_error(
            &error,
            format!(
                "query_lifecycle failed to resolve tree for commit `{}`",
                commit.id()
            ),
        )
    })?;

    let parent_tree = if commit.parent_count() == 0 {
        None
    } else {
        Some(
            commit
                .parent(0)
                .and_then(|parent| parent.tree())
                .map_err(|error| {
                    map_query_error(
                        &error,
                        format!(
                            "query_lifecycle failed to resolve parent tree for commit `{}`",
                            commit.id()
                        ),
                    )
                })?,
        )
    };

    let mut diff_options = DiffOptions::new();
    diff_options.include_typechange(true);
    let mut diff = repository
        .diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_options),
        )
        .map_err(|error| {
            map_query_error(
                &error,
                format!(
                    "query_lifecycle failed to collect tree diff for commit `{}`",
                    commit.id()
                ),
            )
        })?;

    enable_rename_detection(&mut diff)?;

    Ok(diff
        .deltas()
        .filter_map(|delta| {
            let status = map_delta_to_diff_status_code(delta.status())?;
            let path = delta_path(&delta)?;
            Some(QueryChangeEntry {
                commit_oid: commit.id().to_string(),
                path,
                status,
            })
        })
        .collect())
}

fn enable_rename_detection(diff: &mut Diff<'_>) -> GitResult<()> {
    let mut find_options = DiffFindOptions::new();
    find_options
        .renames(true)
        .renames_from_rewrites(true)
        .rename_threshold(50);

    diff.find_similar(Some(&mut find_options)).map_err(|error| {
        map_query_error(&error, "query_lifecycle failed to detect renames in diff")
    })
}

const fn map_delta_to_diff_status_code(delta: Delta) -> Option<DiffStatusCode> {
    match delta {
        Delta::Added => Some(DiffStatusCode::Added),
        Delta::Modified => Some(DiffStatusCode::Modified),
        Delta::Deleted => Some(DiffStatusCode::Deleted),
        Delta::Renamed => Some(DiffStatusCode::Renamed),
        Delta::Copied => Some(DiffStatusCode::Copied),
        Delta::Typechange => Some(DiffStatusCode::TypeChange),
        Delta::Untracked => Some(DiffStatusCode::Untracked),
        Delta::Conflicted => Some(DiffStatusCode::Conflicted),
        Delta::Unmodified | Delta::Ignored | Delta::Unreadable => None,
    }
}

fn delta_path(delta: &git2::DiffDelta<'_>) -> Option<String> {
    let raw_path = if matches!(delta.status(), Delta::Deleted) {
        delta.old_file().path().or_else(|| delta.new_file().path())
    } else {
        delta.new_file().path().or_else(|| delta.old_file().path())
    }?;

    Some(raw_path.to_string_lossy().into_owned())
}

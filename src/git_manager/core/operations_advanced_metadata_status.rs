use crate::git_manager::core::operations_advanced_support::{
    add_status_pathspec, empty_advanced_result, map_advanced_error, normalize_non_empty,
    normalize_optional_non_empty,
};
use crate::git_manager::core::{AdvancedResult, GitError, GitErrorCode, GitResult};
use git2::{Repository, StatusOptions, StatusShow};

pub(crate) fn execute_status_scan_operation(
    repository: &Repository,
    show: &str,
    pathspec: Option<&str>,
    include_untracked: bool,
) -> GitResult<AdvancedResult> {
    let show = normalize_non_empty(show, "advanced.status_scan.show")?;
    let pathspec = normalize_optional_non_empty(pathspec, "advanced.status_scan.pathspec")?;
    let status_show = parse_status_show(show)?;

    let mut status_options = StatusOptions::new();
    status_options.show(status_show);
    if include_untracked {
        status_options
            .include_untracked(true)
            .recurse_untracked_dirs(true);
    }
    if let Some(pathspec) = pathspec {
        add_status_pathspec(&mut status_options, pathspec);
    }

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            map_advanced_error(
                &error,
                "advanced.status_scan failed to read repository status",
            )
        })?;

    let mut result = empty_advanced_result();
    result.changed = !statuses.is_empty();
    result.items.push(format!("show:{show}"));
    result
        .items
        .push(format!("include_untracked:{include_untracked}"));
    if let Some(pathspec) = pathspec {
        result.items.push(format!("pathspec:{pathspec}"));
    }

    let status_iter: git2::StatusIter<'_> = statuses.iter();
    for entry in status_iter {
        if let Some(path) = entry.path() {
            result
                .items
                .push(format!("status:{path}:{:?}", entry.status()));
        }
    }

    append_repository_time_diagnostics(repository, &mut result)?;
    result.summary = Some(format!("status scan completed: {} entries", statuses.len()));
    Ok(result)
}

fn parse_status_show(show: &str) -> GitResult<StatusShow> {
    match show.to_ascii_lowercase().as_str() {
        "index" => Ok(StatusShow::Index),
        "workdir" => Ok(StatusShow::Workdir),
        "all" | "index_and_workdir" => Ok(StatusShow::IndexAndWorkdir),
        _ => Err(GitError::new(
            GitErrorCode::AdvancedInvalidInput,
            "advanced.status_scan.show must be `index`, `workdir`, or `all`",
        )),
    }
}

fn append_repository_time_diagnostics(
    repository: &Repository,
    result: &mut AdvancedResult,
) -> GitResult<()> {
    if let Ok(head_commit) = repository.head().and_then(|head| head.peel_to_commit()) {
        let signature_time: git2::Time = head_commit.time();
        result
            .items
            .push(format!("head_time_seconds:{}", signature_time.seconds()));
        result.items.push(format!(
            "head_time_offset:{}",
            signature_time.offset_minutes()
        ));
        result
            .items
            .push(format!("head_time_sign:{}", signature_time.sign()));
    }

    let index = repository.index().map_err(|error| {
        map_advanced_error(
            &error,
            "advanced.status_scan failed to open repository index for time diagnostics",
        )
    })?;
    if let Some(first_entry) = index.iter().next() {
        let ctime: git2::IndexTime = first_entry.ctime;
        let mtime: git2::IndexTime = first_entry.mtime;
        result
            .items
            .push(format!("index_ctime_s:{}", ctime.seconds()));
        result
            .items
            .push(format!("index_ctime_ns:{}", ctime.nanoseconds()));
        result
            .items
            .push(format!("index_mtime_s:{}", mtime.seconds()));
        result
            .items
            .push(format!("index_mtime_ns:{}", mtime.nanoseconds()));
    }

    Ok(())
}

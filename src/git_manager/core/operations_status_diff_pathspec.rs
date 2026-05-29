use super::invalid_pathspec_error;
use super::mapping_operations::{delta_path, map_delta_to_diff_status_code};
use crate::git_manager::core::{GitResult, PathspecDiffEntryDetails, PathspecMatchDetails};
use git2::{
    Diff, DiffOptions, Pathspec, PathspecDiffEntries, PathspecEntries, PathspecFailedEntries,
    PathspecFlags, PathspecMatchList, Repository, StatusOptions,
};

pub(super) fn build_pathspec_diff_details(
    diff: &Diff<'_>,
    pathspecs: &[String],
) -> GitResult<PathspecMatchDetails> {
    let compiled_pathspec = compile_pathspec(pathspecs)?;
    let flags = PathspecFlags::DEFAULT | PathspecFlags::FIND_FAILURES;
    let match_list: PathspecMatchList<'_> =
        compiled_pathspec.match_diff(diff, flags).map_err(|error| {
            invalid_pathspec_error(format!(
                "failed to match pathspec against diff entries: {error}"
            ))
        })?;
    Ok(pathspec_match_details(pathspecs, &match_list))
}

pub(super) fn build_pathspec_workdir_details(
    repository: &Repository,
    pathspecs: &[String],
) -> GitResult<PathspecMatchDetails> {
    let compiled_pathspec = compile_pathspec(pathspecs)?;
    let flags = PathspecFlags::DEFAULT | PathspecFlags::FIND_FAILURES;
    let match_list: PathspecMatchList<'_> = compiled_pathspec
        .match_workdir(repository, flags)
        .map_err(|error| {
            invalid_pathspec_error(format!(
                "failed to match pathspec against workdir entries: {error}"
            ))
        })?;
    Ok(pathspec_match_details(pathspecs, &match_list))
}

fn pathspec_match_details(
    pathspecs: &[String],
    match_list: &PathspecMatchList<'_>,
) -> PathspecMatchDetails {
    let entries_iter: PathspecEntries<'_> = match_list.entries();
    let failed_iter: PathspecFailedEntries<'_> = match_list.failed_entries();
    let diff_entries_iter: PathspecDiffEntries<'_> = match_list.diff_entries();
    PathspecMatchDetails {
        patterns: pathspecs.to_vec(),
        entries: entries_iter.map(bytes_to_string).collect(),
        failed_entries: failed_iter.map(bytes_to_string).collect(),
        diff_entries: diff_entries_iter
            .filter_map(|delta| {
                let code = map_delta_to_diff_status_code(delta.status())?;
                let path = delta_path(&delta)?;
                Some(PathspecDiffEntryDetails { path, code })
            })
            .collect(),
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(super) fn compile_pathspec(pathspecs: &[String]) -> GitResult<Pathspec> {
    Pathspec::new(pathspecs.iter().map(String::as_str)).map_err(|error| {
        invalid_pathspec_error(format!(
            "failed to compile pathspec set for status/diff operation: {error}"
        ))
    })
}

pub(super) fn apply_pathspecs_to_diff_options(
    diff_options: &mut DiffOptions,
    pathspecs: &[String],
) {
    for pathspec in pathspecs {
        diff_options.pathspec(pathspec.as_str());
    }
}

pub(super) fn apply_pathspecs_to_status_options(
    status_options: &mut StatusOptions,
    pathspecs: &[String],
) {
    for pathspec in pathspecs {
        status_options.pathspec(pathspec.as_str());
    }
}

pub(super) fn validate_pathspecs(pathspecs: &[String]) -> GitResult<()> {
    for pathspec in pathspecs {
        if pathspec.is_empty() {
            return Err(invalid_pathspec_error(
                "status/diff pathspec values must not be empty",
            ));
        }
        if pathspec.contains('\0') {
            return Err(invalid_pathspec_error(
                "status/diff pathspec values must not contain NUL bytes",
            ));
        }
    }
    Ok(())
}

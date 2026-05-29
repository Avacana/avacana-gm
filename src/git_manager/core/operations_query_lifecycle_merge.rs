use super::history_operations::resolve_revision_commit;
use crate::git_manager::core::operations_query_lifecycle_support::{
    map_query_error, normalize_non_empty,
};
use crate::git_manager::core::query_lifecycle_contracts::QueryMergeFilePreview;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, QueryLifecycleResult};
use git2::{MergeFileOptions, MergeFileResult, Repository};
use std::path::Path;

pub(super) fn execute_merge_file_preview_operation(
    repository: &Repository,
    path: &str,
    ours: &str,
    theirs: &str,
) -> GitResult<QueryLifecycleResult> {
    let path = normalize_non_empty(path, "query_lifecycle.merge_file.path")?;
    let ours = normalize_non_empty(ours, "query_lifecycle.merge_file.ours")?;
    let theirs = normalize_non_empty(theirs, "query_lifecycle.merge_file.theirs")?;
    let ours_commit = resolve_revision_commit(repository, ours, "query_lifecycle.merge_file")?;
    let theirs_commit = resolve_revision_commit(repository, theirs, "query_lifecycle.merge_file")?;
    let index = repository
        .merge_commits(&ours_commit, &theirs_commit, None)
        .map_err(|error| {
            map_query_error(
                &error,
                "query_lifecycle.merge_file failed to compute merge index",
            )
        })?;
    let path_in_tree = Path::new(path);
    let ancestor = stage_entry(&index, path_in_tree, 1, "ancestor")?;
    let ours_entry = stage_entry(&index, path_in_tree, 2, "ours")?;
    let theirs_entry = stage_entry(&index, path_in_tree, 3, "theirs")?;
    let mut merge_file_options = MergeFileOptions::new();
    merge_file_options
        .ancestor_label("ancestor")
        .our_label("ours")
        .their_label("theirs")
        .style_diff3(true);
    let merge_file_result: MergeFileResult = repository
        .merge_file_from_index(
            &ancestor,
            &ours_entry,
            &theirs_entry,
            Some(&mut merge_file_options),
        )
        .map_err(|error| {
            map_query_error(
                &error,
                "query_lifecycle.merge_file failed to merge staged conflict entries",
            )
        })?;
    let mut result =
        crate::git_manager::core::operations_query_lifecycle_support::empty_query_result();
    result.merge_file_preview = Some(QueryMergeFilePreview {
        automergeable: merge_file_result.is_automergeable(),
        path: merge_file_result.path().map(str::to_owned),
        file_mode: merge_file_result.mode(),
        content: String::from_utf8_lossy(merge_file_result.content()).into_owned(),
    });
    result.summary = Some(format!("merge-file preview for `{path}`"));
    Ok(result)
}

fn stage_entry(
    index: &git2::Index,
    path: &Path,
    stage: i32,
    label: &str,
) -> GitResult<git2::IndexEntry> {
    index.get_path(path, stage).ok_or_else(|| {
        GitError::new(
            GitErrorCode::QueryLifecycleInvalidInput,
            format!(
                "query_lifecycle.merge_file cannot read {label} stage entry for `{}`",
                path.display()
            ),
        )
    })
}

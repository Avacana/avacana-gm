use super::{
    map_plumbing_error, normalize_non_empty, resolve_object_oid, PACK_HEADER_BYTES,
    PACK_HEADER_SIGNATURE,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, PackBuildProgress, PlumbingResult, TreeEntrySpec,
};
use git2::{ErrorCode, IndexAddOption, Repository};
use std::path::Path;

fn map_index_entry_to_tree_spec(entry: git2::IndexEntry) -> TreeEntrySpec {
    TreeEntrySpec {
        path: String::from_utf8_lossy(entry.path.as_slice()).into_owned(),
        object_id: entry.id.to_string(),
        file_mode: entry.mode,
    }
}

fn map_index_conflict_to_tree_specs(
    conflict: git2::IndexConflict,
) -> (
    Option<TreeEntrySpec>,
    Option<TreeEntrySpec>,
    Option<TreeEntrySpec>,
) {
    (
        conflict.ancestor.map(map_index_entry_to_tree_spec),
        conflict.our.map(map_index_entry_to_tree_spec),
        conflict.their.map(map_index_entry_to_tree_spec),
    )
}

#[allow(clippy::too_many_lines)]
pub(crate) fn inspect_index_entries_and_conflicts(
    repository: &Repository,
    prefix: Option<&str>,
    pathspecs: &[String],
    conflict_pair: Option<&(String, String)>,
) -> GitResult<PlumbingResult> {
    let index = if let Some((our_commit_spec, their_commit_spec)) = conflict_pair {
        let our_oid = resolve_object_oid(
            repository,
            our_commit_spec,
            "plumbing.inspect_index.conflict_pair.our",
        )?;
        let their_oid = resolve_object_oid(
            repository,
            their_commit_spec,
            "plumbing.inspect_index.conflict_pair.their",
        )?;
        let our_commit = repository.find_commit(our_oid).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingObjectNotFound,
                format!(
                    "plumbing.inspect_index failed to resolve `our` commit `{our_commit_spec}`"
                ),
            )
        })?;
        let their_commit = repository.find_commit(their_oid).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingObjectNotFound,
                format!(
                    "plumbing.inspect_index failed to resolve `their` commit `{their_commit_spec}`"
                ),
            )
        })?;
        repository
            .merge_commits(&our_commit, &their_commit, None)
            .map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::IndexUpdateFailed,
                    "plumbing.inspect_index failed to build merge index for conflict inspection",
                )
            })?
    } else {
        repository.index().map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::IndexUpdateFailed,
                "plumbing.inspect_index failed to open repository index",
            )
        })?
    };

    let mut inspection = PackBuildProgress {
        operation: Some("index_inspection".to_string()),
        index_entries: index.iter().map(map_index_entry_to_tree_spec).collect(),
        ..PackBuildProgress::default()
    };
    if index.has_conflicts() {
        let conflicts = index.conflicts().map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::IndexUpdateFailed,
                "plumbing.inspect_index failed to open index conflicts iterator",
            )
        })?;
        for conflict_result in conflicts {
            let conflict = conflict_result.map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::IndexUpdateFailed,
                    "plumbing.inspect_index failed to read conflict item",
                )
            })?;
            inspection
                .index_conflicts
                .push(map_index_conflict_to_tree_specs(conflict));
        }
    }

    if let Some(prefix_value) = prefix {
        let prefix_value = normalize_non_empty(prefix_value, "plumbing.inspect_index.prefix")?;
        match index.find_prefix(prefix_value) {
            Ok(position) => inspection.index_prefix_position = Some(position),
            Err(error) if error.code() == ErrorCode::NotFound => {
                inspection.index_prefix_position = None;
            }
            Err(error) => {
                return Err(map_plumbing_error(
                    &error,
                    GitErrorCode::IndexUpdateFailed,
                    format!("plumbing.inspect_index failed to resolve prefix `{prefix_value}`"),
                ));
            }
        }
    }

    if !pathspecs.is_empty() {
        let mut callback_index = repository.index().map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::IndexUpdateFailed,
                "plumbing.inspect_index failed to open callback index",
            )
        })?;
        let pathspec_refs: Vec<&str> = pathspecs.iter().map(String::as_str).collect();
        let mut matched_callback = |path: &Path, matched_pathspec: &[u8]| -> i32 {
            inspection.index_matched_paths.push((
                path.to_string_lossy().into_owned(),
                String::from_utf8_lossy(matched_pathspec).into_owned(),
            ));
            1
        };
        let callback: &mut git2::IndexMatchedPath<'_> = &mut matched_callback;
        callback_index
            .add_all(pathspec_refs, IndexAddOption::DEFAULT, Some(callback))
            .map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::IndexUpdateFailed,
                    "plumbing.inspect_index failed during pathspec dry-run",
                )
            })?;
    }

    let entry_count = inspection.index_entries.len();
    let conflict_count = inspection.index_conflicts.len();
    Ok(PlumbingResult {
        object_id: None,
        object_kind: Some("index".to_string()),
        object_size: None,
        index_entry_count: Some(entry_count),
        indexed_objects: conflict_count,
        packed_objects: entry_count,
        pack_progress: Some(inspection),
        indexer_progress: None,
    })
}

pub(crate) fn parse_pack_header(header: &[u8], operation: &str) -> GitResult<usize> {
    if header.len() < PACK_HEADER_BYTES {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "{operation} requires at least {PACK_HEADER_BYTES} bytes for a valid pack header"
            ),
        ));
    }

    if &header[..PACK_HEADER_SIGNATURE.len()] != PACK_HEADER_SIGNATURE {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{operation} received an invalid pack signature"),
        ));
    }

    let version = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    if version != 2 && version != 3 {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{operation} received unsupported pack version `{version}`"),
        ));
    }

    let declared_objects = u32::from_be_bytes([header[8], header[9], header[10], header[11]]);
    if declared_objects == 0 {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{operation} received a pack header with zero objects"),
        ));
    }

    usize::try_from(declared_objects).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "{operation} cannot represent declared object count `{declared_objects}`: {error}"
            ),
        )
    })
}

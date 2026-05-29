use super::{map_plumbing_error, normalize_non_empty};
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, TreeEntrySpec};
use git2::{Oid, Repository};

pub(crate) fn insert_tree_entry(
    object_database: &git2::Odb<'_>,
    tree_builder: &mut git2::TreeBuilder<'_>,
    entry: &TreeEntrySpec,
) -> GitResult<()> {
    let entry_path = normalize_tree_entry_path(entry.path.as_str())?;
    let object_id = parse_object_id(
        entry.object_id.as_str(),
        "plumbing.build_tree.entries.object_id",
    )?;

    if !object_database.exists(object_id) {
        return Err(GitError::new(
            GitErrorCode::PlumbingObjectNotFound,
            format!(
                "plumbing.build_tree entry `{entry_path}` references missing object `{}`",
                entry.object_id
            ),
        ));
    }

    let file_mode = normalize_tree_entry_mode(entry.file_mode)?;
    tree_builder
        .insert(entry_path, object_id, file_mode)
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                format!("plumbing.build_tree failed to insert entry `{entry_path}`"),
            )
        })?;

    Ok(())
}

pub(crate) fn resolve_tree<'repo>(
    repository: &'repo Repository,
    value: &str,
    field_name: &str,
) -> GitResult<git2::Tree<'repo>> {
    let value = normalize_non_empty(value, field_name)?;
    let object = repository.revparse_single(value).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingObjectNotFound,
            format!("{field_name} cannot resolve `{value}` to a git object"),
        )
    })?;

    object.peel_to_tree().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingObjectNotFound,
            format!("{field_name} value `{value}` does not resolve to a tree object"),
        )
    })
}

fn parse_object_id(value: &str, field_name: &str) -> GitResult<Oid> {
    let value = normalize_non_empty(value, field_name)?;
    Oid::from_str(value).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{field_name} must be a valid hexadecimal OID: {error}"),
        )
    })
}

fn normalize_tree_entry_path(path: &str) -> GitResult<&str> {
    let path = normalize_non_empty(path, "plumbing.build_tree.entries.path")?;
    if path.contains('\0') {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.build_tree.entries.path must not contain NUL bytes",
        ));
    }

    if path.contains('/') || path.contains('\\') {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("plumbing.build_tree.entries.path `{path}` must be a single tree component"),
        ));
    }

    Ok(path)
}

fn normalize_tree_entry_mode(mode: u32) -> GitResult<i32> {
    const VALID_FILE_MODES: [u32; 5] = [0o040_000, 0o100_644, 0o100_755, 0o120_000, 0o160_000];

    if !VALID_FILE_MODES.contains(&mode) {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("plumbing.build_tree.entries.file_mode `{mode:o}` is not supported"),
        ));
    }

    i32::try_from(mode).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("plumbing.build_tree.entries.file_mode conversion failed: {error}"),
        )
    })
}

use crate::git_manager::core::operations_plumbing_support::{
    insert_tree_entry, map_plumbing_error, read_object_header, resolve_tree,
};
use crate::git_manager::core::{GitErrorCode, GitResult, PlumbingResult, TreeEntrySpec};
use git2::Repository;

pub(super) fn execute_build_tree_operation(
    repository: &Repository,
    base_tree_spec: Option<&str>,
    entries: &[TreeEntrySpec],
) -> GitResult<PlumbingResult> {
    let object_database = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.build_tree failed to open object database",
        )
    })?;
    let base_tree = base_tree_spec
        .map(|spec| resolve_tree(repository, spec, "plumbing.build_tree.base_tree"))
        .transpose()?;
    let mut tree_builder = repository
        .treebuilder(base_tree.as_ref())
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                "plumbing.build_tree failed to initialize tree builder",
            )
        })?;
    for entry in entries {
        insert_tree_entry(&object_database, &mut tree_builder, entry)?;
    }
    let tree_oid = tree_builder.write().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.build_tree failed to write tree object",
        )
    })?;
    let (object_size, object_kind) =
        read_object_header(repository, tree_oid, "plumbing.build_tree")?;
    Ok(PlumbingResult {
        object_id: Some(tree_oid.to_string()),
        object_kind: Some(object_kind),
        object_size: Some(object_size),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: None,
        indexer_progress: None,
    })
}

pub(super) fn execute_index_snapshot_operation(
    repository: &Repository,
    source_tree_spec: Option<&str>,
) -> GitResult<PlumbingResult> {
    let mut index = repository.index().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::IndexUpdateFailed,
            "plumbing.index_snapshot failed to open repository index",
        )
    })?;
    if let Some(source_tree_spec) = source_tree_spec {
        let tree = resolve_tree(
            repository,
            source_tree_spec,
            "plumbing.index_snapshot.source_tree",
        )?;
        index.read_tree(&tree).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::IndexUpdateFailed,
                "plumbing.index_snapshot failed to read source tree into index",
            )
        })?;
    }
    index.write().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::IndexUpdateFailed,
            "plumbing.index_snapshot failed to write index to disk",
        )
    })?;
    let index_entry_count = index.len();
    let tree_oid = index.write_tree_to(repository).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::IndexUpdateFailed,
            "plumbing.index_snapshot failed to write index tree",
        )
    })?;
    let (object_size, object_kind) =
        read_object_header(repository, tree_oid, "plumbing.index_snapshot")?;
    Ok(PlumbingResult {
        object_id: Some(tree_oid.to_string()),
        object_kind: Some(object_kind),
        object_size: Some(object_size),
        index_entry_count: Some(index_entry_count),
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: None,
        indexer_progress: None,
    })
}

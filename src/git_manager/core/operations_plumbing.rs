//! `plumbing` domain operations (`odb/object/treebuilder/indexer/packbuilder`) for `GitManager`.
#[path = "operations_plumbing_indexer.rs"]
mod indexer;
#[path = "operations_plumbing_pack.rs"]
mod pack_operations;
#[path = "operations_plumbing_streams.rs"]
mod stream_operations;
#[path = "operations_plumbing_tree.rs"]
mod tree_operations;

use self::indexer::{execute_index_pack_bytes_operation, execute_index_pack_operation};
use crate::git_manager::core::operations_plumbing_support::{
    inspect_index_entries_and_conflicts, map_plumbing_error, open_repository,
};
use crate::git_manager::core::{
    GitErrorCode, GitResult, PlumbingOperation, PlumbingRequest, PlumbingResult,
};
use git2::Repository;

use pack_operations::execute_build_pack_operation;
use stream_operations::{
    execute_read_blob_stream_operation, execute_write_blob_operation,
    execute_write_blob_stream_operation,
};
use tree_operations::{execute_build_tree_operation, execute_index_snapshot_operation};

/// # Errors
/// Returns a typed `GitError` if the operation cannot be carried out
/// because of invalid arguments, missing objects, an unavailable
/// repository, or libgit2/indexer/packbuilder failures.
#[cfg_attr(all(debug_assertions, feature = "trace_logs"), tracing::instrument(skip_all, fields(repository = %request.repository_path.display(), operation = ?request.operation)))]
pub(super) fn execute_plumbing_operation(request: &PlumbingRequest) -> GitResult<PlumbingResult> {
    let repository = open_repository(&request.repository_path, "plumbing")?;
    match &request.operation {
        PlumbingOperation::ReadObject { oid } => execute_read_object_operation(&repository, oid),
        PlumbingOperation::WriteBlobStreamFromChunks {
            hint_path,
            chunks,
            use_odb_writer,
        } => execute_write_blob_stream_operation(
            &repository,
            hint_path.as_deref(),
            chunks,
            *use_odb_writer,
        ),
        PlumbingOperation::ReadBlobStream { oid, chunk_size } => {
            execute_read_blob_stream_operation(&repository, oid, *chunk_size)
        }
        PlumbingOperation::WriteBlobFromPath { source_path } => {
            execute_write_blob_operation(&repository, &request.repository_path, source_path, true)
        }
        PlumbingOperation::HashBlobFromPath { source_path } => {
            execute_write_blob_operation(&repository, &request.repository_path, source_path, false)
        }
        PlumbingOperation::BuildTree { base_tree, entries } => {
            execute_build_tree_operation(&repository, base_tree.as_deref(), entries)
        }
        PlumbingOperation::IndexSnapshot { source_tree } => {
            execute_index_snapshot_operation(&repository, source_tree.as_deref())
        }
        PlumbingOperation::InspectIndexEntriesAndConflicts {
            prefix,
            pathspecs,
            conflict_pair,
        } => inspect_index_entries_and_conflicts(
            &repository,
            prefix.as_deref(),
            pathspecs,
            conflict_pair.as_ref(),
        ),
        PlumbingOperation::BuildPack { include_references } => {
            execute_build_pack_operation(&repository, include_references, None)
        }
        PlumbingOperation::BuildPackWithOptions {
            include_references,
            threads,
        } => execute_build_pack_operation(&repository, include_references, *threads),
        PlumbingOperation::IndexPack { pack_path } => {
            execute_index_pack_operation(&repository, &request.repository_path, pack_path, true)
        }
        PlumbingOperation::IndexPackWithOptions {
            pack_path,
            fix_thin,
        } => execute_index_pack_operation(
            &repository,
            &request.repository_path,
            pack_path,
            *fix_thin,
        ),
        PlumbingOperation::IndexPackStream { pack_data } => execute_index_pack_bytes_operation(
            &repository,
            pack_data,
            true,
            "plumbing.index_pack_stream",
        ),
        PlumbingOperation::IndexPackStreamWithOptions {
            pack_data,
            fix_thin,
        } => execute_index_pack_bytes_operation(
            &repository,
            pack_data,
            *fix_thin,
            "plumbing.index_pack_stream",
        ),
    }
}

fn execute_read_object_operation(
    repository: &Repository,
    oid_spec: &str,
) -> GitResult<PlumbingResult> {
    let oid = crate::git_manager::core::operations_plumbing_support::resolve_object_oid(
        repository,
        oid_spec,
        "plumbing.read_object.oid",
    )?;
    let object_database = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.read_object failed to open object database",
        )
    })?;
    let object = object_database.read(oid).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            format!("plumbing.read_object failed to read object `{oid}`"),
        )
    })?;
    Ok(PlumbingResult {
        object_id: Some(object.id().to_string()),
        object_kind: Some(object.kind().str().to_owned()),
        object_size: Some(object.len()),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: None,
        indexer_progress: None,
    })
}

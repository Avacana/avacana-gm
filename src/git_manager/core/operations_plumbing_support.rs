//! Helper utilities of the `plumbing` domain for `GitManager`.

#![allow(clippy::needless_pass_by_value, clippy::redundant_pub_crate)]

#[path = "operations_plumbing_support_index.rs"]
mod index_support;
#[path = "operations_plumbing_support_pack.rs"]
mod pack_support;
#[path = "operations_plumbing_support_tree.rs"]
mod tree_support;

pub(super) use crate::git_manager::core::repository_access::open_repository;
pub(super) use index_support::{inspect_index_entries_and_conflicts, parse_pack_header};
pub(super) use pack_support::{
    create_pack_output_directory, ensure_pack_size_within_limit, map_pack_builder_stage,
    parse_pack_size_bytes, read_generated_pack_size,
};
pub(super) use tree_support::{insert_tree_entry, resolve_tree};

use crate::git_manager::core::{GitError, GitErrorCode, GitResult};
use git2::{ErrorCode, Oid, Repository};
use std::path::{Path, PathBuf};

pub(super) const MAX_SUPPORTED_PACK_BYTES: usize = 256 * 1024 * 1024;
pub(super) const PACK_HEADER_BYTES: usize = 12;
const PACK_HEADER_SIGNATURE: &[u8; 4] = b"PACK";

pub(super) fn resolve_object_oid(
    repository: &Repository,
    value: &str,
    field_name: &str,
) -> GitResult<Oid> {
    let value = normalize_non_empty(value, field_name)?;
    if let Ok(oid) = Oid::from_str(value) {
        return Ok(oid);
    }

    repository
        .revparse_single(value)
        .map(|object| object.id())
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingObjectNotFound,
                format!("{field_name} cannot resolve `{value}` to a git object"),
            )
        })
}

pub(super) fn read_object_header(
    repository: &Repository,
    object_id: Oid,
    operation: &str,
) -> GitResult<(usize, String)> {
    let object_database = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            format!("{operation} failed to open object database"),
        )
    })?;

    let (object_size, object_kind) = object_database.read_header(object_id).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            format!("{operation} failed to read object header for `{object_id}`"),
        )
    })?;

    Ok((object_size, object_kind.str().to_owned()))
}

pub(super) fn normalize_non_empty<'a>(value: &'a str, field_name: &str) -> GitResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{field_name} must not be empty"),
        ));
    }
    Ok(value)
}

pub(super) fn resolve_request_path(repository_path: &Path, requested_path: &Path) -> PathBuf {
    if requested_path.is_absolute() {
        return requested_path.to_path_buf();
    }

    repository_path.join(requested_path)
}

pub(super) fn map_indexer_io_error(error: &std::io::Error, operation: &str) -> GitError {
    let code = match error.kind() {
        std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::InvalidInput => GitErrorCode::PlumbingInvalidInput,
        _ => GitErrorCode::PlumbingIndexerFailed,
    };
    GitError::new(
        code,
        format!("{operation} failed while streaming pack bytes into indexer: {error}"),
    )
}

pub(super) fn map_plumbing_io_error(
    error: &std::io::Error,
    code: GitErrorCode,
    operation: &str,
    action: &str,
) -> GitError {
    GitError::new(code, format!("{operation} failed to {action}: {error}"))
}

pub(super) fn validate_stream_chunks(
    chunks: &[Vec<u8>],
    field_name: &str,
    operation: &str,
) -> GitResult<(usize, usize)> {
    if chunks.is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{field_name} requires at least one chunk"),
        ));
    }

    let mut total_bytes = 0_usize;
    for (index, chunk) in chunks.iter().enumerate() {
        total_bytes = total_bytes.checked_add(chunk.len()).ok_or_else(|| {
            GitError::new(
                GitErrorCode::PlumbingInvalidInput,
                format!("{operation} overflow while summing stream chunk #{index}"),
            )
        })?;
    }

    if total_bytes == 0 {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{field_name} must contain at least one non-empty byte"),
        ));
    }

    Ok((chunks.len(), total_bytes))
}

pub(super) fn map_plumbing_error(
    error: &git2::Error,
    fallback: GitErrorCode,
    context: impl AsRef<str>,
) -> GitError {
    let code = match error.code() {
        ErrorCode::NotFound
        | ErrorCode::UnbornBranch
        | ErrorCode::InvalidSpec
        | ErrorCode::Ambiguous => GitErrorCode::PlumbingObjectNotFound,
        ErrorCode::Invalid | ErrorCode::User => GitErrorCode::PlumbingInvalidInput,
        ErrorCode::Locked => GitErrorCode::LockContention,
        _ => fallback,
    };

    GitError::new(code, format!("{}: {error}", context.as_ref()))
}

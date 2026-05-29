//! indexer/index-pack operations of the `plumbing` domain.

use crate::git_manager::core::operations_plumbing_support::{
    ensure_pack_size_within_limit, map_indexer_io_error, map_plumbing_error, parse_pack_header,
    parse_pack_size_bytes, resolve_request_path, PACK_HEADER_BYTES,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, IndexerProgressSnapshot, PlumbingResult,
};
use git2::{Indexer, Repository};
use std::cell::Cell;
use std::io::{Read, Write};
use std::path::Path;

const INDEXER_CHUNK_SIZE_BYTES: usize = 16 * 1024;

pub(super) fn execute_index_pack_operation(
    repository: &Repository,
    repository_path: &Path,
    pack_path: &Path,
    fix_thin: bool,
) -> GitResult<PlumbingResult> {
    if pack_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.index_pack requires a non-empty pack_path",
        ));
    }

    let resolved_pack_path = resolve_request_path(repository_path, pack_path);
    let metadata = std::fs::metadata(&resolved_pack_path).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "plumbing.index_pack failed to stat `{}`: {error}",
                resolved_pack_path.display()
            ),
        )
    })?;
    if !metadata.is_file() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "plumbing.index_pack requires a file path, got `{}`",
                resolved_pack_path.display()
            ),
        ));
    }

    let pack_size = parse_pack_size_bytes(
        metadata.len(),
        "plumbing.index_pack",
        GitErrorCode::PlumbingInvalidInput,
    )?;
    ensure_pack_size_within_limit(
        pack_size,
        "plumbing.index_pack",
        GitErrorCode::PlumbingInvalidInput,
    )?;

    let pack_file = std::fs::File::open(&resolved_pack_path).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "plumbing.index_pack failed to open `{}`: {error}",
                resolved_pack_path.display()
            ),
        )
    })?;
    execute_index_pack_reader_operation(
        repository,
        pack_file,
        pack_size,
        fix_thin,
        "plumbing.index_pack",
    )
}

pub(super) fn execute_index_pack_bytes_operation(
    repository: &Repository,
    pack_data: &[u8],
    fix_thin: bool,
    operation: &str,
) -> GitResult<PlumbingResult> {
    if pack_data.is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{operation} requires a non-empty pack stream"),
        ));
    }

    ensure_pack_size_within_limit(
        pack_data.len(),
        operation,
        GitErrorCode::PlumbingInvalidInput,
    )?;
    let pack_reader = std::io::Cursor::new(pack_data);
    execute_index_pack_reader_operation(
        repository,
        pack_reader,
        pack_data.len(),
        fix_thin,
        operation,
    )
}

#[allow(clippy::too_many_lines)]
fn execute_index_pack_reader_operation(
    repository: &Repository,
    mut pack_reader: impl Read,
    expected_pack_size: usize,
    fix_thin: bool,
    operation: &str,
) -> GitResult<PlumbingResult> {
    if expected_pack_size < PACK_HEADER_BYTES {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "{operation} requires at least {PACK_HEADER_BYTES} bytes for a valid pack stream"
            ),
        ));
    }

    let object_database = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingIndexerFailed,
            format!("{operation} failed to open object database"),
        )
    })?;
    let pack_directory = repository.path().join("objects").join("pack");
    std::fs::create_dir_all(&pack_directory).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingIndexerFailed,
            format!(
                "{operation} failed to create pack directory `{}`: {error}",
                pack_directory.display()
            ),
        )
    })?;

    let progress_snapshot = Cell::new(IndexerProgressSnapshot::default());
    let mut indexer =
        Indexer::new(Some(&object_database), &pack_directory, 0, fix_thin).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingIndexerFailed,
                format!("{operation} failed to initialize indexer"),
            )
        })?;
    indexer.progress(|progress| {
        progress_snapshot.set(IndexerProgressSnapshot {
            total_objects: progress.total_objects(),
            indexed_objects: progress.indexed_objects(),
            received_objects: progress.received_objects(),
            local_objects: progress.local_objects(),
            total_deltas: progress.total_deltas(),
            indexed_deltas: progress.indexed_deltas(),
            received_bytes: progress.received_bytes(),
        });
        true
    });

    let mut header = [0_u8; PACK_HEADER_BYTES];
    pack_reader
        .read_exact(&mut header)
        .map_err(|error| map_indexer_io_error(&error, operation))?;
    let declared_objects = parse_pack_header(&header, operation)?;
    indexer
        .write_all(&header)
        .map_err(|error| map_indexer_io_error(&error, operation))?;

    let mut streamed_pack_size = PACK_HEADER_BYTES;
    let mut header_progress = progress_snapshot.get();
    header_progress.total_objects = declared_objects;
    header_progress.received_bytes = streamed_pack_size;
    progress_snapshot.set(header_progress);

    let mut chunk_buffer = [0_u8; INDEXER_CHUNK_SIZE_BYTES];
    loop {
        let bytes_read = pack_reader
            .read(&mut chunk_buffer)
            .map_err(|error| map_indexer_io_error(&error, operation))?;
        if bytes_read == 0 {
            break;
        }

        streamed_pack_size = streamed_pack_size.checked_add(bytes_read).ok_or_else(|| {
            GitError::new(
                GitErrorCode::PlumbingInvalidInput,
                format!("{operation} streamed pack size overflowed internal counters"),
            )
        })?;
        ensure_pack_size_within_limit(
            streamed_pack_size,
            operation,
            GitErrorCode::PlumbingInvalidInput,
        )?;
        indexer
            .write_all(&chunk_buffer[..bytes_read])
            .map_err(|error| map_indexer_io_error(&error, operation))?;
    }

    if streamed_pack_size != expected_pack_size {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!("{operation} expected {expected_pack_size} pack bytes, received {streamed_pack_size}"),
        ));
    }

    let index_name = indexer.commit().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingIndexerFailed,
            format!("{operation} failed to finalize indexer output"),
        )
    })?;
    let mut progress_snapshot = progress_snapshot.get();
    if progress_snapshot.total_objects == 0 {
        progress_snapshot.total_objects = declared_objects;
    }
    if progress_snapshot.indexed_objects == 0 {
        progress_snapshot.indexed_objects = progress_snapshot.total_objects;
    }
    if progress_snapshot.received_objects == 0 {
        progress_snapshot.received_objects = progress_snapshot.indexed_objects;
    }
    progress_snapshot.received_bytes = streamed_pack_size;
    Ok(PlumbingResult {
        object_id: Some(index_name),
        object_kind: Some("pack".to_string()),
        object_size: Some(streamed_pack_size),
        index_entry_count: None,
        indexed_objects: progress_snapshot.indexed_objects,
        packed_objects: progress_snapshot.total_objects,
        pack_progress: None,
        indexer_progress: Some(progress_snapshot),
    })
}

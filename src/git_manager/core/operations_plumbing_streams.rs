use crate::git_manager::core::operations_plumbing_support::{
    map_plumbing_error, map_plumbing_io_error, parse_pack_size_bytes, read_object_header,
    resolve_object_oid, resolve_request_path, validate_stream_chunks,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, PackBuildProgress, PlumbingResult,
};
use git2::{ObjectType, Oid, Repository};
use std::io::{Read, Write};
use std::path::Path;

fn build_stream_progress(
    operation: &str,
    backend: &str,
    chunk_count: usize,
    stream_bytes: usize,
    chunk_size: Option<usize>,
) -> PackBuildProgress {
    PackBuildProgress {
        operation: Some(operation.to_string()),
        stream_backend: Some(backend.to_string()),
        stream_chunk_count: Some(chunk_count),
        stream_bytes: Some(stream_bytes),
        stream_chunk_size: chunk_size,
        ..PackBuildProgress::default()
    }
}

fn ensure_non_empty_hint_path(hint_path: Option<&Path>) -> GitResult<()> {
    if hint_path.is_some_and(|path| path.as_os_str().is_empty()) {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.write_blob_stream_from_chunks.hint_path must not be empty",
        ));
    }
    Ok(())
}

fn write_stream_chunks<W: Write>(
    writer: &mut W,
    chunks: &[Vec<u8>],
    destination: &str,
) -> GitResult<()> {
    for chunk in chunks {
        writer.write_all(chunk).map_err(|error| {
            map_plumbing_io_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                "plumbing.write_blob_stream_from_chunks",
                destination,
            )
        })?;
    }
    Ok(())
}

fn write_blob_stream_to_odb(
    repository: &Repository,
    chunks: &[Vec<u8>],
    stream_bytes: usize,
) -> GitResult<Oid> {
    let odb = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.write_blob_stream_from_chunks failed to open object database",
        )
    })?;
    let mut writer: git2::OdbWriter<'_> =
        odb.writer(stream_bytes, ObjectType::Blob)
            .map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::PlumbingOperationFailed,
                    "plumbing.write_blob_stream_from_chunks failed to initialize ODB writer",
                )
            })?;
    write_stream_chunks(&mut writer, chunks, "stream chunk into ODB writer")?;
    writer.finalize().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.write_blob_stream_from_chunks failed to finalize ODB writer",
        )
    })
}

fn write_blob_stream_to_blob_writer(
    repository: &Repository,
    hint_path: Option<&Path>,
    chunks: &[Vec<u8>],
) -> GitResult<Oid> {
    let mut writer: git2::BlobWriter<'_> = repository.blob_writer(hint_path).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.write_blob_stream_from_chunks failed to initialize blob writer",
        )
    })?;
    write_stream_chunks(&mut writer, chunks, "stream chunk into blob writer")?;
    writer.commit().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.write_blob_stream_from_chunks failed to commit blob writer",
        )
    })
}

fn read_blob_stream_result_header(
    repository: &Repository,
    blob_oid: Oid,
) -> GitResult<(String, usize)> {
    let odb = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.write_blob_stream_from_chunks failed to open object database",
        )
    })?;
    let object: git2::OdbObject<'_> = odb.read(blob_oid).map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            format!(
                "plumbing.write_blob_stream_from_chunks failed to read resulting blob `{blob_oid}`"
            ),
        )
    })?;
    Ok((object.kind().str().to_string(), object.len()))
}

pub(super) fn execute_write_blob_stream_operation(
    repository: &Repository,
    hint_path: Option<&Path>,
    chunks: &[Vec<u8>],
    use_odb_writer: bool,
) -> GitResult<PlumbingResult> {
    let (chunk_count, stream_bytes) = validate_stream_chunks(
        chunks,
        "plumbing.write_blob_stream_from_chunks.chunks",
        "plumbing.write_blob_stream_from_chunks",
    )?;
    ensure_non_empty_hint_path(hint_path)?;
    let blob_oid = if use_odb_writer {
        write_blob_stream_to_odb(repository, chunks, stream_bytes)?
    } else {
        write_blob_stream_to_blob_writer(repository, hint_path, chunks)?
    };
    let (object_kind, object_size) = read_blob_stream_result_header(repository, blob_oid)?;
    Ok(PlumbingResult {
        object_id: Some(blob_oid.to_string()),
        object_kind: Some(object_kind),
        object_size: Some(object_size),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: Some(build_stream_progress(
            "blob_stream_write",
            if use_odb_writer {
                "odb_writer"
            } else {
                "blob_writer"
            },
            chunk_count,
            stream_bytes,
            None,
        )),
        indexer_progress: None,
    })
}

pub(super) fn execute_read_blob_stream_operation(
    repository: &Repository,
    oid_spec: &str,
    chunk_size: usize,
) -> GitResult<PlumbingResult> {
    if chunk_size == 0 {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.read_blob_stream.chunk_size must be greater than zero",
        ));
    }
    let object_id = resolve_object_oid(repository, oid_spec, "plumbing.read_blob_stream.oid")?;
    let odb = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingOperationFailed,
            "plumbing.read_blob_stream failed to open object database",
        )
    })?;
    let (mut reader, expected_size, object_type): (git2::OdbReader<'_>, usize, ObjectType) =
        odb.reader(object_id).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                format!("plumbing.read_blob_stream failed to open reader for `{object_id}`"),
            )
        })?;
    let (mut streamed_bytes, mut streamed_chunks, mut buffer) =
        (0_usize, 0_usize, vec![0_u8; chunk_size]);
    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|error| {
            map_plumbing_io_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                "plumbing.read_blob_stream",
                "read from ODB stream",
            )
        })?;
        if bytes_read == 0 {
            break;
        }
        streamed_chunks = streamed_chunks.checked_add(1).ok_or_else(|| {
            GitError::new(
                GitErrorCode::PlumbingInvalidInput,
                "plumbing.read_blob_stream overflow while counting stream chunks",
            )
        })?;
        streamed_bytes = streamed_bytes.checked_add(bytes_read).ok_or_else(|| {
            GitError::new(
                GitErrorCode::PlumbingInvalidInput,
                "plumbing.read_blob_stream overflow while counting stream bytes",
            )
        })?;
    }
    if streamed_bytes != expected_size {
        return Err(GitError::new(
            GitErrorCode::PlumbingOperationFailed,
            format!(
                "plumbing.read_blob_stream expected {expected_size} bytes, received {streamed_bytes}"
            ),
        ));
    }
    Ok(PlumbingResult {
        object_id: Some(object_id.to_string()),
        object_kind: Some(object_type.str().to_string()),
        object_size: Some(streamed_bytes),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: Some(build_stream_progress(
            "blob_stream_read",
            "odb_reader",
            streamed_chunks,
            streamed_bytes,
            Some(chunk_size),
        )),
        indexer_progress: None,
    })
}

pub(super) fn execute_write_blob_operation(
    repository: &Repository,
    repository_path: &Path,
    source_path: &Path,
    write_to_odb: bool,
) -> GitResult<PlumbingResult> {
    if source_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.write_blob_from_path requires a non-empty source path",
        ));
    }
    let resolved_source_path = resolve_request_path(repository_path, source_path);
    let metadata = std::fs::metadata(&resolved_source_path).map_err(|error| {
        GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "plumbing.write_blob_from_path failed to stat `{}`: {error}",
                resolved_source_path.display()
            ),
        )
    })?;
    if !metadata.is_file() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            format!(
                "plumbing.write_blob_from_path requires a file path, got `{}`",
                resolved_source_path.display()
            ),
        ));
    }
    let object_size = parse_pack_size_bytes(
        metadata.len(),
        "plumbing.write_blob_from_path",
        GitErrorCode::PlumbingInvalidInput,
    )?;
    let blob_oid = if write_to_odb {
        repository
            .blob_path(&resolved_source_path)
            .map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::PlumbingOperationFailed,
                    format!(
                        "plumbing.write_blob_from_path failed to persist blob from `{}`",
                        resolved_source_path.display()
                    ),
                )
            })?
    } else {
        Oid::hash_file(ObjectType::Blob, &resolved_source_path).map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingOperationFailed,
                format!(
                    "plumbing.write_blob_from_path failed to hash blob from `{}`",
                    resolved_source_path.display()
                ),
            )
        })?
    };
    let (object_size, object_kind) = if write_to_odb {
        read_object_header(repository, blob_oid, "plumbing.write_blob_from_path")?
    } else {
        (object_size, "blob".to_string())
    };
    Ok(PlumbingResult {
        object_id: Some(blob_oid.to_string()),
        object_kind: Some(object_kind),
        object_size: Some(object_size),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: 0,
        pack_progress: None,
        indexer_progress: None,
    })
}

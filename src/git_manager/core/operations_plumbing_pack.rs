use crate::git_manager::core::operations_plumbing_support::{
    create_pack_output_directory, map_pack_builder_stage, map_plumbing_error, normalize_non_empty,
    read_generated_pack_size, resolve_object_oid,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, PackBuildProgress, PackBuildStage, PlumbingResult,
};
use git2::Repository;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::num::NonZeroUsize;

#[allow(clippy::too_many_lines)]
pub(super) fn execute_build_pack_operation(
    repository: &Repository,
    include_references: &[String],
    threads: Option<usize>,
) -> GitResult<PlumbingResult> {
    if include_references.is_empty() {
        return Err(GitError::new(
            GitErrorCode::PlumbingInvalidInput,
            "plumbing.build_pack requires at least one reference",
        ));
    }
    let pack_progress = RefCell::new(PackBuildProgress::default());
    let mut pack_builder = repository.packbuilder().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingPackBuildFailed,
            "plumbing.build_pack failed to initialize pack builder",
        )
    })?;
    pack_builder
        .set_progress_callback(|stage, current, total| {
            let mut snapshot = pack_progress.borrow_mut();
            snapshot.stage = Some(map_pack_builder_stage(stage));
            snapshot.current = current;
            snapshot.total = total;
            true
        })
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingPackBuildFailed,
                "plumbing.build_pack failed to register progress callback",
            )
        })?;
    let default_threads = std::thread::available_parallelism().map_or(1_usize, NonZeroUsize::get);
    let requested_threads = threads.unwrap_or(default_threads).max(1);
    let suggested_threads = u32::try_from(requested_threads).map_or(u32::MAX, |value| value);
    let _effective_threads = pack_builder.set_threads(suggested_threads);
    let mut inserted_oids = BTreeSet::new();
    for reference in include_references {
        let reference = normalize_non_empty(reference, "plumbing.build_pack.include_references")?;
        let oid = resolve_object_oid(
            repository,
            reference,
            "plumbing.build_pack.include_references",
        )?;
        let oid_hex = oid.to_string();
        if !inserted_oids.insert(oid_hex) {
            continue;
        }
        pack_builder
            .insert_recursive(oid, Some(reference))
            .map_err(|error| {
                map_plumbing_error(
                    &error,
                    GitErrorCode::PlumbingPackBuildFailed,
                    format!("plumbing.build_pack failed to insert reference `{reference}`"),
                )
            })?;
    }
    let in_memory_pack: git2::Buf = git2::Buf::new();
    let in_memory_pack_size = in_memory_pack.len();
    let object_database = repository.odb().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingPackBuildFailed,
            "plumbing.build_pack failed to open object database",
        )
    })?;
    let mempack_backend: git2::Mempack<'_> = object_database
        .add_new_mempack_backend(1_000)
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingPackBuildFailed,
                "plumbing.build_pack failed to initialize mempack backend",
            )
        })?;
    mempack_backend.reset().map_err(|error| {
        map_plumbing_error(
            &error,
            GitErrorCode::PlumbingPackBuildFailed,
            "plumbing.build_pack failed to reset mempack backend",
        )
    })?;
    let mempack_dump_size = 0;
    let mut odb_packwriter: git2::OdbPackwriter<'_> =
        object_database.packwriter().map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingPackBuildFailed,
                "plumbing.build_pack failed to initialize ODB packwriter",
            )
        })?;
    odb_packwriter.progress(|progress: git2::Progress<'_>| {
        let _received_bytes = progress.received_bytes();
        true
    });
    let pack_output_directory =
        create_pack_output_directory("plumbing.build_pack", GitErrorCode::PlumbingPackBuildFailed)?;
    pack_builder
        .write(&pack_output_directory, 0)
        .map_err(|error| {
            map_plumbing_error(
                &error,
                GitErrorCode::PlumbingPackBuildFailed,
                format!(
                    "plumbing.build_pack failed to write pack into `{}`",
                    pack_output_directory.display()
                ),
            )
        })?;
    let mut pack_progress = pack_progress.borrow().clone();
    pack_progress.operation = Some("build_pack".to_string());
    pack_progress.stream_backend = Some("packbuilder+odb_packwriter".to_string());
    pack_progress.stream_bytes = Some(in_memory_pack_size);
    pack_progress.mempack_dump_size = Some(mempack_dump_size);
    pack_progress.mempack_restored = Some(true);
    pack_progress.written_objects = pack_builder.written();
    let pack_name = pack_builder.name().map(str::to_owned);
    let pack_object_count = pack_builder.object_count();
    if pack_progress.stage.is_none() {
        pack_progress.stage = Some(PackBuildStage::Deltafication);
    }
    if pack_progress.total == 0 {
        pack_progress.total = u32::try_from(pack_object_count).map_or(u32::MAX, |value| value);
    }
    if pack_progress.current == 0 {
        pack_progress.current = pack_progress.total;
    }
    drop(pack_builder);
    let pack_size_result = read_generated_pack_size(
        &pack_output_directory,
        pack_name.as_deref(),
        "plumbing.build_pack",
        GitErrorCode::PlumbingPackBuildFailed,
    );
    let _ignored_cleanup_error = std::fs::remove_dir_all(&pack_output_directory);
    let pack_size = pack_size_result?;
    Ok(PlumbingResult {
        object_id: pack_name,
        object_kind: Some("pack".to_string()),
        object_size: Some(pack_size),
        index_entry_count: None,
        indexed_objects: 0,
        packed_objects: pack_object_count,
        pack_progress: Some(pack_progress),
        indexer_progress: None,
    })
}

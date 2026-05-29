use super::apply_patch_error;
use super::pathspec_operations::compile_pathspec;
use crate::git_manager::core::{ApplyPatchLocation, GitResult, StatusDiffRequest};
use git2::{ApplyLocation, ApplyOptions, Diff, PathspecFlags, Repository};
use std::path::{Path, PathBuf};

pub(super) fn apply_patch_if_requested(
    repository: &Repository,
    request: &StatusDiffRequest,
) -> GitResult<()> {
    let Some(apply_request) = request.apply.as_ref() else {
        return Ok(());
    };
    if apply_request.patch.is_empty() {
        return Err(apply_patch_error(
            "status/diff apply request requires a non-empty patch buffer",
        ));
    }
    let patch_diff = Diff::from_buffer(apply_request.patch.as_slice()).map_err(|error| {
        apply_patch_error(format!(
            "failed to parse patch buffer for apply operation: {error}"
        ))
    })?;
    let mut apply_options = ApplyOptions::new();
    apply_options.check(apply_request.check);
    if !request.pathspecs.is_empty() {
        let compiled_pathspec = compile_pathspec(request.pathspecs.as_slice())?;
        apply_options.delta_callback(move |delta| {
            let Some(delta) = delta else {
                return false;
            };
            let Some(path) = patch_delta_path(&delta) else {
                return false;
            };
            compiled_pathspec.matches_path(path.as_path(), PathspecFlags::DEFAULT)
        });
    }
    repository
        .apply(
            &patch_diff,
            map_apply_location(apply_request.location),
            Some(&mut apply_options),
        )
        .map_err(|error| {
            apply_patch_error(format!(
                "failed to apply patch in repository `{}`: {error}",
                repository.path().display()
            ))
        })
}

fn patch_delta_path(delta: &git2::DiffDelta<'_>) -> Option<PathBuf> {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(Path::to_path_buf)
}

const fn map_apply_location(location: ApplyPatchLocation) -> ApplyLocation {
    match location {
        ApplyPatchLocation::Workdir => ApplyLocation::WorkDir,
        ApplyPatchLocation::Index => ApplyLocation::Index,
        ApplyPatchLocation::Both => ApplyLocation::Both,
    }
}

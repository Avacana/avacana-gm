use super::MAX_SUPPORTED_PACK_BYTES;
use crate::git_manager::core::{GitError, GitErrorCode, GitResult, PackBuildStage};
use git2::PackBuilderStage;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(crate) const fn map_pack_builder_stage(stage: PackBuilderStage) -> PackBuildStage {
    match stage {
        PackBuilderStage::AddingObjects => PackBuildStage::AddingObjects,
        PackBuilderStage::Deltafication => PackBuildStage::Deltafication,
    }
}

pub(crate) fn create_pack_output_directory(
    operation: &str,
    error_code: GitErrorCode,
) -> GitResult<PathBuf> {
    let output_directory =
        std::env::temp_dir().join(format!("avacana-gm-plumbing-pack-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&output_directory).map_err(|error| {
        GitError::new(
            error_code,
            format!(
                "{operation} failed to create temporary pack directory `{}`: {error}",
                output_directory.display()
            ),
        )
    })?;

    Ok(output_directory)
}

pub(crate) fn read_generated_pack_size(
    pack_output_directory: &Path,
    pack_name: Option<&str>,
    operation: &str,
    error_code: GitErrorCode,
) -> GitResult<usize> {
    let pack_name = pack_name.ok_or_else(|| {
        GitError::new(
            error_code,
            format!("{operation} completed without resulting pack name"),
        )
    })?;

    let generated_pack_path = pack_output_directory.join(format!("pack-{pack_name}.pack"));
    let metadata = std::fs::metadata(&generated_pack_path).map_err(|error| {
        GitError::new(
            error_code,
            format!(
                "{operation} failed to stat generated pack `{}`: {error}",
                generated_pack_path.display()
            ),
        )
    })?;

    if !metadata.is_file() {
        return Err(GitError::new(
            error_code,
            format!(
                "{operation} generated path `{}` is not a pack file",
                generated_pack_path.display()
            ),
        ));
    }

    let pack_size = parse_pack_size_bytes(metadata.len(), operation, error_code)?;
    ensure_pack_size_within_limit(pack_size, operation, error_code)?;
    Ok(pack_size)
}

pub(crate) fn parse_pack_size_bytes(
    pack_size_bytes: u64,
    operation: &str,
    error_code: GitErrorCode,
) -> GitResult<usize> {
    usize::try_from(pack_size_bytes).map_err(|_error| {
        GitError::new(
            error_code,
            format!("{operation} pack size `{pack_size_bytes}` exceeds platform-supported range"),
        )
    })
}

pub(crate) fn ensure_pack_size_within_limit(
    pack_size_bytes: usize,
    operation: &str,
    error_code: GitErrorCode,
) -> GitResult<()> {
    if pack_size_bytes > MAX_SUPPORTED_PACK_BYTES {
        return Err(GitError::new(
            error_code,
            format!(
                "{operation} does not support pack streams larger than {MAX_SUPPORTED_PACK_BYTES} bytes (received {pack_size_bytes})"
            ),
        ));
    }

    Ok(())
}

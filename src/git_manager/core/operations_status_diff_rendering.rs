//! Rendering and mapping helpers for `status/diff`.

use crate::git_manager::core::{
    DiffBinaryDetails, DiffBinaryFileDetails, DiffBinaryKindDetails, DiffFileDetails,
    DiffFileModeKind, DiffOutputFormat, GitError, GitErrorCode, GitResult,
};
use git2::{Diff, DiffBinary, DiffBinaryFile, DiffBinaryKind, DiffFormat, FileMode};

pub(super) fn map_diff_file(file: &git2::DiffFile<'_>) -> DiffFileDetails {
    let mode_kind = map_file_mode(file.mode());
    DiffFileDetails {
        id: file.id().to_string(),
        path: file.path().map(|path| path.to_string_lossy().into_owned()),
        size: file.size(),
        exists: file.exists(),
        is_binary: file.is_binary(),
        is_not_binary: file.is_not_binary(),
        is_valid_id: file.is_valid_id(),
        mode_kind,
        mode: format!("{:?}", file.mode()),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(super) fn map_diff_binary(binary: DiffBinary<'_>) -> DiffBinaryDetails {
    let contains_data = binary.contains_data();
    DiffBinaryDetails {
        contains_data,
        old_file: map_diff_binary_file(binary.old_file(), contains_data),
        new_file: map_diff_binary_file(binary.new_file(), contains_data),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn map_diff_binary_file(file: DiffBinaryFile<'_>, contains_data: bool) -> DiffBinaryFileDetails {
    let kind = map_diff_binary_kind(file.kind());
    let has_payload = contains_data && !matches!(kind, DiffBinaryKindDetails::None);
    DiffBinaryFileDetails {
        kind,
        compressed_size: if has_payload { file.data().len() } else { 0 },
        inflated_size: if has_payload { file.inflated_len() } else { 0 },
    }
}

const fn map_diff_binary_kind(kind: DiffBinaryKind) -> DiffBinaryKindDetails {
    match kind {
        DiffBinaryKind::None => DiffBinaryKindDetails::None,
        DiffBinaryKind::Literal => DiffBinaryKindDetails::Literal,
        DiffBinaryKind::Delta => DiffBinaryKindDetails::Delta,
    }
}

pub(super) fn render_diff_output(diff: &Diff<'_>, format: DiffOutputFormat) -> GitResult<String> {
    let mut rendered = String::new();
    diff.print(map_diff_output_format(format), |_delta, _hunk, line| {
        rendered.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|error| {
        GitError::new(
            GitErrorCode::StatusDiffFailed,
            format!("failed to render diff output: {error}"),
        )
    })?;
    Ok(rendered)
}

const fn map_diff_output_format(format: DiffOutputFormat) -> DiffFormat {
    match format {
        DiffOutputFormat::Patch => DiffFormat::Patch,
        DiffOutputFormat::PatchHeader => DiffFormat::PatchHeader,
        DiffOutputFormat::Raw => DiffFormat::Raw,
        DiffOutputFormat::NameOnly => DiffFormat::NameOnly,
        DiffOutputFormat::NameStatus => DiffFormat::NameStatus,
    }
}

const fn map_file_mode(mode: FileMode) -> DiffFileModeKind {
    match mode {
        FileMode::Unreadable => DiffFileModeKind::Unreadable,
        FileMode::Tree => DiffFileModeKind::Tree,
        FileMode::Blob => DiffFileModeKind::Blob,
        FileMode::BlobGroupWritable => DiffFileModeKind::BlobGroupWritable,
        FileMode::BlobExecutable => DiffFileModeKind::BlobExecutable,
        FileMode::Link => DiffFileModeKind::Link,
        FileMode::Commit => DiffFileModeKind::Commit,
    }
}


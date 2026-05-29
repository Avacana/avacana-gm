use crate::git_manager::core::{
    DiffHunkDetails, DiffLineDetails, DiffLineTypeDetails, DiffPatchDetails, GitError, GitResult,
};
use git2::{Diff, DiffLine, DiffLineType, Patch};

pub(super) fn collect_patch_details_from_diff(
    diff: &Diff<'_>,
    index: usize,
    error_factory: fn(String) -> GitError,
) -> GitResult<Option<DiffPatchDetails>> {
    let patch = Patch::from_diff(diff, index).map_err(|error| {
        error_factory(format!(
            "failed to build patch view for diff delta #{index}: {error}"
        ))
    })?;
    patch.map_or(Ok(None), |patch| {
        build_patch_details(&patch, error_factory).map(Some)
    })
}

pub(super) fn patch_line_count(patch: &DiffPatchDetails) -> usize {
    patch.hunks.iter().map(|hunk| hunk.lines.len()).sum()
}

fn build_patch_details(
    patch: &Patch<'_>,
    error_factory: fn(String) -> GitError,
) -> GitResult<DiffPatchDetails> {
    let (context_lines, additions, deletions) = patch
        .line_stats()
        .map_err(|error| error_factory(format!("failed to compute patch line stats: {error}")))?;
    let mut hunks = Vec::new();
    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count): (git2::DiffHunk<'_>, usize) =
            patch.hunk(hunk_index).map_err(|error| {
                error_factory(format!(
                    "failed to read hunk #{hunk_index} from patch: {error}"
                ))
            })?;
        let mut lines = Vec::with_capacity(line_count);
        for line_index in 0..line_count {
            let line = patch
                .line_in_hunk(hunk_index, line_index)
                .map_err(|error| {
                    error_factory(format!(
                    "failed to read line #{line_index} from hunk #{hunk_index} in patch: {error}"
                ))
                })?;
            lines.push(map_diff_line(line));
        }
        hunks.push(DiffHunkDetails {
            old_start: hunk.old_start(),
            old_lines: hunk.old_lines(),
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
            header: String::from_utf8_lossy(hunk.header()).into_owned(),
            lines,
        });
    }
    Ok(DiffPatchDetails {
        size_bytes: patch.size(true, true, true),
        context_lines,
        additions,
        deletions,
        hunks,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn map_diff_line(line: DiffLine<'_>) -> DiffLineDetails {
    DiffLineDetails {
        old_lineno: line.old_lineno(),
        new_lineno: line.new_lineno(),
        line_type: map_diff_line_type(line.origin_value()),
        origin: line.origin(),
        num_lines: line.num_lines(),
        content_offset: line.content_offset(),
        content: String::from_utf8_lossy(line.content()).into_owned(),
    }
}

const fn map_diff_line_type(line_type: DiffLineType) -> DiffLineTypeDetails {
    match line_type {
        DiffLineType::Context => DiffLineTypeDetails::Context,
        DiffLineType::Addition => DiffLineTypeDetails::Addition,
        DiffLineType::Deletion => DiffLineTypeDetails::Deletion,
        DiffLineType::ContextEOFNL => DiffLineTypeDetails::ContextEofNl,
        DiffLineType::AddEOFNL => DiffLineTypeDetails::AddEofNl,
        DiffLineType::DeleteEOFNL => DiffLineTypeDetails::DeleteEofNl,
        DiffLineType::FileHeader => DiffLineTypeDetails::FileHeader,
        DiffLineType::HunkHeader => DiffLineTypeDetails::HunkHeader,
        DiffLineType::Binary => DiffLineTypeDetails::Binary,
    }
}

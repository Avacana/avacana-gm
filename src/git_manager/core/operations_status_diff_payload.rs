use super::diff_operations::{build_staged_diff, build_unstaged_diff};
use super::mapping_operations::{
    count_unique_paths, delta_identity_key, map_delta_to_diff_status_code,
};
use super::pathspec_operations::{build_pathspec_diff_details, build_pathspec_workdir_details};
use super::rendering::{map_diff_binary, map_diff_file, render_diff_output};
use super::status_diff_error;
use crate::git_manager::core::operations_diff_patch_support::collect_patch_details_from_diff;
use crate::git_manager::core::{
    DiffBinaryDetails, DiffDeltaDetails, DiffOutputFormat, DiffPatchDetails, DiffSegmentDetails,
    DiffStatusEntry, DiffSummary, GitResult, StatusDiffDetails, StatusScope,
};
use git2::{Deltas, Diff, DiffPatchidOptions, DiffStats, Repository};
use std::collections::BTreeMap;

pub(super) fn build_diff_payload(
    repository: &Repository,
    scope: StatusScope,
    pathspecs: &[String],
    entries: &[DiffStatusEntry],
    render_diff_format: Option<DiffOutputFormat>,
) -> GitResult<(DiffSummary, StatusDiffDetails)> {
    match scope {
        StatusScope::Staged => build_single_scope_payload(
            repository,
            StatusScope::Staged,
            pathspecs,
            render_diff_format,
        ),
        StatusScope::Unstaged => build_single_scope_payload(
            repository,
            StatusScope::Unstaged,
            pathspecs,
            render_diff_format,
        ),
        StatusScope::Untracked => {
            let pathspec = if pathspecs.is_empty() {
                None
            } else {
                Some(build_pathspec_workdir_details(repository, pathspecs)?)
            };
            Ok((
                DiffSummary {
                    changed_files: entries.len(),
                    insertions: 0,
                    deletions: 0,
                    has_binary_changes: false,
                },
                StatusDiffDetails {
                    segments: vec![DiffSegmentDetails {
                        scope: StatusScope::Untracked,
                        patch_id: None,
                        rendered_diff: None,
                        deltas: Vec::new(),
                        pathspec,
                    }],
                },
            ))
        }
        StatusScope::All => {
            let staged_diff = build_staged_diff(repository, pathspecs)?;
            let unstaged_diff = build_unstaged_diff(repository, pathspecs)?;
            let staged_summary = diff_summary_from_diff(&staged_diff)?;
            let unstaged_summary = diff_summary_from_diff(&unstaged_diff)?;
            let staged_details = build_diff_segment_details(
                StatusScope::Staged,
                &staged_diff,
                pathspecs,
                render_diff_format,
            )?;
            let unstaged_details = build_diff_segment_details(
                StatusScope::Unstaged,
                &unstaged_diff,
                pathspecs,
                render_diff_format,
            )?;
            Ok((
                DiffSummary {
                    changed_files: count_unique_paths(entries),
                    insertions: staged_summary.insertions + unstaged_summary.insertions,
                    deletions: staged_summary.deletions + unstaged_summary.deletions,
                    has_binary_changes: staged_summary.has_binary_changes
                        || unstaged_summary.has_binary_changes,
                },
                StatusDiffDetails {
                    segments: vec![staged_details, unstaged_details],
                },
            ))
        }
    }
}

fn build_single_scope_payload(
    repository: &Repository,
    scope: StatusScope,
    pathspecs: &[String],
    render_diff_format: Option<DiffOutputFormat>,
) -> GitResult<(DiffSummary, StatusDiffDetails)> {
    let diff = match scope {
        StatusScope::Staged => build_staged_diff(repository, pathspecs)?,
        StatusScope::Unstaged => build_unstaged_diff(repository, pathspecs)?,
        StatusScope::Untracked | StatusScope::All => {
            unreachable!("single scope payload supports only staged/unstaged")
        }
    };
    let summary = diff_summary_from_diff(&diff)?;
    let segment = build_diff_segment_details(scope, &diff, pathspecs, render_diff_format)?;
    Ok((
        summary,
        StatusDiffDetails {
            segments: vec![segment],
        },
    ))
}

fn diff_summary_from_diff(diff: &Diff<'_>) -> GitResult<DiffSummary> {
    let stats: DiffStats = diff.stats().map_err(|error| {
        status_diff_error(format!("failed to compute diff statistics: {error}"))
    })?;
    Ok(DiffSummary {
        changed_files: stats.files_changed(),
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        has_binary_changes: diff.deltas().any(|delta| {
            delta.flags().is_binary()
                || delta.new_file().is_binary()
                || delta.old_file().is_binary()
        }),
    })
}

fn build_diff_segment_details(
    scope: StatusScope,
    diff: &Diff<'_>,
    pathspecs: &[String],
    render_diff_format: Option<DiffOutputFormat>,
) -> GitResult<DiffSegmentDetails> {
    let mut patchid_options = DiffPatchidOptions::new();
    let patch_id = diff.patchid(Some(&mut patchid_options)).map_err(|error| {
        status_diff_error(format!(
            "failed to compute patch-id for diff segment `{scope:?}`: {error}"
        ))
    })?;
    let binary_by_delta = collect_binary_details(diff)?;
    let deltas = collect_diff_delta_details(diff, &binary_by_delta)?;
    let rendered_diff = render_diff_format
        .map(|format| render_diff_output(diff, format))
        .transpose()?;
    let pathspec = if pathspecs.is_empty() {
        None
    } else {
        Some(build_pathspec_diff_details(diff, pathspecs)?)
    };
    Ok(DiffSegmentDetails {
        scope,
        patch_id: Some(patch_id.to_string()),
        rendered_diff,
        deltas,
        pathspec,
    })
}

fn collect_binary_details(diff: &Diff<'_>) -> GitResult<BTreeMap<String, DiffBinaryDetails>> {
    let mut by_delta = BTreeMap::new();
    diff.foreach(
        &mut |_delta, _progress| true,
        Some(&mut |delta, binary| {
            if let Some(key) = delta_identity_key(&delta) {
                by_delta.insert(key, map_diff_binary(binary));
            }
            true
        }),
        None,
        None,
    )
    .map_err(|error| {
        status_diff_error(format!(
            "failed to iterate diff callbacks for binary diagnostics: {error}"
        ))
    })?;
    Ok(by_delta)
}

fn collect_diff_delta_details(
    diff: &Diff<'_>,
    binary_by_delta: &BTreeMap<String, DiffBinaryDetails>,
) -> GitResult<Vec<DiffDeltaDetails>> {
    let deltas: Deltas<'_> = diff.deltas();
    deltas
        .enumerate()
        .filter_map(|(index, delta)| {
            let code = map_delta_to_diff_status_code(delta.status())?;
            Some((index, delta, code))
        })
        .map(|(index, delta, code)| {
            let key = delta_identity_key(&delta);
            let patch = collect_patch_details(diff, index)?;
            Ok(DiffDeltaDetails {
                code,
                old_file: map_diff_file(&delta.old_file()),
                new_file: map_diff_file(&delta.new_file()),
                binary: key.and_then(|delta_key| binary_by_delta.get(&delta_key).cloned()),
                patch,
            })
        })
        .collect()
}

fn collect_patch_details(diff: &Diff<'_>, index: usize) -> GitResult<Option<DiffPatchDetails>> {
    collect_patch_details_from_diff(diff, index, status_diff_error)
}

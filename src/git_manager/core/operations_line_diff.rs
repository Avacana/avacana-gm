//! `line_diff` operation for `GitManager`.

#![allow(clippy::too_many_lines)]

use crate::git_manager::core::operations_diff_patch_support::{
    collect_patch_details_from_diff, patch_line_count,
};
use crate::git_manager::core::operations_repository_path_support::{
    normalize_filesystem_path_to_repository_relative, FilesystemPathValidationFailure,
    NormalizedRepositoryPath,
};
use crate::git_manager::core::repository_access::{
    describe_opened_repository, open_repository_context,
};
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, LineDiffPayload, LineDiffRequest, LineDiffResult,
};
use git2::{AttrCheckFlags, AttrValue, DiffOptions, ErrorCode, Repository};
use std::path::Path;
use std::time::Instant;

const BASELINE_HEAD_TO_WORKTREE: &str = "head_to_worktree";
const BASELINE_EMPTY_UNTRACKED_TO_WORKTREE: &str = "empty_to_worktree_untracked";
const BASELINE_EMPTY_UNBORN_HEAD_TO_WORKTREE: &str = "empty_to_worktree_unborn_head";

pub(super) struct LineDiffExecutionOutput {
    pub(super) result: LineDiffResult,
    pub(super) normalized_target_path: String,
    pub(super) baseline_kind: &'static str,
    pub(super) binary: bool,
    pub(super) hunk_count: usize,
    pub(super) line_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineDiffBaseline {
    HeadToWorktree,
    EmptyToWorktreeUntracked,
    EmptyToWorktreeUnbornHead,
}

impl LineDiffBaseline {
    const fn kind_name(self) -> &'static str {
        match self {
            Self::HeadToWorktree => BASELINE_HEAD_TO_WORKTREE,
            Self::EmptyToWorktreeUntracked => BASELINE_EMPTY_UNTRACKED_TO_WORKTREE,
            Self::EmptyToWorktreeUnbornHead => BASELINE_EMPTY_UNBORN_HEAD_TO_WORKTREE,
        }
    }
}

/// Executes the canonical single-path line diff via `libgit2`.
///
/// # Errors
/// Returns a typed `GitError` if repository discovery, path normalization,
/// or diff collection fails.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "line_diff",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            target_path = %request.target_path.display(),
            normalized_target_path = tracing::field::Empty,
            baseline_kind = tracing::field::Empty,
            binary = tracing::field::Empty,
            hunk_count = tracing::field::Empty,
            line_count = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_line_diff_operation(
    request: &LineDiffRequest,
) -> GitResult<LineDiffExecutionOutput> {
    let started_at = Instant::now();
    let opened_repository = open_repository_context(&request.repository_path, "line_diff")?;
    let repository_descriptor = describe_opened_repository(&opened_repository);

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(repository_descriptor.repo_root.display()),
    );

    if opened_repository.is_bare {
        return Err(line_diff_failed(format!(
            "line_diff requires a worktree, but repository `{}` is bare",
            repository_descriptor.repo_root.display()
        )));
    }

    let normalized_target_path = normalize_target_path(
        request.target_path.as_path(),
        repository_descriptor.repo_root.as_path(),
    )?;
    tracing::Span::current().record(
        "normalized_target_path",
        tracing::field::display(normalized_target_path.as_str()),
    );

    let worktree_root = opened_repository.worktree_root.as_deref().ok_or_else(|| {
        line_diff_failed(format!(
            "line_diff requires a worktree, but repository `{}` has no worktree root",
            repository_descriptor.repo_root.display()
        ))
    })?;
    reject_directory_target(worktree_root, normalized_target_path.as_str())?;

    let head_tree = resolve_head_tree(&opened_repository.repository)?;
    let baseline = resolve_baseline(head_tree.as_ref(), normalized_target_path.as_str())?;

    if has_external_diff_driver(
        &opened_repository.repository,
        normalized_target_path.as_str(),
    )? {
        return finalize_line_diff_execution(
            repository_descriptor,
            normalized_target_path,
            baseline,
            LineDiffPayload::ExternalDriverConfigured,
            started_at,
        );
    }

    let diff = build_head_to_worktree_diff(
        &opened_repository.repository,
        head_tree.as_ref(),
        normalized_target_path.as_str(),
    )?;
    let delta_count = diff.deltas().count();

    if delta_count == 0 {
        return finalize_line_diff_execution(
            repository_descriptor,
            normalized_target_path,
            baseline,
            LineDiffPayload::NoChanges,
            started_at,
        );
    }

    if delta_count != 1 {
        return Err(line_diff_failed(format!(
            "line_diff expected a single diff delta for target `{normalized_target_path}`, but collected {delta_count}"
        )));
    }

    let delta = diff.get_delta(0).ok_or_else(|| {
        line_diff_failed(format!(
            "line_diff failed to access diff delta for target `{normalized_target_path}`"
        ))
    })?;

    if delta.old_file().is_binary() || delta.new_file().is_binary() {
        return finalize_line_diff_execution(
            repository_descriptor,
            normalized_target_path,
            baseline,
            LineDiffPayload::BinaryContent,
            started_at,
        );
    }

    let patch = collect_patch_details_from_diff(&diff, 0, line_diff_failed)?.ok_or_else(|| {
            line_diff_failed(format!(
                "line_diff could not materialize typed patch details for target `{normalized_target_path}`"
            ))
        })?;

    finalize_line_diff_execution(
        repository_descriptor,
        normalized_target_path,
        baseline,
        LineDiffPayload::TextDiff { patch },
        started_at,
    )
}

fn normalize_target_path(target_path: &Path, repo_root: &Path) -> GitResult<String> {
    match normalize_filesystem_path_to_repository_relative(target_path, repo_root) {
        Ok(NormalizedRepositoryPath::RepositoryRelative(path)) => Ok(path),
        Ok(NormalizedRepositoryPath::RepositoryRoot) => Err(line_diff_invalid_path(format!(
            "line_diff target path `{}` resolves to repository root `{}` instead of a file path",
            target_path.display(),
            repo_root.display()
        ))),
        Err(FilesystemPathValidationFailure::Empty) => Err(line_diff_invalid_path(
            "line_diff target path must not be empty",
        )),
        Err(FilesystemPathValidationFailure::ContainsNul) => Err(line_diff_invalid_path(
            "line_diff target path must not contain NUL bytes",
        )),
        Err(FilesystemPathValidationFailure::EscapesRepository) => {
            Err(line_diff_invalid_path(format!(
                "line_diff target path `{}` escapes repository root `{}`",
                target_path.display(),
                repo_root.display()
            )))
        }
        Err(FilesystemPathValidationFailure::OutsideRepository) => {
            Err(line_diff_invalid_path(format!(
                "line_diff target path `{}` is outside repository root `{}`",
                target_path.display(),
                repo_root.display()
            )))
        }
        Err(FilesystemPathValidationFailure::CannotNormalize) => {
            Err(line_diff_invalid_path(format!(
                "line_diff target path `{}` cannot be normalized into repository-relative form",
                target_path.display()
            )))
        }
    }
}

fn reject_directory_target(worktree_root: &Path, normalized_target_path: &str) -> GitResult<()> {
    let target_path = worktree_root.join(normalized_target_path);
    if std::fs::symlink_metadata(&target_path).is_ok_and(|metadata| metadata.file_type().is_dir()) {
        return Err(line_diff_invalid_path(format!(
            "line_diff target `{}` resolves to a directory in worktree `{}`",
            normalized_target_path,
            worktree_root.display()
        )));
    }

    Ok(())
}

fn resolve_head_tree(repository: &Repository) -> GitResult<Option<git2::Tree<'_>>> {
    match repository.head() {
        Ok(head) => head.peel_to_tree().map(Some).map_err(|error| {
            line_diff_failed(format!(
                "failed to resolve HEAD tree for repository `{}`: {error}",
                repository.path().display()
            ))
        }),
        Err(error) if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) => {
            Ok(None)
        }
        Err(error) => Err(line_diff_failed(format!(
            "failed to resolve HEAD for repository `{}`: {error}",
            repository.path().display()
        ))),
    }
}

fn resolve_baseline(
    head_tree: Option<&git2::Tree<'_>>,
    normalized_target_path: &str,
) -> GitResult<LineDiffBaseline> {
    let Some(head_tree) = head_tree else {
        return Ok(LineDiffBaseline::EmptyToWorktreeUnbornHead);
    };

    match head_tree.get_path(Path::new(normalized_target_path)) {
        Ok(entry) => {
            if entry.kind() == Some(git2::ObjectType::Tree) {
                return Err(line_diff_invalid_path(format!(
                    "line_diff target `{normalized_target_path}` points to a directory tree in HEAD"
                )));
            }

            if entry.kind() == Some(git2::ObjectType::Commit) {
                return Err(line_diff_failed(format!(
                    "line_diff target `{normalized_target_path}` points to a gitlink/submodule entry that cannot produce line-level diff"
                )));
            }

            Ok(LineDiffBaseline::HeadToWorktree)
        }
        Err(error) if error.code() == ErrorCode::NotFound => {
            Ok(LineDiffBaseline::EmptyToWorktreeUntracked)
        }
        Err(error) => Err(line_diff_failed(format!(
            "failed to inspect HEAD entry for target `{normalized_target_path}`: {error}"
        ))),
    }
}

fn has_external_diff_driver(
    repository: &Repository,
    normalized_target_path: &str,
) -> GitResult<bool> {
    let config = repository.config().map_err(|error| {
        line_diff_failed(format!(
            "failed to open repository config while checking diff driver for `{normalized_target_path}`: {error}"
        ))
    })?;

    if config_key_has_text(&config, "diff.external")? {
        return Ok(true);
    }

    let diff_attr = repository
        .get_attr(
            Path::new(normalized_target_path),
            "diff",
            AttrCheckFlags::FILE_THEN_INDEX,
        )
        .map_err(|error| {
            line_diff_failed(format!(
                "failed to resolve `diff` attribute for target `{normalized_target_path}`: {error}"
            ))
        })?;

    let AttrValue::String(driver_name) = AttrValue::from_string(diff_attr) else {
        return Ok(false);
    };
    let driver_name = driver_name.trim();
    if driver_name.is_empty() {
        return Ok(false);
    }

    Ok(
        config_key_has_text(&config, &format!("diff.{driver_name}.command"))?
            || config_key_has_text(&config, &format!("diff.{driver_name}.textconv"))?,
    )
}

fn config_key_has_text(config: &git2::Config, key: &str) -> GitResult<bool> {
    match config.get_string(key) {
        Ok(value) => Ok(!value.trim().is_empty()),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
        Err(error) => Err(line_diff_failed(format!(
            "failed to read git config key `{key}` while resolving line_diff semantics: {error}"
        ))),
    }
}

fn build_head_to_worktree_diff<'repo>(
    repository: &'repo Repository,
    head_tree: Option<&git2::Tree<'_>>,
    normalized_target_path: &str,
) -> GitResult<git2::Diff<'repo>> {
    let mut diff_options = DiffOptions::new();
    diff_options
        .include_typechange(true)
        .include_untracked(true)
        .disable_pathspec_match(true)
        .pathspec(normalized_target_path);

    repository
        .diff_tree_to_workdir(head_tree, Some(&mut diff_options))
        .map_err(|error| {
            line_diff_failed(format!(
                "failed to build line_diff (`HEAD -> worktree`) for target `{normalized_target_path}` in repository `{}`: {error}",
                repository.path().display()
            ))
        })
}

fn finalize_line_diff_execution(
    repository: crate::git_manager::core::RepositoryDescriptor,
    normalized_target_path: String,
    baseline: LineDiffBaseline,
    payload: LineDiffPayload,
    started_at: Instant,
) -> GitResult<LineDiffExecutionOutput> {
    let binary = matches!(payload, LineDiffPayload::BinaryContent);
    let (hunk_count, line_count) = match &payload {
        LineDiffPayload::TextDiff { patch } => (patch.hunks.len(), patch_line_count(patch)),
        LineDiffPayload::BinaryContent
        | LineDiffPayload::NoChanges
        | LineDiffPayload::ExternalDriverConfigured => (0, 0),
    };
    let result = LineDiffResult::new(repository, normalized_target_path.as_str(), payload)
        .ok_or_else(|| {
            line_diff_failed(format!(
                "line_diff produced an invalid repository-relative target path `{normalized_target_path}`"
            ))
        })?;
    let baseline_kind = baseline.kind_name();
    let elapsed_ms = started_at.elapsed().as_millis();

    tracing::Span::current().record("baseline_kind", tracing::field::display(baseline_kind));
    tracing::Span::current().record("binary", tracing::field::display(binary));
    tracing::Span::current().record("hunk_count", tracing::field::display(hunk_count));
    tracing::Span::current().record("line_count", tracing::field::display(line_count));
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "line_diff",
        repo_root = %result.repository.repo_root.display(),
        normalized_target_path = %result.target_path,
        baseline_kind,
        binary,
        hunk_count,
        line_count,
        elapsed_ms,
        "collected typed single-path line diff"
    );

    Ok(LineDiffExecutionOutput {
        result,
        normalized_target_path,
        baseline_kind,
        binary,
        hunk_count,
        line_count,
    })
}

fn line_diff_invalid_path(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::LineDiffInvalidPath, message)
}

fn line_diff_failed(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::LineDiffFailed, message)
}

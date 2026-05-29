//! `working_copy_status` operation for `GitManager`.

#![allow(
    clippy::map_unwrap_or,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_value,
    clippy::question_mark,
    clippy::redundant_clone,
    clippy::too_many_lines
)]

use crate::git_manager::core::operations_repository_path_support::{
    normalize_filesystem_path_to_repository_relative, FilesystemPathValidationFailure,
    NormalizedRepositoryPath,
};
use crate::git_manager::core::repository_access::{
    describe_opened_repository, open_repository_context,
};
use crate::git_manager::core::{
    normalize_repository_relative_path, GitError, GitErrorCode, GitErrorRetryClassification,
    GitResult, RepositoryPathKind, TrackedChangeKind, WorkingCopyEntry, WorkingCopyEntryKind,
    WorkingCopyEntryOrigin, WorkingCopyScope, WorkingCopyStatusRequest, WorkingCopyStatusResult,
};
use git2::{
    Diff, DiffDelta, DiffFindOptions, DiffOptions, ErrorClass, ErrorCode, FileMode, Repository,
    Status, StatusOptions,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

const RENAME_THRESHOLD: u16 = 50;
const FALLBACK_EMPTY_SCOPE: &str = "empty_scope";
const FALLBACK_ROOT_WIDE_SCOPE: &str = "root_wide_scope";
const FALLBACK_RENAME_OR_COPY_FIDELITY: &str = "rename_or_copy_fidelity_requires_full_snapshot";

#[derive(Debug, Clone, PartialEq, Eq)]
enum EffectiveSnapshotScope {
    Full,
    Paths { pathspecs: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedSnapshotScope {
    requested_paths: Vec<String>,
    effective_scope: EffectiveSnapshotScope,
    fallback_reason: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct WorkingCopySnapshot {
    entries: Vec<WorkingCopyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackedEntryState {
    path: String,
    path_kind: RepositoryPathKind,
    index: Option<TrackedChangeKind>,
    worktree: Option<TrackedChangeKind>,
    conflicted: bool,
    rename_from: Option<String>,
    copy_from: Option<String>,
}

impl TrackedEntryState {
    fn new(path: String, path_kind: RepositoryPathKind) -> Self {
        Self {
            path,
            path_kind,
            index: None,
            worktree: None,
            conflicted: false,
            rename_from: None,
            copy_from: None,
        }
    }

    fn update_path_kind(&mut self, path_kind: RepositoryPathKind) {
        if !matches!(path_kind, RepositoryPathKind::Other) {
            self.path_kind = path_kind;
        }
    }

    fn update_index_delta(&mut self, delta: &DiffDelta<'_>) {
        if let Some(change_kind) = map_delta_to_tracked_change(delta.status()) {
            self.index = Some(change_kind);
            self.update_path_kind(path_kind_from_delta(delta));

            match change_kind {
                TrackedChangeKind::Renamed => {
                    self.rename_from = delta
                        .old_file()
                        .path()
                        .and_then(path_to_repo_relative_string);
                    self.copy_from = None;
                }
                TrackedChangeKind::Copied => {
                    self.copy_from = delta
                        .old_file()
                        .path()
                        .and_then(path_to_repo_relative_string);
                    self.rename_from = None;
                }
                _ => {}
            }
        }
    }

    fn update_worktree_delta(&mut self, delta: &DiffDelta<'_>) {
        if let Some(change_kind) = map_delta_to_tracked_change(delta.status()) {
            self.worktree = Some(change_kind);
            self.update_path_kind(path_kind_from_delta(delta));

            match change_kind {
                TrackedChangeKind::Renamed => {
                    self.rename_from = delta
                        .old_file()
                        .path()
                        .and_then(path_to_repo_relative_string);
                    self.copy_from = None;
                }
                TrackedChangeKind::Copied => {
                    self.copy_from = delta
                        .old_file()
                        .path()
                        .and_then(path_to_repo_relative_string);
                    self.rename_from = None;
                }
                _ => {}
            }
        }
    }

    fn update_conflict(&mut self, path_kind: RepositoryPathKind) {
        self.conflicted = true;
        self.update_path_kind(path_kind);
    }

    fn build(self) -> Option<WorkingCopyEntry> {
        let kind = WorkingCopyEntryKind::tracked(
            self.index,
            self.worktree,
            self.conflicted,
            self.rename_from,
            self.copy_from,
        )?;
        WorkingCopyEntry::new(
            self.path,
            self.path_kind,
            WorkingCopyEntryOrigin::Leaf,
            kind,
        )
    }
}

/// Reads a typed working copy snapshot via `libgit2`.
///
/// # Errors
/// Returns a typed `GitError` if repository discovery, scope normalization,
/// or reading the snapshot fails.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            operation = "working_copy_status",
            requested_path = %request.repository_path.display(),
            repo_root = tracing::field::Empty,
            scope_kind = request.scope.kind_name(),
            path_count = request.scope.path_count(),
            pathspec_count = tracing::field::Empty,
            include_ignored = request.include_ignored,
            include_untracked = request.include_untracked,
            detect_renames = request.detect_renames,
            detect_copies = request.detect_copies,
            entry_count = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
            scoped_to_full_fallback_reason = tracing::field::Empty
        )
    )
)]
pub(super) fn execute_working_copy_status_operation(
    request: &WorkingCopyStatusRequest,
) -> GitResult<WorkingCopyStatusResult> {
    let started_at = Instant::now();
    let opened_repository =
        open_repository_context(&request.repository_path, "working_copy_status")?;
    let repository_descriptor = describe_opened_repository(&opened_repository);
    let normalized_scope = normalize_scope(request, repository_descriptor.repo_root.as_path())?;

    tracing::Span::current().record(
        "repo_root",
        tracing::field::display(repository_descriptor.repo_root.display()),
    );
    tracing::Span::current().record(
        "pathspec_count",
        tracing::field::display(normalized_scope.requested_paths.len()),
    );
    tracing::Span::current().record(
        "scoped_to_full_fallback_reason",
        tracing::field::display(normalized_scope.fallback_reason.unwrap_or("none")),
    );

    let mut snapshot = if opened_repository.is_bare {
        WorkingCopySnapshot {
            entries: Vec::new(),
        }
    } else {
        collect_snapshot(
            &opened_repository.repository,
            repository_descriptor.worktree_root.as_deref(),
            request,
            &normalized_scope,
        )?
    };

    if matches!(
        normalized_scope.effective_scope,
        EffectiveSnapshotScope::Full
    ) && !normalized_scope.requested_paths.is_empty()
    {
        snapshot.entries =
            filter_snapshot_entries(snapshot.entries, &normalized_scope.requested_paths);
    }

    normalize_snapshot_entries(&mut snapshot.entries);

    let elapsed_ms = started_at.elapsed().as_millis();
    tracing::Span::current().record(
        "entry_count",
        tracing::field::display(snapshot.entries.len()),
    );
    tracing::Span::current().record("elapsed_ms", tracing::field::display(elapsed_ms));
    tracing::trace!(
        operation = "working_copy_status",
        requested_path = %request.repository_path.display(),
        repo_root = %repository_descriptor.repo_root.display(),
        scope_kind = request.scope.kind_name(),
        path_count = request.scope.path_count(),
        pathspec_count = normalized_scope.requested_paths.len(),
        include_ignored = request.include_ignored,
        include_untracked = request.include_untracked,
        detect_renames = request.detect_renames,
        detect_copies = request.detect_copies,
        entry_count = snapshot.entries.len(),
        elapsed_ms,
        scoped_to_full_fallback_reason = normalized_scope.fallback_reason.unwrap_or("none"),
        "collected typed working copy snapshot"
    );

    Ok(WorkingCopyStatusResult::new(
        repository_descriptor,
        snapshot.entries,
    ))
}

fn collect_snapshot(
    repository: &Repository,
    worktree_root: Option<&Path>,
    request: &WorkingCopyStatusRequest,
    normalized_scope: &NormalizedSnapshotScope,
) -> GitResult<WorkingCopySnapshot> {
    let pathspecs = match &normalized_scope.effective_scope {
        EffectiveSnapshotScope::Full => &[][..],
        EffectiveSnapshotScope::Paths { pathspecs } => pathspecs.as_slice(),
    };

    let statuses = collect_statuses(repository, request, pathspecs)?;
    let staged_diff = build_staged_diff(repository, request, pathspecs)?;
    let unstaged_diff = build_unstaged_diff(repository, request, pathspecs)?;

    Ok(build_snapshot_from_git2(
        repository,
        worktree_root,
        request,
        statuses,
        &staged_diff,
        &unstaged_diff,
    ))
}

fn normalize_scope(
    request: &WorkingCopyStatusRequest,
    repo_root: &Path,
) -> GitResult<NormalizedSnapshotScope> {
    match &request.scope {
        WorkingCopyScope::Full => Ok(NormalizedSnapshotScope {
            requested_paths: Vec::new(),
            effective_scope: EffectiveSnapshotScope::Full,
            fallback_reason: None,
        }),
        WorkingCopyScope::Paths { paths } => {
            if paths.is_empty() {
                return Ok(NormalizedSnapshotScope {
                    requested_paths: Vec::new(),
                    effective_scope: EffectiveSnapshotScope::Full,
                    fallback_reason: Some(FALLBACK_EMPTY_SCOPE),
                });
            }

            let mut requested_paths = Vec::new();
            let mut saw_repository_root = false;
            for path in paths {
                match normalize_scope_path(path.as_path(), repo_root)? {
                    ScopePathNormalization::RepositoryRoot => {
                        saw_repository_root = true;
                    }
                    ScopePathNormalization::RepositoryRelative(path) => {
                        requested_paths.push(path);
                    }
                }
            }
            requested_paths.sort();
            requested_paths.dedup();

            if requested_paths.is_empty() || saw_repository_root {
                return Ok(NormalizedSnapshotScope {
                    requested_paths,
                    effective_scope: EffectiveSnapshotScope::Full,
                    fallback_reason: Some(FALLBACK_ROOT_WIDE_SCOPE),
                });
            }

            if request.detect_renames || request.detect_copies {
                return Ok(NormalizedSnapshotScope {
                    requested_paths,
                    effective_scope: EffectiveSnapshotScope::Full,
                    fallback_reason: Some(FALLBACK_RENAME_OR_COPY_FIDELITY),
                });
            }

            Ok(NormalizedSnapshotScope {
                effective_scope: EffectiveSnapshotScope::Paths {
                    pathspecs: requested_paths.clone(),
                },
                requested_paths,
                fallback_reason: None,
            })
        }
    }
}

fn collect_statuses<'repo>(
    repository: &'repo Repository,
    request: &WorkingCopyStatusRequest,
    pathspecs: &[String],
) -> GitResult<git2::Statuses<'repo>> {
    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(request.include_untracked)
        .include_ignored(request.include_ignored)
        .recurse_untracked_dirs(request.include_untracked && !request.include_directories)
        .recurse_ignored_dirs(request.include_ignored && !request.include_directories)
        .disable_pathspec_match(true)
        .renames_head_to_index(request.detect_renames || request.detect_copies)
        .renames_index_to_workdir(request.detect_renames || request.detect_copies)
        .renames_from_rewrites(request.detect_renames || request.detect_copies)
        .rename_threshold(RENAME_THRESHOLD);

    for pathspec in pathspecs {
        status_options.pathspec(pathspec.as_str());
    }

    repository
        .statuses(Some(&mut status_options))
        .map_err(|error| {
            working_copy_snapshot_failed(
                format!(
                    "failed to collect working copy status list for repository `{}`: {error}",
                    repository.path().display()
                ),
                Some(&error),
            )
        })
}

fn build_staged_diff<'repo>(
    repository: &'repo Repository,
    request: &WorkingCopyStatusRequest,
    pathspecs: &[String],
) -> GitResult<Diff<'repo>> {
    let head_tree = resolve_head_tree(repository)?;
    let mut diff_options = DiffOptions::new();
    diff_options
        .include_typechange(true)
        .disable_pathspec_match(true);
    for pathspec in pathspecs {
        diff_options.pathspec(pathspec.as_str());
    }

    let mut diff = repository
        .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_options))
        .map_err(|error| {
            working_copy_snapshot_failed(
                format!(
                    "failed to build staged working copy diff for repository `{}`: {error}",
                    repository.path().display()
                ),
                Some(&error),
            )
        })?;

    if request.detect_renames || request.detect_copies {
        enable_similarity_detection(&mut diff, request)?;
    }

    Ok(diff)
}

fn build_unstaged_diff<'repo>(
    repository: &'repo Repository,
    request: &WorkingCopyStatusRequest,
    pathspecs: &[String],
) -> GitResult<Diff<'repo>> {
    let mut diff_options = DiffOptions::new();
    diff_options
        .include_typechange(true)
        .disable_pathspec_match(true);
    for pathspec in pathspecs {
        diff_options.pathspec(pathspec.as_str());
    }

    let mut diff = repository
        .diff_index_to_workdir(None, Some(&mut diff_options))
        .map_err(|error| {
            working_copy_snapshot_failed(
                format!(
                    "failed to build unstaged working copy diff for repository `{}`: {error}",
                    repository.path().display()
                ),
                Some(&error),
            )
        })?;

    if request.detect_renames || request.detect_copies {
        enable_similarity_detection(&mut diff, request)?;
    }

    Ok(diff)
}

fn enable_similarity_detection(
    diff: &mut Diff<'_>,
    request: &WorkingCopyStatusRequest,
) -> GitResult<()> {
    let mut find_options = DiffFindOptions::new();
    find_options
        .renames(request.detect_renames || request.detect_copies)
        .copies(request.detect_copies)
        .copies_from_unmodified(request.detect_copies)
        .renames_from_rewrites(request.detect_renames || request.detect_copies)
        .rename_threshold(RENAME_THRESHOLD);
    diff.find_similar(Some(&mut find_options)).map_err(|error| {
        working_copy_snapshot_failed(
            format!("failed to run rename/copy detection for working copy snapshot: {error}"),
            Some(&error),
        )
    })
}

fn resolve_head_tree(repository: &Repository) -> GitResult<Option<git2::Tree<'_>>> {
    match repository.head() {
        Ok(head) => head.peel_to_tree().map(Some).map_err(|error| {
            working_copy_snapshot_failed(
                format!(
                    "failed to resolve HEAD tree for repository `{}`: {error}",
                    repository.path().display()
                ),
                Some(&error),
            )
        }),
        Err(error) if matches!(error.code(), ErrorCode::NotFound | ErrorCode::UnbornBranch) => {
            Ok(None)
        }
        Err(error) => Err(working_copy_snapshot_failed(
            format!(
                "failed to resolve HEAD for repository `{}`: {error}",
                repository.path().display()
            ),
            Some(&error),
        )),
    }
}

fn build_snapshot_from_git2(
    repository: &Repository,
    worktree_root: Option<&Path>,
    request: &WorkingCopyStatusRequest,
    statuses: git2::Statuses<'_>,
    staged_diff: &Diff<'_>,
    unstaged_diff: &Diff<'_>,
) -> WorkingCopySnapshot {
    let mut tracked_entries = BTreeMap::<String, TrackedEntryState>::new();
    let mut entries = Vec::new();

    for delta in staged_diff.deltas() {
        update_tracked_entries(&mut tracked_entries, &delta, true);
    }
    for delta in unstaged_diff.deltas() {
        update_tracked_entries(&mut tracked_entries, &delta, false);
    }

    for status_entry in statuses.iter() {
        let raw_path = status_entry_path(&status_entry);
        let Some(path) = normalize_repository_relative_path(&raw_path) else {
            continue;
        };

        if status_entry.status().contains(Status::CONFLICTED) {
            let state = tracked_entries
                .entry(path.clone())
                .or_insert_with(|| TrackedEntryState::new(path.clone(), RepositoryPathKind::Other));
            state.update_conflict(resolve_status_path_kind(
                repository,
                worktree_root,
                status_entry.path(),
                path.as_str(),
                &status_entry,
            ));
        }

        if request.include_ignored && status_entry.status().contains(Status::IGNORED) {
            if let Some(entry) = build_non_tracked_entry(
                repository,
                worktree_root,
                status_entry.path(),
                path.as_str(),
                &status_entry,
                WorkingCopyEntryKind::Ignored,
            ) {
                entries.push(entry);
            }
        }

        if request.include_untracked
            && status_entry.status().contains(Status::WT_NEW)
            && !status_entry.status().contains(Status::INDEX_NEW)
        {
            if let Some(entry) = build_non_tracked_entry(
                repository,
                worktree_root,
                status_entry.path(),
                path.as_str(),
                &status_entry,
                WorkingCopyEntryKind::Untracked,
            ) {
                entries.push(entry);
            }
        }
    }

    entries.extend(
        tracked_entries
            .into_values()
            .filter_map(TrackedEntryState::build),
    );
    WorkingCopySnapshot { entries }
}

fn build_non_tracked_entry(
    repository: &Repository,
    worktree_root: Option<&Path>,
    raw_path: Option<&str>,
    normalized_path: &str,
    status_entry: &git2::StatusEntry<'_>,
    kind: WorkingCopyEntryKind,
) -> Option<WorkingCopyEntry> {
    let origin = if is_direct_directory_entry(raw_path) {
        WorkingCopyEntryOrigin::DirectDirectory
    } else {
        WorkingCopyEntryOrigin::Leaf
    };

    let path_kind = resolve_status_path_kind(
        repository,
        worktree_root,
        raw_path,
        normalized_path,
        status_entry,
    );

    WorkingCopyEntry::new(normalized_path, path_kind, origin, kind)
}

fn update_tracked_entries(
    tracked_entries: &mut BTreeMap<String, TrackedEntryState>,
    delta: &DiffDelta<'_>,
    index_delta: bool,
) {
    let Some(path) = delta_effective_path(delta) else {
        return;
    };
    let path_kind = path_kind_from_delta(delta);
    let entry = tracked_entries
        .entry(path.clone())
        .or_insert_with(|| TrackedEntryState::new(path, path_kind));
    entry.update_path_kind(path_kind);
    if index_delta {
        entry.update_index_delta(delta);
    } else {
        entry.update_worktree_delta(delta);
    }
}

fn delta_effective_path(delta: &DiffDelta<'_>) -> Option<String> {
    let raw_path = if matches!(delta.status(), git2::Delta::Deleted) {
        delta.old_file().path().or_else(|| delta.new_file().path())
    } else {
        delta.new_file().path().or_else(|| delta.old_file().path())
    }?;
    path_to_repo_relative_string(raw_path)
}

fn build_scope_invalid_pathspec(message: impl Into<String>) -> GitError {
    GitError::new(GitErrorCode::InvalidPathspec, message)
}

fn working_copy_snapshot_failed(
    message: impl Into<String>,
    error: Option<&git2::Error>,
) -> GitError {
    let retry_classification = error
        .map(classify_snapshot_retryability)
        .unwrap_or(GitErrorRetryClassification::Permanent);
    GitError::new(GitErrorCode::WorkingCopySnapshotFailed, message)
        .with_retry_classification(retry_classification)
}

fn classify_snapshot_retryability(error: &git2::Error) -> GitErrorRetryClassification {
    if matches!(
        error.code(),
        ErrorCode::Locked | ErrorCode::Modified | ErrorCode::NotFound | ErrorCode::Exists
    ) {
        return GitErrorRetryClassification::Retryable;
    }

    if matches!(error.class(), ErrorClass::Os | ErrorClass::Index) {
        let normalized = error.message().to_ascii_lowercase();
        if normalized.contains("no such file")
            || normalized.contains("file changed")
            || normalized.contains("index is locked")
            || normalized.contains("resource temporarily unavailable")
        {
            return GitErrorRetryClassification::Retryable;
        }
    }

    GitErrorRetryClassification::Permanent
}

enum ScopePathNormalization {
    RepositoryRoot,
    RepositoryRelative(String),
}

fn normalize_scope_path(path: &Path, repo_root: &Path) -> GitResult<ScopePathNormalization> {
    match normalize_filesystem_path_to_repository_relative(path, repo_root) {
        Ok(NormalizedRepositoryPath::RepositoryRoot) => Ok(ScopePathNormalization::RepositoryRoot),
        Ok(NormalizedRepositoryPath::RepositoryRelative(path)) => {
            Ok(ScopePathNormalization::RepositoryRelative(path))
        }
        Err(FilesystemPathValidationFailure::Empty) => Err(build_scope_invalid_pathspec(
            "working_copy_status scope paths must not be empty",
        )),
        Err(FilesystemPathValidationFailure::ContainsNul) => Err(build_scope_invalid_pathspec(
            "working_copy_status scope paths must not contain NUL bytes",
        )),
        Err(FilesystemPathValidationFailure::EscapesRepository) => {
            Err(build_scope_invalid_pathspec(format!(
                "working_copy_status scope path `{}` escapes repository root `{}`",
                path.display(),
                repo_root.display()
            )))
        }
        Err(FilesystemPathValidationFailure::OutsideRepository) => {
            Err(build_scope_invalid_pathspec(format!(
                "working_copy_status scope path `{}` is outside repository root `{}`",
                path.display(),
                repo_root.display()
            )))
        }
        Err(FilesystemPathValidationFailure::CannotNormalize) => {
            Err(build_scope_invalid_pathspec(format!(
                "working_copy_status scope path `{}` cannot be normalized into repository-relative form",
                path.display()
            )))
        }
    }
}

fn resolve_status_path_kind(
    repository: &Repository,
    worktree_root: Option<&Path>,
    raw_path: Option<&str>,
    normalized_path: &str,
    status_entry: &git2::StatusEntry<'_>,
) -> RepositoryPathKind {
    if is_direct_directory_entry(raw_path) {
        return RepositoryPathKind::Directory;
    }

    if let Some(path_kind) = status_entry
        .index_to_workdir()
        .map(|delta| path_kind_from_delta(&delta))
        .filter(|path_kind| !matches!(path_kind, RepositoryPathKind::Other))
    {
        return path_kind;
    }

    if let Some(path_kind) = status_entry
        .head_to_index()
        .map(|delta| path_kind_from_delta(&delta))
        .filter(|path_kind| !matches!(path_kind, RepositoryPathKind::Other))
    {
        return path_kind;
    }

    if let Some(worktree_root) = worktree_root {
        let candidate_path = worktree_root.join(normalized_path);
        if let Ok(metadata) = std::fs::symlink_metadata(&candidate_path) {
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                return RepositoryPathKind::Directory;
            }
            if file_type.is_symlink() {
                return RepositoryPathKind::Symlink;
            }
            if file_type.is_file() {
                return RepositoryPathKind::File;
            }
        }
    }

    if repository.find_submodule(normalized_path).is_ok() {
        return RepositoryPathKind::Submodule;
    }

    RepositoryPathKind::Other
}

fn path_kind_from_delta(delta: &DiffDelta<'_>) -> RepositoryPathKind {
    let file = if matches!(delta.status(), git2::Delta::Deleted) {
        delta.old_file()
    } else {
        delta.new_file()
    };

    if matches!(file.mode(), FileMode::Unreadable)
        && !matches!(delta.old_file().mode(), FileMode::Unreadable)
    {
        return map_file_mode(delta.old_file().mode());
    }

    map_file_mode(file.mode())
}

const fn map_file_mode(file_mode: FileMode) -> RepositoryPathKind {
    match file_mode {
        FileMode::Tree => RepositoryPathKind::Directory,
        FileMode::Blob | FileMode::BlobExecutable | FileMode::BlobGroupWritable => {
            RepositoryPathKind::File
        }
        FileMode::Link => RepositoryPathKind::Symlink,
        FileMode::Commit => RepositoryPathKind::GitLink,
        FileMode::Unreadable => RepositoryPathKind::Other,
    }
}

fn map_delta_to_tracked_change(delta: git2::Delta) -> Option<TrackedChangeKind> {
    match delta {
        git2::Delta::Added => Some(TrackedChangeKind::Added),
        git2::Delta::Modified => Some(TrackedChangeKind::Modified),
        git2::Delta::Deleted => Some(TrackedChangeKind::Deleted),
        git2::Delta::Renamed => Some(TrackedChangeKind::Renamed),
        git2::Delta::Copied => Some(TrackedChangeKind::Copied),
        git2::Delta::Typechange => Some(TrackedChangeKind::TypeChange),
        git2::Delta::Conflicted
        | git2::Delta::Ignored
        | git2::Delta::Untracked
        | git2::Delta::Unmodified
        | git2::Delta::Unreadable => None,
    }
}

fn filter_snapshot_entries(
    entries: Vec<WorkingCopyEntry>,
    requested_paths: &[String],
) -> Vec<WorkingCopyEntry> {
    entries
        .into_iter()
        .filter(|entry| entry_matches_requested_scope(entry, requested_paths))
        .collect()
}

fn entry_matches_requested_scope(entry: &WorkingCopyEntry, requested_paths: &[String]) -> bool {
    requested_paths.iter().any(|requested_path| {
        path_is_within(entry.path.as_str(), requested_path.as_str())
            || matches!(entry.origin, WorkingCopyEntryOrigin::DirectDirectory)
                && path_is_within(requested_path.as_str(), entry.path.as_str())
            || tracked_metadata_matches_scope(&entry.kind, requested_path.as_str())
    })
}

fn tracked_metadata_matches_scope(kind: &WorkingCopyEntryKind, requested_path: &str) -> bool {
    match kind {
        WorkingCopyEntryKind::Tracked {
            rename_from,
            copy_from,
            ..
        } => {
            rename_from
                .as_deref()
                .is_some_and(|path| path_is_within(path, requested_path))
                || copy_from
                    .as_deref()
                    .is_some_and(|path| path_is_within(path, requested_path))
        }
        WorkingCopyEntryKind::Untracked | WorkingCopyEntryKind::Ignored => false,
    }
}

fn path_is_within(path: &str, parent: &str) -> bool {
    path == parent
        || path
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalize_snapshot_entries(entries: &mut Vec<WorkingCopyEntry>) {
    entries.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(entry_origin_rank(left.origin).cmp(&entry_origin_rank(right.origin)))
            .then(entry_kind_rank(&left.kind).cmp(&entry_kind_rank(&right.kind)))
            .then(path_kind_rank(left.path_kind).cmp(&path_kind_rank(right.path_kind)))
    });
    entries.dedup();
}

const fn entry_origin_rank(origin: WorkingCopyEntryOrigin) -> u8 {
    match origin {
        WorkingCopyEntryOrigin::Leaf => 0,
        WorkingCopyEntryOrigin::DirectDirectory => 1,
    }
}

fn entry_kind_rank(kind: &WorkingCopyEntryKind) -> u8 {
    match kind {
        WorkingCopyEntryKind::Tracked {
            index,
            worktree,
            conflicted,
            ..
        } => {
            if *conflicted {
                8
            } else {
                tracked_change_rank(*index).min(tracked_change_rank(*worktree))
            }
        }
        WorkingCopyEntryKind::Untracked => 9,
        WorkingCopyEntryKind::Ignored => 10,
    }
}

const fn tracked_change_rank(change: Option<TrackedChangeKind>) -> u8 {
    match change {
        Some(TrackedChangeKind::Added) => 0,
        Some(TrackedChangeKind::Modified) => 1,
        Some(TrackedChangeKind::Deleted) => 2,
        Some(TrackedChangeKind::Renamed) => 3,
        Some(TrackedChangeKind::Copied) => 4,
        Some(TrackedChangeKind::TypeChange) => 5,
        None => 7,
    }
}

const fn path_kind_rank(path_kind: RepositoryPathKind) -> u8 {
    match path_kind {
        RepositoryPathKind::File => 0,
        RepositoryPathKind::Directory => 1,
        RepositoryPathKind::Symlink => 2,
        RepositoryPathKind::Submodule => 3,
        RepositoryPathKind::GitLink => 4,
        RepositoryPathKind::Other => 5,
    }
}

fn is_direct_directory_entry(raw_path: Option<&str>) -> bool {
    raw_path.is_some_and(|path| path.ends_with('/'))
}

fn status_entry_path(entry: &git2::StatusEntry<'_>) -> String {
    entry.path().map_or_else(
        || String::from_utf8_lossy(entry.path_bytes()).into_owned(),
        str::to_owned,
    )
}

fn path_to_repo_relative_string(path: &Path) -> Option<String> {
    normalize_repository_relative_path(path.to_string_lossy())
}


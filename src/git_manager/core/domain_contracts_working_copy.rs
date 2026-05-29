//! Contracts of the read-only `working_copy_status` domain.

#![allow(
    clippy::fn_params_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::option_option,
    clippy::struct_excessive_bools
)]

use super::RepositoryDescriptor;
use std::path::PathBuf;

/// Request to read a typed working copy snapshot.
///
/// `repository_path` accepts an untrusted path to the repository root, its `git_dir`, or any
/// nested entity inside the worktree. The `scope` field selects a full snapshot or a scoped
/// refresh, while the scope normalization details and fallback policy are implemented by the
/// operation layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyStatusRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
    /// Scope of the snapshot refresh: the full repository or a limited set of paths.
    pub scope: WorkingCopyScope,
    /// Include `Untracked` entries in the result.
    pub include_untracked: bool,
    /// Include `Ignored` entries and direct-directory ignored entries in the result.
    pub include_ignored: bool,
    /// Allow the backend layer to detect rename semantics for tracked entries.
    pub detect_renames: bool,
    /// Allow the backend layer to detect copy semantics for tracked entries.
    pub detect_copies: bool,
    /// Include direct-directory entries when the status API returns a directory entry without recursing.
    pub include_directories: bool,
}

impl WorkingCopyStatusRequest {
    /// Creates a typed request to read a working-copy snapshot.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                scope_kind = scope.kind_name(),
                scope_path_count = scope.path_count(),
                include_untracked,
                include_ignored,
                detect_renames,
                detect_copies,
                include_directories
            )
        )
    )]
    #[must_use]
    pub fn new(
        repository_path: PathBuf,
        scope: WorkingCopyScope,
        include_untracked: bool,
        include_ignored: bool,
        detect_renames: bool,
        detect_copies: bool,
        include_directories: bool,
    ) -> Self {
        Self {
            repository_path,
            scope,
            include_untracked,
            include_ignored,
            detect_renames,
            detect_copies,
            include_directories,
        }
    }
}

/// Typed scope for a `working_copy_status` request.
///
/// Scoped paths are passed as filesystem input and then normalized by the operation layer
/// relative to the discovered `repo_root`. An empty or root-wide scope may be deterministically
/// collapsed to `Full` during the operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WorkingCopyScope {
    /// Full working copy snapshot.
    #[default]
    Full,
    /// Limited refresh over specific filesystem paths.
    Paths {
        /// List of input paths for which the backend collects a scoped snapshot.
        paths: Vec<PathBuf>,
    },
}

impl WorkingCopyScope {
    /// Creates a full-scope request.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn full() -> Self {
        Self::Full
    }

    /// Creates a scoped request over a set of filesystem paths.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn paths(paths: Vec<PathBuf>) -> Self {
        Self::Paths { paths }
    }

    /// Returns the machine-readable name of the scope kind.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Paths { .. } => "paths",
        }
    }

    /// Returns the number of paths in a scoped request.
    #[must_use]
    pub fn path_count(&self) -> usize {
        match self {
            Self::Full => 0,
            Self::Paths { paths } => paths.len(),
        }
    }
}

/// Typed result of the `working_copy_status` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyStatusResult {
    /// Canonical typed description of the discovered repository.
    pub repository: RepositoryDescriptor,
    /// Typed working copy entries without losing ignored/copy/rename semantics.
    pub entries: Vec<WorkingCopyEntry>,
}

impl WorkingCopyStatusResult {
    /// Creates a typed `working_copy_status` result.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(entry_count = entries.len()))
    )]
    #[must_use]
    pub fn new(repository: RepositoryDescriptor, entries: Vec<WorkingCopyEntry>) -> Self {
        Self {
            repository,
            entries,
        }
    }
}

/// Request to read a typed overview of the working tree.
///
/// The summary logic of this API is implemented by a separate operation layer; at the API-split
/// stage, the request establishes a distinct typed boundary for the future read-only overview
/// surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyOverviewRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
}

impl WorkingCopyOverviewRequest {
    /// Creates a typed request to read a read-only overview of the working tree.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(repository_path: PathBuf) -> Self {
        Self { repository_path }
    }
}

/// Typed read model of a brief working-tree overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyOverview {
    /// Canonical repository descriptor, including branch/upstream/ahead-behind metadata.
    pub repository: RepositoryDescriptor,
    /// Number of staged tracked changes.
    pub staged_count: usize,
    /// Number of unstaged tracked changes.
    pub unstaged_count: usize,
    /// Number of untracked entries.
    pub untracked_count: usize,
    /// Number of ignored entries.
    pub ignored_count: usize,
    /// Number of conflicted entries.
    pub conflicted_count: usize,
}

impl WorkingCopyOverview {
    /// Creates a typed working-tree overview.
    ///
    /// `repository` carries the canonical repository summary without string placeholders: the
    /// optional branch/upstream/ahead-behind metadata remains `None` if the backend cannot obtain it.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        repository: RepositoryDescriptor,
        staged_count: usize,
        unstaged_count: usize,
        untracked_count: usize,
        ignored_count: usize,
        conflicted_count: usize,
    ) -> Self {
        Self {
            repository,
            staged_count,
            unstaged_count,
            untracked_count,
            ignored_count,
            conflicted_count,
        }
    }
}

/// Typed result of the `working_copy_overview` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyOverviewResult {
    /// Read-only summary of the working tree.
    pub overview: WorkingCopyOverview,
}

impl WorkingCopyOverviewResult {
    /// Creates a typed `working_copy_overview` result.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(overview: WorkingCopyOverview) -> Self {
        Self { overview }
    }
}

/// Machine-readable tracked change kind within a working copy snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrackedChangeKind {
    /// Path addition.
    Added,
    /// Modification of content or metadata.
    Modified,
    /// Path deletion.
    Deleted,
    /// Path rename.
    Renamed,
    /// Path copy.
    Copied,
    /// Object type change (`file -> symlink`, `blob -> gitlink`, ...).
    TypeChange,
}

/// Typed classification of a working copy snapshot entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkingCopyEntryKind {
    /// A tracked change with separate index/worktree semantics.
    Tracked {
        /// Typed change kind in the index.
        index: Option<TrackedChangeKind>,
        /// Typed change kind in the working tree.
        worktree: Option<TrackedChangeKind>,
        /// Whether the entry is in a conflicted state.
        conflicted: bool,
        /// Source path for rename semantics.
        rename_from: Option<String>,
        /// Source path for copy semantics.
        copy_from: Option<String>,
    },
    /// An untracked path.
    Untracked,
    /// An ignored path.
    Ignored,
}

impl WorkingCopyEntryKind {
    /// Creates a tracked entry without losing rename/copy metadata.
    ///
    /// Returns `None` if the metadata violates the contract invariants:
    ///
    /// - `rename_from` is required for `Renamed` and forbidden for all other states;
    /// - `copy_from` is required for `Copied` and forbidden for all other states;
    /// - `rename_from` and `copy_from` cannot be set at the same time;
    /// - at least one of `index`, `worktree`, or `conflicted` must describe the entry's actual
    ///   state.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn tracked(
        index: Option<TrackedChangeKind>,
        worktree: Option<TrackedChangeKind>,
        conflicted: bool,
        rename_from: Option<String>,
        copy_from: Option<String>,
    ) -> Option<Self> {
        let rename_from = normalize_optional_repository_relative_path(rename_from)?;
        let copy_from = normalize_optional_repository_relative_path(copy_from)?;
        let requires_rename = change_requires_rename(index) || change_requires_rename(worktree);
        let requires_copy = change_requires_copy(index) || change_requires_copy(worktree);

        if requires_rename && requires_copy {
            return None;
        }
        if requires_rename != rename_from.is_some() {
            return None;
        }
        if requires_copy != copy_from.is_some() {
            return None;
        }
        if index.is_none() && worktree.is_none() && !conflicted {
            return None;
        }

        Some(Self::Tracked {
            index,
            worktree,
            conflicted,
            rename_from,
            copy_from,
        })
    }
}

/// Kind of path within the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RepositoryPathKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Submodule entry at the backend-model level.
    Submodule,
    /// Gitlink/commit entry.
    GitLink,
    /// Another type not covered by this enumeration.
    Other,
}

/// Origin of a typed entry within a working copy snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkingCopyEntryOrigin {
    /// An ordinary leaf entry.
    Leaf,
    /// An explicitly collapsed directory entry at the status API level.
    ///
    /// Used only for directory entries the backend obtained directly from the status API without
    /// recursively expanding their contents. The most typical cases are top-level
    /// ignored/untracked directories in the snapshot.
    DirectDirectory,
}

/// Typed working-tree entry without mixing in diff-oriented models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopyEntry {
    /// Repository-relative path without a leading `/`, without `..`, and with `/` as the separator.
    pub path: String,
    /// Kind of path within the repository.
    pub path_kind: RepositoryPathKind,
    /// Origin of the entry within the snapshot.
    pub origin: WorkingCopyEntryOrigin,
    /// Typed semantics of the working copy entry.
    pub kind: WorkingCopyEntryKind,
}

impl WorkingCopyEntry {
    /// Creates a typed working copy entry, normalizing the repo-relative path.
    ///
    /// Returns `None` if the path cannot be reduced to the canonical repository-relative form, or
    /// if `origin=DirectDirectory` is set for a non-directory entry.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        path: impl AsRef<str>,
        path_kind: RepositoryPathKind,
        origin: WorkingCopyEntryOrigin,
        kind: WorkingCopyEntryKind,
    ) -> Option<Self> {
        let path = normalize_repository_relative_path(path)?;

        if matches!(origin, WorkingCopyEntryOrigin::DirectDirectory)
            && !matches!(path_kind, RepositoryPathKind::Directory)
        {
            return None;
        }

        Some(Self {
            path,
            path_kind,
            origin,
            kind,
        })
    }
}

/// Normalizes a string path to the crate API's canonical repository-relative form.
///
/// Returns `None` if the path is empty, absolute, contains `..` or a NUL byte, or cannot be
/// reduced to the canonical form using `/` as the separator.
#[cfg_attr(
    all(debug_assertions, feature = "trace_logs"),
    tracing::instrument(skip_all)
)]
#[must_use]
pub fn normalize_repository_relative_path(path: impl AsRef<str>) -> Option<String> {
    let raw_path = path.as_ref().trim();
    if raw_path.is_empty() || raw_path.contains('\0') {
        return None;
    }
    if raw_path.starts_with('/') || raw_path.starts_with('\\') {
        return None;
    }
    if has_windows_drive_prefix(raw_path) {
        return None;
    }

    let segments = raw_path
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>();

    if segments.is_empty() || segments.contains(&"..") {
        return None;
    }

    Some(segments.join("/"))
}

const fn change_requires_rename(change: Option<TrackedChangeKind>) -> bool {
    matches!(change, Some(TrackedChangeKind::Renamed))
}

const fn change_requires_copy(change: Option<TrackedChangeKind>) -> bool {
    matches!(change, Some(TrackedChangeKind::Copied))
}

fn normalize_optional_repository_relative_path(path: Option<String>) -> Option<Option<String>> {
    match path {
        Some(path) => Some(Some(normalize_repository_relative_path(path)?)),
        None => Some(None),
    }
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return false;
    }

    bytes.len() == 2 || matches!(bytes.get(2), Some(b'/' | b'\\'))
}


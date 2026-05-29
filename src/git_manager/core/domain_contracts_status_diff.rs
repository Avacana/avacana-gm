//! Contracts of the diff-oriented `status_diff` domain.
//!
//! This API is intended for diff/payload/render/apply/pathspec scenarios and must not be used as
//! a production backend for UI/file-tree semantics. For a typed snapshot of the working tree and
//! ignored/untracked semantics, use `working_copy_status`.

use std::path::PathBuf;

/// Request for the `status/diff` domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusDiffRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// Semantic scope of the statuses.
    pub scope: StatusScope,
    /// Pathspec restriction.
    pub pathspecs: Vec<String>,
    /// Include the aggregated diff summary.
    pub include_patch: bool,
    /// Optional rendering format for diff segments.
    ///
    /// If `None`, the segments contain no serialized diff text.
    pub render_diff_format: Option<DiffOutputFormat>,
    /// Optional request to apply a patch before computing statuses.
    pub apply: Option<ApplyPatchRequest>,
}

/// Request to apply a patch within the `status/diff` domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPatchRequest {
    /// Patch text in unified-diff format.
    pub patch: Vec<u8>,
    /// Target location for applying the patch.
    pub location: ApplyPatchLocation,
    /// Only check applicability without modifying the repository.
    pub check: bool,
}

/// Location at which a patch is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyPatchLocation {
    /// Apply the patch to the working directory only.
    Workdir,
    /// Apply the patch to the index only.
    Index,
    /// Apply the patch to both the working directory and the index.
    Both,
}

/// Selection scope for statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusScope {
    /// Staged changes only.
    Staged,
    /// Unstaged changes only.
    Unstaged,
    /// Untracked files only.
    Untracked,
    /// The full status slice.
    #[default]
    All,
}

/// Typed result of the `status/diff` domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusDiffResult {
    /// The semantic scope actually applied to the selection.
    pub scope: StatusScope,
    /// Normalized diff status entries.
    pub entries: Vec<DiffStatusEntry>,
    /// Short diff summary, if the request required patch data.
    pub diff_summary: Option<DiffSummary>,
    /// Detailed diff diagnostics without leaking raw `git2` types.
    pub diff_details: Option<StatusDiffDetails>,
}

/// Status entry for a file/path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffStatusEntry {
    /// Repository path.
    pub path: String,
    /// Machine-readable diff status of the change.
    pub code: DiffStatusCode,
}

/// Machine-readable diff codes for a path change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiffStatusCode {
    /// File addition.
    Added,
    /// File modification.
    Modified,
    /// File deletion.
    Deleted,
    /// File rename.
    Renamed,
    /// File copy.
    Copied,
    /// Object type change (for example, file -> symlink).
    TypeChange,
    /// Untracked file.
    Untracked,
    /// Conflicted state.
    Conflicted,
}

/// Aggregated summary of diff changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    /// Number of changed files.
    pub changed_files: usize,
    /// Number of inserted lines.
    pub insertions: usize,
    /// Number of deleted lines.
    pub deletions: usize,
    /// Whether binary changes are present.
    pub has_binary_changes: bool,
}

/// Extended typed details of the `status/diff` operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusDiffDetails {
    /// Detailed diff segments for the slices actually computed.
    pub segments: Vec<DiffSegmentDetails>,
}

/// Diff details for a single computed slice (`staged` or `unstaged`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSegmentDetails {
    /// The scope for which the details were collected.
    pub scope: StatusScope,
    /// The slice's patch-id in hex form.
    pub patch_id: Option<String>,
    /// The serialized diff slice in the requested format.
    pub rendered_diff: Option<String>,
    /// Per-delta change details.
    pub deltas: Vec<DiffDeltaDetails>,
    /// Pathspec matching details, if a pathspec was supplied.
    pub pathspec: Option<PathspecMatchDetails>,
}

/// Rendering format for diff data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOutputFormat {
    /// Full patch (`git diff`).
    Patch,
    /// Patch headers only.
    PatchHeader,
    /// Raw format (`git diff --raw`).
    Raw,
    /// Names of changed files only (`git diff --name-only`).
    NameOnly,
    /// Name and status (`git diff --name-status`).
    NameStatus,
}

/// Typed representation of a single diff delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffDeltaDetails {
    /// Machine-readable change status.
    pub code: DiffStatusCode,
    /// Old side of the diff entry.
    pub old_file: DiffFileDetails,
    /// New side of the diff entry.
    pub new_file: DiffFileDetails,
    /// Binary characteristics of the delta, if present.
    pub binary: Option<DiffBinaryDetails>,
    /// Text patch for the delta, if available.
    pub patch: Option<DiffPatchDetails>,
}

/// Typed metadata for a file within a diff delta.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DiffFileDetails {
    /// File OID in hex form.
    pub id: String,
    /// Repository path of the file, if present.
    pub path: Option<String>,
    /// Object size in bytes.
    pub size: u64,
    /// Whether the object exists on the corresponding side of the delta.
    pub exists: bool,
    /// Whether the content is binary.
    pub is_binary: bool,
    /// Whether the content is text.
    pub is_not_binary: bool,
    /// Whether the OID is valid.
    pub is_valid_id: bool,
    /// File mode kind on this side of the diff entry.
    pub mode_kind: DiffFileModeKind,
    /// File mode.
    pub mode: String,
}

/// Typed `file mode` for diff entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffFileModeKind {
    /// Unreadable mode.
    Unreadable,
    /// Tree (`tree`).
    Tree,
    /// Regular file (`blob`).
    Blob,
    /// Group-writable blob.
    BlobGroupWritable,
    /// Executable blob.
    BlobExecutable,
    /// Symbolic link.
    Link,
    /// Gitlink/commit.
    Commit,
}

/// Typed details of a binary diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffBinaryDetails {
    /// Whether inline binary data is present.
    pub contains_data: bool,
    /// Old side of the binary diff.
    pub old_file: DiffBinaryFileDetails,
    /// New side of the binary diff.
    pub new_file: DiffBinaryFileDetails,
}

/// Typed details of one side of a binary diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffBinaryFileDetails {
    /// Kind of binary data.
    pub kind: DiffBinaryKindDetails,
    /// Size of the deflated data.
    pub compressed_size: usize,
    /// Size after inflation.
    pub inflated_size: usize,
}

/// Typed representation of the binary data kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffBinaryKindDetails {
    /// No binary payload.
    None,
    /// Full binary snapshot.
    Literal,
    /// Binary delta between the sides.
    Delta,
}

/// Typed representation of a patch for a single delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPatchDetails {
    /// Patch size in bytes.
    pub size_bytes: usize,
    /// Number of context lines.
    pub context_lines: usize,
    /// Number of added lines.
    pub additions: usize,
    /// Number of deleted lines.
    pub deletions: usize,
    /// List of hunks in the patch.
    pub hunks: Vec<DiffHunkDetails>,
}

/// Typed representation of a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunkDetails {
    /// Start line in the old version.
    pub old_start: u32,
    /// Number of lines in the old version.
    pub old_lines: u32,
    /// Start line in the new version.
    pub new_start: u32,
    /// Number of lines in the new version.
    pub new_lines: u32,
    /// Hunk header.
    pub header: String,
    /// Hunk lines.
    pub lines: Vec<DiffLineDetails>,
}

/// Typed representation of a diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLineDetails {
    /// Line number in the old version (if applicable).
    pub old_lineno: Option<u32>,
    /// Line number in the new version (if applicable).
    pub new_lineno: Option<u32>,
    /// Diff line classification.
    pub line_type: DiffLineTypeDetails,
    /// Character origin of the line.
    pub origin: char,
    /// Number of newline characters in the payload.
    pub num_lines: u32,
    /// Content offset in the source file.
    pub content_offset: i64,
    /// Line content.
    pub content: String,
}

/// Typed classification of a diff line type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineTypeDetails {
    /// Context line.
    Context,
    /// Added line.
    Addition,
    /// Deleted line.
    Deletion,
    /// Context line in an EOF block.
    ContextEofNl,
    /// Addition in an EOF block.
    AddEofNl,
    /// Deletion in an EOF block.
    DeleteEofNl,
    /// File header.
    FileHeader,
    /// Hunk header.
    HunkHeader,
    /// Binary diff line.
    Binary,
}

/// Typed diagnostics of pathspec matching in a diff context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathspecMatchDetails {
    /// The original pathspec patterns.
    pub patterns: Vec<String>,
    /// Successfully matched paths.
    pub entries: Vec<String>,
    /// Patterns for which no match was found.
    pub failed_entries: Vec<String>,
    /// Matched diff delta entries.
    pub diff_entries: Vec<PathspecDiffEntryDetails>,
}

/// Typed representation of a delta entry matched by a pathspec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathspecDiffEntryDetails {
    /// Repository path.
    pub path: String,
    /// Machine-readable change status.
    pub code: DiffStatusCode,
}

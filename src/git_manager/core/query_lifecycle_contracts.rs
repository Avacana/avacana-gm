//! Contracts of the `GitManager` query/lifecycle/verification domain.

use super::domain_contracts::DiffStatusCode;
use std::path::PathBuf;

/// Request for the query/lifecycle/verification domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryLifecycleRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// The query/lifecycle operation to perform.
    pub operation: QueryLifecycleOperation,
}

/// Typed query/lifecycle/verification operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryLifecycleOperation {
    /// `git annotate` with a line range and an optional mailmap.
    Annotate {
        /// Path to the file within the repository.
        path: String,
        /// Minimum line of the blame range (1-based).
        min_line: Option<usize>,
        /// Maximum line of the blame range (inclusive).
        max_line: Option<usize>,
        /// Use `.mailmap` for canonical identity.
        use_mailmap: bool,
    },
    /// `git blame` with a line range and an optional mailmap.
    Blame {
        /// Path to the file within the repository.
        path: String,
        /// Minimum line of the blame range (1-based).
        min_line: Option<usize>,
        /// Maximum line of the blame range (inclusive).
        max_line: Option<usize>,
        /// Use `.mailmap` for canonical identity.
        use_mailmap: bool,
    },
    /// `git config --get <key>`.
    ConfigGet {
        /// Configuration key name.
        key: String,
    },
    /// `git init` (lifecycle operation that creates a repository).
    Init {
        /// Create a bare repository.
        bare: bool,
        /// Optional initial branch name for the new repository's HEAD.
        initial_branch: Option<String>,
    },
    /// Explicit classification of an unsupported CLI command in the
    /// `query/lifecycle/verification` ownership group (T-086.13).
    UnsupportedCommand {
        /// Canonical name of the unsupported command.
        command: String,
    },
    /// `git log` via `revwalk`.
    Log {
        /// Optional range (`A..B`) or revision.
        revision_range: Option<String>,
        /// Maximum number of commit entries in the result.
        max_count: usize,
    },
    /// `git rev-parse` via `revspec`.
    Revparse {
        /// Revision/range specifier (`HEAD~2..HEAD`, `main^@`, ...).
        spec: String,
    },
    /// `git ls-tree`.
    LsTree {
        /// Optional treeish (`HEAD^{tree}`, `<commit>`, `<tag>`, ...).
        revision: Option<String>,
        /// Recursively traverse nested trees.
        recursive: bool,
    },
    /// `git ls-tree`/`tree.walk` with traversal order control.
    TreeWalk {
        /// Optional treeish (`HEAD^{tree}`, `<commit>`, `<tag>`, ...).
        revision: Option<String>,
        /// Use post-order traversal instead of pre-order.
        post_order: bool,
    },
    /// `git shortlog` via `revwalk` aggregation.
    Shortlog {
        /// Optional range (`A..B`) or revision.
        revision_range: Option<String>,
        /// Maximum number of commit entries to aggregate.
        max_count: usize,
    },
    /// `git show` (commit/object).
    Show {
        /// Optional revision/object specifier (`HEAD`, `<oid>`, `<tag>`, ...).
        revision: Option<String>,
    },
    /// Commit-message normalization and trailer parsing.
    MessageTrailers {
        /// Optional commit-ish (`HEAD`, `<oid>`, `<ref>`); defaults to `HEAD`.
        revision: Option<String>,
    },
    /// Merge-file preview via `merge_file_from_index`.
    MergeFilePreview {
        /// Path of the conflicting file in the tree.
        path: String,
        /// Left side of the merge (`ours`) in commit-ish form.
        ours: String,
        /// Right side of the merge (`theirs`) in commit-ish form.
        theirs: String,
    },
    /// Format a commit into an email patch via `EmailCreateOptions`.
    FormatEmail {
        /// Optional commit-ish (`HEAD`, `<oid>`, `<ref>`); defaults to `HEAD`.
        revision: Option<String>,
        /// Optional subject prefix for the email (`PATCH`, `RFC`, ...).
        subject_prefix: Option<String>,
    },
    /// `git version` via the `git2::Version` API.
    Version,
    /// `git whatchanged` via `revwalk` and tree-diff.
    Whatchanged {
        /// Optional range (`A..B`) or revision.
        revision_range: Option<String>,
        /// Maximum number of commit entries in the result.
        max_count: usize,
    },
}

/// Typed result of the query/lifecycle/verification domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryLifecycleResult {
    /// Whether the repository state changed (`init`).
    pub changed: bool,
    /// Path of the initialized repository (if the operation was `Init`).
    pub initialized_repository: Option<PathBuf>,
    /// Result of reading `git config`.
    pub config_value: Option<QueryConfigValue>,
    /// Set of blame/annotate hunks.
    pub blame_hunks: Vec<QueryBlameHunk>,
    /// Commit list (`log/show/whatchanged`).
    pub commits: Vec<QueryCommitSummary>,
    /// Result of `revparse`/`revspec`.
    pub revspec: Option<QueryRevspec>,
    /// List of tree entries (`ls-tree/show`).
    pub tree_entries: Vec<QueryTreeEntry>,
    /// Result of the merge-file preview.
    pub merge_file_preview: Option<QueryMergeFilePreview>,
    /// Author aggregation (`shortlog`).
    pub shortlog_entries: Vec<QueryShortlogEntry>,
    /// Result of commit-message normalization and trailer parsing.
    pub message_details: Option<QueryMessageDetails>,
    /// List of per-path changes (`whatchanged/show`).
    pub change_entries: Vec<QueryChangeEntry>,
    /// Formatted email patch (`format-email`).
    pub formatted_email: Option<String>,
    /// `git2/libgit2` version (`version`).
    pub version: Option<QueryVersionInfo>,
    /// Explicit unsupported classification of an ownership command.
    pub unsupported: Option<QueryUnsupportedClassification>,
    /// Additional textual summary of the operation.
    pub summary: Option<String>,
}

/// Explicit unsupported classification of a query/lifecycle ownership command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryUnsupportedClassification {
    /// Canonical name of the unsupported command.
    pub command: String,
    /// Machine-readable reason code for the unsupported status.
    pub reason_code: String,
    /// Human-readable impact of the unsupported status.
    pub impact: String,
}

/// Result of reading configuration via `git config --get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryConfigValue {
    /// Configuration key name.
    pub key: String,
    /// Value of the key (`None` if the key was not found).
    pub value: Option<String>,
}

/// Normalized blame/annotate hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBlameHunk {
    /// OID of the commit that determined the final version of the line.
    pub final_commit_oid: String,
    /// Start line in the final version of the file.
    pub final_start_line: usize,
    /// Number of lines in the hunk.
    pub lines_in_hunk: usize,
    /// Source path for the hunk (if available).
    pub source_path: Option<String>,
    /// Author name.
    pub author_name: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
}

/// Normalized commit entry of the query domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryCommitSummary {
    /// Commit OID.
    pub oid: String,
    /// Short commit summary.
    pub summary: String,
    /// Author name.
    pub author_name: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// UNIX timestamp of the commit time.
    pub timestamp_seconds: i64,
    /// Number of commit parents.
    pub parent_count: usize,
    /// OIDs of the commit parents in traversal order.
    pub parent_oids: Vec<String>,
}

/// Normalized tree entry (`ls-tree/show`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTreeEntry {
    /// Repository path of the entry.
    pub path: String,
    /// Object OID.
    pub object_id: String,
    /// Object type (`blob/tree/commit/tag`).
    pub kind: String,
    /// File mode in git format.
    pub file_mode: u32,
}

/// Normalized representation of a git object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryObjectInfo {
    /// Object OID.
    pub object_id: String,
    /// Object type (`blob/tree/commit/tag`).
    pub kind: String,
}

/// Result of `revparse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRevspec {
    /// The original spec passed to `revparse`.
    pub spec: String,
    /// Left endpoint of the range (`from`), if present.
    pub from: Option<QueryObjectInfo>,
    /// Right endpoint of the range (`to`), if present.
    pub to: Option<QueryObjectInfo>,
    /// `revparse` mode flags.
    pub mode: QueryRevspecMode,
}

/// `revparse` mode flags.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct QueryRevspecMode {
    /// The spec describes a single revision.
    pub single: bool,
    /// The spec describes a range.
    pub range: bool,
    /// The spec requests merge-base behavior.
    pub merge_base: bool,
}

/// Result of a merge file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMergeFilePreview {
    /// Whether the merge was clean (auto-merge with no conflict markers).
    pub automergeable: bool,
    /// Path of the resulting file (if available).
    pub path: Option<String>,
    /// Mode of the resulting file.
    pub file_mode: u32,
    /// Text of the merge result (lossy UTF-8).
    pub content: String,
}

/// Result of message prettify + trailers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMessageDetails {
    /// Normalized commit message (after `message_prettify`).
    pub prettified_message: String,
    /// Value of `DEFAULT_COMMENT_CHAR`.
    pub default_comment_char: Option<u8>,
    /// Trailers in string form.
    pub trailers_strs: Vec<QueryMessageTrailerStr>,
    /// Trailers in byte form.
    pub trailers_bytes: Vec<QueryMessageTrailerBytes>,
}

/// Trailer pair (`key`, `value`) in UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMessageTrailerStr {
    /// Trailer key.
    pub key: String,
    /// Trailer value.
    pub value: String,
}

/// Trailer pair (`key`, `value`) in raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryMessageTrailerBytes {
    /// Trailer key bytes.
    pub key: Vec<u8>,
    /// Trailer value bytes.
    pub value: Vec<u8>,
}

/// Normalized shortlog aggregation entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryShortlogEntry {
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Number of commits for the author.
    pub commit_count: usize,
}

/// Normalized path change entry (`whatchanged/show`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryChangeEntry {
    /// OID of the commit that is the source of the change.
    pub commit_oid: String,
    /// Path affected by the change.
    pub path: String,
    /// Machine-readable code for the path change.
    pub status: DiffStatusCode,
}

/// `git2/libgit2` version and compile-time capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct QueryVersionInfo {
    /// Version of the `git2` crate.
    pub git2_crate_version: String,
    /// `libgit2` version (`major`, `minor`, `rev`).
    pub libgit2_version: (u32, u32, u32),
    /// Whether the build uses a vendored `libgit2`.
    pub vendored: bool,
    /// Whether `libgit2` has thread-aware support.
    pub threads: bool,
    /// Whether `libgit2` has HTTPS support.
    pub https: bool,
    /// Whether `libgit2` has SSH support.
    pub ssh: bool,
    /// Whether sub-second mtimes are supported.
    pub nsec: bool,
}

//! Contracts of the read-only `tag_summaries` domain.

use std::path::PathBuf;

/// Request to read a typed summary of the tags that point to commits.
///
/// `repository_path` accepts an untrusted path to the repository root, its `git_dir`, or any
/// nested entity inside the worktree. The operation normalizes the path through the unified
/// repository foundation and returns only those tags whose final target, after peeling to a
/// non-tag object, is a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSummariesRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
}

impl TagSummariesRequest {
    /// Creates a typed request to read the tag summaries.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(repository_path: PathBuf) -> Self {
        Self { repository_path }
    }
}

/// Typed summary of an individual tag whose final target is a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSummary {
    /// Full canonical reference name, for example `refs/tags/v1.2.3`.
    pub reference_name: String,
    /// Short name of the tag without the `refs/tags/` prefix.
    pub short_name: String,
    /// OID of the commit the tag ultimately points to.
    pub target_commit_oid: String,
    /// Unix timestamp of the target commit's committer time, in seconds (`commit.time().seconds()`).
    ///
    /// This is specifically the commit time, not the author time or the tagger time. The timezone
    /// offset is deliberately not exposed in this field: the contract is intended for typed
    /// sorting and comparison by commit time.
    pub target_commit_timestamp: i64,
}

/// Typed result of the `tag_summaries` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSummariesResult {
    /// Summaries of commit-backed tags, sorted deterministically by `reference_name`.
    pub tags: Vec<TagSummary>,
}

impl TagSummariesResult {
    /// Creates a typed `tag_summaries` result.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(tag_count = tags.len()))
    )]
    #[must_use]
    pub fn new(tags: Vec<TagSummary>) -> Self {
        Self { tags }
    }
}

//! Remote-oriented contracts of the public `GitManager` API.

use std::path::PathBuf;

/// Parameters for the `clone` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneRequest {
    pub repository_url: String,
    pub destination_path: PathBuf,
    pub branch: Option<String>,
    pub depth: Option<usize>,
    pub tag_mode: FetchTagMode,
    pub mirror: bool,
}

/// Result of the `clone` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneResult {
    pub repository_path: PathBuf,
    pub checked_out_branch: Option<String>,
}

/// Parameters for the `fetch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchRequest {
    pub repository_path: PathBuf,
    pub remote_name: String,
    pub branch: Option<String>,
    pub depth: Option<usize>,
    pub tag_mode: FetchTagMode,
    pub refspecs: Vec<String>,
    pub prune: bool,
    pub mirror: bool,
}

/// Result of the `fetch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchResult {
    pub fetched: bool,
}

/// Parameters for the `pull` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub repository_path: PathBuf,
    pub remote_name: String,
    pub branch: Option<String>,
    pub tag_mode: FetchTagMode,
    pub refspecs: Vec<String>,
    pub prune: bool,
    pub file_favor: MergeFileFavor,
    pub mode: PullMode,
}

/// Execution mode for the `pull` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PullMode {
    #[default]
    FetchAndMerge,
}

/// Policy for automatically downloading tags during fetch/clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FetchTagMode {
    Unspecified,
    #[default]
    Auto,
    None,
    All,
}

/// Policy for resolving merge conflicts at the file level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeFileFavor {
    #[default]
    Normal,
    Ours,
    Theirs,
    Union,
}

/// Result of the `pull` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullResult {
    pub updated: bool,
    pub fast_forward: bool,
}

use super::errors::GitWarning;
use super::local_workflow::HooksPolicy;

/// Parameters for the `push` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRequest {
    pub repository_path: PathBuf,
    pub remote_name: String,
    pub branch: Option<String>,
    pub refspecs: Vec<String>,
    pub mirror: bool,
    pub prune: bool,
    pub force_with_lease: Option<ForceWithLeasePolicy>,
    pub hooks_policy: HooksPolicy,
}

/// Policy for a safe force-push with lease validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceWithLeasePolicy {
    pub expected_refs: Vec<ForceWithLeaseRef>,
}

/// Expectation for a single remote reference under `force-with-lease`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceWithLeaseRef {
    pub remote_ref: String,
    pub expected_oid: String,
}

/// Result of the `push` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushResult {
    pub updated_refs: Vec<String>,
    pub warnings: Vec<GitWarning>,
}

/// Parameters for the `ls-remote` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsRemoteRequest {
    pub repository_path: PathBuf,
    pub remote_name: String,
    pub include_heads: bool,
    pub include_tags: bool,
    pub include_symrefs: bool,
}

/// A normalized remote reference (`ls-remote`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteReference {
    pub name: String,
    pub oid: String,
    pub symbolic_target: Option<String>,
}

/// Result of the `ls-remote` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsRemoteResult {
    pub references: Vec<RemoteReference>,
}

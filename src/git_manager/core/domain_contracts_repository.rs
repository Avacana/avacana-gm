//! Contracts of the read-only `repository_descriptor` domain.

use std::path::PathBuf;

/// Request to build a typed descriptor for an open Git repository.
///
/// The operation accepts an untrusted path to the repository root, its `git_dir`, or any nested
/// entity inside the worktree and normalizes it through the unified repository foundation.
///
/// Performance budget for a warm repository: `p50 <= 10 ms`, `p95 <= 50 ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDescriptorRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
}

/// Canonical typed description of an open Git repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDescriptor {
    /// Absolute canonical repository root.
    pub repo_root: PathBuf,
    /// Absolute canonical worktree root, or `None` for a bare repo.
    pub worktree_root: Option<PathBuf>,
    /// Absolute canonical path to the git directory.
    pub git_dir: PathBuf,
    /// Whether this is a bare repository.
    pub is_bare: bool,
    /// Full name of the symbolic HEAD reference, if HEAD is attached to a branch.
    pub head_reference: Option<String>,
    /// OID of the current HEAD, if available.
    pub head_oid: Option<String>,
    /// Short name of the current local branch, if HEAD is attached.
    pub current_branch: Option<String>,
    /// Short name of the upstream branch (`origin/main`), if configured.
    pub upstream_branch: Option<String>,
    /// Number of commits ahead of upstream, if the metadata is available.
    pub ahead: Option<usize>,
    /// Number of commits behind upstream, if the metadata is available.
    pub behind: Option<usize>,
}

/// Result of the `repository_descriptor` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDescriptorResult {
    /// Typed descriptor of the discovered repository.
    pub repository: RepositoryDescriptor,
}

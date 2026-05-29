//! Unified repository foundation for discovery/open/canonicalize semantics.

use super::{GitError, GitErrorCode, GitResult, RepositoryDescriptor};
use git2::{BranchType, Repository};
use std::path::{Path, PathBuf};

/// Typed collaborator for the repository foundation at the edge of `GitManager`.
///
/// On the production path all `discover/open/canonicalize` operations must go through this module's
/// unified foundation helper rather than through local `Repository::open(...)` calls in the domains.
#[derive(Debug, Clone, Default)]
pub struct RepositoryAccess;

impl RepositoryAccess {
    /// Creates a typed collaborator for the repository foundation.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Canonical result of repository discovery/open for the production path.
pub(super) struct OpenedRepository {
    pub(crate) repository: Repository,
    pub(crate) repo_root: PathBuf,
    pub(crate) worktree_root: Option<PathBuf>,
    pub(crate) git_dir: PathBuf,
    pub(crate) is_bare: bool,
}

pub(super) fn open_repository(requested_path: &Path, operation: &str) -> GitResult<Repository> {
    Ok(open_repository_context(requested_path, operation)?.repository)
}

pub(super) fn open_repository_context(
    requested_path: &Path,
    operation: &str,
) -> GitResult<OpenedRepository> {
    ensure_non_empty_path(requested_path, operation)?;

    let repository = Repository::discover(requested_path)
        .or_else(|discover_error| {
            Repository::open(requested_path).map_err(|open_error| {
                GitError::new(
                    GitErrorCode::RepositoryOpenFailed,
                    format!(
                        "operation `{operation}` failed to discover/open repository `{}`: discover error: {discover_error}; open error: {open_error}",
                        requested_path.display()
                    ),
                )
            })
        })
        .map_err(|error| {
            if error.code() == GitErrorCode::RepositoryOpenFailed {
                return error;
            }

            GitError::new(
                GitErrorCode::RepositoryOpenFailed,
                format!(
                    "operation `{operation}` failed to discover repository `{}`: {error}",
                    requested_path.display()
                ),
            )
        })?;

    let git_dir = canonicalize_path(repository.path(), operation, "git_dir")?;
    let worktree_root = repository
        .workdir()
        .map(|path| canonicalize_path(path, operation, "worktree_root"))
        .transpose()?;
    let repo_root = worktree_root.clone().unwrap_or_else(|| git_dir.clone());
    let is_bare = repository.is_bare();

    Ok(OpenedRepository {
        repository,
        repo_root,
        worktree_root,
        git_dir,
        is_bare,
    })
}

#[must_use]
pub(super) fn describe_opened_repository(
    opened_repository: &OpenedRepository,
) -> RepositoryDescriptor {
    let current_branch = resolve_current_branch(opened_repository);
    let upstream_branch = resolve_upstream_branch(opened_repository, current_branch.as_deref());
    let (ahead, behind) = resolve_ahead_behind(
        opened_repository,
        current_branch.as_deref(),
        upstream_branch.as_deref(),
    );

    RepositoryDescriptor {
        repo_root: opened_repository.repo_root.clone(),
        worktree_root: opened_repository.worktree_root.clone(),
        git_dir: opened_repository.git_dir.clone(),
        is_bare: opened_repository.is_bare,
        head_reference: resolve_head_reference(opened_repository),
        head_oid: resolve_head_oid(opened_repository),
        current_branch,
        upstream_branch,
        ahead,
        behind,
    }
}

#[must_use]
fn resolve_symbolic_head_reference(repository: &Repository) -> Option<String> {
    let symbolic_head_reference = repository
        .head()
        .ok()
        .and_then(|head| {
            head.symbolic_target()
                .and_then(non_empty)
                .map(str::to_owned)
        })
        .or_else(|| {
            repository.find_reference("HEAD").ok().and_then(|head| {
                head.symbolic_target()
                    .and_then(non_empty)
                    .map(str::to_owned)
            })
        });

    #[cfg(all(debug_assertions, feature = "trace_logs"))]
    if let Some(head_reference) = symbolic_head_reference.as_deref() {
        tracing::trace!(
            head_reference,
            "repository_access_resolved_symbolic_head_reference"
        );
    }

    symbolic_head_reference
}

#[must_use]
fn resolve_current_branch_from_head_reference(head_reference: &str) -> Option<String> {
    head_reference
        .strip_prefix("refs/heads/")
        .and_then(non_empty)
        .map(str::to_owned)
}

fn ensure_non_empty_path(requested_path: &Path, operation: &str) -> GitResult<()> {
    if requested_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRepoPath,
            format!("operation `{operation}` requires a non-empty repository path"),
        ));
    }

    Ok(())
}

fn canonicalize_path(path: &Path, operation: &str, label: &str) -> GitResult<PathBuf> {
    std::fs::canonicalize(path).map_err(|error| {
        GitError::new(
            GitErrorCode::RepositoryOpenFailed,
            format!(
                "operation `{operation}` failed to canonicalize {label} `{}`: {error}",
                path.display()
            ),
        )
    })
}

#[must_use]
fn resolve_head_reference(opened_repository: &OpenedRepository) -> Option<String> {
    resolve_symbolic_head_reference(&opened_repository.repository).or_else(|| {
        opened_repository.repository.head().ok().and_then(|head| {
            head.name()
                .and_then(non_empty)
                .filter(|name| *name != "HEAD")
                .map(str::to_owned)
        })
    })
}

#[must_use]
fn resolve_head_oid(opened_repository: &OpenedRepository) -> Option<String> {
    opened_repository
        .repository
        .head()
        .ok()
        .and_then(|head| {
            head.target()
                .or_else(|| head.peel_to_commit().ok().map(|commit| commit.id()))
        })
        .map(|oid| oid.to_string())
}

#[must_use]
fn resolve_current_branch(opened_repository: &OpenedRepository) -> Option<String> {
    opened_repository
        .repository
        .head()
        .ok()
        .filter(git2::Reference::is_branch)
        .and_then(|head| head.shorthand().and_then(non_empty).map(str::to_owned))
        .or_else(|| {
            resolve_symbolic_head_reference(&opened_repository.repository)
                .as_deref()
                .and_then(resolve_current_branch_from_head_reference)
        })
}

#[must_use]
fn resolve_upstream_branch(
    opened_repository: &OpenedRepository,
    current_branch: Option<&str>,
) -> Option<String> {
    let current_branch = current_branch?;
    let local_branch = opened_repository
        .repository
        .find_branch(current_branch, BranchType::Local)
        .ok()?;
    let upstream_branch = local_branch.upstream().ok()?;

    upstream_branch
        .name()
        .ok()
        .flatten()
        .and_then(non_empty)
        .map(str::to_owned)
}

#[must_use]
fn resolve_ahead_behind(
    opened_repository: &OpenedRepository,
    current_branch: Option<&str>,
    upstream_branch: Option<&str>,
) -> (Option<usize>, Option<usize>) {
    let Some(current_branch) = current_branch else {
        return (None, None);
    };
    let Some(upstream_branch) = upstream_branch else {
        return (None, None);
    };

    let Ok(local_branch) = opened_repository
        .repository
        .find_branch(current_branch, BranchType::Local)
    else {
        return (None, None);
    };
    let Ok(upstream_branch) = opened_repository
        .repository
        .find_branch(upstream_branch, BranchType::Remote)
    else {
        return (None, None);
    };

    let Some(local_oid) = local_branch.get().target() else {
        return (None, None);
    };
    let Some(upstream_oid) = upstream_branch.get().target() else {
        return (None, None);
    };

    match opened_repository
        .repository
        .graph_ahead_behind(local_oid, upstream_oid)
    {
        Ok((ahead, behind)) => (Some(ahead), Some(behind)),
        Err(_) => (None, None),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

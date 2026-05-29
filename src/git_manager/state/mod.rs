//! `GitManager` state and locks.

mod locks;
pub(crate) mod worktree_policy;

pub use locks::{GitLockGuard, GitLockManager, GitLockMode};

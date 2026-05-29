//! Contracts of the `advanced` domain.
//!
//! `advanced` provides a diagnostic/utility surface for runner/parity/debug scenarios and is not
//! a stable, machine-readable production API for UI integrations.

use std::path::PathBuf;

/// Request for the advanced domain (`stash/submodule/worktree/attr/mailmap/describe`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// The advanced operation to perform.
    pub operation: AdvancedOperation,
}

/// Typed operations of the advanced domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvancedOperation {
    /// Save a stash.
    StashSave {
        /// Stash message.
        message: Option<String>,
        /// Include untracked files.
        include_untracked: bool,
        /// Keep the staged state of the index.
        keep_index: bool,
    },
    /// Retrieve the list of stash entries.
    StashList,
    /// Apply a stash.
    StashApply {
        /// Index of the stash entry.
        index: usize,
        /// Restore the staged state.
        reinstate_index: bool,
        /// Drop the entry after applying (`pop`).
        pop: bool,
    },
    /// Drop a stash entry.
    StashDrop {
        /// Index of the stash entry.
        index: usize,
    },
    /// Synchronize submodule configuration.
    SyncSubmodule {
        /// Submodule name, or `None` for all.
        name: Option<String>,
        /// Recursive mode.
        recursive: bool,
    },
    /// Update a submodule via typed `SubmoduleUpdateOptions`.
    SubmoduleUpdate {
        /// Submodule name, or `None` for all.
        name: Option<String>,
        /// Recursive mode for traversing nested submodules.
        recursive: bool,
        /// Initialize the submodule when no local checkout exists.
        init: bool,
        /// Allow a fetch when the target commit is not present locally.
        allow_fetch: bool,
    },
    /// Add a worktree.
    AddWorktree {
        /// Path of the new working directory.
        path: PathBuf,
        /// Target revision/branch.
        reference: Option<String>,
        /// Create a detached worktree.
        detach: bool,
    },
    /// Remove a worktree.
    RemoveWorktree {
        /// Path to an existing worktree.
        path: PathBuf,
        /// Forced removal mode.
        force: bool,
    },
    /// Manage the lock state of a worktree.
    WorktreeLock {
        /// Path to an existing worktree.
        path: PathBuf,
        /// Target lock action (`lock`/`unlock`/`query`).
        action: String,
        /// Lock reason (used only with `Lock`).
        reason: Option<String>,
    },
    /// Query the value of a git attribute.
    QueryAttribute {
        /// Path of the file to inspect.
        path: PathBuf,
        /// Attribute name.
        name: String,
    },
    /// Diagnostic status scan via `StatusShow/StatusIter`.
    ///
    /// Used as a supplementary/debug path; it does not replace the typed working-copy status/read
    /// models for production integrations.
    StatusScan {
        /// Selection mode for status entries (`index`/`workdir`/`all`).
        show: String,
        /// Optional pathspec to filter the list.
        pathspec: Option<String>,
        /// Include untracked paths.
        include_untracked: bool,
    },
    /// Configure the global libgit2 trace subscriber.
    TraceSet {
        /// Target trace message level (`none`/`fatal`/`error`/`warn`/`info`/`debug`/`trace`).
        level: String,
    },
    /// Check whether ignore rules apply to the given path.
    CheckIgnore {
        /// Path of the file to inspect.
        path: PathBuf,
    },
    /// Resolve an identity via `.mailmap`.
    ResolveMailmap {
        /// Optional author name.
        name: Option<String>,
        /// Optional author email.
        email: Option<String>,
    },
    /// Compute `describe` for a revision.
    DescribeRevision {
        /// Target revision (defaults to HEAD).
        revision: Option<String>,
    },
}

/// Diagnostic/utility-only result of the advanced domain.
///
/// The surface is intentionally stringly typed and intended for runner/parity/debug scenarios.
/// Production integrations must not rely on it as a stable, machine-readable contract; for that,
/// use the dedicated typed read models and specialized domains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedResult {
    /// Whether the repository state changed.
    pub changed: bool,
    /// Textual diagnostic summary.
    pub summary: Option<String>,
    /// Additional diagnostic result items (paths/refs/IDs).
    ///
    /// This field remains a utility-only surface and may change freely between advanced operation
    /// implementations. It must not be parsed as a production read model for UI/ADE integrations.
    pub items: Vec<String>,
}

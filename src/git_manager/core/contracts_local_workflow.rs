//! Local workflow contracts of the public `GitManager` API.

use super::errors::GitWarning;
use super::remote::MergeFileFavor;
use std::path::PathBuf;

/// Parameters for the `stage` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRequest {
    pub repository_path: PathBuf,
    mode: StageMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StageMode {
    All,
    Selective(Vec<String>),
}

impl StageRequest {
    #[must_use]
    pub const fn all(repository_path: PathBuf) -> Self {
        Self {
            repository_path,
            mode: StageMode::All,
        }
    }

    #[must_use]
    pub const fn selective(repository_path: PathBuf, pathspecs: Vec<String>) -> Self {
        Self {
            repository_path,
            mode: StageMode::Selective(pathspecs),
        }
    }

    #[must_use]
    pub(crate) const fn is_stage_all(&self) -> bool {
        matches!(self.mode, StageMode::All)
    }

    #[must_use]
    pub(crate) const fn selective_pathspecs(&self) -> Option<&[String]> {
        match &self.mode {
            StageMode::All => None,
            StageMode::Selective(pathspecs) => Some(pathspecs.as_slice()),
        }
    }

    #[must_use]
    pub(crate) const fn staged_pathspec_count(&self) -> usize {
        match &self.mode {
            StageMode::All => 1,
            StageMode::Selective(pathspecs) => pathspecs.len(),
        }
    }
}

/// Result of the `stage` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageResult {
    pub staged_pathspec_count: usize,
    pub index_entry_count: usize,
}

/// Policy for handling an empty commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyCommitPolicy {
    Allow,
    #[default]
    Reject,
}

/// Policy for handling Git hooks under `NO_SUBPROCESS` conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HooksPolicy {
    pub fail_if_hooks_present: bool,
}

/// Parameters for the `commit` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest {
    pub repository_path: PathBuf,
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub empty_commit_policy: EmptyCommitPolicy,
    pub hooks_policy: HooksPolicy,
}

/// Result of the `commit` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitResult {
    pub commit_oid: String,
    pub empty_commit: bool,
    pub warnings: Vec<GitWarning>,
}

/// Parameters for the `create_branch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBranchRequest {
    pub repository_path: PathBuf,
    pub branch_name: String,
    pub start_point: Option<String>,
}

/// Result of the `create_branch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBranchResult {
    pub branch_name: String,
}

/// Parameters for the `switch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchBranchRequest {
    pub repository_path: PathBuf,
    pub branch_name: String,
    pub force: bool,
    pub allow_dirty: bool,
}

/// Result of the `switch` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchBranchResult {
    pub current_branch: String,
    pub previous_branch: Option<String>,
}

/// Parameters for the `merge` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRequest {
    pub repository_path: PathBuf,
    pub source_ref: String,
    pub target_ref: Option<String>,
    pub file_favor: MergeFileFavor,
}

/// Result of the `merge` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    pub merged: bool,
    pub fast_forward: bool,
}

//! Contracts of the read-only `line_diff` domain.
//!
//! This API is intended for single-path diff facts in editor/gutter-like scenarios and other
//! consumers that need a machine-readable diff for a single path. The `status_diff` and
//! `working_copy_status` domains remain separate surfaces: the former handles general
//! render/apply/pathspec semantics, the latter a typed snapshot of the working tree.

#![allow(clippy::missing_const_for_fn)]

use super::{normalize_repository_relative_path, DiffPatchDetails, RepositoryDescriptor};
use std::path::{Path, PathBuf};

/// Request to read typed single-path line diff facts.
///
/// `repository_path` and `target_path` are treated as untrusted filesystem input. `target_path`
/// may be an absolute path inside the worktree or a repository-relative path. Normalization and
/// repository-membership checks are performed by the `line_diff` operation layer, not by consumer
/// code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiffRequest {
    /// Untrusted input path from which the repository is to be discovered.
    pub repository_path: PathBuf,
    /// Untrusted filesystem input for the target file.
    ///
    /// The value may be an absolute path or a repository-relative path. The operation layer must
    /// normalize it relative to the discovered `repo_root`; the consumer must not do this itself.
    pub target_path: PathBuf,
}

impl LineDiffRequest {
    /// Creates a typed request to read a single-path line diff.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                repository_path_empty = repository_path.as_os_str().is_empty(),
                target_path_kind = classify_filesystem_path_kind(&target_path)
            )
        )
    )]
    #[must_use]
    pub fn new(repository_path: PathBuf, target_path: PathBuf) -> Self {
        Self {
            repository_path,
            target_path,
        }
    }

    /// Returns a machine-readable view of the input `target_path` for trace/diagnostics.
    #[must_use]
    pub fn target_path_kind(&self) -> &'static str {
        classify_filesystem_path_kind(&self.target_path)
    }
}

/// Typed payload of the `line_diff` result.
///
/// Unlike `status_diff`, the consumer is not required to parse a string patch as the primary
/// machine-readable contract. Unlike `working_copy_status`, the payload remains diff-oriented and
/// is limited to exactly one target path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineDiffPayload {
    /// A text diff reusing the canonical patch/hunk/line primitives.
    TextDiff {
        /// Typed single-path diff patch without stringly-typed fallbacks.
        patch: DiffPatchDetails,
    },
    /// The target path holds a binary or non-text diff.
    BinaryContent,
    /// There are no changes for the target path between `HEAD` and the working tree.
    NoChanges,
    /// An external text diff driver is configured for the path, so the canonical backend builds no text hunks.
    ExternalDriverConfigured,
}

impl LineDiffPayload {
    /// Returns the machine-readable name of the payload variant.
    #[must_use]
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::TextDiff { .. } => "text_diff",
            Self::BinaryContent => "binary_content",
            Self::NoChanges => "no_changes",
            Self::ExternalDriverConfigured => "external_driver_configured",
        }
    }
}

/// Typed result of the read-only `line_diff` operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiffResult {
    /// Canonical typed description of the discovered repository.
    pub repository: RepositoryDescriptor,
    /// Normalized repository-relative path without a leading `/` and without `..`.
    pub target_path: String,
    /// Typed single-path line diff payload without an `ADE`-specific presentation shape.
    pub payload: LineDiffPayload,
}

impl LineDiffResult {
    /// Creates a typed `line_diff` result, normalizing the repository-relative target path.
    ///
    /// Returns `None` if `target_path` cannot be reduced to the crate API's canonical
    /// repository-relative form using `/` as the separator.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(payload_kind = payload.kind_name()))
    )]
    #[must_use]
    pub fn new(
        repository: RepositoryDescriptor,
        target_path: impl AsRef<str>,
        payload: LineDiffPayload,
    ) -> Option<Self> {
        let target_path = normalize_repository_relative_path(target_path)?;

        Some(Self {
            repository,
            target_path,
            payload,
        })
    }
}

fn classify_filesystem_path_kind(path: &Path) -> &'static str {
    if path.as_os_str().is_empty() {
        "empty"
    } else if path.is_absolute() {
        "absolute"
    } else {
        "relative"
    }
}

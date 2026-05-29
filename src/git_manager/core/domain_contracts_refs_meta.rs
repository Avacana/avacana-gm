//! Contracts of the `refs/meta` domain.

use std::path::PathBuf;

/// Request for the `refs/meta` domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsRequest {
    /// Path to the local repository.
    pub repository_path: PathBuf,
    /// The refs/meta operation to perform.
    pub operation: RefsOperation,
}

/// Typed operations over refs/tag/reflog/notes/transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefsOperation {
    /// List refs by an optional pattern.
    List { pattern: Option<String> },
    /// List branch references, filtering local/remote.
    ListBranches {
        /// Include local branches (`refs/heads/*`).
        include_local: bool,
        /// Include remote branches (`refs/remotes/*`).
        include_remote: bool,
    },
    /// List reference names by an optional glob pattern.
    ListReferenceNames {
        /// Optional glob to filter reference names.
        pattern: Option<String>,
    },
    /// List configuration entries by an optional glob pattern.
    ListConfigEntries {
        /// Optional glob to filter config keys.
        glob: Option<String>,
    },
    /// Create a branch.
    CreateBranch {
        /// Branch name.
        name: String,
        /// Branch start point.
        start_point: Option<String>,
    },
    /// Create a tag.
    CreateTag {
        /// Tag name.
        name: String,
        /// Target OID/ref.
        target: String,
        /// Annotated tag message.
        message: Option<String>,
        /// Allow force-overwriting an existing tag.
        force: bool,
    },
    /// Delete a reference.
    DeleteReference {
        /// Full reference name.
        name: String,
        /// Expected old target value.
        expected_target: Option<String>,
    },
    /// Update a reference.
    UpdateReference {
        /// Full reference name.
        name: String,
        /// New target value.
        new_target: String,
        /// Expected old target value.
        expected_old_target: Option<String>,
        /// Reflog message.
        reflog_message: Option<String>,
    },
    /// Read the reflog.
    ReadReflog {
        /// Reference name.
        reference: String,
        /// Entry limit.
        limit: usize,
        /// Return entries from newest to oldest.
        newest_first: bool,
    },
    /// Write a note.
    WriteNote {
        /// Target revision for the note.
        target: String,
        /// Notes namespace.
        namespace: Option<String>,
        /// Note text.
        message: String,
        /// Allow overwriting.
        force: bool,
    },
    /// Read notes for a target revision.
    ReadNote {
        /// Target revision to look up the note for.
        target: String,
        /// Notes namespace.
        namespace: Option<String>,
    },
    /// Batch reference update.
    Transaction {
        /// List of atomic updates.
        updates: Vec<RefUpdateSpec>,
        /// Reflog message for the transaction commit.
        reflog_message: Option<String>,
    },
}

/// Specification of a single reference update in transaction mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdateSpec {
    /// Full reference name.
    pub reference_name: String,
    /// New target value.
    pub new_target: String,
    /// Expected old target value.
    pub expected_old_target: Option<String>,
}

/// Result of the refs/meta domain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RefsResult {
    /// Whether the reference space actually changed.
    pub changed: bool,
    /// List of refs after the operation.
    pub references: Vec<ReferenceDescriptor>,
    /// List of branch references after `ListBranches`.
    pub branches: Vec<ReferenceDescriptor>,
    /// List of reference names after `ListReferenceNames`.
    pub reference_names: Vec<String>,
    /// List of config entries after `ListConfigEntries`.
    ///
    /// Element format: `(name, value, level)`.
    /// For boolean shorthand keys, `value = None`.
    pub config_entries: Vec<(String, Option<String>, String)>,
    /// Returned reflog entries.
    pub reflog_entries: Vec<ReflogEntry>,
    /// Note OID, if the operation worked with notes.
    pub note_oid: Option<String>,
}

/// Normalized representation of a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDescriptor {
    /// Full reference name.
    pub name: String,
    /// Reference type (`direct`/`symbolic`), if available.
    pub reference_kind: Option<ReferenceKind>,
    /// Target OID/ref.
    pub target: Option<String>,
    /// Symbolic target for symbolic refs.
    pub symbolic_target: Option<String>,
}

/// Typed reference kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceKind {
    /// A direct reference to an OID.
    Direct,
    /// A symbolic reference to another reference name.
    Symbolic,
}

/// Normalized reflog entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflogEntry {
    /// Previous OID value.
    pub old_oid: String,
    /// New OID value.
    pub new_oid: String,
    /// Reflog message.
    pub message: String,
}

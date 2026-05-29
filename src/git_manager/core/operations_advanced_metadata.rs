//! Metadata and utility operations of the `advanced` domain.
//!
//! `StatusScan`, `CheckIgnore`, `QueryAttribute`, `ResolveMailmap`, `DescribeRevision`, and related
//! stringly typed `AdvancedResult.items` responses remain a supplementary/debug path for runner/parity and
//! manual diagnostics. They are not a production API for `ADE` and must not be used as a
//! source of typed read models in place of the specialized status/descriptor-overview domains.
#![allow(clippy::redundant_pub_crate)]

#[path = "operations_advanced_metadata_attributes.rs"]
mod attributes;
#[path = "operations_advanced_metadata_identity.rs"]
mod identity;
#[path = "operations_advanced_metadata_status.rs"]
mod status_scan;
#[path = "operations_advanced_metadata_submodules.rs"]
mod submodules;
#[path = "operations_advanced_metadata_worktree.rs"]
mod worktree_lock;

pub(crate) use attributes::{execute_check_ignore_operation, execute_query_attribute_operation};
pub(crate) use identity::{execute_describe_revision_operation, execute_resolve_mailmap_operation};
pub(crate) use status_scan::execute_status_scan_operation;
pub(crate) use submodules::execute_submodule_update_operation;
pub(crate) use worktree_lock::execute_worktree_lock_operation;

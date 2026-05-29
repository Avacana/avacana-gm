//! Helper utilities of the `refs/meta` domain for `GitManager`.

#![allow(clippy::redundant_pub_crate)]

pub(super) use crate::git_manager::core::repository_access::open_repository;

#[path = "operations_refs_meta_support_listing.rs"]
mod listing;
#[path = "operations_refs_meta_support_mutation.rs"]
mod mutation;
#[path = "operations_refs_meta_support_validation.rs"]
mod validation;

pub(super) use listing::{
    collect_branch_descriptors, collect_config_entries, collect_reference_descriptors,
    collect_reference_names,
};
pub(super) use mutation::{
    apply_reference_update, create_reference_with_target, find_reference, find_reference_optional,
    resolve_notes_reference_name, resolve_reference_target, resolve_target_oid,
    target_matches_reference, ResolvedReferenceTarget,
};
pub(super) use validation::{
    ensure_expected_target_matches, map_refs_error, normalize_non_empty, normalize_notes_ref,
    normalize_optional_message, normalize_reference_name, normalize_tag_name, reference_descriptor,
    sort_reference_descriptors,
};

use crate::git_manager::core::{GitResult, ReferenceDescriptor};
use git2::Repository;

pub(super) fn find_reference_descriptor(
    repository: &Repository,
    reference_name: &str,
    operation: &str,
) -> GitResult<ReferenceDescriptor> {
    let reference = find_reference(repository, reference_name, operation)?;
    Ok(reference_descriptor(&reference))
}

//! mirror-refspec helper functions for `push`.

use crate::git_manager::core::operations_clone_fetch_push::{
    list_remote_reference_snapshots, map_transport_error_to_git_error,
};
use crate::git_manager::core::{
    ForceWithLeasePolicy, ForceWithLeaseRef, GitError, GitErrorCode, GitResult,
};
use crate::git_manager::transport::{Git2TransportBridge, TransportRequest};
use git2::{Oid, Repository};
use std::collections::{BTreeSet, HashSet};

pub(super) fn build_mirror_push_refspecs(
    repository: &Repository,
    remote: &mut git2::Remote<'_>,
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
) -> GitResult<Vec<String>> {
    let local_reference_names = collect_local_mirror_reference_names(repository)?;
    let remote_reference_snapshots =
        list_remote_reference_snapshots(transport_bridge, transport_request, remote)
            .map_err(|error| map_transport_error_to_git_error(&error))?;

    let mut refspecs: Vec<String> = local_reference_names
        .iter()
        .map(|reference_name| format!("+{reference_name}:{reference_name}"))
        .collect();

    for snapshot in remote_reference_snapshots {
        if !is_mirror_managed_reference(snapshot.name.as_str()) {
            continue;
        }

        if !local_reference_names.contains(snapshot.name.as_str()) {
            refspecs.push(format!(":{}", snapshot.name));
        }
    }

    deduplicate_refspecs_in_order(&mut refspecs);
    Ok(refspecs)
}

pub(super) fn apply_force_with_lease_to_push_refspecs(
    push_refspecs: &mut [String],
    force_with_lease_policy: &ForceWithLeasePolicy,
) -> GitResult<()> {
    let expected_remote_refs = force_with_lease_policy
        .expected_refs
        .iter()
        .map(normalize_force_with_lease_ref)
        .collect::<GitResult<HashSet<_>>>()?
        .into_iter()
        .map(|(remote_ref, _expected_oid)| remote_ref)
        .collect::<HashSet<_>>();

    let mut matched_remote_refs = HashSet::new();
    for push_refspec in push_refspecs.iter_mut() {
        let Some(destination_ref) = parse_push_refspec_destination(push_refspec.as_str()) else {
            continue;
        };

        if !expected_remote_refs.contains(destination_ref) {
            continue;
        }

        matched_remote_refs.insert(destination_ref.to_owned());
        if !push_refspec.starts_with('+') {
            *push_refspec = format!("+{push_refspec}");
        }
    }

    let mut unmatched_remote_refs: Vec<String> = expected_remote_refs
        .into_iter()
        .filter(|remote_ref| !matched_remote_refs.contains(remote_ref.as_str()))
        .collect();
    unmatched_remote_refs.sort_unstable();

    if !unmatched_remote_refs.is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            format!(
                "push force-with-lease policy refs are not targeted by push refspecs: {}",
                unmatched_remote_refs.join(", ")
            ),
        ));
    }

    Ok(())
}

pub(super) fn normalize_force_with_lease_ref(
    expected_ref: &ForceWithLeaseRef,
) -> GitResult<(String, String)> {
    let remote_ref = expected_ref.remote_ref.trim();
    if remote_ref.is_empty() || !remote_ref.starts_with("refs/") {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            format!(
                "push force-with-lease remote ref `{}` must be a full ref name",
                expected_ref.remote_ref
            ),
        ));
    }

    let expected_oid = expected_ref.expected_oid.trim();
    if expected_oid.is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            format!("push force-with-lease expected oid for `{remote_ref}` must not be empty"),
        ));
    }

    let expected_oid = Oid::from_str(expected_oid).map_err(|error| {
        GitError::new(
            GitErrorCode::InvalidRefspec,
            format!("push force-with-lease expected oid `{expected_oid}` is invalid: {error}"),
        )
    })?;

    Ok((remote_ref.to_owned(), expected_oid.to_string()))
}

fn collect_local_mirror_reference_names(repository: &Repository) -> GitResult<BTreeSet<String>> {
    let references = repository.references().map_err(|error| {
        GitError::new(
            GitErrorCode::RefNotFound,
            format!("push mirror failed to enumerate local refs: {error}"),
        )
    })?;

    let mut local_reference_names = BTreeSet::new();
    for reference_entry in references {
        let reference = reference_entry.map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("push mirror failed to read local ref entry: {error}"),
            )
        })?;

        let Some(reference_name) = reference.name().and_then(non_empty) else {
            continue;
        };

        if !is_mirror_managed_reference(reference_name) || reference.target().is_none() {
            continue;
        }

        local_reference_names.insert(reference_name.to_owned());
    }

    Ok(local_reference_names)
}

fn is_mirror_managed_reference(reference_name: &str) -> bool {
    reference_name.starts_with("refs/")
}

fn deduplicate_refspecs_in_order(refspecs: &mut Vec<String>) {
    let mut seen_refspecs = HashSet::new();
    refspecs.retain(|refspec| seen_refspecs.insert(refspec.clone()));
}

fn parse_push_refspec_destination(push_refspec: &str) -> Option<&str> {
    let push_refspec = push_refspec.strip_prefix('+').unwrap_or(push_refspec);
    if push_refspec.starts_with(':') {
        return None;
    }

    if let Some((_source_ref, destination_ref)) = push_refspec.split_once(':') {
        return non_empty(destination_ref);
    }

    let destination_ref = non_empty(push_refspec)?;
    destination_ref
        .starts_with("refs/")
        .then_some(destination_ref)
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}


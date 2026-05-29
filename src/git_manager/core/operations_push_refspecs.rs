use crate::git_manager::core::operations_clone_fetch_push::{
    list_remote_reference_snapshots, map_transport_error_to_git_error, normalize_refspecs,
};
use crate::git_manager::core::operations_push_mirror_support::{
    build_mirror_push_refspecs, normalize_force_with_lease_ref,
};
use crate::git_manager::core::{
    ForceWithLeasePolicy, GitError, GitErrorCode, GitResult, PushRequest,
};
use crate::git_manager::transport::{Git2TransportBridge, TransportRequest};
use git2::{BranchType, Repository};
use std::collections::{BTreeSet, HashMap, HashSet};

pub(super) fn build_push_refspecs(
    repository: &Repository,
    remote: &mut git2::Remote<'_>,
    request: &PushRequest,
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
) -> GitResult<Vec<String>> {
    if request.mirror {
        return build_mirror_push_refspecs(repository, remote, transport_bridge, transport_request);
    }

    let mut refspecs = if request.refspecs.is_empty() {
        let branch_name =
            super::validation::resolve_push_branch_name(repository, request.branch.as_deref())?;
        vec![build_default_push_refspec(branch_name.as_str())]
    } else {
        normalize_refspecs(request.refspecs.as_slice(), "push")?
    };

    if request.prune {
        let prune_refspecs =
            build_prune_delete_refspecs(repository, remote, transport_bridge, transport_request)?;
        refspecs.extend(prune_refspecs);
    }

    deduplicate_refspecs_in_order(&mut refspecs);
    Ok(refspecs)
}

pub(super) fn enforce_force_with_lease_preflight(
    remote: &mut git2::Remote<'_>,
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
    force_with_lease_policy: &ForceWithLeasePolicy,
) -> GitResult<()> {
    let remote_reference_snapshots =
        list_remote_reference_snapshots(transport_bridge, transport_request, remote)
            .map_err(|error| map_transport_error_to_git_error(&error))?;

    let remote_oid_by_ref: HashMap<String, String> = remote_reference_snapshots
        .into_iter()
        .map(|snapshot| (snapshot.name, snapshot.oid))
        .collect();

    for expected_ref in &force_with_lease_policy.expected_refs {
        let (remote_ref, expected_oid) = normalize_force_with_lease_ref(expected_ref)?;

        let Some(actual_oid) = remote_oid_by_ref.get(remote_ref.as_str()) else {
            return Err(GitError::new(
                GitErrorCode::PushForceWithLeaseRejected,
                format!("push force-with-lease rejected: remote ref `{remote_ref}` is missing"),
            ));
        };

        if actual_oid != &expected_oid {
            return Err(GitError::new(
                GitErrorCode::PushForceWithLeaseRejected,
                format!(
                    "push force-with-lease rejected for `{remote_ref}`: expected `{expected_oid}`, got `{actual_oid}`"
                ),
            ));
        }
    }

    Ok(())
}

fn build_default_push_refspec(branch_name: &str) -> String {
    format!("refs/heads/{branch_name}:refs/heads/{branch_name}")
}

fn build_prune_delete_refspecs(
    repository: &Repository,
    remote: &mut git2::Remote<'_>,
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
) -> GitResult<Vec<String>> {
    let remote_reference_snapshots =
        list_remote_reference_snapshots(transport_bridge, transport_request, remote)
            .map_err(|error| map_transport_error_to_git_error(&error))?;

    let local_branches = collect_local_branch_names(repository)?;
    let mut delete_refspecs = Vec::new();

    for snapshot in remote_reference_snapshots {
        let Some(remote_branch_name) = snapshot.name.strip_prefix("refs/heads/") else {
            continue;
        };

        if !local_branches.contains(remote_branch_name) {
            delete_refspecs.push(format!(":refs/heads/{remote_branch_name}"));
        }
    }

    Ok(delete_refspecs)
}

fn collect_local_branch_names(repository: &Repository) -> GitResult<BTreeSet<String>> {
    let local_branches = repository
        .branches(Some(BranchType::Local))
        .map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("push failed to enumerate local branches: {error}"),
            )
        })?;

    let mut branch_names = BTreeSet::new();
    for branch_entry in local_branches {
        let (branch, _branch_type) = branch_entry.map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("push failed to read local branch entry: {error}"),
            )
        })?;

        let branch_name = branch.name().map_err(|error| {
            GitError::new(
                GitErrorCode::RefNotFound,
                format!("push failed to resolve local branch name: {error}"),
            )
        })?;

        if let Some(name) = branch_name.and_then(super::non_empty) {
            branch_names.insert(name.to_owned());
        }
    }

    Ok(branch_names)
}

fn deduplicate_refspecs_in_order(refspecs: &mut Vec<String>) {
    let mut seen_refspecs = HashSet::new();
    refspecs.retain(|refspec| seen_refspecs.insert(refspec.clone()));
}

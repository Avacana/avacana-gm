use crate::git_manager::core::operations_clone_fetch_push::normalize_refspecs;
use crate::git_manager::core::operations_push_mirror_support::normalize_force_with_lease_ref;
use crate::git_manager::core::{
    ForceWithLeasePolicy, GitError, GitErrorCode, GitResult, PushRequest,
};
use git2::Repository;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::ErrorKind;

pub(super) fn validate_push_request(request: &PushRequest) -> GitResult<()> {
    if request.remote_name.trim().is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "push operation requires a non-empty remote name",
        ));
    }

    if request.mirror
        && (request
            .branch
            .as_deref()
            .and_then(super::non_empty)
            .is_some()
            || !request.refspecs.is_empty())
    {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            "push mirror mode cannot be combined with explicit branch/refspec",
        ));
    }

    if let Some(force_with_lease_policy) = request.force_with_lease.as_ref() {
        validate_force_with_lease_policy(force_with_lease_policy)?;
    }

    if !request.refspecs.is_empty() {
        normalize_refspecs(request.refspecs.as_slice(), "push")?;
    }

    Ok(())
}

pub(super) fn enforce_push_hooks_policy(
    repository: &Repository,
    request: &PushRequest,
) -> GitResult<()> {
    let hooks_present = has_non_sample_hooks(repository)?;
    if request.hooks_policy.fail_if_hooks_present && hooks_present {
        return Err(GitError::new(
            GitErrorCode::HooksPresent,
            "push aborted because hooks are present and fail_if_hooks_present policy is enabled",
        ));
    }
    Ok(())
}

pub(super) fn resolve_push_branch_name(
    repository: &Repository,
    branch: Option<&str>,
) -> GitResult<String> {
    if let Some(branch_name) = branch.and_then(super::non_empty) {
        return normalize_push_branch_input(branch_name);
    }

    let head = repository.head().map_err(|error| {
        GitError::new(
            GitErrorCode::DetachedHead,
            format!("push requires an attached branch HEAD when branch is omitted: {error}"),
        )
    })?;

    if !head.is_branch() {
        return Err(GitError::new(
            GitErrorCode::DetachedHead,
            "push requires an attached branch HEAD when branch is omitted",
        ));
    }

    let branch_name = head.shorthand().and_then(super::non_empty).ok_or_else(|| {
        GitError::new(
            GitErrorCode::DetachedHead,
            "push failed to resolve shorthand branch name for current HEAD",
        )
    })?;

    Ok(branch_name.to_owned())
}

fn validate_force_with_lease_policy(policy: &ForceWithLeasePolicy) -> GitResult<()> {
    if policy.expected_refs.is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            "push force-with-lease policy requires at least one expected ref",
        ));
    }

    let mut seen_remote_refs = HashSet::new();
    for expected_ref in &policy.expected_refs {
        let (remote_ref, _expected_oid) = normalize_force_with_lease_ref(expected_ref)?;
        if !seen_remote_refs.insert(remote_ref) {
            return Err(GitError::new(
                GitErrorCode::InvalidRefspec,
                "push force-with-lease policy contains duplicate remote refs",
            ));
        }
    }

    Ok(())
}

fn normalize_push_branch_input(branch: &str) -> GitResult<String> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            "push branch must not be empty",
        ));
    }

    if let Some(branch_name) = branch
        .strip_prefix("refs/heads/")
        .and_then(super::non_empty)
    {
        return Ok(branch_name.to_owned());
    }

    if branch.starts_with("refs/") {
        return Err(GitError::new(
            GitErrorCode::RefNotFound,
            format!("push branch `{branch}` must be a local branch name or `refs/heads/<name>`"),
        ));
    }

    Ok(branch.to_owned())
}

fn has_non_sample_hooks(repository: &Repository) -> GitResult<bool> {
    let hooks_dir = repository.path().join("hooks");
    let read_dir = match std::fs::read_dir(&hooks_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(GitError::new(
                GitErrorCode::HookDiscoveryFailed,
                format!(
                    "failed to read hooks directory `{}`: {error}",
                    hooks_dir.display()
                ),
            ))
        }
    };

    for entry in read_dir {
        let path = entry
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::HookDiscoveryFailed,
                    format!(
                        "failed to inspect entry in hooks directory `{}`: {error}",
                        hooks_dir.display()
                    ),
                )
            })?
            .path();

        if !path.is_file() {
            continue;
        }

        let is_sample = path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sample"));
        if is_sample {
            continue;
        }

        return Ok(true);
    }

    Ok(false)
}

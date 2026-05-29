use super::clone_local_shallow::{
    apply_local_shallow_metadata, should_retry_local_shallow_transport,
};
use super::shared::{
    map_fetch_tag_mode, map_transport_error_to_git_error_impl, normalize_depth,
    take_remote_callbacks,
};
use crate::git_manager::core::operations_remote_transport_support::{
    apply_fetch_network_options, build_auth_context_from_remote_url, is_local_remote_url,
};
use crate::git_manager::core::{CloneRequest, CloneResult, GitError, GitErrorCode, GitResult};
use crate::git_manager::transport::{
    Git2TransportBridge, TransportError, TransportOutcome, TransportRequest,
};
use git2::{build::RepoBuilder, FetchOptions, Repository};

pub(super) fn execute_clone_operation_impl(
    request: &CloneRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<CloneResult> {
    validate_clone_request(request)?;
    let depth = normalize_depth(request.depth)?;
    let auth_context = build_auth_context_from_remote_url("clone", &request.repository_url)?;
    let transport_request = TransportRequest::for_clone(auth_context);

    match execute_clone_via_transport(transport_bridge, &transport_request, request, depth) {
        Ok(clone_result) => Ok(clone_result),
        Err(error)
            if should_retry_local_shallow_transport(
                depth,
                is_local_remote_url(&request.repository_url),
                &error,
            ) =>
        {
            tracing::trace!(
                repository_url = request.repository_url,
                depth = ?depth,
                "local transport does not support shallow clone, falling back to local metadata"
            );

            let fallback_result =
                execute_clone_via_transport(transport_bridge, &transport_request, request, None)
                    .map_err(|retry_error| map_transport_error_to_git_error_impl(&retry_error))?;
            apply_local_shallow_metadata(&fallback_result.repository_path, depth.unwrap_or(1))?;
            Ok(fallback_result)
        }
        Err(error) => Err(map_transport_error_to_git_error_impl(&error)),
    }
}

fn execute_clone_via_transport(
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
    request: &CloneRequest,
    depth: Option<i32>,
) -> Result<CloneResult, TransportError> {
    transport_bridge
        .execute(transport_request, |callbacks| {
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(take_remote_callbacks(callbacks));
            apply_fetch_network_options(&mut fetch_options);
            fetch_options.download_tags(map_fetch_tag_mode(request.tag_mode));
            if let Some(depth) = depth {
                fetch_options.depth(depth);
            }

            let mut repo_builder = RepoBuilder::new();
            repo_builder.fetch_options(fetch_options);
            if request.mirror {
                repo_builder.bare(true);
            }
            if let Some(branch) = request.branch.as_deref() {
                repo_builder.branch(branch);
            }

            let repository =
                repo_builder.clone(&request.repository_url, &request.destination_path)?;
            if request.mirror {
                configure_clone_mirror_remote(&repository)?;
            }

            Ok(CloneResult {
                repository_path: request.destination_path.clone(),
                checked_out_branch: if request.mirror {
                    None
                } else {
                    resolve_checked_out_branch(&repository).or_else(|| request.branch.clone())
                },
            })
        })
        .map(TransportOutcome::into_value)
}

fn configure_clone_mirror_remote(repository: &Repository) -> Result<(), git2::Error> {
    repository.remote_add_fetch("origin", "+refs/*:refs/*")?;

    let mut config = repository.config()?;
    config.set_bool("remote.origin.mirror", true)?;
    Ok(())
}

fn validate_clone_request(request: &CloneRequest) -> GitResult<()> {
    if request.destination_path.as_os_str().is_empty() {
        return Err(GitError::new(
            GitErrorCode::InvalidRepoPath,
            "clone operation requires a non-empty destination path",
        ));
    }

    if request.repository_url.trim().is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "clone operation requires a non-empty repository URL",
        ));
    }

    if request.mirror
        && request
            .branch
            .as_deref()
            .is_some_and(|branch| !branch.trim().is_empty())
    {
        return Err(GitError::new(
            GitErrorCode::InvalidRefspec,
            "clone mirror mode cannot be combined with an explicit branch",
        ));
    }

    Ok(())
}

fn resolve_checked_out_branch(repository: &Repository) -> Option<String> {
    let head = repository.head().ok()?;
    if !head.is_branch() {
        return None;
    }
    head.shorthand().map(str::to_owned)
}

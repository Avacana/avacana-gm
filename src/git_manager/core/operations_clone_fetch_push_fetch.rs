use super::clone_local_shallow::{
    apply_local_shallow_metadata, should_retry_local_shallow_transport,
};
use super::shared::{
    build_fetch_refspecs, map_fetch_tag_mode, map_transport_error_to_git_error_impl,
    normalize_depth, take_remote_callbacks, transport_request_for_operation,
};
use super::FetchOperationRequest;
use crate::git_manager::core::operations_remote_transport_support::{
    apply_fetch_network_options, build_auth_context_from_remote_url, is_local_remote_url,
};
use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{FetchRequest, FetchResult, GitError, GitErrorCode, GitResult};
use crate::git_manager::transport::Git2TransportBridge;
use git2::{FetchOptions, FetchPrune};

pub(super) fn execute_public_fetch_operation_impl(
    request: &FetchRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<FetchResult> {
    if request.remote_name.trim().is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "fetch operation requires a non-empty remote name",
        ));
    }

    let fetch_operation_request = FetchOperationRequest {
        repository_path: &request.repository_path,
        remote_name: request.remote_name.as_str(),
        branch: request.branch.as_deref(),
        depth: request.depth,
        tag_mode: request.tag_mode,
        refspecs: request.refspecs.as_slice(),
        prune: request.prune,
        mirror: request.mirror,
        operation_name: "fetch",
    };
    execute_fetch_operation_impl(&fetch_operation_request, transport_bridge)?;

    Ok(FetchResult { fetched: true })
}

pub(super) fn execute_fetch_operation_impl(
    request: &FetchOperationRequest<'_>,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<()> {
    let depth = normalize_depth(request.depth)?;
    let repository = open_repository(request.repository_path, request.operation_name)?;
    let mut remote = repository
        .find_remote(request.remote_name)
        .map_err(|error| {
            GitError::new(
                GitErrorCode::TransportFailure,
                format!(
                    "operation `{}` failed to resolve remote `{}`: {error}",
                    request.operation_name, request.remote_name
                ),
            )
        })?;

    let remote_url = remote.url().map(str::to_owned).ok_or_else(|| {
        GitError::new(
            GitErrorCode::TransportFailure,
            format!(
                "operation `{}` failed to resolve URL for remote `{}`",
                request.operation_name, request.remote_name
            ),
        )
    })?;
    let configured_remote_refspecs: Vec<String> = remote
        .refspecs()
        .filter_map(|refspec| refspec.str().map(str::to_owned))
        .collect();
    tracing::trace!(
        operation = request.operation_name,
        remote = request.remote_name,
        configured_remote_refspecs = ?configured_remote_refspecs,
        "resolved remote refspecs from git2 remote configuration"
    );
    let auth_context = build_auth_context_from_remote_url(request.operation_name, &remote_url)?;
    let transport_request = transport_request_for_operation(request.operation_name, auth_context);

    let refspecs = build_fetch_refspecs(
        request.remote_name,
        request.branch,
        request.refspecs,
        request.mirror,
    )?;
    let refspec_refs: Vec<&str> = refspecs.iter().map(String::as_str).collect();

    let mut execute_fetch_with_depth = |fetch_depth: Option<i32>| {
        transport_bridge.execute(&transport_request, |callbacks| {
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(take_remote_callbacks(callbacks));
            apply_fetch_network_options(&mut fetch_options);
            fetch_options.download_tags(map_fetch_tag_mode(request.tag_mode));
            fetch_options.prune(if request.prune {
                FetchPrune::On
            } else {
                FetchPrune::Off
            });
            if let Some(fetch_depth) = fetch_depth {
                fetch_options.depth(fetch_depth);
            }
            remote.fetch(&refspec_refs, Some(&mut fetch_options), None)?;
            Ok(())
        })
    };

    match execute_fetch_with_depth(depth) {
        Ok(_) => Ok(()),
        Err(error)
            if should_retry_local_shallow_transport(
                depth,
                is_local_remote_url(&remote_url),
                &error,
            ) =>
        {
            tracing::trace!(
                repository_url = remote_url.as_str(),
                depth = ?depth,
                "local transport does not support shallow fetch, falling back to local metadata"
            );

            execute_fetch_with_depth(None)
                .map_err(|retry_error| map_transport_error_to_git_error_impl(&retry_error))?;
            apply_local_shallow_metadata(request.repository_path, depth.unwrap_or(1))
        }
        Err(error) => Err(map_transport_error_to_git_error_impl(&error)),
    }
}

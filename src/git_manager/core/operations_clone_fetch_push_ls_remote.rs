use super::shared::{
    list_remote_reference_snapshots_impl, map_transport_error_to_git_error_impl,
    transport_request_for_operation,
};
use crate::git_manager::core::operations_remote_transport_support::build_auth_context_from_remote_url;
use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{
    GitError, GitErrorCode, GitResult, LsRemoteRequest, LsRemoteResult, RemoteReference,
};
use crate::git_manager::transport::Git2TransportBridge;

pub(super) fn execute_ls_remote_operation_impl(
    request: &LsRemoteRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<LsRemoteResult> {
    if request.remote_name.trim().is_empty() {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "ls-remote operation requires a non-empty remote name",
        ));
    }

    let repository = open_repository(&request.repository_path, "ls-remote")?;
    let mut remote = repository
        .find_remote(request.remote_name.as_str())
        .map_err(|error| {
            GitError::new(
                GitErrorCode::LsRemoteFailed,
                format!(
                    "ls-remote failed to resolve remote `{}`: {error}",
                    request.remote_name
                ),
            )
        })?;

    let remote_url = remote.url().map(str::to_owned).ok_or_else(|| {
        GitError::new(
            GitErrorCode::LsRemoteFailed,
            format!(
                "ls-remote failed to resolve URL for remote `{}`",
                request.remote_name
            ),
        )
    })?;

    let auth_context = build_auth_context_from_remote_url("ls-remote", &remote_url)?;
    let transport_request = transport_request_for_operation("ls-remote", auth_context);

    let snapshots =
        list_remote_reference_snapshots_impl(transport_bridge, &transport_request, &mut remote)
            .map_err(|error| {
                GitError::new(
                    GitErrorCode::LsRemoteFailed,
                    map_transport_error_to_git_error_impl(&error).to_string(),
                )
            })?;

    let references = snapshots
        .into_iter()
        .filter(|snapshot| {
            matches_reference_filter(
                snapshot.name.as_str(),
                request.include_heads,
                request.include_tags,
            )
        })
        .map(|snapshot| RemoteReference {
            name: snapshot.name,
            oid: snapshot.oid,
            symbolic_target: if request.include_symrefs {
                snapshot.symbolic_target
            } else {
                None
            },
        })
        .collect();

    Ok(LsRemoteResult { references })
}

fn matches_reference_filter(reference_name: &str, include_heads: bool, include_tags: bool) -> bool {
    let matches_heads = include_heads && reference_name.starts_with("refs/heads/");
    let matches_tags = include_tags && reference_name.starts_with("refs/tags/");

    if include_heads || include_tags {
        return matches_heads || matches_tags;
    }

    true
}

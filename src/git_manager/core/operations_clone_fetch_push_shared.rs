use super::RemoteReferenceSnapshot;
use crate::git_manager::auth::AuthContext;
use crate::git_manager::core::operations_remote_transport_support::connect_proxy_options;
use crate::git_manager::core::{FetchTagMode, GitError, GitErrorCode, GitResult};
use crate::git_manager::transport::{
    Git2TransportBridge, TransportError, TransportErrorCode, TransportOutcome, TransportRequest,
};
use git2::{AutotagOption, Direction, RemoteCallbacks};

pub(super) fn normalize_depth(depth: Option<usize>) -> GitResult<Option<i32>> {
    let Some(depth) = depth else {
        return Ok(None);
    };

    let depth = i32::try_from(depth).map_err(|_| {
        GitError::new(
            GitErrorCode::TransportFailure,
            format!("clone/fetch depth `{depth}` exceeds supported i32 range"),
        )
    })?;
    if depth <= 0 {
        return Err(GitError::new(
            GitErrorCode::TransportFailure,
            "clone/fetch depth must be greater than zero",
        ));
    }

    Ok(Some(depth))
}

pub(super) const fn map_fetch_tag_mode(mode: FetchTagMode) -> AutotagOption {
    match mode {
        FetchTagMode::Unspecified => AutotagOption::Unspecified,
        FetchTagMode::Auto => AutotagOption::Auto,
        FetchTagMode::None => AutotagOption::None,
        FetchTagMode::All => AutotagOption::All,
    }
}

pub(super) fn build_fetch_refspecs(
    remote_name: &str,
    branch: Option<&str>,
    explicit_refspecs: &[String],
    mirror: bool,
) -> GitResult<Vec<String>> {
    if mirror {
        if branch.and_then(non_empty).is_some() || !explicit_refspecs.is_empty() {
            return Err(GitError::new(
                GitErrorCode::InvalidRefspec,
                "fetch mirror mode cannot be combined with explicit branch/refspec",
            ));
        }

        return Ok(vec!["+refs/*:refs/*".to_string()]);
    }

    let normalized_refspecs = normalize_refspecs_impl(explicit_refspecs, "fetch")?;
    if !normalized_refspecs.is_empty() {
        return Ok(normalized_refspecs);
    }

    let branch = branch.and_then(non_empty);
    let remote_name = non_empty(remote_name);
    match (remote_name, branch) {
        (Some(remote_name), Some(branch)) => Ok(vec![format!(
            "+refs/heads/{branch}:refs/remotes/{remote_name}/{branch}"
        )]),
        _ => Ok(Vec::new()),
    }
}

pub(super) fn normalize_refspecs_impl(
    refspecs: &[String],
    operation_name: &str,
) -> GitResult<Vec<String>> {
    let mut normalized_refspecs = Vec::with_capacity(refspecs.len());
    for refspec in refspecs {
        let trimmed_refspec = refspec.trim();
        if trimmed_refspec.is_empty() {
            return Err(GitError::new(
                GitErrorCode::InvalidRefspec,
                format!("{operation_name} operation received empty refspec"),
            ));
        }
        normalized_refspecs.push(trimmed_refspec.to_string());
    }

    Ok(normalized_refspecs)
}

pub(super) fn transport_request_for_operation(
    operation_name: &str,
    auth_context: AuthContext,
) -> TransportRequest {
    match operation_name {
        "pull" => TransportRequest::for_pull(auth_context),
        "ls-remote" => TransportRequest::for_ls_remote(auth_context),
        _ => TransportRequest::for_fetch(auth_context),
    }
}

pub(super) fn list_remote_reference_snapshots_impl(
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
    remote: &mut git2::Remote<'_>,
) -> Result<Vec<RemoteReferenceSnapshot>, TransportError> {
    transport_bridge
        .execute(transport_request, |callbacks| {
            let connection = remote.connect_auth(
                Direction::Fetch,
                Some(take_remote_callbacks(callbacks)),
                connect_proxy_options(),
            )?;
            let connection_default_branch = connection
                .default_branch()
                .ok()
                .and_then(|branch_buffer| branch_buffer.as_str().map(str::to_owned));

            let snapshots = connection
                .list()?
                .iter()
                .map(|head| RemoteReferenceSnapshot {
                    name: head.name().to_string(),
                    oid: head.oid().to_string(),
                    local_oid: head.loid().to_string(),
                    is_local: head.is_local(),
                    connection_default_branch: connection_default_branch.clone(),
                    symbolic_target: head.symref_target().map(str::to_owned),
                })
                .collect();
            Ok(snapshots)
        })
        .map(TransportOutcome::into_value)
}

pub(super) fn map_transport_error_to_git_error_impl(error: &TransportError) -> GitError {
    let code = match error.code() {
        TransportErrorCode::AuthDenied => GitErrorCode::AuthDenied,
        TransportErrorCode::AuthNoCredentials => GitErrorCode::AuthNoCredentials,
        TransportErrorCode::AuthTimeout => GitErrorCode::AuthTimeout,
        TransportErrorCode::AuthUnsupportedSshDirective => {
            GitErrorCode::AuthUnsupportedSshDirective
        }
        TransportErrorCode::HostKeyUnknown => GitErrorCode::AuthHostKeyUnknown,
        TransportErrorCode::HostKeyMismatch => GitErrorCode::AuthHostKeyMismatch,
        TransportErrorCode::TransportTimeout => GitErrorCode::TransportTimeout,
        TransportErrorCode::TransportTlsError => GitErrorCode::TransportTlsError,
        TransportErrorCode::TransportTemporaryNetwork => GitErrorCode::TransportTemporaryNetwork,
        TransportErrorCode::TransportNetworkFailure => GitErrorCode::TransportNetworkFailure,
        TransportErrorCode::TransportFailure => GitErrorCode::TransportFailure,
    };

    GitError::new(code, format!("{error} (attempts={})", error.attempts()))
}

pub(super) fn take_remote_callbacks<'callbacks>(
    callbacks: &mut RemoteCallbacks<'callbacks>,
) -> RemoteCallbacks<'callbacks> {
    let mut owned_callbacks = RemoteCallbacks::new();
    std::mem::swap(&mut owned_callbacks, callbacks);
    owned_callbacks
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

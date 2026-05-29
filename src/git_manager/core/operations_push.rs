//! `push` operation for `GitManager`.

use crate::git_manager::core::operations_remote_transport_support::build_auth_context_from_remote_url;
use crate::git_manager::core::repository_access::open_repository;
use crate::git_manager::core::{GitResult, PushRequest, PushResult};
use crate::git_manager::transport::{Git2TransportBridge, TransportRequest};

#[path = "operations_push_callback_errors.rs"]
mod callback_errors;
#[path = "operations_push_callbacks.rs"]
mod callbacks;
#[path = "operations_push_refspecs.rs"]
mod refspecs;
#[path = "operations_push_validation.rs"]
mod validation;

/// Performs a `push` through the shared transport bridge and auth chain.
///
/// # Errors
/// Returns a typed `GitError` on repository/remote problems,
/// transport/auth errors, policy hooks, and rejected refs.
#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            remote = request.remote_name,
            branch = ?request.branch,
            refspec_count = tracing::field::Empty,
            mirror = request.mirror,
            prune = request.prune,
            force_with_lease = request.force_with_lease.is_some(),
            fail_if_hooks_present = request.hooks_policy.fail_if_hooks_present
        )
    )
)]
pub fn execute_push_operation(
    request: &PushRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<PushResult> {
    validation::validate_push_request(request)?;
    let repository = open_repository(&request.repository_path, "push")?;
    validation::enforce_push_hooks_policy(&repository, request)?;
    let mut remote = repository
        .find_remote(request.remote_name.as_str())
        .map_err(|error| {
            crate::git_manager::core::GitError::new(
                crate::git_manager::core::GitErrorCode::TransportFailure,
                format!(
                    "push failed to resolve remote `{}`: {error}",
                    request.remote_name
                ),
            )
        })?;
    let remote_url = remote.url().map(str::to_owned).ok_or_else(|| {
        crate::git_manager::core::GitError::new(
            crate::git_manager::core::GitErrorCode::TransportFailure,
            format!(
                "push failed to resolve URL for remote `{}`",
                request.remote_name
            ),
        )
    })?;
    let auth_context = build_auth_context_from_remote_url("push", &remote_url)?;
    let transport_request = TransportRequest::for_push(auth_context);
    if let Some(force_with_lease_policy) = request.force_with_lease.as_ref() {
        refspecs::enforce_force_with_lease_preflight(
            &mut remote,
            transport_bridge,
            &transport_request,
            force_with_lease_policy,
        )?;
    }
    let mut push_refspecs = refspecs::build_push_refspecs(
        &repository,
        &mut remote,
        request,
        transport_bridge,
        &transport_request,
    )?;
    if let Some(force_with_lease_policy) = request.force_with_lease.as_ref() {
        crate::git_manager::core::operations_push_mirror_support::apply_force_with_lease_to_push_refspecs(
            &mut push_refspecs,
            force_with_lease_policy,
        )?;
    }
    tracing::Span::current().record("refspec_count", push_refspecs.len());
    tracing::trace!(refspecs = ?push_refspecs, "resolved push refspec set");
    let transport_outcome = transport_bridge
        .execute(&transport_request, |callbacks| {
            callbacks::execute_push_with_callbacks(&mut remote, callbacks, push_refspecs.as_slice())
        })
        .map_err(|error| {
            callback_errors::map_push_transport_error(&error, request.force_with_lease.is_some())
        })?;
    let push_outcome = transport_outcome.into_value();
    if !push_outcome.rejected_refs.is_empty() {
        tracing::trace!(
            rejected_refs = ?push_outcome.rejected_refs,
            updated_refs = ?push_outcome.updated_refs,
            "push completed with rejected refs"
        );
        return Err(callback_errors::push_rejected_refs_error(
            push_outcome.rejected_refs.as_slice(),
            request.force_with_lease.is_some(),
        ));
    }
    Ok(PushResult {
        updated_refs: push_outcome.updated_refs,
        warnings: vec![callback_errors::hooks_not_executed_warning()],
    })
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

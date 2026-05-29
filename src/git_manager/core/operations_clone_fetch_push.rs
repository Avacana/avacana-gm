//! `clone/fetch/ls-remote` operations for `GitManager`.

pub(super) use crate::git_manager::core::repository_access::open_repository;

use crate::git_manager::core::{
    CloneRequest, CloneResult, FetchRequest, FetchResult, FetchTagMode, GitError, GitResult,
    LsRemoteRequest, LsRemoteResult,
};
use crate::git_manager::transport::{Git2TransportBridge, TransportError, TransportRequest};
use std::path::Path;

#[path = "operations_clone_fetch_push_clone.rs"]
mod clone;
#[path = "operations_clone_local_shallow.rs"]
mod clone_local_shallow;
#[path = "operations_clone_fetch_push_fetch.rs"]
mod fetch;
#[path = "operations_clone_fetch_push_ls_remote.rs"]
mod ls_remote;
#[path = "operations_clone_fetch_push_shared.rs"]
mod shared;

#[derive(Debug, Clone, Copy)]
pub(super) struct FetchOperationRequest<'a> {
    pub(super) repository_path: &'a Path,
    pub(super) remote_name: &'a str,
    pub(super) branch: Option<&'a str>,
    pub(super) depth: Option<usize>,
    pub(super) tag_mode: FetchTagMode,
    pub(super) refspecs: &'a [String],
    pub(super) prune: bool,
    pub(super) mirror: bool,
    pub(super) operation_name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RemoteReferenceSnapshot {
    pub(super) name: String,
    pub(super) oid: String,
    pub(super) local_oid: String,
    pub(super) is_local: bool,
    pub(super) connection_default_branch: Option<String>,
    pub(super) symbolic_target: Option<String>,
}

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            destination = %request.destination_path.display(),
            branch = ?request.branch,
            depth = ?request.depth,
            mirror = request.mirror
        )
    )
)]
pub(super) fn execute_clone_operation(
    request: &CloneRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<CloneResult> {
    clone::execute_clone_operation_impl(request, transport_bridge)
}

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            remote = request.remote_name,
            branch = ?request.branch,
            depth = ?request.depth,
            tag_mode = ?request.tag_mode,
            refspec_count = request.refspecs.len(),
            prune = request.prune,
            mirror = request.mirror
        )
    )
)]
pub(super) fn execute_public_fetch_operation(
    request: &FetchRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<FetchResult> {
    fetch::execute_public_fetch_operation_impl(request, transport_bridge)
}

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            remote = request.remote_name,
            include_heads = request.include_heads,
            include_tags = request.include_tags,
            include_symrefs = request.include_symrefs
        )
    )
)]
pub(super) fn execute_ls_remote_operation(
    request: &LsRemoteRequest,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<LsRemoteResult> {
    ls_remote::execute_ls_remote_operation_impl(request, transport_bridge)
}

#[cfg_attr(
    feature = "trace_logs",
    tracing::instrument(
        skip_all,
        fields(
            repository = %request.repository_path.display(),
            remote = request.remote_name,
            branch = ?request.branch,
            depth = ?request.depth,
            tag_mode = ?request.tag_mode,
            refspec_count = request.refspecs.len(),
            prune = request.prune,
            mirror = request.mirror,
            operation = request.operation_name
        )
    )
)]
pub(super) fn execute_fetch_operation(
    request: &FetchOperationRequest<'_>,
    transport_bridge: &Git2TransportBridge,
) -> GitResult<()> {
    fetch::execute_fetch_operation_impl(request, transport_bridge)
}

pub(super) fn list_remote_reference_snapshots(
    transport_bridge: &Git2TransportBridge,
    transport_request: &TransportRequest,
    remote: &mut git2::Remote<'_>,
) -> Result<Vec<RemoteReferenceSnapshot>, TransportError> {
    shared::list_remote_reference_snapshots_impl(transport_bridge, transport_request, remote)
}

pub(super) fn map_transport_error_to_git_error(error: &TransportError) -> GitError {
    shared::map_transport_error_to_git_error_impl(error)
}

pub(super) fn normalize_refspecs(
    refspecs: &[String],
    operation_name: &str,
) -> GitResult<Vec<String>> {
    shared::normalize_refspecs_impl(refspecs, operation_name)
}

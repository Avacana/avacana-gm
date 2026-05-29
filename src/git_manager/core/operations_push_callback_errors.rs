use crate::git_manager::core::operations_clone_fetch_push::map_transport_error_to_git_error;
use crate::git_manager::core::{GitError, GitErrorCode, GitWarning, GitWarningCode};
use crate::git_manager::transport::{TransportError, TransportErrorCode};

pub(super) fn is_rejected_refs_error(error: &git2::Error) -> bool {
    let normalized = error.message().to_ascii_lowercase();
    normalized.contains("failed to push some refs")
        || normalized.contains("non-fast-forward")
        || normalized.contains("cannot push because a reference")
        || normalized.contains("not present locally")
        || (normalized.contains("rejected") && normalized.contains("refs"))
}

pub(super) fn fallback_rejected_ref_message(error: &git2::Error) -> String {
    format!(
        "<unknown-ref>: push rejected by remote ({:?}/{:?})",
        error.class(),
        error.code()
    )
}

pub(super) fn hooks_not_executed_warning() -> GitWarning {
    GitWarning::new(
        GitWarningCode::HooksNotExecuted,
        "git hooks were not executed due to NO_SUBPROCESS policy",
    )
}

pub(super) fn push_rejected_refs_error(
    rejected_refs: &[String],
    force_with_lease_enabled: bool,
) -> GitError {
    if force_with_lease_enabled
        && rejected_refs
            .iter()
            .any(|message| is_force_with_lease_rejection(message))
    {
        return GitError::new(
            GitErrorCode::PushForceWithLeaseRejected,
            format!(
                "push force-with-lease rejected by remote refs: {}",
                rejected_refs.join(", ")
            ),
        );
    }

    GitError::new(
        GitErrorCode::PushRejectedRefs,
        format!("push rejected by remote refs: {}", rejected_refs.join(", ")),
    )
}

pub(super) fn map_push_transport_error(
    error: &TransportError,
    force_with_lease_enabled: bool,
) -> GitError {
    let rendered = error.to_string();

    if matches!(
        error.code(),
        TransportErrorCode::TransportFailure | TransportErrorCode::TransportNetworkFailure
    ) {
        if force_with_lease_enabled && is_force_with_lease_rejection(rendered.as_str()) {
            return GitError::new(GitErrorCode::PushForceWithLeaseRejected, rendered);
        }

        if is_rejected_refs_transport_message(rendered.as_str()) {
            return GitError::new(GitErrorCode::PushRejectedRefs, rendered);
        }
    }

    map_transport_error_to_git_error(error)
}

fn is_rejected_refs_transport_message(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("failed to push some refs")
        || normalized.contains("non-fast-forward")
        || normalized.contains("cannot push because a reference")
        || normalized.contains("not present locally")
        || (normalized.contains("rejected") && normalized.contains("refs"))
}

fn is_force_with_lease_rejection(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("force-with-lease")
        || normalized.contains("stale info")
        || normalized.contains("stale")
}

use super::{RetryDecision, TransportErrorCode};
use crate::git_manager::auth::AuthErrorCode;
use crate::git_manager::transport::redact_url_userinfo;
use git2::{Error, ErrorClass, ErrorCode};

const TEMPORARY_NETWORK_MARKERS: [&str; 8] = [
    "timed out",
    "timeout",
    "temporary",
    "try again",
    "connection reset",
    "connection refused",
    "network is unreachable",
    "operation would block",
];

pub(super) fn classify_git2_error(error: &Error) -> (TransportErrorCode, RetryDecision) {
    if let Some(auth_error_code) = parse_embedded_auth_error_code(error.message()) {
        return classify_auth_error_code(auth_error_code);
    }

    if matches!(error.code(), ErrorCode::Auth) || is_auth_denied_message(error.message()) {
        return (
            TransportErrorCode::AuthDenied,
            RetryDecision::StopAuthDenied,
        );
    }

    if matches!(error.code(), ErrorCode::Certificate) || matches!(error.class(), ErrorClass::Ssl) {
        return (
            TransportErrorCode::TransportTlsError,
            RetryDecision::StopNonRetryable,
        );
    }

    if matches!(error.code(), ErrorCode::Timeout) {
        return (
            TransportErrorCode::TransportTimeout,
            RetryDecision::RetryTemporaryNetwork,
        );
    }

    if is_temporary_network_error(error) {
        return (
            TransportErrorCode::TransportTemporaryNetwork,
            RetryDecision::RetryTemporaryNetwork,
        );
    }

    if matches!(
        error.class(),
        ErrorClass::Net | ErrorClass::Http | ErrorClass::Ssh | ErrorClass::Os
    ) {
        return (
            TransportErrorCode::TransportNetworkFailure,
            RetryDecision::StopNonRetryable,
        );
    }

    (
        TransportErrorCode::TransportFailure,
        RetryDecision::StopNonRetryable,
    )
}

const fn classify_auth_error_code(
    auth_error_code: AuthErrorCode,
) -> (TransportErrorCode, RetryDecision) {
    match auth_error_code {
        AuthErrorCode::NoCredentials => (
            TransportErrorCode::AuthNoCredentials,
            RetryDecision::StopNonRetryable,
        ),
        AuthErrorCode::Denied => (
            TransportErrorCode::AuthDenied,
            RetryDecision::StopAuthDenied,
        ),
        AuthErrorCode::Timeout => (
            TransportErrorCode::AuthTimeout,
            RetryDecision::StopNonRetryable,
        ),
        AuthErrorCode::UnsupportedSshDirective => (
            TransportErrorCode::AuthUnsupportedSshDirective,
            RetryDecision::StopNonRetryable,
        ),
        AuthErrorCode::HostKeyUnknown => (
            TransportErrorCode::HostKeyUnknown,
            RetryDecision::StopNonRetryable,
        ),
        AuthErrorCode::HostKeyMismatch => (
            TransportErrorCode::HostKeyMismatch,
            RetryDecision::StopNonRetryable,
        ),
    }
}

fn parse_embedded_auth_error_code(message: &str) -> Option<AuthErrorCode> {
    let (_, code_and_rest) = message.split_once('[')?;
    let (code, _) = code_and_rest.split_once(']')?;
    match code {
        "AUTH_NO_CREDENTIALS" => Some(AuthErrorCode::NoCredentials),
        "AUTH_DENIED" => Some(AuthErrorCode::Denied),
        "AUTH_TIMEOUT" => Some(AuthErrorCode::Timeout),
        "AUTH_UNSUPPORTED_SSH_DIRECTIVE" => Some(AuthErrorCode::UnsupportedSshDirective),
        "AUTH_HOSTKEY_UNKNOWN" => Some(AuthErrorCode::HostKeyUnknown),
        "AUTH_HOSTKEY_MISMATCH" => Some(AuthErrorCode::HostKeyMismatch),
        _ => None,
    }
}

fn is_auth_denied_message(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("authentication failed")
        || normalized.contains("auth failed")
        || normalized.contains("permission denied")
}

fn is_temporary_network_error(error: &Error) -> bool {
    if matches!(error.code(), ErrorCode::Timeout | ErrorCode::Eof) {
        return true;
    }

    if contains_temporary_network_markers(error.message()) {
        return true;
    }

    matches!(
        error.class(),
        ErrorClass::Net | ErrorClass::Http | ErrorClass::Ssh | ErrorClass::Os
    ) && contains_temporary_network_markers(error.message())
}

fn contains_temporary_network_markers(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    TEMPORARY_NETWORK_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

pub(super) fn normalize_error_message(error: &Error) -> String {
    let normalized = error
        .message()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let redacted = redact_url_userinfo(&normalized);

    if redacted.is_empty() {
        format!(
            "git2 returned empty error (class={:?}, code={:?})",
            error.class(),
            error.code()
        )
    } else {
        redacted
    }
}

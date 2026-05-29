//! Unified `GitManager` transport bridge for `git2` callbacks and retry policy.

mod callbacks;
mod certificates;
mod git2_bridge;

pub use certificates::{HostKeyPolicy, HostKeyPolicyResult, KnownHostsVerifier};
pub use git2_bridge::{
    Git2TransportBridge, RetryDecision, TransportError, TransportErrorCode,
    TransportErrorDiagnostic, TransportOutcome, TransportRequest, TransportResult,
    TransportRetryPolicy,
};

fn redact_url_userinfo(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_url_userinfo_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_url_userinfo_token(token: &str) -> String {
    let Some((scheme, after_scheme)) = token.split_once("://") else {
        return token.to_string();
    };

    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    let Some(userinfo_end) = authority.rfind('@') else {
        return token.to_string();
    };

    format!(
        "{scheme}://<redacted>@{}",
        &after_scheme[(userinfo_end + 1)..]
    )
}

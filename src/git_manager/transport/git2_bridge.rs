use super::callbacks::configure_git2_remote_callbacks;
use super::certificates::{HostKeyPolicy, KnownHostsVerifier};
use crate::git_manager::auth::AuthChain;
use git2::{Error, RemoteCallbacks};
use std::sync::Arc;
use std::time::Duration;

#[path = "git2_bridge_error.rs"]
mod error_classification;
#[path = "git2_bridge_error_types.rs"]
mod error_types;
#[path = "git2_bridge_outcome.rs"]
mod outcome;
#[path = "git2_bridge_request.rs"]
mod request;
#[path = "git2_bridge_shared.rs"]
mod shared;

use error_classification::{classify_git2_error, normalize_error_message};
pub use error_types::{
    TransportError, TransportErrorCode, TransportErrorDiagnostic, TransportResult,
};
pub use outcome::TransportOutcome;
pub use request::TransportRequest;
use shared::TransportCallbackSummary;
pub use shared::{RetryDecision, TransportRetryPolicy};

/// Centralized `git2` transport bridge with callback, auth, and retry classification.
#[derive(Debug, Clone)]
pub struct Git2TransportBridge {
    auth_chain: Arc<AuthChain>,
    known_hosts_verifier: KnownHostsVerifier,
    retry_policy: TransportRetryPolicy,
}

impl Git2TransportBridge {
    /// Creates a transport bridge with explicit dependencies.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        auth_chain: Arc<AuthChain>,
        known_hosts_verifier: KnownHostsVerifier,
        retry_policy: TransportRetryPolicy,
    ) -> Self {
        Self {
            auth_chain,
            known_hosts_verifier,
            retry_policy,
        }
    }

    /// Creates a bridge with the default `known_hosts` (strict) and default retry policy.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_defaults(auth_chain: Arc<AuthChain>) -> Self {
        Self::new(
            auth_chain,
            KnownHostsVerifier::with_default_path(HostKeyPolicy::strict()),
            TransportRetryPolicy::default(),
        )
    }

    /// Returns the current retry policy of the transport bridge.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn retry_policy(&self) -> TransportRetryPolicy {
        self.retry_policy
    }

    /// Returns the host key verifier used in the certificate callback.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn known_hosts_verifier(&self) -> &KnownHostsVerifier {
        &self.known_hosts_verifier
    }

    /// Executes a remote operation through the shared transport bridge.
    ///
    /// # Errors
    /// Returns a typed `TransportError` with a machine-readable code and the
    /// retry decisions that were made.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                operation = %request.operation(),
                target = %request.auth_context().redacted_target()
            )
        )
    )]
    pub fn execute<T, F>(
        &self,
        request: &TransportRequest,
        mut remote_operation: F,
    ) -> TransportResult<TransportOutcome<T>>
    where
        F: for<'callbacks> FnMut(&mut RemoteCallbacks<'callbacks>) -> Result<T, Error>,
    {
        let retry_limit = request
            .retry_limit_override()
            .unwrap_or_else(|| self.retry_policy.temporary_network_retries());
        let mut retry_decisions = Vec::new();
        let mut auth_budget = request.auth_attempt_budget().clone();
        let mut temporary_retries_used = 0_usize;
        let mut attempts = 0_usize;
        let mut callback_summary = TransportCallbackSummary::default();

        loop {
            attempts += 1;

            let (operation_result, callback_runtime) = {
                let mut remote_callbacks = RemoteCallbacks::new();
                let callback_runtime = configure_git2_remote_callbacks(
                    &mut remote_callbacks,
                    self.auth_chain.as_ref(),
                    request.auth_context(),
                    &mut auth_budget,
                    self.known_hosts_verifier.clone(),
                );
                let operation_result = remote_operation(&mut remote_callbacks);
                (operation_result, callback_runtime)
            };
            callback_summary.merge_attempt(
                callback_runtime.credential_callback_count(),
                callback_runtime.transport_messages().len(),
                callback_runtime.update_tips().len(),
                callback_runtime.push_updates().len(),
            );

            match operation_result {
                Ok(value) => {
                    return Ok(TransportOutcome::new(
                        value,
                        attempts,
                        retry_decisions,
                        callback_summary,
                    ));
                }
                Err(error) => {
                    let (error_code, retry_decision) = classify_git2_error(&error);
                    let should_retry =
                        matches!(retry_decision, RetryDecision::RetryTemporaryNetwork)
                            && temporary_retries_used < retry_limit;

                    if should_retry {
                        let backoff_delay = Duration::from_millis(
                            100_u64
                                .saturating_mul(2_u64.saturating_pow(
                                    u32::try_from(temporary_retries_used).unwrap_or(u32::MAX),
                                ))
                                .min(1_000),
                        );
                        temporary_retries_used += 1;
                        retry_decisions.push(RetryDecision::RetryTemporaryNetwork);
                        tracing::trace!(
                            attempt = attempts,
                            retry_limit = retry_limit,
                            retry_backoff_ms = backoff_delay.as_millis(),
                            error = %normalize_error_message(&error),
                            "transport bridge retries temporary network failure"
                        );
                        std::thread::sleep(backoff_delay);
                        continue;
                    }

                    if matches!(retry_decision, RetryDecision::StopAuthDenied) {
                        if let Some(fingerprint) = callback_runtime.last_used_material_fingerprint()
                        {
                            auth_budget.invalidate(&fingerprint);
                            tracing::trace!(
                                fingerprint = fingerprint.as_str(),
                                "transport bridge marked denied material as invalid"
                            );
                        }
                    }

                    let final_retry_decision =
                        if matches!(retry_decision, RetryDecision::RetryTemporaryNetwork) {
                            RetryDecision::StopNonRetryable
                        } else {
                            retry_decision
                        };

                    retry_decisions.push(final_retry_decision);
                    return Err(TransportError::new(
                        error_code,
                        attempts,
                        retry_decisions,
                        format!(
                            "transport operation `{}` failed: {}",
                            request.operation(),
                            normalize_error_message(&error)
                        ),
                    ));
                }
            }
        }
    }
}

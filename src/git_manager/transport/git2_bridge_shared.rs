const DEFAULT_TEMPORARY_NETWORK_RETRIES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::redundant_pub_crate, clippy::struct_field_names)]
pub(crate) struct TransportCallbackSummary {
    credential_callback_count: usize,
    transport_message_count: usize,
    update_tip_count: usize,
    push_update_count: usize,
}

impl TransportCallbackSummary {
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn merge_attempt(
        &mut self,
        credential_callback_count: usize,
        transport_message_count: usize,
        update_tip_count: usize,
        push_update_count: usize,
    ) {
        self.credential_callback_count += credential_callback_count;
        self.transport_message_count += transport_message_count;
        self.update_tip_count += update_tip_count;
        self.push_update_count += push_update_count;
    }
}

/// Retry policy for the transport bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportRetryPolicy {
    temporary_network_retries: usize,
}

impl TransportRetryPolicy {
    /// Creates a policy with the given retry limit for temporary network errors.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(temporary_network_retries: usize) -> Self {
        Self {
            temporary_network_retries,
        }
    }

    /// Returns the retry limit for temporary network errors only.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn temporary_network_retries(self) -> usize {
        self.temporary_network_retries
    }
}

impl Default for TransportRetryPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_TEMPORARY_NETWORK_RETRIES)
    }
}

/// The bridge's decision after classifying a transport operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetryDecision {
    /// Retry the operation because the error was classified as a temporary network failure.
    RetryTemporaryNetwork,
    /// Stop without retrying because of `AUTH_DENIED`.
    StopAuthDenied,
    /// Stop without retrying because of a non-retryable error.
    StopNonRetryable,
}

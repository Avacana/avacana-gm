use super::shared::{RetryDecision, TransportCallbackSummary};

/// Result of executing a transport operation.
#[derive(Debug)]
pub struct TransportOutcome<T> {
    value: T,
    attempts: usize,
    retry_decisions: Vec<RetryDecision>,
    #[allow(dead_code)]
    callback_summary: TransportCallbackSummary,
}

impl<T> TransportOutcome<T> {
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn new(
        value: T,
        attempts: usize,
        retry_decisions: Vec<RetryDecision>,
        callback_summary: TransportCallbackSummary,
    ) -> Self {
        Self {
            value,
            attempts,
            retry_decisions,
            callback_summary,
        }
    }

    /// Returns the number of actual transport operation attempts.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn attempts(&self) -> usize {
        self.attempts
    }

    /// Returns the list of retry decisions made.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn retry_decisions(&self) -> &[RetryDecision] {
        &self.retry_decisions
    }

    /// Returns a reference to the payload of the successful operation.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consumes the outcome and returns the payload of the successful transport operation.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

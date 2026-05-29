use std::fmt;

use super::shared::RetryDecision;

/// Machine-readable error codes for the transport bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportErrorCode {
    /// The remote rejected the credential (`AUTH_DENIED`).
    AuthDenied,
    /// No credentials were found (`AUTH_NO_CREDENTIALS`).
    AuthNoCredentials,
    /// The auth flow timed out (`AUTH_TIMEOUT`).
    AuthTimeout,
    /// An unsupported directive was encountered in `ssh_config`.
    AuthUnsupportedSshDirective,
    /// Unknown host key under the strict `known_hosts` policy.
    HostKeyUnknown,
    /// Host key mismatch against `known_hosts`.
    HostKeyMismatch,
    /// Network timeout at the transport level.
    TransportTimeout,
    /// TLS/certificate error at the transport level.
    TransportTlsError,
    /// Temporary network error at the transport level.
    TransportTemporaryNetwork,
    /// Non-retryable network error at the transport level.
    TransportNetworkFailure,
    /// Any other transport bridge error.
    TransportFailure,
}

impl TransportErrorCode {
    /// Returns the stable machine-readable code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthDenied => "AUTH_DENIED",
            Self::AuthNoCredentials => "AUTH_NO_CREDENTIALS",
            Self::AuthTimeout => "AUTH_TIMEOUT",
            Self::AuthUnsupportedSshDirective => "AUTH_UNSUPPORTED_SSH_DIRECTIVE",
            Self::HostKeyUnknown => "AUTH_HOSTKEY_UNKNOWN",
            Self::HostKeyMismatch => "AUTH_HOSTKEY_MISMATCH",
            Self::TransportTimeout => "TRANSPORT_TIMEOUT",
            Self::TransportTlsError => "TRANSPORT_TLS_ERROR",
            Self::TransportTemporaryNetwork => "TRANSPORT_TEMPORARY_NETWORK",
            Self::TransportNetworkFailure => "TRANSPORT_NETWORK_FAILURE",
            Self::TransportFailure => "TRANSPORT_FAILURE",
        }
    }
}

impl fmt::Display for TransportErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed transport bridge error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    code: TransportErrorCode,
    message: String,
    attempts: usize,
    retry_decisions: Vec<RetryDecision>,
}

/// Typed diagnostics snapshot of a transport bridge error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportErrorDiagnostic {
    code: TransportErrorCode,
    message: String,
    attempts: usize,
    retry_decisions: Vec<RetryDecision>,
}

impl TransportErrorDiagnostic {
    /// Returns the machine-readable transport bridge error code.
    #[must_use]
    pub const fn code(&self) -> TransportErrorCode {
        self.code
    }

    /// Returns the redaction-safe error text.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the number of actual attempts made before the error.
    #[must_use]
    pub const fn attempts(&self) -> usize {
        self.attempts
    }

    /// Returns the history of retry decisions.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn retry_decisions(&self) -> &[RetryDecision] {
        &self.retry_decisions
    }
}

impl TransportError {
    pub(crate) fn new(
        code: TransportErrorCode,
        attempts: usize,
        retry_decisions: Vec<RetryDecision>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            attempts,
            retry_decisions,
        }
    }

    /// Returns the machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> TransportErrorCode {
        self.code
    }

    /// Returns the number of attempts spent before the final error.
    #[must_use]
    pub const fn attempts(&self) -> usize {
        self.attempts
    }

    /// Returns the redaction-safe error text without the code.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the history of retry decisions leading up to the final error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn retry_decisions(&self) -> &[RetryDecision] {
        &self.retry_decisions
    }

    /// Returns a typed diagnostics snapshot of the transport error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn diagnostic(&self) -> TransportErrorDiagnostic {
        TransportErrorDiagnostic {
            code: self.code,
            message: self.message.clone(),
            attempts: self.attempts,
            retry_decisions: self.retry_decisions.clone(),
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for TransportError {}

/// Standard result alias for the transport bridge.
pub type TransportResult<T> = Result<T, TransportError>;

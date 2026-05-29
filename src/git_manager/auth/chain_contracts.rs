use super::{AuthCapabilities, AuthContext, AuthMaterial};
use std::fmt;

/// Result of auth-layer operations.
pub type AuthResult<T> = Result<T, AuthError>;

/// Machine-readable auth-layer error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthErrorCode {
    /// No usable credentials were found in the provider chain.
    NoCredentials,
    /// Credentials were rejected by the remote side (`401/403`).
    Denied,
    /// The wall-clock timeout for the auth operation was exceeded.
    Timeout,
    /// An unsupported SSH directive was encountered in the active config path.
    UnsupportedSshDirective,
    /// The host key is absent from `known_hosts` under a strict policy.
    HostKeyUnknown,
    /// The host key in `known_hosts` does not match the presented key.
    HostKeyMismatch,
}

impl AuthErrorCode {
    /// Returns the stable string representation of the code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCredentials => "AUTH_NO_CREDENTIALS",
            Self::Denied => "AUTH_DENIED",
            Self::Timeout => "AUTH_TIMEOUT",
            Self::UnsupportedSshDirective => "AUTH_UNSUPPORTED_SSH_DIRECTIVE",
            Self::HostKeyUnknown => "AUTH_HOSTKEY_UNKNOWN",
            Self::HostKeyMismatch => "AUTH_HOSTKEY_MISMATCH",
        }
    }
}

impl fmt::Display for AuthErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed auth-layer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthError {
    code: AuthErrorCode,
    message: String,
}

/// Typed diagnostic snapshot of an auth-layer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthErrorDiagnostic {
    code: AuthErrorCode,
    message: String,
}

impl AuthErrorDiagnostic {
    /// Returns the machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> AuthErrorCode {
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
}

impl AuthError {
    /// Creates a new auth-layer error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(code: AuthErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> AuthErrorCode {
        self.code
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

    /// Returns a typed diagnostic snapshot of the error.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn diagnostic(&self) -> AuthErrorDiagnostic {
        AuthErrorDiagnostic {
            code: self.code,
            message: self.message.clone(),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AuthError {}

/// Outcome of validating a specific credential material at the transport level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAttemptOutcome {
    /// The material was accepted; auth completed successfully.
    Authenticated,
    /// The material was rejected by the remote side and must be invalidated.
    Denied,
    /// A transient failure (e.g. network); advancing to the next material is allowed.
    RetryableError,
}

/// Auth-provider contract for the `GitAuth` chain.
pub trait GitAuthProvider: Send + Sync {
    /// Returns the stable provider identifier.
    fn id(&self) -> &'static str;

    /// Determines whether the provider applies to the current context and capabilities.
    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool;

    /// Loads credential material.
    ///
    /// # Errors
    /// Returns a typed `AuthError` if the provider's source is unavailable
    /// or the data is corrupted.
    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>>;

    /// Attempts to save credential material to the provider's backing store.
    ///
    /// The default implementation does nothing.
    ///
    /// # Errors
    /// Returns a typed `AuthError` if the provider cannot perform the save.
    fn save(&self, _ctx: &AuthContext, _material: &AuthMaterial) -> AuthResult<()> {
        Ok(())
    }

    /// Invalidates credential material within the scope of the current operation.
    ///
    /// The default implementation does nothing.
    ///
    /// # Errors
    /// Returns a typed `AuthError` if the invalidation could not be performed.
    fn invalidate(&self, _ctx: &AuthContext, _material: &AuthMaterial) -> AuthResult<()> {
        Ok(())
    }
}

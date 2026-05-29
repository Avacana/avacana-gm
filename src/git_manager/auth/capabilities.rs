use super::context::AuthTransport;
use std::fmt;

/// Execution mode that affects which auth providers are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthEnvironmentMode {
    /// Full desktop environment (may be interactive).
    DesktopFull,
    /// Non-interactive CI/server mode.
    HeadlessCi,
    /// Restricted sandbox environment.
    RestrictedSandbox,
}

impl AuthEnvironmentMode {
    /// Returns the stable machine-readable mode code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DesktopFull => "desktop_full",
            Self::HeadlessCi => "headless_ci",
            Self::RestrictedSandbox => "restricted_sandbox",
        }
    }
}

impl fmt::Display for AuthEnvironmentMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Auth-layer capability flags for a specific operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCapabilities {
    environment_mode: AuthEnvironmentMode,
    allow_interactive: bool,
    allow_ssh: bool,
    allow_https: bool,
}

impl AuthCapabilities {
    /// Creates capabilities with default flags for the given mode.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(environment_mode: AuthEnvironmentMode) -> Self {
        let allow_interactive = matches!(environment_mode, AuthEnvironmentMode::DesktopFull);
        Self {
            environment_mode,
            allow_interactive,
            allow_ssh: true,
            allow_https: true,
        }
    }

    /// Returns the environment mode.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn environment_mode(&self) -> AuthEnvironmentMode {
        self.environment_mode
    }

    /// Returns the flag that permits interactive providers.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_interactive(&self) -> bool {
        self.allow_interactive
    }

    /// Returns `true` if SSH providers are allowed.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_ssh(&self) -> bool {
        self.allow_ssh
    }

    /// Returns `true` if HTTPS providers are allowed.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_https(&self) -> bool {
        self.allow_https
    }

    /// Returns a copy of the capabilities with the interactive flag overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_allow_interactive(mut self, allow_interactive: bool) -> Self {
        self.allow_interactive = allow_interactive;
        self
    }

    /// Returns a copy of the capabilities with transport support overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_transport_support(mut self, allow_ssh: bool, allow_https: bool) -> Self {
        self.allow_ssh = allow_ssh;
        self.allow_https = allow_https;
        self
    }

    /// Checks whether the given transport is allowed by the current capabilities.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn supports_transport(&self, transport: AuthTransport) -> bool {
        match transport {
            AuthTransport::Ssh => self.allow_ssh,
            AuthTransport::Https => self.allow_https,
        }
    }
}

impl Default for AuthCapabilities {
    fn default() -> Self {
        Self::new(AuthEnvironmentMode::HeadlessCi)
    }
}


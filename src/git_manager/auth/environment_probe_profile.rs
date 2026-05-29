use super::{AuthCapabilities, AuthEnvironmentMode};
use std::fmt;

/// Snapshot of the auth-pipeline environment computed by the probe module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEnvironmentProfile {
    mode: AuthEnvironmentMode,
    interaction_policy: InteractionPolicy,
    transport_policy: TransportPolicy,
    security_policy: SecurityPolicy,
}

/// Source of the decision about the auth-environment mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthEnvironmentModeSource {
    /// The mode was set directly via `AVACANA_GM_AUTH_ENV_MODE`.
    ExplicitEnvironmentOverride,
    /// The mode was selected by the `AVACANA_GM_AUTH_RESTRICTED=true` flag.
    RestrictedFlag,
    /// The mode was selected from headless signals (`CI=true` or `AVACANA_GM_AUTH_HEADLESS=true`).
    HeadlessSignal,
    /// The default desktop mode was used.
    DesktopDefault,
}

impl AuthEnvironmentModeSource {
    /// Returns the stable machine-readable source code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitEnvironmentOverride => "explicit_env_override",
            Self::RestrictedFlag => "restricted_flag",
            Self::HeadlessSignal => "headless_signal",
            Self::DesktopDefault => "desktop_default",
        }
    }
}

impl fmt::Display for AuthEnvironmentModeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Machine-readable warning code for the env probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthEnvironmentProbeWarningCode {
    /// Invalid value of `AVACANA_GM_AUTH_ENV_MODE`.
    InvalidEnvironmentMode,
    /// Invalid boolean value in a policy env variable.
    InvalidBooleanValue,
}

impl AuthEnvironmentProbeWarningCode {
    /// Returns the stable machine-readable warning code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEnvironmentMode => "invalid_environment_mode",
            Self::InvalidBooleanValue => "invalid_boolean_value",
        }
    }
}

impl fmt::Display for AuthEnvironmentProbeWarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Diagnostic warning from the env probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEnvironmentProbeWarning {
    code: AuthEnvironmentProbeWarningCode,
    variable: &'static str,
    raw_value: String,
}

impl AuthEnvironmentProbeWarning {
    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn invalid_mode(variable: &'static str, raw_value: String) -> Self {
        Self {
            code: AuthEnvironmentProbeWarningCode::InvalidEnvironmentMode,
            variable,
            raw_value,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn invalid_boolean(variable: &'static str, raw_value: String) -> Self {
        Self {
            code: AuthEnvironmentProbeWarningCode::InvalidBooleanValue,
            variable,
            raw_value,
        }
    }

    /// Returns the machine-readable warning code.
    #[must_use]
    pub const fn code(&self) -> AuthEnvironmentProbeWarningCode {
        self.code
    }

    /// Returns the name of the env variable that triggered the warning.
    #[must_use]
    pub const fn variable(&self) -> &'static str {
        self.variable
    }

    /// Returns the original (trimmed) value of the env variable.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn raw_value(&self) -> &str {
        &self.raw_value
    }
}

/// Typed diagnostics from the env probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEnvironmentProbeDiagnostics {
    mode_source: AuthEnvironmentModeSource,
    warnings: Vec<AuthEnvironmentProbeWarning>,
}

impl AuthEnvironmentProbeDiagnostics {
    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn set_mode_source(&mut self, mode_source: AuthEnvironmentModeSource) {
        self.mode_source = mode_source;
    }

    pub(super) fn push_warning(&mut self, warning: AuthEnvironmentProbeWarning) {
        self.warnings.push(warning);
    }

    /// Returns the source of the selected mode.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn mode_source(&self) -> AuthEnvironmentModeSource {
        self.mode_source
    }

    /// Returns the list of env-probe warnings.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn warnings(&self) -> &[AuthEnvironmentProbeWarning] {
        &self.warnings
    }
}

impl Default for AuthEnvironmentProbeDiagnostics {
    fn default() -> Self {
        Self {
            mode_source: AuthEnvironmentModeSource::DesktopDefault,
            warnings: Vec::new(),
        }
    }
}

/// Snapshot of the env-probe result: profile + diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEnvironmentProbeSnapshot {
    profile: AuthEnvironmentProfile,
    diagnostics: AuthEnvironmentProbeDiagnostics,
}

impl AuthEnvironmentProbeSnapshot {
    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn new(
        profile: AuthEnvironmentProfile,
        diagnostics: AuthEnvironmentProbeDiagnostics,
    ) -> Self {
        Self {
            profile,
            diagnostics,
        }
    }

    /// Returns the auth-environment profile.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn profile(&self) -> &AuthEnvironmentProfile {
        &self.profile
    }

    /// Returns the typed env-probe diagnostics.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn diagnostics(&self) -> &AuthEnvironmentProbeDiagnostics {
        &self.diagnostics
    }

    /// Extracts the profile from the probe snapshot.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn into_profile(self) -> AuthEnvironmentProfile {
        self.profile
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InteractionPolicy {
    allow_interactive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TransportPolicy {
    allow_ssh: bool,
    allow_https: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SecurityPolicy {
    os_store_available: bool,
    accept_new_host: bool,
}

impl AuthEnvironmentProfile {
    /// Creates an environment profile with default flags for the given mode.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn from_mode(mode: AuthEnvironmentMode) -> Self {
        let allow_interactive = matches!(mode, AuthEnvironmentMode::DesktopFull);
        Self {
            mode,
            interaction_policy: InteractionPolicy { allow_interactive },
            transport_policy: TransportPolicy {
                allow_ssh: true,
                allow_https: true,
            },
            security_policy: SecurityPolicy {
                os_store_available: false,
                accept_new_host: false,
            },
        }
    }

    /// Returns the environment mode.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn mode(&self) -> AuthEnvironmentMode {
        self.mode
    }

    /// Returns `true` if interactive auth may be enabled.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_interactive(&self) -> bool {
        self.interaction_policy.allow_interactive
    }

    /// Returns `true` if SSH transport is allowed by the policy profile.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_ssh(&self) -> bool {
        self.transport_policy.allow_ssh
    }

    /// Returns `true` if HTTPS transport is allowed by the policy profile.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn allow_https(&self) -> bool {
        self.transport_policy.allow_https
    }

    /// Returns `true` if the OS secret store is available in the current environment.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn os_store_available(&self) -> bool {
        self.security_policy.os_store_available
    }

    /// Returns `true` if an explicit `accept_new_host` policy is enabled.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn accept_new_host(&self) -> bool {
        self.security_policy.accept_new_host
    }

    /// Returns the profile with the interactive flag overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_allow_interactive(mut self, allow_interactive: bool) -> Self {
        self.interaction_policy.allow_interactive = allow_interactive;
        self
    }

    /// Returns the profile with transport support overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_transport_support(mut self, allow_ssh: bool, allow_https: bool) -> Self {
        self.transport_policy.allow_ssh = allow_ssh;
        self.transport_policy.allow_https = allow_https;
        self
    }

    /// Returns the profile with OS-store availability overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_os_store_available(mut self, os_store_available: bool) -> Self {
        self.security_policy.os_store_available = os_store_available;
        self
    }

    /// Returns the profile with the `accept_new_host` flag overridden.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_accept_new_host(mut self, accept_new_host: bool) -> Self {
        self.security_policy.accept_new_host = accept_new_host;
        self
    }

    /// Builds the auth-chain capabilities from the probe profile.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn capabilities(&self) -> AuthCapabilities {
        AuthCapabilities::new(self.mode)
            .with_allow_interactive(self.interaction_policy.allow_interactive)
            .with_transport_support(
                self.transport_policy.allow_ssh,
                self.transport_policy.allow_https,
            )
    }
}

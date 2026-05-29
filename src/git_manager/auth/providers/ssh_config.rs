use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthMaterialKind, AuthResult, AuthTransport,
    GitAuthProvider,
};
use std::path::PathBuf;

#[path = "ssh_config_parser.rs"]
mod parser;
#[path = "ssh_config_paths.rs"]
mod paths;

use parser::{parse_config_file, ResolveState};
use paths::default_ssh_config_path;

const SSH_CONFIG_PROVIDER_ID: &str = "ssh-config";

/// The result of resolving `~/.ssh/config` for a specific remote host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConfigResolved {
    hostname: String,
    user: Option<String>,
    port: Option<u16>,
    identity_files: Vec<PathBuf>,
    identities_only: bool,
}

impl SshConfigResolved {
    /// Creates a baseline resolved object for the given host.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(requested_host: impl Into<String>) -> Self {
        let requested_host = requested_host.into();
        Self {
            hostname: requested_host,
            user: None,
            port: None,
            identity_files: Vec::new(),
            identities_only: false,
        }
    }

    /// Returns the effective host after processing `Hostname`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns the effective username.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Returns the effective port.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the list of `IdentityFile` entries after processing Include.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn identity_files(&self) -> &[PathBuf] {
        &self.identity_files
    }

    /// Returns the state of the `IdentitiesOnly` flag.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn identities_only(&self) -> bool {
        self.identities_only
    }
}

/// Provider that resolves the `.ssh/config` subset for the SSH auth chain.
#[derive(Debug, Clone)]
pub struct SshConfigProvider {
    config_path: Option<PathBuf>,
}

impl SshConfigProvider {
    /// Creates a provider with an explicit path to the `config` file.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(config_path: Option<PathBuf>) -> Self {
        Self { config_path }
    }

    /// Creates a provider using the default user path `~/.ssh/config`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_path() -> Self {
        Self::new(default_ssh_config_path())
    }

    /// Resolves the `.ssh/config` subset for the given auth context.
    ///
    /// # Errors
    /// Returns `AUTH_UNSUPPORTED_SSH_DIRECTIVE` if the active block
    /// contains an unsupported directive or an invalid value.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve(&self, ctx: &AuthContext) -> AuthResult<SshConfigResolved> {
        let mut state = ResolveState::new(ctx);

        let Some(config_path) = self.config_path.as_deref() else {
            return Ok(state.into_resolved());
        };

        let mut visit_state = parser::ParseVisitState::default();
        parse_config_file(config_path, ctx.remote_host(), &mut state, &mut visit_state)?;
        Ok(state.into_resolved())
    }
}

impl Default for SshConfigProvider {
    fn default() -> Self {
        Self::with_default_path()
    }
}

impl GitAuthProvider for SshConfigProvider {
    fn id(&self) -> &'static str {
        SSH_CONFIG_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Ssh && caps.allow_ssh()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let resolved = self.resolve(ctx)?;
        let Some(user) = resolved.user().map(str::to_owned) else {
            return Ok(None);
        };

        Ok(Some(AuthMaterial::without_secret(
            self.id(),
            AuthMaterialKind::UsernameOnly,
            Some(user.clone()),
            format!("ssh-config-user:{user}"),
        )))
    }
}

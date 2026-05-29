use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthMaterialKind, AuthResult, AuthTransport,
    GitAuthProvider, SshCredentialSource,
};
use std::path::{Path, PathBuf};

const SSH_AGENT_PROVIDER_ID: &str = "ssh-agent";

/// Provider of SSH credential material from `ssh-agent` (`SSH_AUTH_SOCK`).
#[derive(Debug, Clone, Default)]
pub struct SshAgentProvider {
    auth_sock_override: Option<PathBuf>,
}

impl SshAgentProvider {
    /// Creates a provider with an optional override for the agent socket path.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(auth_sock_override: Option<PathBuf>) -> Self {
        Self { auth_sock_override }
    }

    /// Returns the `SSH_AUTH_SOCK` path, if the agent is available.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn auth_sock_path(&self) -> Option<&Path> {
        self.auth_sock_override.as_deref()
    }

    fn resolve_auth_sock(&self) -> Option<PathBuf> {
        self.auth_sock_override
            .clone()
            .or_else(resolve_env_auth_sock)
    }
}

impl GitAuthProvider for SshAgentProvider {
    fn id(&self) -> &'static str {
        SSH_AGENT_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Ssh && caps.allow_ssh()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let Some(auth_sock) = self.resolve_auth_sock() else {
            return Ok(None);
        };

        #[cfg(not(target_os = "windows"))]
        if !auth_sock.exists() {
            return Ok(None);
        }

        let principal = ctx.username_hint().map(str::to_owned);
        let auth_sock_display = auth_sock.to_string_lossy().into_owned();
        let fingerprint = format!(
            "ssh-agent:{}:{auth_sock_display}",
            principal.as_deref().unwrap_or("<none>")
        );

        Ok(Some(AuthMaterial::new(
            self.id(),
            AuthMaterialKind::SshKey(SshCredentialSource::Agent),
            principal,
            Some(auth_sock_display),
            fingerprint,
        )))
    }
}

fn resolve_env_auth_sock() -> Option<PathBuf> {
    std::env::var_os("SSH_AUTH_SOCK").map(PathBuf::from)
}


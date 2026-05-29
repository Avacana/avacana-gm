use super::ssh_config::SshConfigProvider;
use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthMaterialKind, AuthResult, AuthTransport,
    GitAuthProvider,
};

const USERNAME_HINT_PROVIDER_ID: &str = "ssh-username-hint";

/// Provider of the username hint for the SSH auth callback (`Cred::username`).
#[derive(Debug, Clone)]
pub struct UsernameHintProvider {
    config_provider: SshConfigProvider,
}

impl UsernameHintProvider {
    /// Creates a provider with the given `SshConfigProvider`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(config_provider: SshConfigProvider) -> Self {
        Self { config_provider }
    }

    /// Returns the username hint from the context or `.ssh/config`.
    ///
    /// # Errors
    /// Returns a typed error if parsing/resolving `.ssh/config`
    /// fails.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve_username(&self, ctx: &AuthContext) -> AuthResult<Option<String>> {
        if let Some(username_hint) = ctx.username_hint() {
            return Ok(Some(username_hint.to_string()));
        }

        let resolved = self.config_provider.resolve(ctx)?;
        Ok(resolved.user().map(str::to_owned))
    }
}

impl Default for UsernameHintProvider {
    fn default() -> Self {
        Self::new(SshConfigProvider::default())
    }
}

impl GitAuthProvider for UsernameHintProvider {
    fn id(&self) -> &'static str {
        USERNAME_HINT_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Ssh && caps.allow_ssh()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let Some(username_hint) = self.resolve_username(ctx)? else {
            return Ok(None);
        };

        Ok(Some(AuthMaterial::without_secret(
            self.id(),
            AuthMaterialKind::UsernameOnly,
            Some(username_hint.clone()),
            format!("ssh-username:{username_hint}"),
        )))
    }
}


use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthMaterialKind, AuthResult, AuthTransport,
    GitAuthProvider,
};

const URL_PROVIDER_ID: &str = "url";

/// HTTPS provider for URL hints (`username@host`), used as the first step of the fallback chain.
#[derive(Debug, Clone, Copy, Default)]
pub struct UrlCredentialProvider;

impl UrlCredentialProvider {
    /// Creates a URL credential provider.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl GitAuthProvider for UrlCredentialProvider {
    fn id(&self) -> &'static str {
        URL_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https && caps.allow_https()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let Some(username_hint) = ctx.username_hint() else {
            return Ok(None);
        };

        Ok(Some(AuthMaterial::without_secret(
            self.id(),
            AuthMaterialKind::UsernameOnly,
            Some(username_hint.to_string()),
            format!("url-username:{username_hint}"),
        )))
    }
}


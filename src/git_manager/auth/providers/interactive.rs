use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthError, AuthErrorCode, AuthMaterial, AuthMaterialKind,
    AuthResult, AuthTransport, GitAuthProvider,
};
use std::fmt;

const INTERACTIVE_PROVIDER_ID: &str = "interactive";

/// HTTPS provider for opt-in interactive credential entry.
///
/// In the current iteration the provider uses an env-backed callback contract,
/// which allows hermetic testing without a subprocess.
#[derive(Clone, PartialEq, Eq)]
pub struct InteractiveCallbackProvider {
    opt_in_enabled: bool,
    username: Option<String>,
    password: Option<String>,
}

impl InteractiveCallbackProvider {
    /// Creates an interactive provider with explicit callback data configuration.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(opt_in_enabled: bool, username: Option<String>, password: Option<String>) -> Self {
        Self {
            opt_in_enabled,
            username,
            password,
        }
    }

    /// Creates a provider that reads opt-in settings from the environment.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_environment_opt_in() -> Self {
        Self::new(
            parse_bool_env("AVACANA_GM_GIT_AUTH_INTERACTIVE_OPT_IN").unwrap_or(false),
            read_non_empty_env("AVACANA_GM_GIT_AUTH_INTERACTIVE_USERNAME"),
            read_non_empty_env("AVACANA_GM_GIT_AUTH_INTERACTIVE_PASSWORD"),
        )
    }

    /// Returns `true` if the interactive provider is enabled by an explicit opt-in flag.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn opt_in_enabled(&self) -> bool {
        self.opt_in_enabled
    }
}

impl Default for InteractiveCallbackProvider {
    fn default() -> Self {
        Self::with_environment_opt_in()
    }
}

impl fmt::Debug for InteractiveCallbackProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InteractiveCallbackProvider")
            .field("opt_in_enabled", &self.opt_in_enabled)
            .field("username", &self.username.as_deref().map(redact_identity))
            .field(
                "password",
                &self
                    .password
                    .as_ref()
                    .map_or("<none>", |_| "<redacted-secret>"),
            )
            .finish()
    }
}

impl GitAuthProvider for InteractiveCallbackProvider {
    fn id(&self) -> &'static str {
        INTERACTIVE_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https
            && caps.allow_https()
            && caps.allow_interactive()
            && self.opt_in_enabled
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        if !self.opt_in_enabled {
            return Ok(None);
        }

        let username = ctx
            .username_hint()
            .map(str::to_string)
            .or_else(|| self.username.clone())
            .ok_or_else(|| {
                AuthError::new(
                    AuthErrorCode::NoCredentials,
                    "interactive auth is enabled but username input is unavailable",
                )
            })?;

        let password = self.password.as_ref().ok_or_else(|| {
            AuthError::new(
                AuthErrorCode::NoCredentials,
                "interactive auth is enabled but password input is unavailable",
            )
        })?;

        Ok(Some(AuthMaterial::new(
            self.id(),
            AuthMaterialKind::UsernamePassword,
            Some(username.clone()),
            Some(password.clone()),
            format!("interactive:{}:{username}", ctx.remote_host()),
        )))
    }
}

fn parse_bool_env(name: &str) -> Option<bool> {
    let value = read_non_empty_env(name)?;
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn read_non_empty_env(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn redact_identity(identity: &str) -> String {
    let mut chars = identity.chars();
    let Some(first_char) = chars.next() else {
        return "<empty>".to_string();
    };
    format!("{first_char}***")
}


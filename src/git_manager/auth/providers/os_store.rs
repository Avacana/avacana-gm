use super::git_config::GitConfigCredentialProvider;
use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthResult, AuthTransport, CredentialLookupKey,
    GitAuthProvider, HttpsAuthMaterial,
};
use std::fmt;

const OS_STORE_PROVIDER_ID: &str = "os-store";

/// A credential entry from the OS secret store abstraction.
#[derive(Clone, PartialEq, Eq)]
pub struct OsStoreEntry {
    host: String,
    port: Option<u16>,
    path_prefix: Option<String>,
    username: String,
    password: String,
}

impl OsStoreEntry {
    /// Creates an OS store entry (`host + username + password`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: None,
            path_prefix: None,
            username: username.into(),
            password: password.into(),
        }
    }

    /// Returns the entry constrained to a specific port.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Returns the entry constrained to a path prefix.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_path_prefix(mut self, path_prefix: impl Into<String>) -> Self {
        let path_prefix = path_prefix.into();
        self.path_prefix = normalize_path_prefix(path_prefix.as_str());
        self
    }

    fn matches_lookup_key(&self, lookup_key: &CredentialLookupKey) -> bool {
        if !self.host.eq_ignore_ascii_case(lookup_key.host()) {
            return false;
        }

        if let Some(port) = self.port {
            if lookup_key.port() != Some(port) {
                return false;
            }
        }

        if let Some(path_prefix) = self.path_prefix.as_deref() {
            let Some(lookup_path) = lookup_key.path() else {
                return false;
            };
            if !lookup_path.starts_with(path_prefix) {
                return false;
            }
        }

        lookup_key
            .username_hint()
            .is_none_or(|username_hint| username_hint == self.username)
    }

    fn fingerprint(&self) -> String {
        format!("os-store:{}:{}", self.host, self.username)
    }
}

impl fmt::Debug for OsStoreEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OsStoreEntry")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path_prefix", &self.path_prefix)
            .field("username", &redact_identity(&self.username))
            .field("password", &"<redacted-secret>")
            .finish()
    }
}

/// HTTPS credential provider backed by the OS secret store abstraction.
#[derive(Debug, Clone)]
pub struct OsSecretStoreProvider {
    git_config_provider: GitConfigCredentialProvider,
    available: bool,
    entries: Vec<OsStoreEntry>,
}

impl OsSecretStoreProvider {
    /// Creates an OS store provider with an explicit set of entries.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        git_config_provider: GitConfigCredentialProvider,
        available: bool,
        entries: Vec<OsStoreEntry>,
    ) -> Self {
        Self {
            git_config_provider,
            available,
            entries,
        }
    }

    /// Creates an OS store provider using an env-backed value for a testable fallback.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_environment(
        git_config_provider: GitConfigCredentialProvider,
        available: bool,
    ) -> Self {
        Self::new(
            git_config_provider,
            available,
            load_entries_from_environment(),
        )
    }

    /// Returns `true` if the OS store path is enabled in the current environment.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn available(&self) -> bool {
        self.available
    }
}

impl GitAuthProvider for OsSecretStoreProvider {
    fn id(&self) -> &'static str {
        OS_STORE_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https && caps.allow_https() && self.available
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        if !self.available {
            tracing::trace!(
                provider = self.id(),
                "os store provider is disabled by environment policy"
            );
            return Ok(None);
        }

        let lookup_key = self.git_config_provider.resolve_lookup_key(ctx)?;
        let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.matches_lookup_key(&lookup_key))
        else {
            tracing::trace!(
                provider = self.id(),
                lookup = %lookup_key.redacted_target(),
                "os store entry not found for lookup key"
            );
            return Ok(None);
        };

        let https_material = HttpsAuthMaterial::new(
            entry.username.clone(),
            entry.password.clone(),
            entry.fingerprint(),
        );
        tracing::trace!(
            provider = self.id(),
            lookup = %lookup_key.redacted_target(),
            selected_username = %redact_identity(&entry.username),
            material = %https_material.redacted_label(),
            "os store entry selected for HTTPS auth"
        );

        Ok(Some(https_material.into_auth_material(self.id())))
    }
}

fn load_entries_from_environment() -> Vec<OsStoreEntry> {
    let Some(host) = read_non_empty_env("AVACANA_GM_GIT_AUTH_OS_STORE_HOST") else {
        return Vec::new();
    };
    let Some(username) = read_non_empty_env("AVACANA_GM_GIT_AUTH_OS_STORE_USERNAME") else {
        return Vec::new();
    };
    let Some(password) = read_non_empty_env("AVACANA_GM_GIT_AUTH_OS_STORE_PASSWORD") else {
        return Vec::new();
    };

    let mut entry = OsStoreEntry::new(host, username, password);
    if let Some(raw_port) = read_non_empty_env("AVACANA_GM_GIT_AUTH_OS_STORE_PORT") {
        if let Ok(port) = raw_port.parse::<u16>() {
            entry = entry.with_port(port);
        }
    }
    if let Some(path_prefix) = read_non_empty_env("AVACANA_GM_GIT_AUTH_OS_STORE_PATH_PREFIX") {
        entry = entry.with_path_prefix(path_prefix);
    }

    vec![entry]
}

fn read_non_empty_env(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalize_path_prefix(path_prefix: &str) -> Option<String> {
    let normalized = path_prefix.trim().trim_start_matches('/').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

fn redact_identity(identity: &str) -> String {
    let mut chars = identity.chars();
    let Some(first_char) = chars.next() else {
        return "<empty>".to_string();
    };
    format!("{first_char}***")
}


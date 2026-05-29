use std::fmt;

use super::auth_material::AuthMaterial;
use super::redaction::{redact_identity, shorten_fingerprint};
use super::types::AuthMaterialKind;

/// HTTPS credential material prior to conversion into the generic `AuthMaterial`.
#[derive(Clone, PartialEq, Eq)]
pub struct HttpsAuthMaterial {
    username: String,
    password: String,
    fingerprint: String,
}

impl HttpsAuthMaterial {
    /// Creates HTTPS credential material (`username/password`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            fingerprint: fingerprint.into(),
        }
    }

    /// Returns the username.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the secret (password/token).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    /// Returns the material fingerprint for invalidation policy.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Converts the HTTPS material into the generic `AuthMaterial`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn into_auth_material(self, provider_id: &'static str) -> AuthMaterial {
        AuthMaterial::new(
            provider_id,
            AuthMaterialKind::UsernamePassword,
            Some(self.username),
            Some(self.password),
            self.fingerprint,
        )
    }

    /// Returns a redaction-safe label for the HTTPS credential.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn redacted_label(&self) -> String {
        format!(
            "username={}, fp={}",
            redact_identity(&self.username),
            shorten_fingerprint(&self.fingerprint)
        )
    }
}

impl fmt::Debug for HttpsAuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpsAuthMaterial")
            .field("username", &redact_identity(&self.username))
            .field("password", &"<redacted-secret>")
            .field("fingerprint", &shorten_fingerprint(&self.fingerprint))
            .finish()
    }
}

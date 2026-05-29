use std::fmt;

use super::redaction::{redact_identity, shorten_fingerprint};
use super::types::{AuthMaterialKind, SshCredentialSource};

/// Credential material suitable for passing to a transport callback.
///
/// The secret is held inside the struct and never appears in the `Debug`/trace representation.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthMaterial {
    provider_id: &'static str,
    kind: AuthMaterialKind,
    principal: Option<String>,
    secret: Option<String>,
    fingerprint: String,
}

impl AuthMaterial {
    /// Creates new auth material.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        provider_id: &'static str,
        kind: AuthMaterialKind,
        principal: Option<String>,
        secret: Option<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            provider_id,
            kind,
            principal,
            secret,
            fingerprint: fingerprint.into(),
        }
    }

    /// Creates auth material without a secret.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn without_secret(
        provider_id: &'static str,
        kind: AuthMaterialKind,
        principal: Option<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self::new(provider_id, kind, principal, None, fingerprint)
    }

    /// Returns the identifier of the source provider.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn provider_id(&self) -> &'static str {
        self.provider_id
    }

    /// Returns the kind of auth material.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn kind(&self) -> AuthMaterialKind {
        self.kind
    }

    /// Returns the machine-readable code of the typed credential source.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn credential_source_kind(&self) -> &'static str {
        self.kind.credential_source_kind()
    }

    /// Returns the typed SSH source if the material is an SSH credential.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn ssh_credential_source(&self) -> Option<SshCredentialSource> {
        self.kind.ssh_credential_source()
    }

    /// Returns the principal (username/login), if present.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn principal(&self) -> Option<&str> {
        self.principal.as_deref()
    }

    /// Returns the secret payload, if present.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    /// Returns the material fingerprint for budget/invalidation policy.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Returns a redaction-safe string for trace logs.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn redacted_label(&self) -> String {
        let principal = self
            .principal
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);

        format!(
            "provider={}, kind={}, credential_source_kind={}, principal={}, fp={}",
            self.provider_id,
            self.kind,
            self.credential_source_kind(),
            principal,
            shorten_fingerprint(&self.fingerprint)
        )
    }
}

impl fmt::Debug for AuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let principal = self
            .principal
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);

        f.debug_struct("AuthMaterial")
            .field("provider_id", &self.provider_id)
            .field("kind", &self.kind)
            .field("credential_source_kind", &self.credential_source_kind())
            .field("principal", &principal)
            .field(
                "secret",
                &self
                    .secret
                    .as_ref()
                    .map_or("<none>", |_| "<redacted-secret>"),
            )
            .field("fingerprint", &shorten_fingerprint(&self.fingerprint))
            .finish()
    }
}

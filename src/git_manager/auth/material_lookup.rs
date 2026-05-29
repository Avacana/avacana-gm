use crate::git_manager::auth::AuthContext;
use std::fmt;

use super::redaction::{normalize_lookup_path, push_unique, redact_identity};

/// Lookup key for finding HTTPS credentials in sources (`git-config`, `.netrc`).
///
/// The key holds only non-secret fields and is suitable for trace logging
/// in redacted form.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialLookupKey {
    host: String,
    port: Option<u16>,
    path: Option<String>,
    username_hint: Option<String>,
    use_http_path: bool,
}

impl CredentialLookupKey {
    /// Creates a lookup key with a required host.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: None,
            path: None,
            username_hint: None,
            use_http_path: false,
        }
    }

    /// Creates a lookup key from the current auth context.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn from_context(ctx: &AuthContext) -> Self {
        let mut lookup_key = Self::new(ctx.remote_host());
        if let Some(port) = ctx.remote_port() {
            lookup_key = lookup_key.with_port(port);
        }
        if let Some(path) = ctx.remote_path() {
            lookup_key = lookup_key.with_path(path);
        }
        if let Some(username_hint) = ctx.username_hint() {
            lookup_key = lookup_key.with_username_hint(username_hint);
        }
        lookup_key
    }

    /// Returns a copy of the lookup key with the given remote port.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Returns a copy of the lookup key with the given remote path.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        self.path = normalize_lookup_path(&path);
        self
    }

    /// Returns a copy of the lookup key with a username hint.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_username_hint(mut self, username_hint: impl Into<String>) -> Self {
        self.username_hint = Some(username_hint.into());
        self
    }

    /// Returns a copy of the lookup key with the `credential.useHttpPath` flag.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_use_http_path(mut self, use_http_path: bool) -> Self {
        self.use_http_path = use_http_path;
        self
    }

    /// Returns the remote host.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the remote port, if set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the remote path, if available.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the username hint, if available.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn username_hint(&self) -> Option<&str> {
        self.username_hint.as_deref()
    }

    /// Returns the value of the `credential.useHttpPath` flag.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn use_http_path(&self) -> bool {
        self.use_http_path
    }

    /// Returns the set of `machine` candidates for lookup in `.netrc`.
    ///
    /// Order: from most specific to most general.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn machine_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();

        if self.use_http_path {
            if let Some(path) = self.path.as_deref() {
                if let Some(port) = self.port {
                    push_unique(&mut candidates, format!("{}:{port}/{path}", self.host));
                }
                push_unique(&mut candidates, format!("{}/{path}", self.host));
            }
        }

        if let Some(port) = self.port {
            push_unique(&mut candidates, format!("{}:{port}", self.host));
        }
        push_unique(&mut candidates, self.host.clone());

        candidates
    }

    /// Returns a redaction-safe representation of the lookup key for trace logs.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn redacted_target(&self) -> String {
        let mut target = self.host.clone();
        if let Some(port) = self.port {
            target.push(':');
            target.push_str(&port.to_string());
        }
        if self.path.is_some() {
            target.push_str("/…");
        }

        let username = self
            .username_hint
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);
        format!("target={target}, user={username}")
    }
}

impl fmt::Debug for CredentialLookupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let username = self
            .username_hint
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);
        let path = self.path.as_ref().map_or("<none>", |_| "…");

        f.debug_struct("CredentialLookupKey")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("path", &path)
            .field("username_hint", &username)
            .field("use_http_path", &self.use_http_path)
            .finish()
    }
}

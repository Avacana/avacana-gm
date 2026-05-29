use std::fmt;

/// Transport protocol for which authentication is performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthTransport {
    /// SSH transport (`ssh://`, `git@host:path`).
    Ssh,
    /// HTTPS transport.
    Https,
}

impl AuthTransport {
    /// Returns the stable machine-readable protocol identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Https => "https",
        }
    }
}

impl fmt::Display for AuthTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Context for a single auth operation.
///
/// Holds only non-secret fields and is passed to the `GitAuth` providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    operation: String,
    transport: AuthTransport,
    remote_host: String,
    remote_port: Option<u16>,
    remote_path: Option<String>,
    username_hint: Option<String>,
}

impl AuthContext {
    /// Creates a new auth-operation context.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        operation: impl Into<String>,
        transport: AuthTransport,
        remote_host: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            transport,
            remote_host: remote_host.into(),
            remote_port: None,
            remote_path: None,
            username_hint: None,
        }
    }

    /// Returns a copy of the context with the given remote port.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_remote_port(mut self, remote_port: u16) -> Self {
        self.remote_port = Some(remote_port);
        self
    }

    /// Returns a copy of the context with the given remote path.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_remote_path(mut self, remote_path: impl Into<String>) -> Self {
        self.remote_path = Some(remote_path.into());
        self
    }

    /// Returns a copy of the context with a username hint.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_username_hint(mut self, username_hint: impl Into<String>) -> Self {
        self.username_hint = Some(username_hint.into());
        self
    }

    /// Returns the name of the operation that initiated the auth flow.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the remote transport protocol.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn transport(&self) -> AuthTransport {
        self.transport
    }

    /// Returns the remote host.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn remote_host(&self) -> &str {
        &self.remote_host
    }

    /// Returns the remote port, if explicitly set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    /// Returns the remote path, if explicitly set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn remote_path(&self) -> Option<&str> {
        self.remote_path.as_deref()
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

    /// Builds a redaction-safe representation of the remote target.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn redacted_target(&self) -> String {
        let mut target = format!("{}://{}", self.transport, self.remote_host);
        if let Some(port) = self.remote_port {
            target.push(':');
            target.push_str(&port.to_string());
        }
        if self.remote_path.is_some() {
            target.push_str("/…");
        }
        target
    }
}


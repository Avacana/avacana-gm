use std::fmt;

use super::redaction::redact_identity;

/// Credential entry from `.netrc`/`_netrc`.
#[derive(Clone, PartialEq, Eq)]
pub struct NetrcEntry {
    machine: Option<String>,
    login: Option<String>,
    password: Option<String>,
}

impl NetrcEntry {
    /// Creates an entry for a specific `machine`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_machine(
        machine: impl Into<String>,
        login: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            machine: Some(machine.into()),
            login,
            password,
        }
    }

    /// Creates a `default` `.netrc` entry.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_default(login: Option<String>, password: Option<String>) -> Self {
        Self {
            machine: None,
            login,
            password,
        }
    }

    /// Returns the `machine` if the entry is not a `default` entry.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn machine(&self) -> Option<&str> {
        self.machine.as_deref()
    }

    /// Returns `true` if the entry is a `default` entry.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.machine.is_none()
    }

    /// Returns the login from `.netrc`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn login(&self) -> Option<&str> {
        self.login.as_deref()
    }

    /// Returns the password/token from `.netrc`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns a redaction-safe label for the entry for trace logs.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn redacted_label(&self) -> String {
        let machine = self
            .machine
            .as_deref()
            .map_or_else(|| "<default>".to_string(), redact_identity);
        let login = self
            .login
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);

        format!("machine={machine}, login={login}")
    }
}

impl fmt::Debug for NetrcEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let machine = self.machine.as_deref().unwrap_or("<default>");
        let login = self
            .login
            .as_deref()
            .map_or_else(|| "<none>".to_string(), redact_identity);

        f.debug_struct("NetrcEntry")
            .field("machine", &machine)
            .field("login", &login)
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

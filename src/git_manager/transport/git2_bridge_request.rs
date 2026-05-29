use crate::git_manager::auth::{AuthAttemptBudget, AuthContext};

/// Parameters for running a single remote operation through the transport bridge.
#[derive(Debug, Clone)]
pub struct TransportRequest {
    operation: String,
    auth_context: AuthContext,
    retry_limit_override: Option<usize>,
    auth_attempt_budget: AuthAttemptBudget,
}

impl TransportRequest {
    /// Creates a transport request for the given operation and auth context.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(operation: impl Into<String>, auth_context: AuthContext) -> Self {
        Self {
            operation: operation.into(),
            auth_context,
            retry_limit_override: None,
            auth_attempt_budget: AuthAttemptBudget::default(),
        }
    }

    /// Creates a transport request for `clone`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_clone(auth_context: AuthContext) -> Self {
        Self::new("clone", auth_context)
    }

    /// Creates a transport request for `fetch`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_fetch(auth_context: AuthContext) -> Self {
        Self::new("fetch", auth_context)
    }

    /// Creates a transport request for `pull`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_pull(auth_context: AuthContext) -> Self {
        Self::new("pull", auth_context)
    }

    /// Creates a transport request for `push`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_push(auth_context: AuthContext) -> Self {
        Self::new("push", auth_context)
    }

    /// Creates a transport request for `ls-remote`.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn for_ls_remote(auth_context: AuthContext) -> Self {
        Self::new("ls-remote", auth_context)
    }

    /// Overrides the retry limit for the current request.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_retry_limit(mut self, retry_limit: usize) -> Self {
        self.retry_limit_override = Some(retry_limit);
        self
    }

    /// Overrides the `AuthAttemptBudget` for the current transport request.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_auth_attempt_budget(mut self, auth_attempt_budget: AuthAttemptBudget) -> Self {
        self.auth_attempt_budget = auth_attempt_budget;
        self
    }

    /// Returns the operation name (`clone/fetch/pull/push/ls-remote`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the auth context of the transport request.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn auth_context(&self) -> &AuthContext {
        &self.auth_context
    }

    /// Returns the retry limit override, if one is set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn retry_limit_override(&self) -> Option<usize> {
        self.retry_limit_override
    }

    /// Returns the auth attempt budget for the transport request.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn auth_attempt_budget(&self) -> &AuthAttemptBudget {
        &self.auth_attempt_budget
    }
}

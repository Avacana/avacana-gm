use super::{
    AuthAttemptBudget, AuthAttemptBudgetRejection, AuthAttemptOutcome, AuthCapabilities,
    AuthContext, AuthError, AuthErrorCode, AuthMaterial, AuthResult, GitAuthProvider,
};
use std::fmt;

/// Executor for the auth provider chain with budget/deadline policies.
pub struct AuthChain {
    capabilities: AuthCapabilities,
    providers: Vec<Box<dyn GitAuthProvider>>,
}

impl AuthChain {
    /// Creates an empty auth chain for the given capabilities.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(capabilities: AuthCapabilities) -> Self {
        Self {
            capabilities,
            providers: Vec::new(),
        }
    }

    /// Creates an auth chain with a predefined list of providers.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_providers(
        capabilities: AuthCapabilities,
        providers: Vec<Box<dyn GitAuthProvider>>,
    ) -> Self {
        Self {
            capabilities,
            providers,
        }
    }

    /// Appends a provider to the end of the fallback chain.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn push_provider(&mut self, provider: Box<dyn GitAuthProvider>) {
        self.providers.push(provider);
    }

    /// Returns the capabilities of the current auth chain.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn capabilities(&self) -> &AuthCapabilities {
        &self.capabilities
    }

    /// Returns the number of providers in the chain.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Returns `true` if the chain contains no providers.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Runs the auth flow: provider chain -> invalidate -> retry -> timeout cut-off.
    ///
    /// `attempt_auth` is the transport-level integration callback; it must not log secrets.
    ///
    /// # Errors
    /// Returns a typed `AuthError` with a machine-readable code:
    /// `AUTH_NO_CREDENTIALS`, `AUTH_DENIED`, or `AUTH_TIMEOUT`.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(operation = %ctx.operation(), target = %ctx.redacted_target())
        )
    )]
    pub fn authenticate<F>(
        &self,
        ctx: &AuthContext,
        budget: &mut AuthAttemptBudget,
        mut attempt_auth: F,
    ) -> AuthResult<AuthMaterial>
    where
        F: FnMut(&AuthMaterial) -> AuthAttemptOutcome,
    {
        self.validate_preconditions(ctx)?;
        let mut flow_state = AuthFlowState::default();
        loop {
            ensure_not_timed_out(
                budget,
                "auth deadline reached before completing provider chain",
            )?;

            let mut pass_progress = false;
            for provider in &self.providers {
                ensure_not_timed_out(
                    budget,
                    "auth deadline reached before completing provider chain",
                )?;

                match self.process_provider(
                    provider.as_ref(),
                    ctx,
                    budget,
                    &mut flow_state,
                    &mut attempt_auth,
                )? {
                    ProviderProcessOutcome::Skipped => {}
                    ProviderProcessOutcome::Attempted => {
                        pass_progress = true;
                    }
                    ProviderProcessOutcome::Authenticated(material) => {
                        ensure_not_timed_out(budget, "auth deadline reached after auth attempt")?;
                        return Ok(material);
                    }
                }
            }
            if !pass_progress {
                break;
            }
        }

        finalize_auth_result(budget, flow_state)
    }

    fn validate_preconditions(&self, ctx: &AuthContext) -> AuthResult<()> {
        if self.providers.is_empty() {
            return Err(no_credentials_error(
                "auth provider chain is empty; no credentials can be resolved",
            ));
        }
        if !self.capabilities.supports_transport(ctx.transport()) {
            return Err(no_credentials_error(
                "requested transport is disabled by auth capabilities",
            ));
        }
        Ok(())
    }

    fn process_provider<F>(
        &self,
        provider: &dyn GitAuthProvider,
        ctx: &AuthContext,
        budget: &mut AuthAttemptBudget,
        flow_state: &mut AuthFlowState,
        attempt_auth: &mut F,
    ) -> AuthResult<ProviderProcessOutcome>
    where
        F: FnMut(&AuthMaterial) -> AuthAttemptOutcome,
    {
        if !provider.supports(ctx, &self.capabilities) {
            tracing::trace!(
                provider = provider.id(),
                "auth provider skipped: not supported by context/capabilities"
            );
            return Ok(ProviderProcessOutcome::Skipped);
        }

        let Some(material) = provider.load(ctx)? else {
            tracing::trace!(
                provider = provider.id(),
                "auth provider returned no material"
            );
            return Ok(ProviderProcessOutcome::Skipped);
        };

        if !reserve_attempt_budget(
            provider.id(),
            &material,
            budget,
            flow_state.has_denied_attempt,
        )? {
            return Ok(ProviderProcessOutcome::Skipped);
        }
        flow_state.has_any_attempt = true;
        let material_label = material.redacted_label();
        match attempt_auth(&material) {
            AuthAttemptOutcome::Authenticated => {
                tracing::trace!(
                    provider = provider.id(),
                    material = %material_label,
                    "auth material accepted"
                );
                Ok(ProviderProcessOutcome::Authenticated(material))
            }
            AuthAttemptOutcome::Denied => {
                tracing::trace!(
                    provider = provider.id(),
                    material = %material_label,
                    "auth material denied; marking invalid"
                );
                flow_state.has_denied_attempt = true;
                budget.invalidate(material.fingerprint());
                provider.invalidate(ctx, &material)?;
                Ok(ProviderProcessOutcome::Attempted)
            }
            AuthAttemptOutcome::RetryableError => {
                tracing::trace!(
                    provider = provider.id(),
                    material = %material_label,
                    "auth attempt returned retryable outcome"
                );
                Ok(ProviderProcessOutcome::Attempted)
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AuthFlowState {
    has_denied_attempt: bool,
    has_any_attempt: bool,
}

enum ProviderProcessOutcome {
    Skipped,
    Attempted,
    Authenticated(AuthMaterial),
}

impl fmt::Debug for AuthChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let provider_ids: Vec<&'static str> = self
            .providers
            .iter()
            .map(|provider| provider.id())
            .collect();
        f.debug_struct("AuthChain")
            .field("capabilities", &self.capabilities)
            .field("providers", &provider_ids)
            .finish()
    }
}

fn ensure_not_timed_out(budget: &AuthAttemptBudget, message: &'static str) -> AuthResult<()> {
    if budget.has_timed_out_now() {
        return Err(timeout_error(message));
    }
    Ok(())
}

fn reserve_attempt_budget(
    provider_id: &'static str,
    material: &AuthMaterial,
    budget: &mut AuthAttemptBudget,
    has_denied_attempt: bool,
) -> AuthResult<bool> {
    let material_label = material.redacted_label();
    match budget.try_start_attempt_now(material.fingerprint()) {
        Ok(()) => Ok(true),
        Err(AuthAttemptBudgetRejection::DeadlineExceeded) => {
            tracing::trace!(
                provider = provider_id,
                material = %material_label,
                "auth deadline exceeded before attempt"
            );
            Err(timeout_error("auth deadline exceeded"))
        }
        Err(AuthAttemptBudgetRejection::TotalAttemptsExhausted) => {
            tracing::trace!(
                provider = provider_id,
                material = %material_label,
                "auth attempt budget exhausted"
            );
            if has_denied_attempt {
                return Err(denied_error(
                    "auth denied and attempt budget exhausted for current operation",
                ));
            }
            Err(no_credentials_error(
                "auth attempt budget exhausted before obtaining usable credentials",
            ))
        }
        Err(
            AuthAttemptBudgetRejection::MaterialAttemptsExhausted
            | AuthAttemptBudgetRejection::MaterialInvalidated,
        ) => {
            tracing::trace!(
                provider = provider_id,
                material = %material_label,
                "auth material skipped by budget policy"
            );
            Ok(false)
        }
    }
}

fn finalize_auth_result(
    budget: &AuthAttemptBudget,
    flow_state: AuthFlowState,
) -> AuthResult<AuthMaterial> {
    if budget.has_timed_out_now() {
        return Err(timeout_error(
            "auth deadline reached before obtaining successful credentials",
        ));
    }

    if flow_state.has_denied_attempt {
        return Err(denied_error(
            "all attempted credentials were denied for current operation",
        ));
    }

    if flow_state.has_any_attempt {
        return Err(no_credentials_error(
            "auth materials were exhausted without successful authorization",
        ));
    }
    Err(no_credentials_error(
        "no usable credentials found in auth provider chain",
    ))
}

fn no_credentials_error(message: &'static str) -> AuthError {
    AuthError::new(AuthErrorCode::NoCredentials, message)
}

fn denied_error(message: &'static str) -> AuthError {
    AuthError::new(AuthErrorCode::Denied, message)
}

fn timeout_error(message: &'static str) -> AuthError {
    AuthError::new(AuthErrorCode::Timeout, message)
}

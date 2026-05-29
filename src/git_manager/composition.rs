//! Typed composition boundary for `GitManager`.

use crate::git_manager::auth::{
    build_default_auth_chain_for_environment, AuthCapabilities, AuthChain, AuthEnvironmentMode,
    AuthEnvironmentProfile,
};
pub use crate::git_manager::core::RepositoryAccess;
use crate::git_manager::state::GitLockManager;
use crate::git_manager::transport::{
    Git2TransportBridge, HostKeyPolicy, KnownHostsVerifier, TransportRetryPolicy,
};
use std::sync::Arc;

/// Typed collaborator for the transport assembly boundary.
///
/// In `T-090.03` the factory enforces that the transport bridge is assembled only at the
/// composition layer, while leaf operations receive an already-built collaborator from above.
#[derive(Debug, Clone)]
pub struct GitTransportFactory {
    bridge: Git2TransportBridge,
    kind: &'static str,
}

impl GitTransportFactory {
    /// Creates an explicit transport factory for test/bootstrap wiring.
    ///
    /// This constructor is not the production default path: canonical production assembly goes
    /// through `GitManagerComponents::production()`, where the transport bridge is wired to an
    /// explicit auth provider factory.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "explicit_transport_factory",
                transport_factory_kind = "explicit_transport_bridge",
                credential_source_kind = "explicit_empty_chain"
            )
        )
    )]
    #[must_use]
    pub fn new() -> Self {
        Self::from_bridge(
            Git2TransportBridge::with_defaults(Arc::new(AuthChain::new(
                AuthCapabilities::default(),
            ))),
            "explicit_transport_bridge",
        )
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "environment_transport_factory",
                transport_factory_kind = "git2_transport_bridge",
                credential_source_kind = %auth_provider_factory.credential_source_kind()
            )
        )
    )]
    fn for_auth_provider_factory(auth_provider_factory: &GitAuthProviderFactory) -> Self {
        let host_key_policy = if auth_provider_factory
            .environment_profile()
            .accept_new_host()
        {
            HostKeyPolicy::accept_new_host()
        } else {
            HostKeyPolicy::strict()
        };

        Self::from_bridge(
            Git2TransportBridge::new(
                auth_provider_factory.auth_chain(),
                KnownHostsVerifier::with_default_path(host_key_policy),
                TransportRetryPolicy::default(),
            ),
            "git2_transport_bridge",
        )
    }

    const fn from_bridge(bridge: Git2TransportBridge, kind: &'static str) -> Self {
        Self { bridge, kind }
    }

    pub(crate) fn bridge(&self) -> Git2TransportBridge {
        self.bridge.clone()
    }

    pub(crate) const fn kind(&self) -> &'static str {
        self.kind
    }
}

impl Default for GitTransportFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed collaborator for the auth provider assembly boundary.
///
/// The type remains a separate dependency boundary without eliminating `Box<dyn GitAuthProvider>`,
/// while still holding the already-assembled explicit provider chain for the transport bridge.
#[derive(Debug, Clone)]
pub struct GitAuthProviderFactory {
    auth_chain: Arc<AuthChain>,
    environment_profile: AuthEnvironmentProfile,
    credential_source_kind: &'static str,
}

impl GitAuthProviderFactory {
    /// Creates an explicit auth provider factory for test/bootstrap wiring.
    ///
    /// The empty chain is used only as a deliberate explicit collaborator when assembling test
    /// fixtures by hand; the production path uses `GitManagerComponents::production()`.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "explicit_auth_provider_factory",
                credential_source_kind = "explicit_empty_chain"
            )
        )
    )]
    #[must_use]
    pub fn new() -> Self {
        Self::from_parts(
            Arc::new(AuthChain::new(AuthCapabilities::default())),
            AuthEnvironmentProfile::from_mode(AuthEnvironmentMode::HeadlessCi),
            "explicit_empty_chain",
        )
    }

    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "environment_auth_provider_factory",
                credential_source_kind = "environment_default_provider_chain"
            )
        )
    )]
    fn for_environment() -> Self {
        let (auth_chain, environment_profile) = build_default_auth_chain_for_environment();
        Self::from_parts(
            Arc::new(auth_chain),
            environment_profile,
            "environment_default_provider_chain",
        )
    }

    const fn from_parts(
        auth_chain: Arc<AuthChain>,
        environment_profile: AuthEnvironmentProfile,
        credential_source_kind: &'static str,
    ) -> Self {
        Self {
            auth_chain,
            environment_profile,
            credential_source_kind,
        }
    }

    pub(crate) fn auth_chain(&self) -> Arc<AuthChain> {
        Arc::clone(&self.auth_chain)
    }

    pub(crate) const fn environment_profile(&self) -> &AuthEnvironmentProfile {
        &self.environment_profile
    }

    pub(crate) const fn credential_source_kind(&self) -> &'static str {
        self.credential_source_kind
    }
}

impl Default for GitAuthProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Explicit typed bundle of production collaborators for `GitManagerFacade`.
///
/// The canonical production construction path goes through the owning composition root, which
/// assembles this bundle at the top and passes it to `GitManagerFacade::new(...)`.
#[derive(Debug, Clone)]
pub struct GitManagerComponents {
    lock_manager: Arc<GitLockManager>,
    repository_access: Arc<RepositoryAccess>,
    transport_factory: Arc<GitTransportFactory>,
    auth_provider_factory: Arc<GitAuthProviderFactory>,
}

impl GitManagerComponents {
    /// Creates a typed bundle for canonical production wiring of `GitManagerFacade`.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "explicit_bundle",
                transport_factory_kind = %transport_factory.kind(),
                credential_source_kind = %auth_provider_factory.credential_source_kind(),
                legacy_default_ctor_used = false
            )
        )
    )]
    #[must_use]
    pub fn new(
        lock_manager: Arc<GitLockManager>,
        repository_access: Arc<RepositoryAccess>,
        transport_factory: Arc<GitTransportFactory>,
        auth_provider_factory: Arc<GitAuthProviderFactory>,
    ) -> Self {
        Self {
            lock_manager,
            repository_access,
            transport_factory,
            auth_provider_factory,
        }
    }

    /// Assembles the standard production bundle at the crate's outer composition boundary.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(
            skip_all,
            fields(
                composition_path = "production_components_bundle",
                transport_factory_kind = tracing::field::Empty,
                credential_source_kind = tracing::field::Empty,
                legacy_default_ctor_used = false
            )
        )
    )]
    #[must_use]
    pub fn production() -> Self {
        let auth_provider_factory = Arc::new(GitAuthProviderFactory::for_environment());
        let transport_factory = Arc::new(GitTransportFactory::for_auth_provider_factory(
            auth_provider_factory.as_ref(),
        ));

        tracing::Span::current().record(
            "transport_factory_kind",
            tracing::field::display(transport_factory.kind()),
        );
        tracing::Span::current().record(
            "credential_source_kind",
            tracing::field::display(auth_provider_factory.credential_source_kind()),
        );
        tracing::trace!(
            composition_path = "production_components_bundle",
            transport_factory_kind = transport_factory.kind(),
            credential_source_kind = auth_provider_factory.credential_source_kind(),
            "assembled production auth/transport collaborators on composition boundary"
        );

        Self::new(
            Arc::new(GitLockManager::new()),
            Arc::new(RepositoryAccess::new()),
            transport_factory,
            auth_provider_factory,
        )
    }

    /// Returns the shared lock manager for pipeline/runtime wiring.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn lock_manager(&self) -> Arc<GitLockManager> {
        Arc::clone(&self.lock_manager)
    }

    /// Returns the typed repository foundation collaborator.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn repository_access(&self) -> Arc<RepositoryAccess> {
        Arc::clone(&self.repository_access)
    }

    /// Returns the typed transport assembly boundary collaborator.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn transport_factory(&self) -> Arc<GitTransportFactory> {
        Arc::clone(&self.transport_factory)
    }

    /// Returns the typed auth assembly boundary collaborator.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn auth_provider_factory(&self) -> Arc<GitAuthProviderFactory> {
        Arc::clone(&self.auth_provider_factory)
    }
}

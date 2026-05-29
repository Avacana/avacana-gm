use super::git_config::GitConfigCredentialProvider;
use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthResult, AuthTransport, CredentialLookupKey,
    GitAuthProvider, NetrcEntry,
};
use std::path::PathBuf;

#[path = "netrc_matching.rs"]
mod matching;
#[path = "netrc_parser.rs"]
mod parser;

use matching::{build_https_material, default_netrc_path, select_entry};
use parser::{netrc_parse_error, parse_netrc_file};

const NETRC_PROVIDER_ID: &str = "netrc";

/// HTTPS auth provider that reads credentials from `.netrc`/`_netrc`.
#[derive(Debug, Clone)]
pub struct NetrcProvider {
    git_config_provider: GitConfigCredentialProvider,
    netrc_path_override: Option<PathBuf>,
}

impl NetrcProvider {
    /// Creates a provider with an explicit `git-config` hint provider and a `.netrc` path override.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(
        git_config_provider: GitConfigCredentialProvider,
        netrc_path_override: Option<PathBuf>,
    ) -> Self {
        Self {
            git_config_provider,
            netrc_path_override,
        }
    }

    /// Creates a provider with the default path resolver (`NETRC`, `~/.netrc`, or `_netrc`).
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_path(git_config_provider: GitConfigCredentialProvider) -> Self {
        Self::new(git_config_provider, None)
    }

    /// Returns the effective `.netrc`/`_netrc` path, if one is set.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn resolve_netrc_path(&self) -> Option<PathBuf> {
        self.netrc_path_override.clone().or_else(default_netrc_path)
    }

    /// Resolves the lookup key from URL/git-config hints.
    ///
    /// # Errors
    /// Returns a typed error if reading the git-config hints fails.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve_lookup_key(&self, ctx: &AuthContext) -> AuthResult<CredentialLookupKey> {
        self.git_config_provider.resolve_lookup_key(ctx)
    }

    /// Loads and parses all `.netrc` entries from the selected source.
    ///
    /// # Errors
    /// Returns a typed error if the `.netrc` file is malformed or cannot be read.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    pub fn load_entries(&self) -> AuthResult<Vec<NetrcEntry>> {
        let Some(netrc_path) = self.resolve_netrc_path() else {
            return Ok(Vec::new());
        };

        if !netrc_path.exists() {
            return Ok(Vec::new());
        }

        parse_netrc_file(&netrc_path)
    }
}

impl Default for NetrcProvider {
    fn default() -> Self {
        Self::with_default_path(GitConfigCredentialProvider::default())
    }
}

impl GitAuthProvider for NetrcProvider {
    fn id(&self) -> &'static str {
        NETRC_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Https && caps.allow_https()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let lookup_key = self.resolve_lookup_key(ctx)?;
        let Some(netrc_path) = self.resolve_netrc_path() else {
            return Ok(None);
        };

        if !netrc_path.exists() {
            return Ok(None);
        }

        let entries = parse_netrc_file(&netrc_path)?;
        if entries.is_empty() {
            return Err(netrc_parse_error(
                &netrc_path,
                0,
                "netrc file is empty or does not contain machine/default entries",
            ));
        }

        let Some(entry) = select_entry(&entries, &lookup_key) else {
            tracing::trace!(
                provider = self.id(),
                lookup = %lookup_key.redacted_target(),
                "netrc entry not found for lookup key"
            );
            return Ok(None);
        };

        let https_material = build_https_material(entry, &lookup_key, &netrc_path)?;
        tracing::trace!(
            provider = self.id(),
            lookup = %lookup_key.redacted_target(),
            selected_entry = %entry.redacted_label(),
            material = %https_material.redacted_label(),
            "netrc entry selected for HTTPS auth"
        );
        Ok(Some(https_material.into_auth_material(self.id())))
    }
}

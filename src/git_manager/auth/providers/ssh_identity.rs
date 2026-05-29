use super::ssh_config::SshConfigProvider;
use crate::git_manager::auth::{
    AuthCapabilities, AuthContext, AuthMaterial, AuthMaterialKind, AuthResult, AuthTransport,
    GitAuthProvider, SshCredentialSource,
};
use dirs::home_dir;
use std::path::{Path, PathBuf};

const SSH_IDENTITY_PROVIDER_ID: &str = "ssh-identity-file";

/// The SSH identity source selected by the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshIdentitySource {
    /// Identity taken from an explicit `IdentityFile` in `.ssh/config`.
    ConfigIdentityFile(PathBuf),
    /// Identity taken from the default set (`id_ed25519`, `id_ecdsa`, `id_rsa`).
    DefaultIdentityFile(PathBuf),
}

impl SshIdentitySource {
    /// Returns the path to the identity file.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::ConfigIdentityFile(path) | Self::DefaultIdentityFile(path) => path,
        }
    }

    /// Returns a machine-readable identifier for the source type.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfigIdentityFile(_) => "config_identity_file",
            Self::DefaultIdentityFile(_) => "default_identity_file",
        }
    }
}

/// Provider of SSH identity-file material.
#[derive(Debug, Clone)]
pub struct SshIdentityFileProvider {
    config_provider: SshConfigProvider,
    default_identity_files: Vec<PathBuf>,
}

impl SshIdentityFileProvider {
    /// Creates a provider with an explicit set of default identity paths.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn new(config_provider: SshConfigProvider, default_identity_files: Vec<PathBuf>) -> Self {
        Self {
            config_provider,
            default_identity_files,
        }
    }

    /// Creates a provider with the system set of default identity paths.
    #[cfg_attr(
        all(debug_assertions, feature = "trace_logs"),
        tracing::instrument(skip_all)
    )]
    #[must_use]
    pub fn with_default_paths(config_provider: SshConfigProvider) -> Self {
        Self::new(config_provider, default_identity_paths())
    }

    /// Returns the selected identity source for the given context.
    ///
    /// # Errors
    /// Returns a typed error if parsing/resolving `.ssh/config`
    /// fails.
    #[cfg_attr(
        feature = "trace_logs",
        tracing::instrument(skip_all, fields(target = %ctx.redacted_target()))
    )]
    pub fn resolve_identity_source(
        &self,
        ctx: &AuthContext,
    ) -> AuthResult<Option<SshIdentitySource>> {
        let resolved = self.config_provider.resolve(ctx)?;
        Ok(select_identity_source(
            &resolved,
            &self.default_identity_files,
        ))
    }
}

impl Default for SshIdentityFileProvider {
    fn default() -> Self {
        Self::with_default_paths(SshConfigProvider::default())
    }
}

impl GitAuthProvider for SshIdentityFileProvider {
    fn id(&self) -> &'static str {
        SSH_IDENTITY_PROVIDER_ID
    }

    fn supports(&self, ctx: &AuthContext, caps: &AuthCapabilities) -> bool {
        ctx.transport() == AuthTransport::Ssh && caps.allow_ssh()
    }

    fn load(&self, ctx: &AuthContext) -> AuthResult<Option<AuthMaterial>> {
        let resolved = self.config_provider.resolve(ctx)?;
        let Some(identity_source) = select_identity_source(&resolved, &self.default_identity_files)
        else {
            return Ok(None);
        };

        let principal = resolved.user().map(str::to_owned);
        let identity_path = identity_source.path().to_string_lossy().into_owned();
        let fingerprint = format!(
            "ssh-identity:{}:{}:{identity_path}",
            identity_source.as_str(),
            principal.as_deref().unwrap_or("<none>")
        );

        Ok(Some(AuthMaterial::new(
            self.id(),
            AuthMaterialKind::SshKey(SshCredentialSource::KeyFile),
            principal,
            Some(identity_path),
            fingerprint,
        )))
    }
}

fn select_identity_source(
    resolved: &super::ssh_config::SshConfigResolved,
    default_identity_files: &[PathBuf],
) -> Option<SshIdentitySource> {
    let mut candidates: Vec<SshIdentitySource> = Vec::new();

    for identity_file in resolved.identity_files() {
        if candidates
            .iter()
            .any(|candidate| candidate.path() == identity_file)
        {
            continue;
        }
        candidates.push(SshIdentitySource::ConfigIdentityFile(identity_file.clone()));
    }

    if !resolved.identities_only() {
        for default_identity_file in default_identity_files {
            if candidates
                .iter()
                .any(|candidate| candidate.path() == default_identity_file)
            {
                continue;
            }
            candidates.push(SshIdentitySource::DefaultIdentityFile(
                default_identity_file.clone(),
            ));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.path().exists())
}

fn default_identity_paths() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };

    ["id_ed25519", "id_ecdsa", "id_rsa"]
        .into_iter()
        .map(|file_name| home.join(".ssh").join(file_name))
        .collect()
}

